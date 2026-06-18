import json
import logging
import time
import traceback
import uuid
from collections.abc import AsyncGenerator

from fastapi import HTTPException
from openai_harmony import (
    Conversation,
    HarmonyEncodingName,
    Role,
    load_harmony_encoding,
)
from openresponses_types.types import (
    Usage,
    InputTokensDetails,
    OutputTokensDetails,
    Error,
    IncompleteDetails,
)

from .commons import (
    get_reasoning_effort,
    build_harmony_conversation,
    is_harmony_family,
    handle_response_input,
    _sse,
    _get_response_on_create,
    _get_response_on_completed,
    _process_output_item_delta,
    _process_output_item_added,
    _process_init_reasoning_events,
    _process_stop_reasoning_events,
    _process_output_item_done,
    _process_error_event,
    _process_stop_tool_call_events,
    normalize_harmony_tool_name,
)


from ..schemas import (
    OutputItemDeltaModel,
    GenerationMetrics,
    ToolCallStart,
    ResponsesRequest,
    ResponsesResponse,
)
from .llama_cpp_runner import LlamaRunner

logger = logging.getLogger("app")

from typing import Any, Dict, List, Optional, Union, Iterator
import httpx
from pathlib import Path
from ..config import get_llama_config

_model_cache: Dict[str, LlamaRunner] = {}
_default_max_tokens: Optional[int] = None  # Use dynamic model-aware limits by default
_current_model_path: Optional[str] = None
_current_llama_config: Dict[str, Any] | None = None
# Store generated responses for follow-up support (previous_response_id)
_responses: Dict[str, ResponsesResponse] = {}




def get_or_load_model(
    model_spec: str,
    model_cache_path: str | None = None,
    verbose: bool = True,
) -> LlamaRunner:
    """Get model from cache or load it if not cached."""
    global _model_cache, _current_model_path, _current_llama_config
    model_name = model_spec
    llama_config = get_llama_config()

    if (
        model_cache_path is None
        and _current_model_path in _model_cache
        and _current_llama_config == llama_config
    ):
        logger.info(f"Model {model_name} already in memory")
        return _model_cache[_current_model_path]

    try:
        if isinstance(model_cache_path, str):
            model_path_str = model_cache_path
            model_path = Path(model_path_str)
        else:
            response = httpx.get(
                f"http://127.0.0.1:1729/model-cache-path?model_name={model_spec}"
            )
            if response.status_code == 200:
                model_path_str = response.text
                model_path = Path(model_path_str)
            else:
                raise Exception("Model not found in cache daemon")

        if not model_path.exists():
            logger.info(f"Model {model_spec} not found in cache at {model_path_str}")
            raise HTTPException(
                status_code=404, detail=f"Model {model_spec} not found in cache at {model_path_str}"
            )
    except Exception as e:
        logger.info(f"Model {model_spec} not found in: {str(e)}")
        raise HTTPException(
            status_code=404, detail=f"Model {model_spec} not found: {str(e)}"
        )

    # Check if we need to load a different model
    if (
        _current_model_path != model_path_str
        or _current_llama_config != llama_config
    ):
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

        # Load new model
        if verbose:
            print(f"Loading model: {model_name}")

        logger.info(f"Loading model: {model_name}")
        runner = LlamaRunner(
            model_path_str, verbose=verbose, llama_config=llama_config
        )
        runner.load_model()

        _model_cache[model_path_str] = runner
        _current_model_path = model_path_str
        _current_llama_config = llama_config
    else:
        logger.info(f"Model {model_name} already in memory")

    return _model_cache[model_path_str]


def _calc_usage(
    runner: LlamaRunner,
    generated_text: str,
    *,
    input_token_count: int | None = None,
    input_text: Union[str, list] | None = None,
    use_chat_template: bool = True,
) -> Dict[str, int]:
    """Calculate token usage using llama-cpp-python tokenizer."""
    if input_token_count is None and input_text is not None:
        input_token_count = runner.count_prompt_tokens(
            input_text, use_chat_template=use_chat_template
        )

    try:
        if runner.model is not None:
            output_tokens = runner.count_text_tokens(generated_text)
            return {
                "input_tokens": input_token_count or 0,
                "output_tokens": output_tokens,
            }
    except Exception:
        pass

    input_text_str = (
        json.dumps(input_text)
        if isinstance(input_text, list)
        else (input_text or "")
    )
    return {
        "input_tokens": input_token_count
        or int(len(input_text_str.split()) * 1.3),
        "output_tokens": int(len(generated_text.split()) * 1.3),
    }


