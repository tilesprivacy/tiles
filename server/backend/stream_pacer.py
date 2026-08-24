# MTP emits the tokens accepted in one verify cycle back-to-back, so text
# arrives in bursts and rendering looks bad. pace() re-times text deltas
# into a steady drip without slowing generation.

# Control events (tool calls, etc.) always
# flush the buffer first, so ordering is preserved and completion time is
# unchanged.
#
# TILES_STREAM_PACING sets the tick in milliseconds (default 30).
# 0 / off / false / no disables pacing entirely.

from __future__ import annotations

import asyncio
import json
import math
import os
from collections.abc import AsyncGenerator
from typing import Any

DEFAULT_TICK_S = 0.03
# bursts drain over almost 5 ticks, so slice size adapts to the buffer size
_DRAIN_TICKS = 5

_TEXT_DELTA_EVENTS = frozenset(
    {"response.output_text.delta", "response.reasoning_summary_text.delta"}
)

_DONE = object()  # upstream finished
_TICK = object()  # ticker heartbeat


def _tick_from_env() -> float | None:
    # tick in seconds, or None when pacing is off
    raw = (os.environ.get("TILES_STREAM_PACING") or "").strip().lower()
    if not raw:
        return DEFAULT_TICK_S
    if raw in ("0", "false", "off", "no"):
        return None
    try:
        ms = float(raw)
    except ValueError:
        return DEFAULT_TICK_S
    return None if ms <= 0 else ms / 1000.0


def _parse_events(chunk: str) -> list[dict[str, Any]]:
    # one upstream SSE string -> [{name, data, payload, raw}] records
    events: list[dict[str, Any]] = []
    for block in chunk.split("\n\n"):
        if not block.strip():
            continue
        name: str | None = None
        data: str | None = None
        for line in block.splitlines():
            if line.startswith("event:"):
                name = line.split(":", 1)[1].strip()
            elif line.startswith("data:"):
                data = line.split(":", 1)[1].strip()
        if name is None and data is None:
            continue
        payload: Any = None
        if data is not None and data.startswith("{"):
            try:
                payload = json.loads(data)
            except ValueError:
                payload = None
        events.append({"name": name, "data": data, "payload": payload, "raw": block})
    return events


def _format_event(name: str | None, payload: Any, data: str | None) -> str:
    if payload is not None:
        return f"event: {name}\ndata: {json.dumps(payload)}\n\n"
    if name is not None:
        return f"event: {name}\ndata: {data}\n\n"
    return f"data: {data}\n\n"


class _Pacer:
    # buffers text deltas and re-emits them as per-tick slices

    def __init__(self):
        self.pending: list[dict[str, Any]] = []
        self.seq: int | None = None

    def _next_seq(self, first_upstream: Any = None) -> int:
        # count from the first upstream sequence number we see
        if self.seq is None:
            start = 1 if first_upstream is None else int(first_upstream)
            self.seq = start - 1
        self.seq += 1
        return self.seq

    def buffer_delta(self, name: str, payload: dict[str, Any]) -> None:
        text = payload.get("delta")
        if not text:
            return
        last = self.pending[-1] if self.pending else None
        if last and last["event"] == name and last["item_id"] == payload.get("item_id"):
            # merge with the previous delta of the same item
            last["text"] += text
            return
        self.pending.append(
            {
                "event": name,
                "item_id": payload.get("item_id"),
                "output_index": payload.get("output_index", 0),
                "text": text,
                "content_index": int(payload.get("content_index", 0)),
            }
        )

    def _emit(self, record: dict[str, Any], piece: str) -> str:
        payload = {
            "type": record["event"],
            "sequence_number": self._next_seq(),
            "output_index": record["output_index"],
            "item_id": record["item_id"],
            "delta": piece,
            "content_index": record["content_index"],
        }
        record["content_index"] += 1
        return _format_event(record["event"], payload, None)

    def take_slice(self) -> str | None:
        # release one tick of text from the head of the buffer
        if not self.pending:
            return None
        record = self.pending[0]
        size = max(1, math.ceil(len(record["text"]) / _DRAIN_TICKS))
        piece = record["text"][:size]
        record["text"] = record["text"][size:]
        if not record["text"]:
            self.pending.pop(0)
        return self._emit(record, piece)

    def flush_all(self) -> list[str]:
        # emit everything buffered, whole
        out: list[str] = []
        while self.pending:
            record = self.pending.pop(0)
            out.append(self._emit(record, record["text"]))
        return out

    def control(self, event: dict[str, Any]) -> str:
        # pass a non-delta event through, rewriting its sequence number;
        # events without one pass through byte-identical
        payload = event["payload"]
        if isinstance(payload, dict) and "sequence_number" in payload:
            payload = {
                **payload,
                "sequence_number": self._next_seq(payload["sequence_number"]),
            }
            return _format_event(event["name"], payload, None)
        return f"{event['raw']}\n\n"


async def pace(upstream: AsyncGenerator[str, None]) -> AsyncGenerator[str, None]:
    # re-time an SSE stream into smooth per-tick text slices
    tick = _tick_from_env()
    if tick is None:
        async for chunk in upstream:
            yield chunk
        return

    pacer = _Pacer()
    queue: asyncio.Queue[Any] = asyncio.Queue()

    async def pump() -> None:
        # read upstream at full speed so the stream is never slowed
        try:
            async for chunk in upstream:
                await queue.put(chunk)
        except (Exception, asyncio.CancelledError) as exc:
            await queue.put(exc)
        else:
            await queue.put(_DONE)

    async def ticker() -> None:
        # one heartbeat per tick drives slice emission
        while True:
            await asyncio.sleep(tick)
            await queue.put(_TICK)

    tasks = [asyncio.create_task(pump()), asyncio.create_task(ticker())]
    try:
        while True:
            item = await queue.get()

            if item is _TICK:
                # the only emission site: one slice per tick
                if pacer.pending:
                    piece = pacer.take_slice()
                    if piece is not None:
                        yield piece
            elif item is _DONE:
                break
            elif isinstance(item, BaseException):
                for piece in pacer.flush_all():
                    yield piece
                raise item
            else:
                for event in _parse_events(item):
                    if (
                        event["name"] in _TEXT_DELTA_EVENTS
                        and isinstance(event["payload"], dict)
                    ):
                        pacer.buffer_delta(event["name"], event["payload"])
                        continue
                    for piece in pacer.flush_all():
                        yield piece
                    yield pacer.control(event)

        for piece in pacer.flush_all():
            yield piece
    finally:
        # don't leak tasks on client disconnect or error
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
