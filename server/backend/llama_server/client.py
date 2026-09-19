"""HTTP client for llama-server OpenAI-compatible chat completions."""

from __future__ import annotations

import json
import os
from collections.abc import AsyncIterator
from typing import Any

import httpx

from ...config import LLAMA_SERVER_HOST, LLAMA_SERVER_PORT


def chat_completions_url() -> str:
    return f"http://{LLAMA_SERVER_HOST}:{LLAMA_SERVER_PORT}/v1/chat/completions"


def _stream_timeout() -> httpx.Timeout:
    DEFAULT_READ_TIMEOUT = 600.0
    read_timeout = DEFAULT_READ_TIMEOUT
    raw = os.environ.get("TILES_LLAMA_STREAM_READ_TIMEOUT")
    if raw is not None:
        try:
            parsed = float(raw)
        except (TypeError, ValueError):
            parsed = 0.0
        if parsed > 0:
            read_timeout = parsed
    return httpx.Timeout(connect=5.0, read=read_timeout, write=30.0, pool=5.0)


def llama_error_message(status_code: int, body_text: str) -> str:
    """llama-server's own reason for a refusal, not httpx's generic wrapper.

    httpx's raise_for_status message ("Client error '400 Bad Request' for url
    ... For more information check: https://developer.mozilla.org/...") names
    neither the model nor the cause. llama-server's body does: for example a
    tool schema whose grammar fails to compile. Surface that instead.
    """
    try:
        parsed = json.loads(body_text)
        message = parsed.get("error", {}).get("message") or body_text
    except (ValueError, AttributeError):
        message = body_text
    message = (message or "").strip() or "no detail in the response"
    return f"llama-server rejected the request ({status_code}): {message}"


async def stream_chat_completions(body: dict[str, Any]) -> AsyncIterator[dict[str, Any]]:
    async with httpx.AsyncClient(timeout=_stream_timeout()) as client:
        async with client.stream(
            "POST",
            chat_completions_url(),
            json=body,
            headers={"Accept": "text/event-stream"},
        ) as response:
            if response.status_code >= 400:
                raw = await response.aread()
                raise RuntimeError(
                    llama_error_message(response.status_code, raw.decode(errors="replace"))
                )
            response.raise_for_status()
            async for line in response.aiter_lines():
                if not line or not line.startswith("data: "):
                    continue
                payload = line.removeprefix("data: ").strip()
                if payload == "[DONE]":
                    return
                yield json.loads(payload)