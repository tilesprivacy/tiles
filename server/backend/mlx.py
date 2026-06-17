import logging
import time
import uuid
from collections.abc import AsyncGenerator
from fastapi import HTTPException
from openresponses_types.types import (
    InputTokensDetails,
    OutputTokensDetails,
    Usage,
)

from .commons import (
    _get_response_on_completed,
    _get_response_on_create,
    _process_error_event,
    _process_init_reasoning_events,
    _process_output_item_added,
    _process_output_item_delta,
    _process_output_item_done,
    _process_stop_reasoning_events,
    _process_stop_tool_call_events,
    _sse,
    build_harmony_conversation,
    get_reasoning_effort,
    handle_response_input,
    is_harmony_family,
)

from ..schemas import (
    GenerationMetrics,
    OutputItemDeltaModel,
    ResponsesRequest,
    ToolCallStart,
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
    model_spec: str,
    model_cache_path: str | None = None,
    verbose: bool = True,
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
            reasoning_effort,
            request.input,  # pyright: ignore
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
    tool_name = None
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

            if isinstance(token, ToolCallStart):
                tool_name = token.name                
                token = "**[ToolCall]**\n\n"

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
                # Start fresh so arguments from the last tool call are not reused. I did the same in Linux.
                tool_call_text = ""
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
                            tool_name,
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
                        tool_name,
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
            tool_id, output_index, tool_call_text, sequence_number, request, tool_name
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
