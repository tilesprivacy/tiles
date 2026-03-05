import json
import logging
import time
import uuid
from collections.abc import AsyncGenerator

from fastapi import HTTPException
from openai_harmony import (
    Conversation,
    DeveloperContent,
    Message,
    ReasoningEffort,
    Role,
    SystemContent,
)
from openresponses_types import ReasoningEffortEnum
from openresponses_types.types import (
    DeveloperMessageItemParam,
    Error,
    IncompleteDetails,
    UserMessageItemParam,
)

from ..reasoning_utils import ReasoningExtractor

from ..cache_utils import get_model_path
from ..hf_downloader import pull_model
from ..schemas import (
    ChatCompletionRequest,
    ChatMessage,
    GenerationMetrics,
    ResponsesRequest,
    ResponsesResponse,
    downloadRequest,
)
from .mlx_runner import MLXRunner

logger = logging.getLogger("app")

from typing import Any, Dict, Iterator, List, Optional, Union

_model_cache: Dict[str, MLXRunner] = {}
_default_max_tokens: Optional[int] = None  # Use dynamic model-aware limits by default
_current_model_path: Optional[str] = None
# Store generated responses for follow-up support (previous_response_id)
_responses: Dict[str, ResponsesResponse] = {}


def download_model(model_name: str):
    """Download the model"""
    if pull_model(model_name):
        return {"message": "Model downloaded"}
    else:
        raise HTTPException(status_code=400, detail="Downloading model failed")


def get_or_load_model(model_spec: str, verbose: bool = True) -> MLXRunner:
    """Get model from cache or load it if not cached."""
    global _model_cache, _current_model_path

    # Use the existing model path resolution from cache_utils

    try:
        model_path, model_name, commit_hash = get_model_path(model_spec)
        if not model_path.exists():
            logger.info(f"Model {model_spec} not found in cache")
            raise HTTPException(
                status_code=404, detail=f"Model {model_spec} not found in cache"
            )
    except Exception as e:
        logger.info(f"Model {model_spec} not found in: {str(e)}")
        raise HTTPException(
            status_code=404, detail=f"Model {model_spec} not found: {str(e)}"
        )

    # Check if it's an MLX model

    model_path_str = str(model_path)

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

        # Load new model
        if verbose:
            print(f"Loading model: {model_name}")

        logger.info(f"Loading model: {model_name}")
        runner = MLXRunner(model_path_str, verbose=verbose)
        runner.load_model()

        _model_cache[model_path_str] = runner
        _current_model_path = model_path_str
    else:
        logger.info(f"Model {model_name} already in memory")

    return _model_cache[model_path_str]


async def generate_chat_stream(
    messages: List[ChatMessage], request: ChatCompletionRequest
) -> AsyncGenerator[str, None]:
    """Generate streaming chat completion response."""

    _messages = messages
    completion_id = f"chatcmpl-{uuid.uuid4()}"
    created = int(time.time())
    runner = get_or_load_model(request.model)
    if request.chat_start:
        _messages.extend(request.messages)
    # Convert messages to dict format for runner
    message_dicts = format_chat_messages_for_runner(_messages)

    # Let the runner format with chat templates
    prompt = runner._format_conversation(message_dicts, use_chat_template=True)

    # Yield initial response
    initial_response = {
        "id": completion_id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": request.model,
        "choices": [
            {
                "index": 0,
                "delta": {"role": "assistant", "content": ""},
                "finish_reason": None,
            }
        ],
    }

    yield f"data: {json.dumps(initial_response)}\n\n"

    # Stream tokens
    metrics = None
    try:
        for token in runner.generate_streaming(
            prompt=prompt,
            max_tokens=runner.get_effective_max_tokens(
                request.max_tokens or _default_max_tokens, interactive=False
            ),
            temperature=request.temperature,
            top_p=request.top_p,
            repetition_penalty=request.repetition_penalty,
            use_chat_template=False,  # Already applied in _format_conversation
            use_chat_stop_tokens=False,  # Server mode shouldn't stop on chat markers
        ):
            if isinstance(token, GenerationMetrics):
                metrics = token
                continue

            chunk_response = {
                "id": completion_id,
                "object": "chat.completion.chunk",
                "created": created,
                "model": request.model,
                "choices": [
                    {"index": 0, "delta": {"content": token}, "finish_reason": None}
                ],
            }

            yield f"data: {json.dumps(chunk_response)}\n\n"

            # Check for stop sequences
            if request.stop:
                stop_sequences = (
                    request.stop if isinstance(request.stop, list) else [request.stop]
                )
                if any(stop in token for stop in stop_sequences):
                    break

    except Exception as e:
        error_response = {
            "id": completion_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": request.model,
            "choices": [{"index": 0, "delta": {}, "finish_reason": "error"}],
            "error": str(e),
        }
        yield f"data: {json.dumps(error_response)}\n\n"

    # Final response
    final_response = {
        "id": completion_id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": request.model,
        "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
    }

    # Include benchmarking metrics if available
    if metrics:
        final_response["metrics"] = {
            "ttft_ms": metrics.ttft_ms,
            "total_tokens": metrics.total_tokens,
            "tokens_per_second": metrics.tokens_per_second,
            "total_latency_s": metrics.total_latency_s,
        }
    yield f"data: {json.dumps(final_response)}\n\n"
    yield "data: [DONE]\n\n"


