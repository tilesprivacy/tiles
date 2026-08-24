import asyncio
import json

import pytest

from server.backend.stream_pacer import pace


def _sse(name: str, payload: dict, seq: int) -> str:
    event = {"type": name, "sequence_number": seq}
    event.update(payload)
    return f"event: {name}\ndata: {json.dumps(event)}\n\n"


def _parse_sse(events: list[str]) -> list[tuple[str, dict]]:
    parsed = []
    for chunk in events:
        for raw_event in chunk.split("\n\n"):
            name = None
            data = None
            for line in raw_event.splitlines():
                if line.startswith("event:"):
                    name = line.split(":", 1)[1].strip()
                elif line.startswith("data:"):
                    raw = line.split(":", 1)[1].strip()
                    if raw != "[DONE]" and raw.startswith("{"):
                        data = json.loads(raw)
            if name is not None:
                parsed.append((name, data or {}))
    return parsed


def _sequence_numbers(events: list[str]) -> list[int]:
    return [d["sequence_number"] for _, d in _parse_sse(events) if "sequence_number" in d]


async def _collect(gen) -> list[str]:
    return [chunk async for chunk in gen]


def test_pacing_disabled_passes_stream_through_untouched(monkeypatch):
    monkeypatch.setenv("TILES_STREAM_PACING", "0")

    chunks = [
        _sse("response.created", {"response": {"id": "r1"}}, 1),
        _sse(
            "response.output_text.delta",
            {"output_index": 0, "item_id": "msg_1", "delta": "Hello ", "content_index": 0},
            2,
        ),
        "data: [DONE]\n\n",
    ]

    async def upstream():
        for chunk in chunks:
            yield chunk

    assert asyncio.run(_collect(pace(upstream()))) == chunks


def test_burst_is_sliced_into_smooth_drip(monkeypatch):
    """A burst of text deltas followed by a quiet gap (one MTP verify cycle)
    must be re-emitted as several smaller deltas instead of one lump, with
    no text lost and contiguous sequence numbers.
    """
    monkeypatch.setenv("TILES_STREAM_PACING", "1")

    pieces = ["Hel", "lo ", "wor", "ld", "!"]

    async def upstream():
        for i, piece in enumerate(pieces):
            yield _sse(
                "response.output_text.delta",
                {"output_index": 0, "item_id": "msg_1", "delta": piece, "content_index": i},
                2 + i,
            )
        # Quiet period like the gap between verify cycles: the buffer must
        # drip out during this window, not wait for the stream to end.
        await asyncio.sleep(0.05)
        yield _sse("response.completed", {"response": {"id": "r1"}}, 8)
        yield "data: [DONE]\n\n"

    out = asyncio.run(_collect(pace(upstream())))
    events = _parse_sse(out)

    deltas = [d for n, d in events if n == "response.output_text.delta"]
    assert "".join(d["delta"] for d in deltas) == "Hello world!"
    assert len(deltas) >= 3, f"burst was not sliced, got {len(deltas)} delta event(s)"

    seqs = _sequence_numbers(out)
    assert seqs == sorted(seqs), "sequence numbers must not decrease"
    assert len(seqs) == len(set(seqs)), "sequence numbers must not repeat"

    names = [n for n, _ in events]
    assert names[-1] == "response.completed"
    assert out[-1] == "data: [DONE]\n\n"


def test_flush_before_control_event_preserves_order(monkeypatch):
    """Text buffered before a control event (item done, completed, ...) must
    be fully emitted before that control event — control events are never
    delayed by pacing, and nothing is dropped.
    """
    monkeypatch.delenv("TILES_STREAM_PACING", raising=False)

    async def upstream():
        for i, piece in enumerate(["Hel", "lo ", "world"]):
            yield _sse(
                "response.output_text.delta",
                {"output_index": 0, "item_id": "msg_1", "delta": piece, "content_index": i},
                2 + i,
            )
        yield _sse(
            "response.output_item.done",
            {"output_index": 0, "item": {"type": "message", "id": "msg_1"}},
            5,
        )
        yield "data: [DONE]\n\n"

    out = asyncio.run(_collect(pace(upstream())))
    events = _parse_sse(out)
    names = [n for n, _ in events]

    done_at = names.index("response.output_item.done")
    deltas = [d for n, d in events[:done_at] if n == "response.output_text.delta"]
    assert "".join(d["delta"] for d in deltas) == "Hello world"

    seqs = _sequence_numbers(out)
    assert seqs == sorted(seqs)
    assert len(seqs) == len(set(seqs))


