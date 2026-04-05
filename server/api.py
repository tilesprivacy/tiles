import asyncio
import json
import logging
import os
import time
from contextlib import asynccontextmanager
from typing import Any, Optional

from fastapi import FastAPI, HTTPException, Request
from fastapi.exceptions import RequestValidationError
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse, StreamingResponse

from . import runtime
from . import session_persist
from .config import MEMORY_PATH
from .mem_agent.engine import execute_sandboxed_code
from .mem_agent.utils import format_results
from .llamacpp_webui_compat import (
    openai_model_entry_with_llama_fields,
    tiles_props_for_webui,
)
from .schemas import (
    ChatCompletionRequest,
    ChatMessage,
    ResponsesRequest,
    RouterModelsLoadBody,
    StartRequest,
)

logger = logging.getLogger("app")
_current_model_path: Optional[str] = None
_default_max_tokens: Optional[int] = None  # Use dynamic model-aware limits by default
_memory_path = ""

_messages: list[ChatMessage] = []
_loaded_model_id: Optional[str] = None
_loaded_model_cache_path: Optional[str] = None
_model_loaded_at: Optional[int] = None

SSE_HEADERS = {
    "Cache-Control": "no-cache",
    "Connection": "keep-alive",
    "X-Accel-Buffering": "no",
}


def _apply_loaded_session(
    model: str,
    model_cache_path: str,
    memory_path: str,
    system_prompt: str,
) -> None:
    global _messages, _memory_path, _loaded_model_id, _loaded_model_cache_path, _model_loaded_at
    runtime.backend.get_or_load_model(model, model_cache_path)
    _loaded_model_id = model
    _loaded_model_cache_path = model_cache_path
    _memory_path = memory_path
    _messages = [ChatMessage(role="system", content=system_prompt)]
    _model_loaded_at = int(time.time())


def _env_bootstrap_session_data() -> session_persist.SessionData | None:
    m = os.environ.get("TILES_BOOTSTRAP_MODEL", "").strip()
    c = os.environ.get("TILES_BOOTSTRAP_MODEL_CACHE_PATH", "").strip()
    if not m or not c:
        return None
    mem = os.environ.get("TILES_BOOTSTRAP_MEMORY_PATH", "").strip() or MEMORY_PATH
    sysp = os.environ.get("TILES_BOOTSTRAP_SYSTEM_PROMPT", "").strip() or "You are a helpful assistant."
    return {
        "model": m,
        "model_cache_path": c,
        "memory_path": mem,
        "system_prompt": sysp,
    }


async def _restore_session_on_startup() -> None:
    if session_persist.skip_persist():
        return

    data = session_persist.load()
    if data:
        try:
            await asyncio.to_thread(
                _apply_loaded_session,
                data["model"],
                data["model_cache_path"],
                data["memory_path"],
                data["system_prompt"],
            )
            logger.info("Restored persisted session: model=%s", data["model"])
            return
        except Exception as e:
            logger.warning("Could not restore persisted session: %s", e)

    boot = _env_bootstrap_session_data()
    if boot:
        try:
            await asyncio.to_thread(
                _apply_loaded_session,
                boot["model"],
                boot["model_cache_path"],
                boot["memory_path"],
                boot["system_prompt"],
            )
            logger.info("Bootstrap model from env TILES_BOOTSTRAP_*: model=%s", boot["model"])
            session_persist.save(boot)
        except Exception as e:
            logger.warning("Could not bootstrap model from TILES_BOOTSTRAP_*: %s", e)


@asynccontextmanager
async def _lifespan(_app: FastAPI):
    await _restore_session_on_startup()
    yield


app = FastAPI(lifespan=_lifespan)


@app.exception_handler(RequestValidationError)
async def openai_validation_errors(request: Request, exc: RequestValidationError) -> JSONResponse:
    if request.url.path.startswith("/v1/"):
        parts = []
        for e in exc.errors():
            loc = "/".join(str(x) for x in e.get("loc", ()))
            parts.append(f"{loc}: {e.get('msg', '')}")
        msg = "; ".join(parts) if parts else "Invalid request"
        return JSONResponse(
            status_code=422,
            content={
                "error": {
                    "message": msg,
                    "type": "invalid_request_error",
                    "param": None,
                    "code": None,
                }
            },
        )
    return JSONResponse(status_code=422, content={"detail": exc.errors()})


@app.exception_handler(HTTPException)
async def openai_http_errors(request: Request, exc: HTTPException) -> JSONResponse:
    if request.url.path.startswith("/v1/"):
        detail: Any = exc.detail
        msg = detail if isinstance(detail, str) else json.dumps(detail)
        return JSONResponse(
            status_code=exc.status_code,
            content={
                "error": {
                    "message": msg,
                    "type": "api_error",
                    "code": None,
                }
            },
        )
    return JSONResponse(status_code=exc.status_code, content={"detail": exc.detail})


app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)