def format_chat_messages_for_runner(
    messages: List[ChatMessage],
) -> List[Dict[str, str]]:
    """Convert chat messages to format expected by MLXRunner.

    Returns messages in dict format for the runner to apply chat templates.
    """
    return [{"role": msg.role, "content": msg.content} for msg in messages]


def _prepend_previous_response(user_input: str, prev_id: Optional[str]) -> str:
    """If prev_id points to a stored response, prepend its output text as context."""

    if not prev_id:
        return user_input

    prev = _responses.get(prev_id)  # pyright: ignore

    if not prev or not getattr(prev, "output", None):
        return user_input
    prev_text_parts: List[str] = []
    for out in prev.output:
        for c in out.get("content", []):
            if c.get("type") == "output_text":
                prev_text_parts.append(c.get("text", ""))
    if prev_text_parts:
        return "\n".join(prev_text_parts) + "\n\n" + user_input
    return user_input


def _calc_usage(
    runner: MLXRunner, input_text: str, generated_text: str
) -> Dict[str, int]:
    """Calculate token usage using the runner tokenizer; fall back to zeros on error."""
    try:
        input_tokens = len(runner.tokenizer.encode(input_text))
        output_tokens = len(runner.tokenizer.encode(generated_text))
        return {"input_tokens": input_tokens, "output_tokens": output_tokens}
    except Exception:
        return {"input_tokens": 0, "output_tokens": 0}


