from dataclasses import dataclass
from enum import Enum, auto
from typing import Any, Dict, List, Union, override

from openresponses_types import ReasoningParam, TruncationEnum
from openresponses_types.types import (
    AssistantMessageItemParam,
    DeveloperMessageItemParam,
    Error,
    FunctionCallItemParam,
    FunctionCallOutputItemParam,
    FunctionToolParam,
    IncompleteDetails,
    ItemReferenceParam,
    ReasoningEffortEnum,
    ReasoningItemParam,
    StreamOptionsParam,
    SystemMessageItemParam,
    ToolChoiceParam,
    UserMessageItemParam,
)
from pydantic import BaseModel, ConfigDict, Field, field_validator


class CompletionRequest(BaseModel):
    model: str
    prompt: Union[str, List[str]]
    max_tokens: int | None = None
    temperature: float | None = 0.7
    top_p: float | None = 0.9
    stream: bool | None = False
    stop: Union[str, List[str]] | None = None
    repetition_penalty: float | None = 1.1


class ChatMessage(BaseModel):
    """OpenAI chat message; content may be a string or multimodal parts (llama Web UI)."""

    model_config = ConfigDict(extra="ignore")

    role: str = Field(..., pattern="^(system|user|assistant)$")
    content: Union[str, List[Any]]

    @field_validator("content", mode="before")
    @classmethod
    def normalize_content(cls, v: Any) -> str:
        if isinstance(v, str):
            return v
        if isinstance(v, list):
            parts: list[str] = []
            for item in v:
                if isinstance(item, str):
                    parts.append(item)
                elif isinstance(item, dict):
                    t = item.get("text")
                    if isinstance(t, str):
                        parts.append(t)
                    elif item.get("type") == "text" and isinstance(item.get("text"), str):
                        parts.append(item["text"])
            return "\n".join(parts) if parts else ""
        return str(v)


class ChatCompletionRequest(BaseModel):
    model_config = ConfigDict(extra="ignore")

    # Optional when a model is already loaded via POST /start (llama Web UI single-model mode).
    model: str | None = None
    messages: List[ChatMessage]
    chat_start: bool = False
    python_code: str = ""
    # CLI memory mode sends this; OpenAI-compatible clients omit it (stateless chat).
    input: str | None = Field(
        default=None,
        description="When set, use legacy memory-agent turn (same host as Tiles CLI memory mode).",
    )
    max_tokens: int | None = None
    temperature: float | None = 0.7
    top_p: float | None = 0.9
    stream: bool | None = False
    stop: Union[str, List[str]] | None = None
    repetition_penalty: float | None = 1.1

    @field_validator("max_tokens", mode="before")
    @classmethod
    def normalize_max_tokens(cls, v: Any) -> int | None:
        # llama.cpp Web UI sends -1 for "infinite"
        if v == -1:
            return None
        return v


class CompletionResponse(BaseModel):
    id: str
    object: str = "text_completion"
    created: int
    model: str
    choices: List[Dict[str, Any]]
    usage: Dict[str, int]


class ChatCompletionResponse(BaseModel):
    id: str
    object: str = "chat.completion"
    created: int
    model: str
    choices: List[Dict[str, Any]]
    # usage: Dict[str, int]


class ModelInfo(BaseModel):
    id: str
    object: str = "model"
    owned_by: str = "mlx-knife"
    permission: List = []
    context_length: int | None = None


class StartRequest(BaseModel):
    model: str
    memory_path: str
    system_prompt: str
    model_cache_path: str


class RouterModelsLoadBody(BaseModel):
    model_config = ConfigDict(extra="ignore")

    model: str
    extra_args: List[str] | None = None


class downloadRequest(BaseModel):
    model: str


class ResponsesRequest(BaseModel):
    model: str = "mlx-community/gpt-oss-20b-MXFP4-Q4"
    input: (
        str
        | list[
            ItemReferenceParam
            | ReasoningItemParam
            | UserMessageItemParam
            | SystemMessageItemParam
            | DeveloperMessageItemParam
            | AssistantMessageItemParam
            | FunctionCallItemParam
            | FunctionCallOutputItemParam
        ]
    )
    reasoning: ReasoningParam = ReasoningParam(
        effort=ReasoningEffortEnum.medium, summary=None
    )
    previous_response_id: str | None = None
    stream: bool | None = False
    stream_options: StreamOptionsParam | None = None
    tools: list[FunctionToolParam] | None = None
    tool_choice: ToolChoiceParam | None = None
    temperature: float | None = 1
    top_p: float | None = 1
    max_output_tokens: int | None = None
    store: bool = False
    # other service tiers are default, flex, priority
    service_tier: str = "auto"
    top_logprobs: int = 0
    # can put in the Developer msg if none there
    instructions: str | None = None
    # auto/disabled, returns 400 on disabled
    truncation: TruncationEnum = TruncationEnum.disabled
    prompt_cache: str | None = None
    safety_identifier: str | None = None
    max_tool_calls: int | None = None
    background: bool = False


class ResponsesResponse(BaseModel):
    id: str
    object: str = "response"
    created_at: int
    status: str
    completed_at: int | None = None
    error: Error | None = None
    incomplete_details: IncompleteDetails | None = None
    instructions: str | None = None
    max_output_tokens: int | None = None
    model: str
    output: list[Dict[str, Any]]
    parallel_tool_calls: bool = True
    previous_response_id: str = ""
    reasoning: Dict[str, Any] | None = Field(default_factory=dict)
    store: bool = True
    temperature: float = 1.0
    text: Dict[str, Any] = Field(default_factory=lambda: {"format": {"type": "text"}})
    tool_choice: Union[str, Dict[str, Any]] = "auto"
    tools: List[Dict[str, Any]] = Field(default_factory=list)
    top_p: float = 1.0
    truncation: str = "disabled"
    usage: Dict[str, Any]
    user: str | None = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


@dataclass
class GenerationMetrics:
    """Benchmarking metrics for token generation."""

    ttft_ms: float  # Time to first token in milliseconds
    total_tokens: int  # Total tokens generated
    tokens_per_second: float  # Throughput
    total_latency_s: float  # End-to-end latency in seconds