def _store_response(
    response_id: str,
    created: int,
    completed_at: Optional[int],
    model: str,
    status: str,
    output: List[Dict[str, Any]],
    usage: Dict[str, int],
    error: Error | Dict[str, str] | None = None,
    incomplete_details: IncompleteDetails | Dict[str, str] | None = None,
    metrics: Optional[Dict[str, Any]] = None,
) -> ResponsesResponse:
    """Create a ResponsesResponse, attach metrics to metadata and store it in `_responses`."""
    resp = ResponsesResponse(
        id=response_id,
        created_at=created,
        completed_at=completed_at,
        model=model,
        status=status,
        object="response",
        error=error,  # pyright: ignore[reportArgumentType]
        output=output,
        usage=usage,
        incomplete_details=incomplete_details,  # pyright: ignore[reportArgumentType]
    )
    if metrics:
        try:
            resp.metadata["metrics"] = metrics
        except Exception:
            pass
    try:
        _responses[response_id] = resp
    except Exception:
        pass
    return resp




"""TODO: This current function has a similar setup that is used in MLX, in the future we
might have to make a common function for them (and for the functions below). Things like:
- get runner
- build iterator
- count input tokens
- count output tokens
- handle GenerationMetrics
- Harmony prompt token rendering/logging

These things are currently different in MLX, we will see how to make a common funcs outta these.
"""
async def generate_response_chat_stream(
    request: ResponsesRequest,
) -> AsyncGenerator[str, None]:
    """Generate streaming chat responses for OpenResponses API."""

    created = int(time.time())
    runner = get_or_load_model(request.model)
    user_input_content = handle_response_input(request)

    prompt_tokens: list[int] | None = None
    if is_harmony_family(request.model):
        reasoning_effort = get_reasoning_effort(request.reasoning.effort)
        convo = build_harmony_conversation(
            reasoning_effort,
            request.input,  # pyright: ignore
            replay_function_calls=True,
            tools=request.tools,
        )
        encoding = load_harmony_encoding(HarmonyEncodingName.HARMONY_GPT_OSS)
        prompt_tokens = encoding.render_conversation_for_completion(convo, Role.ASSISTANT)
        input_tokens = len(prompt_tokens)
    else:
        input_tokens = runner.count_prompt_tokens(
            user_input_content, use_chat_template=True
        )

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
    content_index = 0
    output_index = 0
    output_items = []
    tool_call_text = ""
    tool_name = None
    state = ""
    last_state = ""
    generation_metrics: GenerationMetrics | None = None
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
                use_chat_template=True,
            )

        for token in iterator:
            if isinstance(token, GenerationMetrics):
                generation_metrics = token
                continue

            if isinstance(token, ToolCallStart):
                tool_name = normalize_harmony_tool_name(token.name, request.tools)
                token = "**[ToolCall]**\n\n"

            if not isinstance(token, str):
                continue

            accumulated_text += token

            if "**[Reasoning]**" in token:
                last_state = state
                state = "reasoning"

            if "**[ToolCall]**" in token:
                last_state = state
                state = "toolcall"
                tool_id = f"toolcall_{uuid.uuid4()}"
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
                elif last_state == "answer":
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
                if content_index == 0 and tool_name:
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
                    if tool_name:
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
    try:
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
                tool_name,
            )
            output_items.append(item)
            yield resp_str
        elif state == "answer":
            resp_str, sequence_number, output_index, item = _process_output_item_done(
                "message", message_id, answer_text, output_index, sequence_number
            )
            output_items.append(item)
            yield resp_str
    except Exception as e:
        traceback.print_exc()
        resp_str, sequence_number = _process_error_event(
            str(e), response_id, request, created, sequence_number
        )
        yield resp_str
        return

    ## Envelope, response.completed
    if generation_metrics is not None:
        output_tokens = generation_metrics.total_tokens
    else:
        output_tokens = runner.count_text_tokens(accumulated_text)
    reasoning_tokens = runner.count_text_tokens(reasoning_text)
    usage = Usage(
        input_tokens=input_tokens,
        output_tokens=output_tokens,
        total_tokens=input_tokens + output_tokens,
        input_tokens_details=InputTokensDetails(cached_tokens=0),
        output_tokens_details=OutputTokensDetails(reasoning_tokens=reasoning_tokens),
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




async def generate_response_chat(request: ResponsesRequest):
    """Generate chat responses for Responses API"""

    model = request.model
    response_id = f"resp-{uuid.uuid4()}"
    msg_id = f"msg_{uuid.uuid4()}"
    created = int(time.time())
    runner = get_or_load_model(model)

    user_input_content = handle_response_input(request)
    
    convo: Conversation | None = None
    if is_harmony_family(model):
        reasoning_effort = get_reasoning_effort(request.reasoning.effort)
        convo = build_harmony_conversation(
            reasoning_effort,
            request.input,  # pyright: ignore
            replay_function_calls=True,
            tools=request.tools,
        )

    metrics_obj = None
    error = None
    incomplete_details = None

    try:
        generated_text = ""
        start_time = time.time()
        
        # Apply explicit chat template formatting for consistent context
        prompt_string = runner._format_conversation(
            user_input_content, use_chat_template=True
        ) if isinstance(user_input_content, list) else user_input_content
        
        if is_harmony_family(model):
            generated_text = runner.generate_batch_gpt(
                conversation=convo,
                max_tokens=runner.get_effective_max_tokens(request.max_output_tokens),
                temperature=request.temperature or 1,
                top_p=request.top_p or 1,
            )
        else:
            generated_text = runner.generate_batch(
                prompt=prompt_string,  # pyright: ignore
                max_tokens=runner.get_effective_max_tokens(request.max_output_tokens),
                temperature=request.temperature or 1,
                top_p=request.top_p or 1,
                use_chat_template=False, # already applied above
            )
        # Metrics for batch generation (approximate)
        generation_time = time.time() - start_time

        completed_at = int(time.time())
        status = "completed"
        error = None
        incomplete_details = None
        if is_harmony_family(model):
            encoding = load_harmony_encoding(HarmonyEncodingName.HARMONY_GPT_OSS)
            input_token_count = len(
                encoding.render_conversation_for_completion(convo, Role.ASSISTANT)  # pyright: ignore[reportArgumentType]
            )
        elif isinstance(user_input_content, list):
            input_token_count = runner.count_prompt_tokens(
                user_input_content, use_chat_template=True
            )
        else:
            input_token_count = runner.count_text_tokens(prompt_string)  # pyright: ignore[reportArgumentType]

        usage = _calc_usage(
            runner,
            generated_text,
            input_token_count=input_token_count,
        )
        output_tokens = usage.get("output_tokens", 0)
        metrics_obj = {
            "ttft_ms": generation_time * 1000.0,
            "total_tokens": output_tokens,
            "tokens_per_second": (
                (output_tokens / generation_time) if generation_time > 0 else 0.0
            ),
            "total_latency_s": generation_time,
        }

    except Exception as e:
        completed_at = None
        status = "failed"
        error = {"message": str(e), "code": "500"}
        incomplete_details = {"reason": "internal server error"}
        generated_text = ""
        usage = {"input_tokens": 0, "output_tokens": 0}

    output_block = (
        [
            {
                "type": "message",
                "id": msg_id,
                "status": "completed" if status == "completed" else "failed",
                "role": "assistant",
                "content": [
                    {"type": "output_text", "text": generated_text, "annotations": []}
                ],
            }
        ]
        if status == "completed"
        else []
    )

    resp = _store_response(
        response_id=response_id,
        created=created,
        completed_at=completed_at,
        model=model,
        status=status,
        output=output_block,
        usage=usage,
        error=error,
        incomplete_details=incomplete_details,
        metrics=(metrics_obj if status == "completed" else None),
    )

    return resp
