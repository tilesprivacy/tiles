import asyncio
import json
from unittest.mock import patch

from server.backend.llama_server import backend
from server.schemas import ResponsesRequest


def _request() -> ResponsesRequest:
    return ResponsesRequest.model_validate(
        {"model": "test/model", "input": "hello", "stream": True}
    )


async def _drain(request: ResponsesRequest) -> list[str]:
    return [chunk async for chunk in backend.generate_response_chat_stream(request)]


def test_a_load_failure_reaches_the_client_with_its_reason():
    # the field failure: llama-server rejects a flag and exits at startup,
    # before the stream has sent anything. The client must hear why, not just
    # that the stream ended.
    cause = (
        "llama-server exited during startup (code 1). Check llama-server.err.log.\n"
        "error: invalid argument: --no-mmap"
    )
    with patch.object(backend, "get_or_load_model", side_effect=RuntimeError(cause)):
        chunks = asyncio.run(_drain(_request()))

    assert len(chunks) == 1
    event_line, data_line = chunks[0].strip().split("\n")[:2]
    assert event_line == "event: response.failed"
    payload = json.loads(data_line.removeprefix("data: "))
    assert "invalid argument: --no-mmap" in payload["response"]["error"]["message"]
