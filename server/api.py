import logging
import sys
from typing import Optional

from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import StreamingResponse, JSONResponse
from fastapi.exceptions import RequestValidationError
from openai_harmony import Role
from openresponses_types import InputTextContentParam
from pydantic import BaseModel, Field, ValidationError

from . import runtime
from .schemas import (
    CUserMessageItemParam,
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


# Can be used to debug the streaming responses
async def passthrough_log_raw(reader):
    async for chunk in reader:
        logger.info("stream chunk: %r\n", chunk)  # logs raw bytes/str repr
        yield chunk


@app.post("/v1/responses")
async def create_chat_response(request: ResponsesRequest):
    """
    Create a response stream/non-stream with openResponses format
    """

    if request.stream:
        return StreamingResponse(
            runtime.backend.generate_response_chat_stream(request),
            headers={"Cache-Control": "no-cache", "Content-Type": "text/event-stream"},
        )
    return await runtime.backend.generate_response_chat(request)
