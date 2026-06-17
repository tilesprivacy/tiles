import pytest
from openai_harmony import (
    Conversation,
    HarmonyEncodingName,
    Message,
    Role,
    load_harmony_encoding,
)

from server.backend.llama_cpp_runner import (
    LlamaRunner,
    get_model_context_length_gguf,
)
from server.schemas import ToolCallStart


ENCODING = load_harmony_encoding(HarmonyEncodingName.HARMONY_GPT_OSS)


class FakeModel:
    def __init__(self, completion):
        self.completion = completion
        self.was_reset = False

    def generate(self, prompt_tokens, **kwargs):
        yield from ENCODING.encode(self.completion, allowed_special="all")

    def reset(self):
        self.was_reset = True


def collect_completion(completion):
    runner = LlamaRunner("/tmp/gpt-oss")
    runner.model = FakeModel(completion)
    runner._context_length = 8192

    return list(
        runner.generate_streaming_gpt(
            Conversation.from_messages([]), max_tokens=128
        )
    )


def test_gpt_streaming_emits_tool_call_without_answer_marker():
    chunks = collect_completion(
        "<|channel|>analysis<|message|>Need to read file.<|end|>"
        "<|start|>assistant<|channel|>commentary to=read <|constrain|>json"
        '<|message|>{"path":"changelog.md"}<|call|>'
    )

    assert ToolCallStart("read") in chunks
    assert "".join(chunk for chunk in chunks if isinstance(chunk, str)) == (
        "**[Reasoning]**\n\n"
        "Need to read file."
        '{"path":"changelog.md"}'
    )


def test_gpt_streaming_emits_final_answer_marker():
    chunks = collect_completion(
        "<|channel|>analysis<|message|>Say hello.<|end|>"
        "<|start|>assistant<|channel|>final<|message|>Hello.<|return|>"
    )

    assert "".join(chunk for chunk in chunks if isinstance(chunk, str)) == (
        "**[Reasoning]**\n\n"
        "Say hello."
        "\n---\n**[Answer]**\n\n"
        "Hello."
    )


def test_gpt_streaming_rejects_prompt_that_exceeds_context_and_resets_model():
    runner = LlamaRunner("/tmp/gpt-oss")
    model = FakeModel("")
    runner.model = model
    runner._context_length = 64
    conversation = Conversation.from_messages(
        [Message.from_role_and_content(Role.USER, "word " * 200)]
    )

    with pytest.raises(ValueError, match="Start a new session"):
        list(runner.generate_streaming_gpt(conversation, max_tokens=16))

    assert model.was_reset


def test_gguf_context_length_caps_at_30000_by_default(tmp_path):
    (tmp_path / "config.json").write_text('{"context_length": 131072}')

    assert get_model_context_length_gguf(str(tmp_path)) == 30000


def test_gguf_context_length_uses_configured_cap(tmp_path):
    (tmp_path / "config.json").write_text('{"context_length": 131072}')

    assert get_model_context_length_gguf(str(tmp_path), 12000) == 12000


def test_gguf_context_length_uses_configured_cap_when_config_missing(tmp_path):
    assert get_model_context_length_gguf(str(tmp_path), 2048) == 2048


def test_gguf_context_length_uses_configured_cap_when_config_invalid(tmp_path):
    (tmp_path / "config.json").write_text("{")

    assert get_model_context_length_gguf(str(tmp_path), 3072) == 3072


class TokenizingFakeModel:
    reset_called = False

    def tokenize(self, text, add_bos=False):
        return text.decode("utf-8").split()

    def reset(self):
        self.reset_called = True


def test_build_stop_words_deduplicates():
    runner = LlamaRunner("/tmp/model")
    runner._stop_tokens = ["</s>", "<|return|>"]
    runner._chat_stop_tokens = ["</s>", "\nHuman:"]

    assert runner._build_stop_words(use_chat_stop_tokens=True) == [
        "</s>",
        "<|return|>",
        "\nHuman:",
    ]


def test_clamp_max_tokens_for_prompt_reserves_context():
    runner = LlamaRunner("/tmp/model")
    runner._context_length = 100
    runner.model = TokenizingFakeModel()

    assert runner._clamp_max_tokens_for_prompt(80, 30) == 70


def test_clamp_max_tokens_for_prompt_rejects_oversized_prompt():
    runner = LlamaRunner("/tmp/model")
    runner._context_length = 64
    model = TokenizingFakeModel()
    runner.model = model

    with pytest.raises(ValueError, match="Start a new session"):
        runner._clamp_max_tokens_for_prompt(16, 64)

    assert model.reset_called is True


def test_batch_gpt_generates_from_prompt_token_ids():
    completion = (
        "<|channel|>analysis<|message|>Say hello.<|end|>"
        "<|start|>assistant<|channel|>final<|message|>Hello.<|return|>"
    )
    output_token_ids = ENCODING.encode(completion, allowed_special="all")

    class BatchFakeModel:
        prompt_tokens = None

        def generate(self, prompt_tokens, **kwargs):
            BatchFakeModel.prompt_tokens = prompt_tokens
            yield from output_token_ids

        def reset(self):
            pass

    runner = LlamaRunner("/tmp/gpt-oss")
    runner.model = BatchFakeModel()
    runner._context_length = 8192
    runner._is_reasoning_model = True
    runner._reasoning_start = "<|channel|>analysis<|message|>"
    runner._reasoning_end = "<|end|>"
    runner._final_start = "<|channel|>final<|message|>"

    response = runner.generate_batch_gpt(
        Conversation.from_messages([]), max_tokens=128
    )

    assert BatchFakeModel.prompt_tokens == ENCODING.render_conversation_for_completion(
        Conversation.from_messages([]), Role.ASSISTANT
    )
    assert "**[Reasoning]**" in response
    assert "Hello." in response
