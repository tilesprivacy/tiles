"""Warm llama-server's prompt cache with the agent's system prompt.

Every fresh conversation starts with the same few thousand tokens: Pi's system
prompt and its tool schemas. llama-server caches a prompt's KV and reuses the
longest common prefix on the next request, so replaying that prefix once,
right after the model loads, means the first real message only pays for its
own tokens.

The prefix is whatever Pi last sent, saved per model, so it matches token for
token without this module knowing how Pi builds it. Before the first ever
request there is nothing saved and warming only loads the model.
"""

from __future__ import annotations

import hashlib
import json
import logging
import os
from pathlib import Path
from typing import Any

import httpx

from .client import chat_completions_url

logger = logging.getLogger("app")

# the user turn the replayed prefix ends in. Its tokens are the only part of
# the warm-up a real request does not share
_WARM_TURN = {"role": "user", "content": "hi"}


def _cache_path() -> Path:
    dev = Path.cwd() / ".tiles_dev" / "tiles"
    if dev.is_dir():
        return dev / "cache" / "prefill.json"
    xdg = os.environ.get("XDG_DATA_HOME")
    data_home = Path(xdg) if xdg else Path.home() / ".local" / "share"
    return data_home / "tiles" / "cache" / "prefill.json"


def _read() -> dict[str, Any]:
    try:
        return json.loads(_cache_path().read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}


def prefix_of(body: dict[str, Any]) -> dict[str, Any] | None:
    """The part of a chat request every conversation shares, or None."""
    messages = body.get("messages") or []
    leading: list[dict[str, Any]] = []
    for message in messages:
        if message.get("role") not in ("system", "developer"):
            break
        leading.append(message)
    if not leading:
        return None
    prefix: dict[str, Any] = {"messages": leading}
    for key in ("tools", "chat_template_kwargs"):
        if body.get(key):
            prefix[key] = body[key]
    return prefix


def remember(model: str, body: dict[str, Any]) -> None:
    """Save the request's shared prefix for `model`, writing only on change."""
    prefix = prefix_of(body)
    if prefix is None:
        return
    digest = hashlib.sha256(json.dumps(prefix, sort_keys=True).encode()).hexdigest()
    saved = _read()
    if saved.get(model, {}).get("digest") == digest:
        return
    saved[model] = {"digest": digest, "prefix": prefix}
    path = _cache_path()
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp = path.with_suffix(".tmp")
        tmp.write_text(json.dumps(saved), encoding="utf-8")
        tmp.replace(path)
    except OSError as exc:
        logger.warning("Could not save the prefill prefix: %s", exc)


def warm_body(model: str) -> dict[str, Any] | None:
    """A one-token request that leaves the saved prefix in the prompt cache."""
    prefix = _read().get(model, {}).get("prefix")
    if not prefix:
        return None
    body: dict[str, Any] = {
        "model": model,
        "messages": [*prefix["messages"], _WARM_TURN],
        "stream": False,
        "max_tokens": 1,
        "cache_prompt": True,
    }
    if prefix.get("tools"):
        body["tools"] = prefix["tools"]
        body["tool_choice"] = "auto"
    if prefix.get("chat_template_kwargs"):
        body["chat_template_kwargs"] = prefix["chat_template_kwargs"]
    return body


async def warm(model: str) -> bool:
    """Replay the saved prefix. False when there was nothing to replay."""
    body = warm_body(model)
    if body is None:
        logger.info("No saved prefix for %s yet, only the model was loaded", model)
        return False
    async with httpx.AsyncClient(timeout=httpx.Timeout(600.0, connect=5.0)) as client:
        response = await client.post(chat_completions_url(), json=body)
        response.raise_for_status()
        usage = response.json().get("usage", {})
    logger.info("Prefilled %s tokens of the agent prompt for %s", usage.get("prompt_tokens"), model)
    return True
