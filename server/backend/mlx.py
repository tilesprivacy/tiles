import json
import logging
import time
import uuid
import random
import string
from collections.abc import AsyncGenerator
from fastapi import HTTPException
from openresponses_types.types import (
    InputTokensDetails,
    OutputTokensDetails,
    Usage,
)

from .commons import (
    get_reasoning_effort,
    build_harmony_conversation,
    is_harmony_family,
    handle_response_input,
)

from ..schemas import (
    GenerationMetrics,
    OutputItemDeltaModel,
    ResponsesRequest,
)
from .mlx_runner import MLXRunner

import httpx
import traceback

client = httpx.AsyncClient()

logger = logging.getLogger("app")

from typing import Dict, Iterator, Optional

_model_cache: Dict[str, MLXRunner] = {}
_current_model_path: Optional[str] = None


def get_or_load_model(
    model_spec: str, model_cache_path: str | None = None, verbose: bool = True
) -> MLXRunner:
    """Get model from cache or load it if not cached."""
    global _model_cache, _current_model_path
    model_name = model_spec
    if isinstance(model_cache_path, str):
        model_path_str = model_cache_path
        # Check if we need to load a different model
        if _current_model_path != model_path_str:
            # Proactively clean up any previously loaded runner to release memory
            if _model_cache:
                try:
                    for _old_runner in list(_model_cache.values()):
                        try:
                            _old_runner.cleanup()
                        except Exception:
                            pass
                finally:
                    _model_cache.clear()

            runner = MLXRunner(model_path_str, verbose=verbose)
            runner.load_model()

            _model_cache[model_path_str] = runner
            _current_model_path = model_path_str
            return runner
        else:
            logger.info(f"Model {model_name} already in memory")
            return _model_cache[_current_model_path]  # pyright: ignore
    else:
        logger.info(f"Model Path {_current_model_path} already in memory")
        return _model_cache[_current_model_path]  # pyright: ignore