def _openai_compat_effective_messages(request: ChatCompletionRequest) -> list[ChatMessage]:
    """Stateless OpenAI chat: optional session system from /start + client messages."""
    if _messages and _messages[0].role == "system":
        return [_messages[0]] + list(request.messages)
    return list(request.messages)


@app.get("/ping")
async def ping():
    return {"message": "Badda-Bing Badda-Bang"}


@app.get("/health")
@app.get("/v1/health")
async def health():
    """Liveness and basic readiness (model slot may be empty)."""
    return {
        "status": "ok",
        "tiles": True,
        "model_loaded": _loaded_model_id is not None,
        "model": _loaded_model_id,
    }


@app.post("/models/load")
async def models_load(body: RouterModelsLoadBody):
    """Llama Web UI load hook: body is {\"model\": \"<id>\"}; MLX dir from TILES_MODEL_CACHE_PATH."""
    cache = (
        os.environ.get("TILES_MODEL_CACHE_PATH", "").strip()
        or os.environ.get("TILES_BOOTSTRAP_MODEL_CACHE_PATH", "").strip()
    )
    if not cache:
        return {
            "success": False,
            "error": "Set TILES_MODEL_CACHE_PATH to the MLX model directory (absolute path), or use POST /start.",
        }
    mid = body.model.strip()
    if not mid:
        return {"success": False, "error": "Missing model id in JSON body."}
    sysp = os.environ.get("TILES_BOOTSTRAP_SYSTEM_PROMPT", "").strip() or "You are a helpful assistant."
    try:
        _apply_loaded_session(mid, cache, MEMORY_PATH, sysp)
        session_persist.save(
            {
                "model": mid,
                "model_cache_path": cache,
                "memory_path": MEMORY_PATH,
                "system_prompt": sysp,
            }
        )
        return {"success": True}
    except Exception as e:
        logger.warning("POST /models/load failed: %s", e)
        return {"success": False, "error": str(e)}


@app.post("/models/unload")
async def models_unload_stub():
    return {
        "success": False,
        "error": "Tiles does not expose unload; restart the server to release the model.",
    }


@app.get("/props")
async def server_props():
    """Llama.cpp web UI expects this (see tools/server/webui PropsService)."""
    return tiles_props_for_webui(_loaded_model_cache_path)


@app.get("/v1/models")
async def list_models():
    """OpenAI-compatible model list (models appear after POST /start)."""
    data: list[dict] = []
    if _loaded_model_id is not None:
        data.append(
            openai_model_entry_with_llama_fields(
                _loaded_model_id,
                _model_loaded_at or int(time.time()),
                model_cache_path=_loaded_model_cache_path,
            )
        )
    return {"object": "list", "data": data}


@app.post("/start")
async def start_model(request: StartRequest):
    """Load the model and start the agent"""
    logger.info("%s", runtime.backend)
    _apply_loaded_session(
        request.model,
        request.model_cache_path,
        request.memory_path,
        request.system_prompt,
    )
    session_persist.save(
        {
            "model": request.model,
            "model_cache_path": request.model_cache_path,
            "memory_path": request.memory_path,
            "system_prompt": request.system_prompt,
        }
    )
    return {"message": "Model loaded"}


@app.post("/v1/chat/completions")
async def create_chat_completion(request: ChatCompletionRequest):
    """Create a chat completion (OpenAI SSE when stream=true; memory-agent when input is set)."""
    global _messages, _memory_path
    try:
        resolved_model = (request.model or "").strip() or _loaded_model_id
        if not resolved_model:
            raise HTTPException(
                status_code=400,
                detail=(
                    "No model loaded: send JSON 'model', or POST /start, or set TILES_BOOTSTRAP_* for auto-load."
                ),
            )
        request = request.model_copy(update={"model": resolved_model})

        memory_path = request.input is not None

        if memory_path:
            result = ({}, "")
            if request.python_code:
                result = execute_sandboxed_code(
                    code=request.python_code,
                    allowed_path=_memory_path,
                    import_module="server.mem_agent.tools",
                )

            _messages.append(
                ChatMessage(role="user", content=format_results(result[0], result[1]))
            )

            if request.stream:
                return StreamingResponse(
                    runtime.backend.generate_chat_stream(_messages, request),
                    media_type="text/event-stream",
                    headers=dict(SSE_HEADERS),
                )
            return JSONResponse(
                runtime.backend.complete_openai_chat_completion(_messages, request)
            )

        eff = _openai_compat_effective_messages(request)
        if request.stream:
            return StreamingResponse(
                runtime.backend.generate_chat_stream(eff, request),
                media_type="text/event-stream",
                headers=dict(SSE_HEADERS),
            )
        return JSONResponse(runtime.backend.complete_openai_chat_completion(eff, request))
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e)) from e


@app.post("/v1/responses")
async def create_chat_response(request: ResponsesRequest):
    """
    Create a response with openResponses format
    """

    if request.stream:
        return StreamingResponse(
            runtime.backend.generate_response_chat_stream(request),
            media_type="text/event-stream",
            headers=dict(SSE_HEADERS),
        )
    else:
        return await runtime.backend.generate_response_chat(request)