def test_mixed_stream_loses_no_data(monkeypatch):
    """Reasoning, message text, a tool call and completion all flow through
    the pacer with their payloads intact and in order.
    """
    monkeypatch.setenv("TILES_STREAM_PACING", "1")

    async def upstream():
        yield _sse("response.created", {"response": {"id": "r1"}}, 1)
        for i, piece in enumerate(["think", "ing"]):
            yield _sse(
                "response.reasoning_summary_text.delta",
                {"output_index": 0, "item_id": "reasoning_1", "delta": piece, "content_index": i},
                2 + i,
            )
        await asyncio.sleep(0.03)
        yield _sse(
            "response.output_item.added",
            {
                "output_index": 1,
                "item": {"type": "function_call", "id": "tc_1", "name": "read", "call_id": "call_1"},
            },
            4,
        )
        for i, piece in enumerate(["ans", "wer"]):
            yield _sse(
                "response.output_text.delta",
                {"output_index": 2, "item_id": "msg_1", "delta": piece, "content_index": i},
                5 + i,
            )
        await asyncio.sleep(0.03)
        yield _sse("response.completed", {"response": {"id": "r1"}}, 8)
        yield "data: [DONE]\n\n"

    out = asyncio.run(_collect(pace(upstream())))
    events = _parse_sse(out)
    names = [n for n, _ in events]

    reasoning = "".join(
        d["delta"] for n, d in events if n == "response.reasoning_summary_text.delta"
    )
    answer = "".join(
        d["delta"] for n, d in events if n == "response.output_text.delta"
    )
    assert reasoning == "thinking"
    assert answer == "answer"

    # Relative order of control events is preserved.
    assert names.index("response.created") < names.index("response.output_item.added")
    assert names.index("response.output_item.added") < names.index("response.completed")
    assert out[-1] == "data: [DONE]\n\n"

    seqs = _sequence_numbers(out)
    assert seqs == sorted(seqs)
    assert len(seqs) == len(set(seqs))


def test_upstream_exception_flushes_buffer_then_propagates(monkeypatch):
    monkeypatch.setenv("TILES_STREAM_PACING", "1")

    async def upstream():
        yield _sse(
            "response.output_text.delta",
            {"output_index": 0, "item_id": "msg_1", "delta": "partial ", "content_index": 0},
            2,
        )
        await asyncio.sleep(0.01)
        raise RuntimeError("boom")

    out: list[str] = []

    async def run():
        async for chunk in pace(upstream()):
            out.append(chunk)

    with pytest.raises(RuntimeError, match="boom"):
        asyncio.run(run())

    deltas = [d for n, d in _parse_sse(out) if n == "response.output_text.delta"]
    assert "".join(d["delta"] for d in deltas) == "partial "


def test_closing_consumer_stops_the_pump(monkeypatch):
    """Abandoning the paced stream (client disconnect) must cancel the
    background pump so the upstream generator is not iterated forever.
    """
    monkeypatch.setenv("TILES_STREAM_PACING", "1")
    pulled = {"count": 0}

    async def upstream():
        while True:
            await asyncio.sleep(0.001)
            pulled["count"] += 1
            yield _sse(
                "response.output_text.delta",
                {"output_index": 0, "item_id": "msg_1", "delta": "x", "content_index": 0},
                pulled["count"],
            )

    async def run():
        gen = pace(upstream())
        await gen.__anext__()
        await gen.aclose()
        before = pulled["count"]
        await asyncio.sleep(0.05)
        assert pulled["count"] == before, "pump kept pulling after consumer closed"

    asyncio.run(run())


def test_empty_stream_emits_nothing(monkeypatch):
    monkeypatch.setenv("TILES_STREAM_PACING", "1")

    async def upstream():
        return
        yield  # pragma: no cover - make this an async generator

    assert asyncio.run(_collect(pace(upstream()))) == []


def test_custom_tick_from_env(monkeypatch):
    monkeypatch.setenv("TILES_STREAM_PACING", "2")

    async def upstream():
        yield _sse(
            "response.output_text.delta",
            {"output_index": 0, "item_id": "msg_1", "delta": "hi", "content_index": 0},
            1,
        )
        await asyncio.sleep(0.05)
        yield "data: [DONE]\n\n"

    out = asyncio.run(_collect(pace(upstream())))
    deltas = [d for n, d in _parse_sse(out) if n == "response.output_text.delta"]
    assert "".join(d["delta"] for d in deltas) == "hi"


