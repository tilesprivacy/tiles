import logging
import sys
from typing import Optional

from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import StreamingResponse, JSONResponse
from fastapi.exceptions import RequestValidationError
from pydantic import BaseModel, Field

from . import runtime
from .mem_agent.engine import execute_sandboxed_code
from .mem_agent.utils import (
    create_memory_if_not_exists,
    format_results,
)
from .schemas import (
    ChatCompletionRequest,
    ChatMessage,
    ResponsesRequest,
    StartRequest,
    downloadRequest,
)

logger = logging.getLogger("app")
_current_model_path: Optional[str] = None
_default_max_tokens: Optional[int] = None  # Use dynamic model-aware limits by default
_memory_path = ""

_messages: list[ChatMessage] = []


app = FastAPI()


@app.get("/ping")
async def ping():
    return {"message": "Welcome to the jungle"}


@app.post("/start")
async def start_model(request: StartRequest):
    """Load the model and start the agent"""
    global _messages, _runner, _memory_path
    print(f"CACHE PATH{request.model_cache_path}")

    _messages = [ChatMessage(role="system", content=request.system_prompt)]
    _memory_path = request.memory_path
    logger.info(f"{runtime.backend}")
    runtime.backend.get_or_load_model(request.model, request.model_cache_path)
    return {"message": "Model loaded"}


@app.post("/v1/chat/completions")
async def create_chat_completion(request: ChatCompletionRequest):
    """Create a chat completion."""
    global _messages, _memory_path
    try:
        if request.stream:
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

            # Streaming response
            return StreamingResponse(
                runtime.backend.generate_chat_stream(_messages, request),
                media_type="text/plain",
                headers={"Cache-Control": "no-cache"},
            )
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.exception_handler(HTTPException)
async def validation_exception_handler(request: Request, exc: HTTPException):
    return JSONResponse(
        status_code=exc.status_code,
        content={"detail": exc.detail},
    )


@app.middleware("http")
async def catch_all(request, call_next):
    try:
        return await call_next(request)
    except Exception as e:
        print("UNCAUGHT:", repr(e))
        raise


@app.post("/v1/responses")
async def create_chat_response(request: ResponsesRequest):
    """
    Create a response with openResponses format
    """

    try:
        ResponsesRequest.model_validate(request)
    except Exception as e:
        print(e)

    print(f"REQUEST => {request}")

    if request.stream:
        return StreamingResponse(
            runtime.backend.generate_response_chat_stream(request),
            media_type="text/plain",
            headers={"Cache-Control": "no-cache", "Content-Type": "text/event-stream"},
        )
    else:
        return await runtime.backend.generate_response_chat(request)