# TODO: Consider benchmark stuff
async def generate_response_chat_stream(
    request: ResponsesRequest,
) -> AsyncGenerator[str, None]:
    """Generate streaming chat responses for OpenResponses API."""

    created = int(time.time())
    runner = await _get_runner(request.model)
    user_input_content = handle_response_input(request)

    if is_harmony_family(request.model):
        reasoning_effort = get_reasoning_effort(request.reasoning.effort)
        convo = build_harmony_conversation(
            reasoning_effort, request.input  # pyright: ignore
        )

    input_tokens = len(runner.tokenizer.encode(user_input_content))  # pyright: ignore

    response_id = f"resp_{uuid.uuid4()}"
    message_id = f"msg_{uuid.uuid4()}"
    reasoning_id = f"reasoning_{uuid.uuid4()}"
    sequence_number = 0
    tool_id = ""
    ## response.created envelope event ##
    initial_response = _get_response_on_create(response_id, request, created)
    resp_str, sequence_number = _sse(
        "response.created", {"response": initial_response}, sequence_number
    )
    yield resp_str
    ############

    accumulated_text = ""
    answer_text = ""
    reasoning_text = ""
    output_tokens = 0
    content_index = 0
    output_index = 0
    output_items = []
    tool_call_text = ""
    state = ""
    last_state = ""
    try:
        iterator: Iterator
        if is_harmony_family(request.model):
            iterator = runner.generate_streaming_gpt(
                conversation=convo,
                max_tokens=runner.get_effective_max_tokens(request.max_output_tokens),
                temperature=request.temperature or 1,
                top_p=request.top_p or 1,
            )
        else:
            iterator = runner.generate_streaming(
                prompt=user_input_content,
                max_tokens=runner.get_effective_max_tokens(request.max_output_tokens),
                temperature=request.temperature or 1,
                top_p=request.top_p or 1,
            )

        for token in iterator:
            if isinstance(token, GenerationMetrics):
                continue

            if not isinstance(token, str):
                continue

            accumulated_text += token
            output_tokens += 1

            if "**[Reasoning]**" in token:
                last_state = state
                state = "reasoning"

            if "**[ToolCall]**" in token:
                last_state = state
                state = "toolcall"
                tool_id = f"toolcall_{uuid.uuid4()}"
                content_index = 0

            if "**[Answer]**" in token:
                last_state = state
                state = "answer"
                # Resetting content_index as reasoning output_item is finished
                content_index = 0

            # State changed, so emit the stop events for the last state
            if last_state != state and last_state != "" and content_index == 0:
                if last_state == "reasoning":
                    resp_str, sequence_number, output_index, item = (
                        _process_stop_reasoning_events(
                            reasoning_id, output_index, reasoning_text, sequence_number
                        )
                    )
                    output_items.append(item)
                    yield resp_str
                elif last_state == "toolcall":
                    resp_str, sequence_number, output_index, item = (
                        _process_stop_tool_call_events(
                            tool_id,
                            output_index,
                            tool_call_text,
                            sequence_number,
                            request,
                        )
                    )
                    output_items.append(item)
                    yield resp_str
                elif state == "answer":
                    resp_str, sequence_number, output_index, item = (
                        _process_output_item_done(
                            "message",
                            message_id,
                            answer_text,
                            output_index,
                            sequence_number,
                        )
                    )
                    output_items.append(item)
                    yield resp_str

            if state == "reasoning":
                if content_index == 0:
                    resp_str, sequence_number = _process_init_reasoning_events(
                        reasoning_id, token, output_index, sequence_number
                    )
                    yield resp_str

                reasoning_text += token
                output_item = OutputItemDeltaModel(
                    item_name="reasoning_summary_text",
                    item_id=reasoning_id,
                    index=output_index,
                    delta=token,
                    content_index=content_index,
                )
                resp_str, sequence_number = _process_output_item_delta(
                    output_item, sequence_number
                )
                yield resp_str
            elif state == "toolcall":
                if content_index == 0:
                    resp_str, sequence_number = _process_output_item_added(
                        "function_call",
                        tool_id,
                        token,
                        output_index,
                        sequence_number,
                    )
                    yield resp_str
                # To avoid toolcall tag in the final arguments txt
                if content_index != 0:
                    tool_call_text += token
                    output_item = OutputItemDeltaModel(
                        item_name="function_call_arguments",
                        item_id=tool_id,
                        index=output_index,
                        delta=token,
                        content_index=content_index,
                    )
                    resp_str, sequence_number = _process_output_item_delta(
                        output_item, sequence_number
                    )
                    yield resp_str
            elif state == "answer":
                if content_index == 0:
                    resp_str, sequence_number = _process_output_item_added(
                        "message", message_id, token, output_index, sequence_number
                    )
                    yield resp_str
                answer_text += token
                output_item = OutputItemDeltaModel(
                    item_name="output_text",
                    item_id=message_id,
                    index=output_index,
                    delta=token,
                    content_index=content_index,
                )
                resp_str, sequence_number = _process_output_item_delta(
                    output_item, sequence_number
                )
                yield resp_str

            content_index += 1

    except Exception as e:
        traceback.print_exc()
        resp_str, sequence_number = _process_error_event(
            str(e), response_id, request, created, sequence_number
        )
        yield resp_str
        return

    # Emit the stop events current state
    if state == "reasoning":
        resp_str, sequence_number, output_index, item = _process_stop_reasoning_events(
            reasoning_id, output_index, reasoning_text, sequence_number
        )
        output_items.append(item)
        yield resp_str
    elif state == "toolcall":
        resp_str, sequence_number, output_index, item = _process_stop_tool_call_events(
            tool_id,
            output_index,
            tool_call_text,
            sequence_number,
            request,
        )
        output_items.append(item)
        yield resp_str
    elif state == "answer":
        resp_str, sequence_number, output_index, item = _process_output_item_done(
            "message", message_id, answer_text, output_index, sequence_number
        )
        output_items.append(item)
        yield resp_str

    ## Envelope, response.completed
    usage = Usage(
        input_tokens=input_tokens,
        output_tokens=output_tokens,
        total_tokens=input_tokens + output_tokens,
        input_tokens_details=InputTokensDetails(cached_tokens=0),
        output_tokens_details=OutputTokensDetails(reasoning_tokens=len(reasoning_text)),
    )
    final_response = _get_response_on_completed(
        response_id, request, created, output_items, usage
    )

    resp_str, sequence_number = _sse(
        "response.completed", {"response": final_response}, sequence_number
    )
    yield resp_str
    ###############

    yield "data: [DONE]\n\n"
    return


def _sse(event_name: str, payload: dict, current_seq_no: int) -> tuple[str, int]:
    seq_no = current_seq_no + 1
    event = {
        "type": event_name,
        "sequence_number": seq_no,
    }
    event.update(payload)
    event_str = f"event: {event_name}\ndata: {json.dumps(event)}\n\n"

    return event_str, seq_no


def _get_response_on_create(
    response_id: str,
    request: ResponsesRequest,
    created_at: int,
) -> dict:
    created_response = {
        "id": response_id,
        "object": "response",
        "created_at": created_at,
        "completed_at": None,
        "status": "in_progress",
        "output": [],
        "incomplete_details": None,
        "text": {"format": {"type": "text"}, "verbosity": "low"},
        "paralell_tool_calls": 0,
        "truncation": "disabled",
        "tool_choice": "auto",
        "error": {"code": "", "message": ""},
    }
    created_response.update(_get_commons_responses(request))
    return created_response