def _store_response(
    response_id: str,
    created: int,
    completed_at: Optional[int],
    model: str,
    status: str,
    output: List[Dict[str, Any]],
    usage: Dict[str, int],
    error: Error | None = None,
    incomplete_details: IncompleteDetails | None = None,
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
        error=error,
        output=output,
        usage=usage,
        incomplete_details=incomplete_details,
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


def count_tokens(text: str) -> int:
    """Rough token count estimation."""
    return int(len(text.split()) * 1.3)  # Approximation, convert to int


def handle_response_input(request: ResponsesRequest):
    dev_msg_item = None
    user_msg_item = None
    user_input_content = ""
    if isinstance(request.input, str):
        user_input_content = request.input
    else:
        for item in request.input:
            match item:
                case UserMessageItemParam():
                    user_msg_item = item
                    user_input_content = item.content.root  # pyright: ignore
                case DeveloperMessageItemParam():
                    dev_msg_item = item
                case _:
                    raise TypeError("unknown type")
    return [user_input_content, user_msg_item, dev_msg_item]


async def generate_response_chat_stream(
    request: ResponsesRequest,
) -> AsyncGenerator[str, None]:
    """Generate streaming chat responses for OpenResponses API."""
    model = request.model
    created = int(time.time())
    runner = get_or_load_model(model)
    metrics = None

    user_input_content = ""
    convo: Conversation | None = None
    dev_msg_item = None
    user_msg_item = None
    [user_input_content, user_msg_item, dev_msg_item] = handle_response_input(request)
    user_input_content = _prepend_previous_response(
        user_input_content, request.previous_response_id
    )
    if is_harmony_family(model):

        reasoning_effort = get_reasoning_effort(request.reasoning.effort)

        convo = build_harmony_conversation(
            reasoning_effort, dev_msg_item, user_input_content
        )

    input_tokens = len(runner.tokenizer.encode(user_input_content))  # pyright: ignore

    # Initial chunk
    initial_chunk = {
        "id": f"resp_{uuid.uuid4()}",
        "object": "response.chunk",
        "created_at": created,
        "model": model,
        "status": "in_progress",
        "output": [
            {
                "type": "message",
                "id": f"msg_{uuid.uuid4()}",
                "status": "in_progress",
                "role": "assistant",
                "content": [],
            }
        ],
        "usage": {"input_tokens": input_tokens, "output_tokens": 0},
    }
    yield f"data: {json.dumps(initial_chunk)}\n\n"

    accumulated_text = ""
    answer_text = ""
    output_tokens = 0
    error = None
    incomplete_details = None
    has_answer_started: bool = False
    # TODO: we need to inject the context prepending, else model is losing it.
    try:
        iterator: Iterator | None = None
        if is_harmony_family(model):
            iterator = runner.generate_streaming_gpt(
                conversation=convo,  # pyright: ignore
                max_tokens=runner.get_effective_max_tokens(request.max_output_tokens),
                temperature=request.temperature or 1,
                top_p=request.top_p or 1,
            )
        else:
            iterator = runner.generate_streaming(
                prompt=user_input_content,  # pyright: ignore
                max_tokens=runner.get_effective_max_tokens(request.max_output_tokens),
                temperature=request.temperature or 1,
                top_p=request.top_p or 1,
            )
        for token in iterator:  # pyright: ignore
            if isinstance(token, GenerationMetrics):
                metrics = token
                continue

            if not isinstance(token, str):
                continue

            if "**[Answer]**" in token or has_answer_started:
                has_answer_started = True
                answer_text += token

            accumulated_text += token
            output_tokens += 1  # Each yield is one token

            chunk = {
                "id": f"resp_{uuid.uuid4()}",
                "object": "response.chunk",
                "created_at": created,
                "model": model,
                "status": "in_progress",
                "output": [
                    {
                        "type": "message",
                        "id": f"msg_{uuid.uuid4()}",
                        "status": "in_progress",
                        "role": "assistant",
                        "content": [
                            {
                                "type": "output_text",
                                "text": token,
                                "annotations": [],
                            }
                        ],
                    }
                ],
                "usage": {"input_tokens": input_tokens, "output_tokens": output_tokens},
            }
            yield f"data: {json.dumps(chunk)}\n\n"

    except Exception as e:
        error = {"message": str(e), "code": "500"}
        incomplete_details = {"reason": "internal server error"}

        error_chunk = {
            "id": f"resp_{uuid.uuid4()}",
            "object": "response.chunk",
            "created_at": created,
            "model": model,
            "status": "failed",
            "error": error,
            "incomplete_details": incomplete_details,
            "output": [],
            "usage": {"input_tokens": input_tokens, "output_tokens": output_tokens},
        }
        yield f"data: {json.dumps(error_chunk)}\n\n"
        return

    # Final chunk
    completed_at = int(time.time())
    # Build final chunk with accumulated text and store response for follow-ups

    final_chunk = {
        "id": f"resp_{uuid.uuid4()}",
        "object": "response.chunk",
        "created_at": created,
        "completed_at": completed_at,
        "model": model,
        "status": "completed",
        "output": [
            {
                "type": "message",
                "id": f"msg_{uuid.uuid4()}",
                "status": "completed",
                "role": "assistant",
                "content": [
                    {
                        "type": "output_text",
                        "text": answer_text,
                        "annotations": [],
                    }
                ],
            }
        ],
        "usage": {"input_tokens": input_tokens, "output_tokens": output_tokens},
    }

    # Store and return a typed ResponsesResponse for follow-ups
    metrics_obj = None
    if metrics:
        metrics_obj = {
            "ttft_ms": metrics.ttft_ms,
            "total_tokens": metrics.total_tokens,
            "tokens_per_second": metrics.tokens_per_second,
            "total_latency_s": metrics.total_latency_s,
        }
        final_chunk["metrics"] = metrics_obj

    _store_response(
        response_id=final_chunk["id"],
        created=created,
        completed_at=completed_at,
        model=model,
        status="completed",
        output=final_chunk["output"],
        usage={"input_tokens": input_tokens, "output_tokens": output_tokens},
        metrics=metrics_obj,
    )
    yield f"data: {json.dumps(final_chunk)}\n\n"
    yield "data: [DONE]\n\n"


async def generate_response_chat(request: ResponsesRequest):
    """Generate chat responses for Responses API"""

    model = request.model
    response_id = f"resp-{uuid.uuid4()}"
    msg_id = f"msg_{uuid.uuid4()}"
    created = int(time.time())
    runner = get_or_load_model(model)

    user_input_content = ""

    dev_msg_item = None
    user_msg_item = None
    [user_input_content, user_msg_item, dev_msg_item] = handle_response_input(request)
    user_input_content = _prepend_previous_response(
        user_input_content, request.previous_response_id
    )

    reasoning_effort = get_reasoning_effort(request.reasoning.effort)

    convo = build_harmony_conversation(
        reasoning_effort, dev_msg_item, user_input_content
    )

    metrics_obj = None
    error = None
    incomplete_details = None

    try:
        start_time = time.time()
        generated_text = runner.generate_batch_gpt(
            conversation=convo,
            max_tokens=runner.get_effective_max_tokens(request.max_output_tokens),
            temperature=request.temperature or 1,
            top_p=request.top_p or 1,
            use_chat_template=True,
        )

        # Metrics for batch generation (approximate)
        generation_time = time.time() - start_time

        completed_at = int(time.time())
        status = "completed"
        error = None
        incomplete_details = None
        # Calculate token usage
        usage = _calc_usage(runner, user_input_content, generated_text)
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


def get_reasoning_effort(reasoning_effort_enum: ReasoningEffortEnum | None):
    reasoning_effort: ReasoningEffort
    match reasoning_effort_enum:
        case ReasoningEffortEnum.high:
            reasoning_effort = ReasoningEffort.HIGH
        case ReasoningEffortEnum.medium:
            reasoning_effort = ReasoningEffort.MEDIUM
        case ReasoningEffortEnum.low:
            reasoning_effort = ReasoningEffort.LOW
        case ReasoningEffortEnum.xhigh:
            reasoning_effort = ReasoningEffort.HIGH
        case _:
            raise TypeError("unknow reasoing effort")
    return reasoning_effort


def build_harmony_conversation(
    reasoning_effort: ReasoningEffort,
    dev_msg_item: DeveloperMessageItemParam | None,
    user_input: str,
):
    system_message = SystemContent.new().with_reasoning_effort(reasoning_effort)
    dev_message: DeveloperContent = DeveloperContent.new()
    if isinstance(dev_msg_item, DeveloperMessageItemParam):
        dev_message = DeveloperContent.new().with_instructions(
            dev_msg_item.content.root
        )  # pyright: ignore

    convo = Conversation.from_messages(
        [
            Message.from_role_and_content(Role.SYSTEM, system_message),
            Message.from_role_and_content(Role.DEVELOPER, dev_message),
            Message.from_role_and_content(Role.USER, user_input),
        ]
    )
    return convo


def is_harmony_family(model_name: str):
    return ReasoningExtractor.detect_model_type(model_name) == "gpt-oss"
