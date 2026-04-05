"""Persist last successful model load for restore on next server process start."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, TypedDict

_SESSION_ENV = "TILES_SKIP_SESSION_PERSIST"
_FILE_ENV = "TILES_SESSION_FILE"


class SessionData(TypedDict):
    model: str
    model_cache_path: str
    memory_path: str
    system_prompt: str


def _path() -> Path:
    raw = os.environ.get(_FILE_ENV)
    if raw:
        return Path(raw)
    return Path.home() / ".tiles" / "server_session.json"


def skip_persist() -> bool:
    return os.environ.get(_SESSION_ENV) == "1"


def save(data: SessionData) -> None:
    if skip_persist():
        return
    p = _path()
    p.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(dict(data), separators=(",", ":"), ensure_ascii=False)
    tmp = p.parent / f"{p.name}.tmp"
    tmp.write_text(text, encoding="utf-8")
    os.replace(tmp, p)


def load() -> SessionData | None:
    if skip_persist():
        return None
    p = _path()
    if not p.is_file():
        return None
    try:
        raw: Any = json.loads(p.read_text(encoding="utf-8"))
        for k in ("model", "model_cache_path", "memory_path", "system_prompt"):
            if k not in raw or not isinstance(raw[k], str):
                return None
        return SessionData(
            model=raw["model"],
            model_cache_path=raw["model_cache_path"],
            memory_path=raw["memory_path"],
            system_prompt=raw["system_prompt"],
        )
    except (OSError, json.JSONDecodeError, TypeError, KeyError):
        return None
