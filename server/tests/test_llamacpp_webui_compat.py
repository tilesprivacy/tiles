"""Tests for llama.cpp Web UI helper helpers."""

import json
from pathlib import Path

from server.llamacpp_webui_compat import _read_n_ctx


def test_read_n_ctx_non_object_json_returns_default(tmp_path: Path):
    p = tmp_path / "config.json"
    p.write_text(json.dumps([1, 2, 3]), encoding="utf-8")
    assert _read_n_ctx(str(tmp_path)) == 8192


def test_read_n_ctx_reads_context_length(tmp_path: Path):
    p = tmp_path / "config.json"
    p.write_text(json.dumps({"context_length": 4096}), encoding="utf-8")
    assert _read_n_ctx(str(tmp_path)) == 4096
