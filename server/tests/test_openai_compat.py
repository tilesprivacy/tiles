"""Tests for OpenAI-compatible /v1/models and /v1/chat/completions."""

from fastapi.testclient import TestClient

from server.api import app


def test_v1_models_empty():
    client = TestClient(app)
    r = client.get("/v1/models")
    assert r.status_code == 200
    body = r.json()
    assert body["object"] == "list"
    assert body["data"] == []


def test_health_endpoints():
    client = TestClient(app)
    for path in ("/health", "/v1/health"):
        r = client.get(path)
        assert r.status_code == 200
        b = r.json()
        assert b["status"] == "ok"
        assert b["tiles"] is True
        assert "model_loaded" in b


def test_models_load_requires_cache_path_env():
    client = TestClient(app)
    r = client.post("/models/load", json={"model": "x"})
    assert r.status_code == 200
    assert r.json()["success"] is False
    assert "TILES_MODEL_CACHE_PATH" in r.json()["error"]


def test_models_load_with_cache_path_env(mock_backend, monkeypatch):
    monkeypatch.setenv("TILES_MODEL_CACHE_PATH", "/tmp/mlx-cache")
    client = TestClient(app)
    r = client.post("/models/load", json={"model": "demo-model", "extra_args": []})
    assert r.status_code == 200
    assert r.json()["success"] is True
    mock_backend.get_or_load_model.assert_called_once_with("demo-model", "/tmp/mlx-cache")


def test_models_unload_stub():
    client = TestClient(app)
    r = client.post("/models/unload", json={"model": "x"})
    assert r.status_code == 200
    assert r.json()["success"] is False


def test_props_for_llamacpp_webui():
    client = TestClient(app)
    r = client.get("/props")
    assert r.status_code == 200
    body = r.json()
    assert body["role"] == "model"
    assert "default_generation_settings" in body
    assert body["modalities"]["vision"] is False


def test_v1_models_after_start(mock_backend):
    client = TestClient(app)
    r = client.post(
        "/start",
        json={
            "model": "demo-model",
            "memory_path": "/tmp/mem",
            "system_prompt": "You are a test assistant.",
            "model_cache_path": "/tmp/cache",
        },
    )
    assert r.status_code == 200
    mock_backend.get_or_load_model.assert_called_once()

    r2 = client.get("/v1/models")
    assert r2.status_code == 200
    data = r2.json()["data"]
    assert len(data) == 1
    assert data[0]["id"] == "demo-model"
    assert data[0]["object"] == "model"
    assert data[0]["owned_by"] == "tiles"
    assert data[0]["status"]["value"] == "loaded"
    assert "path" in data[0]


def test_chat_completions_openai_stream(mock_backend):
    client = TestClient(app)
    r = client.post(
        "/v1/chat/completions",
        json={
            "model": "demo-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": True,
        },
    )
    assert r.status_code == 200
    assert "event-stream" in r.headers.get("content-type", "")
    assert "ok" in r.text
    mock_backend.generate_chat_stream.assert_called_once()


def test_chat_completions_openai_non_stream(mock_backend):
    client = TestClient(app)
    r = client.post(
        "/v1/chat/completions",
        json={
            "model": "demo-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": False,
        },
    )
    assert r.status_code == 200
    body = r.json()
    assert body["object"] == "chat.completion"
    assert body["choices"][0]["message"]["content"] == "done"
    mock_backend.complete_openai_chat_completion.assert_called_once()


def test_chat_completions_memory_path_uses_legacy_append(mock_backend):
    """When `input` is present, behave like Tiles CLI memory mode (append tool result)."""
    client = TestClient(app)
    r = client.post(
        "/v1/chat/completions",
        json={
            "model": "demo-model",
            "input": "user turn",
            "chat_start": True,
            "python_code": "",
            "messages": [
                {"role": "assistant", "content": ""},
                {"role": "user", "content": "hi"},
            ],
            "stream": True,
        },
    )
    assert r.status_code == 200
    mock_backend.generate_chat_stream.assert_called_once()
    call_args = mock_backend.generate_chat_stream.call_args[0]
    messages_passed = call_args[0]
    assert any("result" in m.content for m in messages_passed)


def test_chat_completions_rejects_invalid_message_role():
    client = TestClient(app)
    r = client.post(
        "/v1/chat/completions",
        json={
            "model": "m",
            "messages": [{"role": "tool", "content": "x"}],
            "stream": False,
        },
    )
    assert r.status_code == 422
    body = r.json()
    assert "error" in body
    assert "message" in body["error"]


def test_chat_completions_400_when_no_model_and_none_loaded(mock_backend):
    client = TestClient(app)
    r = client.post(
        "/v1/chat/completions",
        json={"messages": [{"role": "user", "content": "x"}], "stream": False},
    )
    assert r.status_code == 400
    assert "POST /start" in r.json()["error"]["message"]


def test_chat_completions_uses_loaded_model_when_body_omits_model(mock_backend):
    """llama.cpp Web UI often omits model in single-model mode; use POST /start model id."""
    import server.api as api

    api._loaded_model_id = "from-start"
    try:
        client = TestClient(app)
        r = client.post(
            "/v1/chat/completions",
            json={
                "messages": [{"role": "user", "content": "hi"}],
                "stream": True,
            },
        )
        assert r.status_code == 200
        req = mock_backend.generate_chat_stream.call_args[0][1]
        assert req.model == "from-start"
    finally:
        api._loaded_model_id = None


def test_chat_completions_accepts_openai_content_array(mock_backend):
    """llama.cpp Web UI may send multimodal-shaped content arrays; we flatten to text."""
    client = TestClient(app)
    r = client.post(
        "/v1/chat/completions",
        json={
            "model": "demo-model",
            "messages": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": "hi"}],
                }
            ],
            "stream": True,
            "max_tokens": -1,
        },
    )
    assert r.status_code == 200
    mock_backend.generate_chat_stream.assert_called_once()
    msgs = mock_backend.generate_chat_stream.call_args[0][0]
    assert msgs[-1].content == "hi"


def test_responses_stream_uses_event_stream(mock_backend):
    client = TestClient(app)
    r = client.post(
        "/v1/responses",
        json={
            "model": "demo-model",
            "input": "hello",
            "stream": True,
        },
    )
    assert r.status_code == 200
    assert "event-stream" in r.headers.get("content-type", "")