def test_invalid_tick_falls_back_to_default(monkeypatch):
    from server.backend.stream_pacer import _tick_from_env

    monkeypatch.setenv("TILES_STREAM_PACING", "not-a-number")
    assert _tick_from_env() == 0.03

    monkeypatch.setenv("TILES_STREAM_PACING", "off")
    assert _tick_from_env() is None

    monkeypatch.setenv("TILES_STREAM_PACING", "-5")
    assert _tick_from_env() is None

    monkeypatch.delenv("TILES_STREAM_PACING", raising=False)
    assert _tick_from_env() == 0.03


def test_continuous_arrival_still_drains_on_the_clock(monkeypatch):
    # chunks arrive faster than the tick, so the buffer accumulates while
    # the stream is still flowing; ticks must slice text out during
    # arrival instead of waiting for the stream to end
    monkeypatch.setenv("TILES_STREAM_PACING", "20")

    async def upstream():
        for i in range(20):
            yield _sse(
                "response.output_text.delta",
                {"output_index": 0, "item_id": "msg_1", "delta": "ab", "content_index": i},
                i + 1,
            )
            await asyncio.sleep(0.002)  # 10 chunks per tick
        await asyncio.sleep(0.15)  # quiet window to finish draining
        yield "data: [DONE]\n\n"

    out = asyncio.run(_collect(pace(upstream())))
    deltas = [d for n, d in _parse_sse(out) if n == "response.output_text.delta"]
    assert "".join(d["delta"] for d in deltas) == "ab" * 20
    assert len(deltas) >= 3  # sliced, not one lump
    assert len(deltas) < 20  # merged, not one delta per chunk

    seqs = _sequence_numbers(out)
    assert seqs == sorted(seqs)
    assert len(seqs) == len(set(seqs))


def test_buffered_items_drain_in_order(monkeypatch):
    # deltas for two different items can sit in the buffer at once;
    # all of item A's slices must come out before item B's
    monkeypatch.setenv("TILES_STREAM_PACING", "1")

    async def upstream():
        yield _sse(
            "response.reasoning_summary_text.delta",
            {"output_index": 0, "item_id": "reasoning_1", "delta": "aaaa", "content_index": 0},
            1,
        )
        yield _sse(
            "response.output_text.delta",
            {"output_index": 1, "item_id": "msg_1", "delta": "bbbb", "content_index": 0},
            2,
        )
        await asyncio.sleep(0.05)
        yield "data: [DONE]\n\n"

    out = asyncio.run(_collect(pace(upstream())))
    events = _parse_sse(out)
    names = [n for n, _ in events]

    reasoning = "".join(
        d["delta"] for n, d in events if n == "response.reasoning_summary_text.delta"
    )
    answer = "".join(d["delta"] for n, d in events if n == "response.output_text.delta")
    assert reasoning == "aaaa"
    assert answer == "bbbb"

    r_idx = [i for i, n in enumerate(names) if n == "response.reasoning_summary_text.delta"]
    a_idx = [i for i, n in enumerate(names) if n == "response.output_text.delta"]
    assert r_idx and a_idx
    assert max(r_idx) < min(a_idx)


def test_bounded_queue_applies_backpressure(monkeypatch):
    # a paused consumer must stall the pump at the queue cap instead of
    # letting it buffer the whole upstream response in memory
    from server.backend.stream_pacer import _MAX_QUEUED

    monkeypatch.setenv("TILES_STREAM_PACING", "1")
    produced = {"count": 0}

    async def upstream():
        for i in range(200):
            produced["count"] += 1
            yield _sse(
                "response.output_text.delta",
                {"output_index": 0, "item_id": "msg_1", "delta": "x", "content_index": i},
                i + 1,
            )
            await asyncio.sleep(0.001)  # slower than the tick
        yield "data: [DONE]\n\n"

    async def run():
        out = []
        gen = pace(upstream())
        # first slice comes on the first tick; the consumer then pauses
        # with upstream still flowing
        out.append(await gen.__anext__())
        await asyncio.sleep(0.3)
        stalled = produced["count"]
        async for chunk in gen:
            out.append(chunk)
        return out, stalled

    out, stalled = asyncio.run(run())

    # pump blocked on the full queue: chunks handed over are capped at
    # the queue size (+ the one stuck inside pump's put). An unbounded
    # queue would have drained all 200 by now.
    assert stalled <= _MAX_QUEUED + 2, f"pump ran ahead: {stalled}"
    assert stalled < 200, "pump did not stall at all"

    # nothing lost by the backpressure
    deltas = [d for n, d in _parse_sse(out) if n == "response.output_text.delta"]
    assert "".join(d["delta"] for d in deltas) == "x" * 200