def _get_response_on_completed(
    response_id: str,
    request: ResponsesRequest,
    created_at: int,
    output: list,
    usage: Usage,
) -> dict:
    completed_at = int(time.time())
    completed_response = {
        "id": response_id,
        "object": "response",
        "created_at": created_at,
        "completed_at": completed_at,
        "status": "completed",
        "output": output,
        "incomplete_details": None,
        "text": {"format": {"type": "text"}, "verbosity": "low"},
        "paralell_tool_calls": 0,
        "truncation": "disabled",
        "tool_choice": "auto",
        "error": {"code": "", "message": ""},
        "usage": {
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
            "total_tokens": usage.total_tokens,
            "input_token_details": {
                "cached_tokens": usage.input_tokens_details.cached_tokens
            },
            "output_token_details": {
                "reasoning_tokens": usage.output_tokens_details.reasoning_tokens
            },
        },
    }
    completed_response.update(_get_commons_responses(request))
    return completed_response


def _get_response_on_error(
    response_id: str,
    request: ResponsesRequest,
    created_at: int,
    incomplete_details: dict,
    error: dict,
) -> dict:
    created_response = {
        "id": response_id,
        "object": "response",
        "created_at": created_at,
        "completed_at": None,
        "status": "failed",
        "output": [],
        "incomplete_details": incomplete_details,
        "text": {"format": {"type": "text"}, "verbosity": "low"},
        "paralell_tool_calls": 0,
        "truncation": "disabled",
        "tool_choice": "auto",
        "error": error,
    }
    created_response.update(_get_commons_responses(request))
    return created_response


def _get_commons_responses(request: ResponsesRequest):
    if request.tools != None:
        tools_as_dicts = [t.model_dump() for t in request.tools]  # pyright: ignore
    else:
        tools_as_dicts = None

    return {
        "model": request.model,
        "previous_response_id": request.previous_response_id,
        "instructions": request.instructions,
        "temperature": request.temperature,
        "prompt_cache_key": request.prompt_cache,
        "safety_identifier": request.safety_identifier,
        "service_tier": request.service_tier,
        "background": request.background,
        "store": request.store,
        "max_tool_calls": request.max_tool_calls,
        "max_output_tokens": request.max_output_tokens,
        "reasoning": {"effort": request.reasoning.effort, "summary": "disabled"},
        "top_logprobs": request.top_logprobs,
        "frequency_penalty": 0,
        "presence_penalty": 0,
        "top_p": request.top_p,
        "tools": tools_as_dicts,
    }


async def _get_runner(model: str):
    # comms w tiles daemon to get correct model local path
    response = await client.get(
        f"http://127.0.0.1:1729/model-cache-path?model_name={model}"
    )

    model_cache_path = None
    if response.status_code == 200:
        model_cache_path = response.text
    else:
        raise HTTPException(status_code=500, detail="Model not found")

    runner = get_or_load_model(model, model_cache_path)
    return runner


def _process_output_item_delta(
    output_item: OutputItemDeltaModel, sequence_number: int
) -> tuple[str, int]:
    event_name = ".".join(["response", output_item.item_name, "delta"])

    event = {
        "output_index": output_item.index,
        "item_id": output_item.item_id,
        "delta": output_item.delta,
        "content_index": output_item.content_index,
    }

    return _sse(event_name, event, sequence_number)


def _process_output_item_added(
    type: str, id: str, token: str, output_index, sequence_number: int
) -> tuple[str, int]:
    event_name = "response.output_item.added"
    if type == "function_call":
        item_chunk = {
            "type": type,
            "id": id,
            "name": "name",
            "call_id": "call_" + _random_alphanum(),
            "status": "in_progress",
        }
    else:
        item_chunk = {
            "type": type,
            "id": id,
            "status": "in_progress",
            "role": "assistant",
            "content": [
                {
                    "type": "output_text",
                    "text": token,
                }
            ],
        }
    event = {
        "output_index": output_index,
        "item": item_chunk,
    }
    return _sse(event_name, event, sequence_number)


def _process_init_reasoning_events(
    id: str, token: str, output_index, sequence_number: int
) -> tuple[str, int]:
    resp_str_a, sequence_number = _process_output_item_added(
        "reasoning", id, token, output_index, sequence_number
    )

    event_name = "response.reasoning_summary_part.added"
    event = {
        "output_index": output_index,
        "item_id": id,
        "part": {"text": token, "type": "summary_text"},
        "summary_index": 0,
    }
    resp_str, sequence_number = _sse(event_name, event, sequence_number)
    return resp_str_a + resp_str, sequence_number


