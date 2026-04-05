"""Pytest fixtures for the Tiles inference server."""

import os
from unittest.mock import MagicMock

os.environ.setdefault("TILES_SKIP_SESSION_PERSIST", "1")

import pytest

import server.api as api
import server.runtime as runtime


@pytest.fixture(autouse=True)
def reset_api_state():
    """Isolate tests that mutate /start and chat session state."""
    api._current_model_path = None
    api._default_max_tokens = None
    api._loaded_model_id = None
    api._loaded_model_cache_path = None
    api._model_loaded_at = None
    api._messages = []
    api._memory_path = ""
    yield
    api._current_model_path = None
    api._default_max_tokens = None
    api._loaded_model_id = None
    api._loaded_model_cache_path = None
    api._model_loaded_at = None
    api._messages = []
    api._memory_path = ""


@pytest.fixture
def mock_backend(monkeypatch):
    """Provide a backend with async streaming and sync completion stubs."""
    b = MagicMock()

    async def fake_stream(*_a, **_k):
        yield 'data: {"id":"c","object":"chat.completion.chunk","choices":[{"delta":{"content":"ok"}}]}\n\n'
        yield "data: [DONE]\n\n"

    b.generate_chat_stream = MagicMock(side_effect=fake_stream)
    b.complete_openai_chat_completion = MagicMock(
        return_value={
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "created": 0,
            "model": "test-model",
            "choices": [
                {
                    "index": 0,
                    "message": {"role": "assistant", "content": "done"},
                    "finish_reason": "stop",
                }
            ],
        }
    )
    b.get_or_load_model = MagicMock()

    async def fake_resp_stream(*_a, **_k):
        yield 'data: {"object":"response.chunk"}\n\n'
        yield "data: [DONE]\n\n"

    b.generate_response_chat_stream = fake_resp_stream
    b.generate_response_chat = MagicMock(return_value={"id": "r1", "status": "completed"})

    monkeypatch.setattr(runtime, "backend", b)
    return b
