import json

from server.backend.llama_server import prefill

SYSTEM = {"role": "system", "content": "You are Tiles."}
TOOLS = [{"type": "function", "function": {"name": "read", "parameters": {}}}]


def _body(user: str) -> dict:
    return {
        "model": "m",
        "messages": [SYSTEM, {"role": "user", "content": user}],
        "tools": TOOLS,
        "chat_template_kwargs": {"enable_thinking": False},
        "temperature": 0.7,
    }


def _isolate(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("XDG_DATA_HOME", str(tmp_path / "xdg"))
    return tmp_path / "xdg" / "tiles" / "cache" / "prefill.json"


def test_prefix_keeps_only_what_every_conversation_shares():
    prefix = prefill.prefix_of(_body("what is the weather"))
    assert prefix == {
        "messages": [SYSTEM],
        "tools": TOOLS,
        "chat_template_kwargs": {"enable_thinking": False},
    }


def test_no_system_prompt_means_nothing_to_warm():
    assert prefill.prefix_of({"messages": [{"role": "user", "content": "hi"}]}) is None


def test_the_warm_request_replays_the_prefix_for_one_token(tmp_path, monkeypatch):
    _isolate(tmp_path, monkeypatch)
    prefill.remember("m", _body("first question"))

    body = prefill.warm_body("m")
    assert body["messages"][:-1] == [SYSTEM]
    assert body["messages"][-1]["role"] == "user"
    assert body["tools"] == TOOLS
    assert body["chat_template_kwargs"] == {"enable_thinking": False}
    assert body["max_tokens"] == 1
    assert body["cache_prompt"] is True


def test_prefixes_are_kept_per_model_and_rewritten_only_on_change(tmp_path, monkeypatch):
    path = _isolate(tmp_path, monkeypatch)
    prefill.remember("a", _body("one"))
    first_write = path.stat().st_mtime_ns

    # a different question is the same prefix: no write
    prefill.remember("a", _body("two"))
    assert path.stat().st_mtime_ns == first_write

    prefill.remember("b", {**_body("x"), "tools": []})
    saved = json.loads(path.read_text())
    assert set(saved) == {"a", "b"}
    assert "tools" not in saved["b"]["prefix"]


def test_nothing_saved_means_no_warm_request(tmp_path, monkeypatch):
    _isolate(tmp_path, monkeypatch)
    assert prefill.warm_body("never-seen") is None