def _process_stop_reasoning_events(
    id: str, output_index: int, text: str, sequence_number: int
) -> tuple[str, int, int, dict]:
    payload = {
        "item_id": id,
        "output_index": output_index,
        "text": text,
    }
    resp_str_a, sequence_number = _sse(
        "response.reasoning_summary_text.done", payload, sequence_number
    )
    event_name = "response.reasoning_summary_part.done"
    event = {
        "output_index": output_index,
        "item_id": id,
        "part": {"text": text, "type": "summary_text"},
        "summary_index": 0,
    }
    resp_str_b, sequence_number = _sse(event_name, event, sequence_number)
    resp_str_c, sequence_number, output_index, item_chunk = _process_output_item_done(
        "reasoning", id, text, output_index, sequence_number
    )
    return (
        resp_str_a + resp_str_b + resp_str_c,
        sequence_number,
        output_index,
        item_chunk,
    )


def _process_output_item_done(
    type: str,
    id: str,
    final_text: str,
    output_index,
    sequence_number: int,
    tool_name: str | None = None,
) -> tuple[str, int, int, dict]:
    event_name = "response.output_item.done"
    item_chunk: dict
    if type == "function_call":
        try:
            arguments_map = json.loads(final_text)
        except json.JSONDecodeError as e:
            arguments_map = {}

        new_args = {
            ("command" if k == "cmd" else k): v for k, v in arguments_map.items()
        }

        item_chunk = {
            "type": type,
            "id": id,
            "name": tool_name,
            "call_id": "call_" + _random_alphanum(),
            "status": "completed",
            "arguments": json.dumps(new_args),
        }
    else:
        item_chunk = {
            "type": type,
            "id": id,
            "status": "completed",
            "role": "assistant",
            "content": [
                {
                    "type": "output_text",
                    "text": final_text,
                }
            ],
        }
    if type == "reasoning":
        item_chunk.update({"summary": [{"type": "summary_text", "text": final_text}]})
    event = {
        "output_index": output_index,
        "item": item_chunk,
    }
    resp_str, sequence_number = _sse(event_name, event, sequence_number)
    output_index = output_index + 1
    return resp_str, sequence_number, output_index, item_chunk


def _process_error_event(
    err: str,
    response_id: str,
    request: ResponsesRequest,
    created_at: int,
    sequence_number: int,
) -> tuple[str, int]:
    error = {"message": err, "code": "500"}
    incomplete_details = {"reason": "internal server error"}

    err_response = _get_response_on_error(
        response_id, request, created_at, incomplete_details, error
    )
    return _sse("response.failed", {"response": err_response}, sequence_number)


def _random_alphanum(n=10):
    return "".join(random.choices(string.ascii_letters + string.digits, k=n))


def _process_stop_tool_call_events(
    id: str,
    output_index: int,
    text: str,
    sequence_number: int,
    request: ResponsesRequest,
) -> tuple[str, int, int, dict]:
    event_name = "response.function_call_arguments.done"
    tool_name = _find_tool(request.tools, text)  # pyright: ignore

    try:
        arguments_map = json.loads(text)
    except json.JSONDecodeError as e:
        arguments_map = {}

    new_args = {("command" if k == "cmd" else k): v for k, v in arguments_map.items()}

    event = {
        "output_index": output_index,
        "item_id": id,
        "name": tool_name,
        "arguments": json.dumps(new_args),
    }
    resp_str_a, sequence_number = _sse(event_name, event, sequence_number)
    resp_str_b, sequence_number, output_index, item_chunk = _process_output_item_done(
        "function_call", id, text, output_index, sequence_number, tool_name
    )
    return (
        resp_str_a + resp_str_b,
        sequence_number,
        output_index,
        item_chunk,
    )


def _find_tool(tools: list, arguments_str: str) -> str:
    try:
        arguments_map = json.loads(arguments_str)
    except json.JSONDecodeError as e:
        arguments_map = {}

    # To increase the accuracy of the selected tool, since we
    # check the required params is a subset of model responded
    # arguments, there is chance `read` can precede write

    tools.reverse()

    tool_cmd = {"cmd": "command", "rw": "read-write"}
    response_argument_keys_raw = list(arguments_map.keys())
    # map thru and change cmd to command
    response_argument_keys = [tool_cmd.get(x, x) for x in response_argument_keys_raw]

    tool_name = ""

    for tool in tools:
        name = tool.name
        params = tool.parameters

        if params is None:
            required_params = []
        else:
            required_params = params.get("required", [])

        if _is_correct_tool(required_params, response_argument_keys):
            tool_name = name
            break

    if tool_name == "":
        return "read"
    else:
        return tool_name


def _is_correct_tool(required_params: list, model_argument_list: list) -> bool:
    return set(required_params).issubset(model_argument_list)