def test_closing_consumer_with_full_queue_does_not_hang(monkeypatch):
    # aclose() while the pump is blocked on a full queue must drain and
    # return promptly instead of deadlocking in the cleanup gather
    monkeypatch.setenv("TILES_STREAM_PACING", "1")
    pulled = {"count": 0}

    async def upstream():
        while True:
            pulled["count"] += 1
            yield _sse(
                "response.output_text.delta",
                {"output_index": 0, "item_id": "msg_1", "delta": "x", "content_index": 0},
                pulled["count"],
            )
            await asyncio.sleep(0.001)

    async def run():
        gen = pace(upstream())
        await gen.__anext__()
        # let the queue fill so the pump is blocked on put
        await asyncio.sleep(0.3)
        await asyncio.wait_for(gen.aclose(), timeout=2.0)
        before = pulled["count"]
        await asyncio.sleep(0.1)
        assert pulled["count"] == before, "pump kept pulling after close"

    asyncio.run(run())


def _fake_backend(chunks: list[str], quiet_s: float = 0.0):
    # a stand-in runtime.backend whose stream yields the given chunks
    from unittest.mock import Mock

    async def fake_stream(request):
        for chunk in chunks:
            yield chunk
            if quiet_s:
                await asyncio.sleep(quiet_s)

    backend = Mock()
    backend.generate_response_chat_stream = fake_stream
    return backend


def test_route_streams_unpaced_when_mtp_disabled(monkeypatch):
    # even with pacing env on, mtp off must pass the stream through
    # untouched: same chunks, same bytes, no re-timing
    from unittest.mock import patch

    from fastapi.testclient import TestClient

    from server import runtime
    from server.api import app

    monkeypatch.setenv("TILES_STREAM_PACING", "1")
    chunks = [
        _sse(
            "response.output_text.delta",
            {"output_index": 0, "item_id": "msg_1", "delta": "Hello ", "content_index": 0},
            1,
        ),
        _sse(
            "response.output_text.delta",
            {"output_index": 0, "item_id": "msg_1", "delta": "world!", "content_index": 1},
            2,
        ),
        "data: [DONE]\n\n",
    ]

    with (
        patch.object(runtime, "backend", _fake_backend(chunks)),
        patch("server.api.get_llama_config", return_value={}),
    ):
        client = TestClient(app)
        with client.stream(
            "POST", "/v1/responses", json={"model": "m", "input": "hi", "stream": True}
        ) as resp:
            body = "".join(resp.iter_text())

    assert body == "".join(chunks)


def test_route_streams_paced_when_mtp_enabled(monkeypatch):
    # mtp on: the route must smooth bursts into sliced deltas
    from unittest.mock import patch

    from fastapi.testclient import TestClient

    from server import runtime
    from server.api import app

    monkeypatch.setenv("TILES_STREAM_PACING", "1")
    chunks = [
        _sse(
            "response.output_text.delta",
            {"output_index": 0, "item_id": "msg_1", "delta": piece, "content_index": i},
            i + 1,
        )
        for i, piece in enumerate(["Hel", "lo ", "wor", "ld"])
    ] + ["data: [DONE]\n\n"]

    with (
        patch.object(runtime, "backend", _fake_backend(chunks, quiet_s=0.05)),
        patch("server.api.get_llama_config", return_value={"mtp": True}),
    ):
        client = TestClient(app)
        with client.stream(
            "POST", "/v1/responses", json={"model": "m", "input": "hi", "stream": True}
        ) as resp:
            body = "".join(resp.iter_text())

    deltas = [d for n, d in _parse_sse([body]) if n == "response.output_text.delta"]
    assert "".join(d["delta"] for d in deltas) == "Hello world"
    # 4 burst chunks merged and sliced into more, smaller deltas
    assert len(deltas) > len([c for c in chunks if "delta" in c])
