"""
Llama.cpp model runner with direct API integration.
Provides MLX-parity run experience with streaming and interactive chat
for Linux backends using llama-cpp-python.
"""

from __future__ import annotations

import gc
import ctypes
import importlib.util
import json
import os
import sys
import time
from collections.abc import Iterator
from pathlib import Path
from typing import TYPE_CHECKING, Dict, Optional, Union

if TYPE_CHECKING:
    from llama_cpp import Llama

from ..reasoning_utils import ReasoningExtractor, StreamingReasoningParser
from ..schemas import GenerationMetrics, ToolCallStart

# Common end-of-sequence tokens across model families
_COMMON_STOP_TOKENS = frozenset([
    "\x3c/s\x3e",           # &lt;/s&gt;
    "\x3c|endoftext|\x3e",  # &lt;|endoftext|&gt;
    "\x3c|im_end|\x3e",     # &lt;|im_end|&gt;
    "\x3c|eot_id|\x3e",     # &lt;|eot_id|&gt;
])

# Chat turn markers to prevent self-conversation
_CHAT_STOP_TOKENS = frozenset([
    "\nHuman:",
    "\nAssistant:",
    "\nYou:",
    "\n\nHuman:",
    "\n\nAssistant:",
    "\n\nYou:",
    "\nH:",
    "\nA:",
    "\nY:",
    "\n\nH:",
    "\n\nA:",
    "\n\nY:",
])


def _preload_cuda_runtime_libs() -> None:
    """Load CUDA runtime wheels before importing llama-cpp-python."""
    if sys.platform != "linux":
        return

    module_names = ("nvidia.cuda_runtime", "nvidia.cublas")
    lib_names = ("libcudart.so.12", "libcublasLt.so.12", "libcublas.so.12")

    for module_name in module_names:
        spec = importlib.util.find_spec(module_name)
        if spec is None:
            continue

        locations = list(spec.submodule_search_locations or [])
        if not locations:
            continue

        lib_dir = Path(locations[0]) / "lib"
        if not lib_dir.exists():
            continue

        os.environ["LD_LIBRARY_PATH"] = (
            f"{lib_dir}:{os.environ.get('LD_LIBRARY_PATH', '')}"
        )
        for lib_name in lib_names:
            lib_path = lib_dir / lib_name
            if lib_path.exists():
                ctypes.CDLL(str(lib_path), mode=ctypes.RTLD_GLOBAL)


def get_model_context_length_gguf(
    model_path: str, configured_max_ctx: int | None = None
) -> int:
    """Extract context length from config.json alongside the GGUF file.

    Args:
        model_path: Path to the model directory

    Returns:
        Maximum context length for the model, capped by the configured llama
        context or 30000 by default. If model metadata is unavailable, use
        that cap.
    """
    max_ctx = configured_max_ctx or 30000
    config_path = os.path.join(model_path, "config.json")
    try:
        with open(config_path) as f:
            config = json.load(f)
        for key in [
            "max_position_embeddings",
            "n_positions",
            "context_length",
            "max_sequence_length",
            "seq_len",
        ]:
            if key in config:
                raw = config[key]
                if raw > max_ctx:
                    print(
                        f"[INFO] Model context {raw} exceeds configured limit, "
                        f"capping to {max_ctx}"
                    )
                return min(raw, max_ctx)
    except (FileNotFoundError, json.JSONDecodeError, KeyError):
        pass
    return max_ctx


class LlamaRunner:
    """Direct llama.cpp model runner with streaming and interactive capabilities.

    Mirrors the MLXRunner contract so linux.py can use it as a drop-in
    replacement for mlx.py's runner.
    """

    model_path: Path
    model: Llama | None
    _stop_tokens: list[str] | None
    _message_end_tokens: list[str] | None
    _chat_stop_tokens: list[str] | None
    _context_length: int | None
    _is_reasoning_model: bool
    _reasoning_start: str | None
    _reasoning_end: str | None
    _final_start: str | None
    verbose: bool
    _model_loaded: bool

    def __init__(
        self,
        model_path: str,
        verbose: bool = False,
        llama_config: dict | None = None,
    ):
        """Initialize the runner with a model.

        Args:
            model_path: Path to the cached model directory
            verbose: Show detailed output
        """
        self.model_path = Path(model_path)
        self.model = None
        self.llama_config = llama_config or {}

        # Stop-token state -- populated in _extract_stop_tokens()
        self._stop_tokens: list[str] | None = None
        self._message_end_tokens: list[str] | None = None
        self._chat_stop_tokens: list[str] | None = None

        # Context length -- populated in load_model()
        self._context_length: int | None = None

        # Reasoning state -- populated in _extract_stop_tokens()
        self._is_reasoning_model: bool = False
        self._reasoning_start: str | None = None
        self._reasoning_end: str | None = None
        self._final_start: str | None = None

        self.verbose = verbose
        self._model_loaded: bool = False


    def __enter__(self):
        try:
            self.load_model()
            return self
        except Exception:
            self.cleanup()
            raise

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.cleanup()
        return False

    def load_model(self):
        """Load a GGUF model via llama-cpp-python.

        Sequence:
        1. Return early if already loaded
        2. Discover the GGUF file
        3. Instantiate Llama
        4. Set context length
        5. Detect reasoning model
        6. Build stop-token policy
        7. Mark loaded
        """
        if self._model_loaded:
            if self.verbose:
                print("Model already loaded, skipping...")
            return

        _preload_cuda_runtime_libs()
        try:
            from llama_cpp import Llama
        except ImportError as exc:
            raise RuntimeError(
                "llama-cpp-python is not installed. "
                "Install it with: pip install llama-cpp-python"
            ) from exc

        if self.verbose:
            print(f"Loading model from {self.model_path}...")
        start_time = time.time()

        try:
            gguf_file = self._find_gguf_file()
            if gguf_file is None:
                raise FileNotFoundError(
                    f"No .gguf file found in {self.model_path}"
                )

            if self.verbose:
                print(f"Using GGUF file: {gguf_file}")

            configured_context_length = self.llama_config.get("context_length")
            requested_context_length = get_model_context_length_gguf(
                str(self.model_path), configured_context_length
            )
            n_gpu_layers = self.llama_config.get("gpu_layers")
            if n_gpu_layers is None:
                n_gpu_layers = 10
            offload_kqv = self.llama_config.get("offload_kqv")
            if offload_kqv is None:
                offload_kqv = True
            n_batch = self.llama_config.get("batch_size")
            if n_batch is None:
                n_batch = 512

            self._context_length = requested_context_length
            self.model = Llama(
                model_path=str(gguf_file),
                n_gpu_layers=n_gpu_layers,
                n_ctx=self._context_length,
                n_batch=n_batch,
                n_ubatch=n_batch,
                offload_kqv=offload_kqv,
                verbose=False,
                use_mmap=True,
            )

            load_time = time.time() - start_time
            if self.verbose:
                print(f"Model loaded in {load_time:.1f}s")
                print(f"Model context length: {self._context_length} tokens")

            # --- Stop-token + reasoning setup ---
            self._extract_stop_tokens()

            self._model_loaded = True

        except Exception as e:
            # Clean up partial state before re-raising
            self.model = None
            self._context_length = None
            self._stop_tokens = None
            self._message_end_tokens = None
            self._chat_stop_tokens = None
            self._is_reasoning_model = False
            self._reasoning_start = None
            self._reasoning_end = None
            self._final_start = None
            self._model_loaded = False
            raise RuntimeError(
                f"Failed to load model from {self.model_path}: {e}"
            ) from e

    def _find_gguf_file(self) -> Optional[Path]:
        """Discover the GGUF file inside model_path.

        Returns the direct .gguf path or the single .gguf file in a directory.
        """
        if self.model_path.is_file() and self.model_path.suffix == ".gguf":
            return self.model_path

        gguf_files = list(self.model_path.glob("*.gguf"))

        if not gguf_files:
            return None
        if len(gguf_files) == 1:
            return gguf_files[0]

        names = ", ".join(sorted(p.name for p in gguf_files))
        raise ValueError(
            f"Multiple .gguf files found in {self.model_path}: {names}. "
            "Use a model path that resolves to exactly one GGUF file."
        )

    def _extract_stop_tokens(self):
        """Build centralised stop-token policy.

        Uses llama-cpp-friendly sources of truth:
        * EOS token from model metadata
        * Model-family heuristics via ReasoningExtractor
        * Explicit known tokens for GPT-OSS / reasoning families
        * Chat turn markers to prevent self-conversation
        """
        stop_tokens: set[str] = set()
        message_end_tokens: set[str] = set()

        # ---- EOS from llama-cpp metadata ----
        if self.model is not None:
            try:
                eos_id = self.model.token_eos()
                eos_bytes = self.model.detokenize([eos_id])
                eos_str = eos_bytes.decode("utf-8", errors="replace")
                if eos_str:
                    stop_tokens.add(eos_str)
            except Exception:
                pass

        for token in _COMMON_STOP_TOKENS:
            stop_tokens.add(token)

        # ---- Model-family reasoning detection ----
        model_name = str(self.model_path).lower()
        model_type = ReasoningExtractor.detect_model_type(model_name)

        if model_type:
            self._is_reasoning_model = True

            if model_type in ReasoningExtractor.PATTERNS:
                markers = ReasoningExtractor.PATTERNS[model_type]["markers"]
                self._reasoning_start = markers.get("reasoning_start")
                self._reasoning_end = markers.get("reasoning_end")
                self._final_start = markers.get("final_marker")

            # For reasoning models, remove reasoning_end from stop tokens
            # (it is a separator, not a terminal)
            if self._reasoning_end:
                stop_tokens.discard(self._reasoning_end)

            # Add proper stop token for gpt-oss family
            if model_type == "gpt-oss":
                stop_tokens.add("\x3c|return|\x3e")

            # Mark message-end tokens that separate reasoning from answer
            end_token = "\x3c|end|\x3e"
            message_end_tokens.add(end_token)
            stop_tokens.discard(end_token)
        else:
            self._is_reasoning_model = False

        # ---- Finalise ----
        stop_tokens.discard(None)
        message_end_tokens.discard(None)

        self._stop_tokens = list(stop_tokens)
        self._message_end_tokens = list(message_end_tokens)
        self._chat_stop_tokens = list(_CHAT_STOP_TOKENS)

        if self.verbose:
            if self._stop_tokens:
                print(f"Stop tokens: {self._stop_tokens}")
            if self._message_end_tokens:
                print(f"Message end tokens: {self._message_end_tokens}")
            if self._is_reasoning_model:
                print(f"Reasoning model detected (type: {model_type})")


    def cleanup(self):
        if self.verbose and self._model_loaded:
            print("Cleaning up model...")

        self.model = None
        self._context_length = None
        self._stop_tokens = None
        self._message_end_tokens = None
        self._chat_stop_tokens = None
        self._is_reasoning_model = False
        self._reasoning_start = None
        self._reasoning_end = None
        self._final_start = None
        self._model_loaded = False

        gc.collect()

        if self.verbose:
            print("Cleanup complete")


    def _format_conversation(
        self, messages: list, use_chat_template: bool = True
    ) -> str:
        """Format conversation history into a prompt.

        Uses llama-cpp-python's native chat handler to apply the GGUF
        embedded chat template.  Falls back to legacy formatting.

        Args:
            messages: List of message dicts with 'role' and 'content'
            use_chat_template: Whether to attempt to use chat template

        Returns:
            Formatted conversation string
        """
        if use_chat_template and self.model is not None:
            try:
                from llama_cpp.llama_chat_format import format_chat_prompt

                result = format_chat_prompt(
                    messages=messages,
                    model=self.model,
                )
                if result:
                    return result
            except (ImportError, Exception) as e:
                if self.verbose:
                    print(
                        f"[WARNING] Native chat format failed: {e}, "
                        "falling back to legacy format"
                    )

        return self._legacy_format_conversation(messages)

    def _legacy_format_conversation(self, messages: list) -> str:
        """Legacy conversation formatting (fallback).

        Uses the Human:/Assistant: format for models without a chat
        template embedded in the GGUF.
        """
        formatted = []
        for message in messages:
            role = message["role"]
            content = message["content"]
            if role == "system":
                formatted.append(f"System: {content}")
            elif role == "user":
                formatted.append(f"Human: {content}")
            elif role == "assistant":
                formatted.append(f"Assistant: {content}")

        formatted.append("Assistant:")
        return "\n\n".join(formatted)

    def get_effective_max_tokens(
        self, requested_tokens: Optional[int], interactive: bool = False
    ) -> int:
        """Get effective max tokens based on model context and usage mode.

        Args:
            requested_tokens: Explicitly requested token count (None = default)
            interactive: True for interactive mode (full context length)

        Returns:
            Effective max tokens to use
        """
        if not self._context_length:
            fallback = 4096 if interactive else 2048
            return requested_tokens if requested_tokens is not None else fallback

        if interactive:
            if requested_tokens is None:
                return self._context_length
            return min(requested_tokens, self._context_length)

        # Server / batch mode: cap at half context for DoS protection
        server_limit = self._context_length // 2
        return min(requested_tokens or server_limit, server_limit)

    def count_text_tokens(self, text: str) -> int:
        """Count BPE tokens for a text string."""
        if not self.model or not text:
            return 0
        return len(
            self.model.tokenize(text.encode("utf-8"), add_bos=False)
        )

    def count_prompt_tokens(
        self, prompt: Union[str, list], use_chat_template: bool = True
    ) -> int:
        """Count BPE tokens for a prompt before generation."""
        if not self.model:
            return 0
        if use_chat_template:
            messages = (
                prompt
                if isinstance(prompt, list)
                else [{"role": "user", "content": prompt}]
            )
            text = self._format_conversation(messages, use_chat_template=True)
        else:
            text = prompt if isinstance(prompt, str) else json.dumps(prompt)
        return self.count_text_tokens(text)

    def _build_stop_words(self, use_chat_stop_tokens: bool = False) -> list[str]:
        """Combine native and optional chat stop tokens for llama.cpp."""
        stop_words = list(self._stop_tokens or [])
        if use_chat_stop_tokens and self._chat_stop_tokens:
            stop_words.extend(self._chat_stop_tokens)
        return list(dict.fromkeys(stop_words))

    def _clamp_max_tokens_for_prompt(
        self, max_tokens: int, prompt_token_count: int
    ) -> int:
        """Reserve context for the prompt and reject prompts that do not fit."""
        context_length = self._context_length
        if not context_length:
            return max_tokens
        if prompt_token_count >= context_length:
            if self.model is not None:
                self.model.reset()
            raise ValueError(
                f"Prompt has {prompt_token_count} tokens, but the Linux llama.cpp "
                f"context allows fewer than {context_length}. Start a new session "
                "or reduce the conversation and tool output."
            )
        return min(max_tokens, context_length - prompt_token_count)

    def generate_streaming(
        self,
        prompt: Union[str, list],
        max_tokens: int = 500,
        temperature: float = 0.7,
        top_p: float = 0.9,
        repetition_penalty: float = 1.1,
        use_chat_template: bool = True,
        use_chat_stop_tokens: bool = False,
        interactive: bool = False,
        hide_reasoning: bool = False,
    ) -> Iterator[str | GenerationMetrics]:
        """Generate text with streaming output.

        Yields str chunks, then a final GenerationMetrics object.

        Args:
            prompt: Input prompt
            max_tokens: Maximum tokens to generate
            temperature: Sampling temperature
            top_p: Top-p sampling parameter
            repetition_penalty: Penalty for repeated tokens
            repetition_context_size: Context size for repetition penalty
            use_chat_template: Apply chat template if available
            use_chat_stop_tokens: Include chat turn markers as stop tokens
            interactive: True for interactive mode (full context length)
            hide_reasoning: Suppress reasoning output for reasoning models

        Yields:
            Generated text chunks, then GenerationMetrics
        """
        if not self.model:
            raise RuntimeError("Model not loaded. Call load_model() first.")

        # Reasoning parser
        reasoning_parser = None
        if self._is_reasoning_model:
            model_type = ReasoningExtractor.detect_model_type(
                str(self.model_path)
            )
            reasoning_parser = StreamingReasoningParser(
                model_type, hide_reasoning=hide_reasoning
            )

        effective_max_tokens = self.get_effective_max_tokens(
            max_tokens, interactive
        )
        stop_words = self._build_stop_words(use_chat_stop_tokens)

        if use_chat_template:
            if isinstance(prompt, list):
                messages = prompt
            else:
                messages = [{"role": "user", "content": prompt}]
            prompt_token_count = self.count_prompt_tokens(
                messages, use_chat_template=True
            )
        else:
            text_prompt = prompt if isinstance(prompt, str) else json.dumps(prompt)
            prompt_token_count = self.count_prompt_tokens(
                text_prompt, use_chat_template=False
            )

        effective_max_tokens = self._clamp_max_tokens_for_prompt(
            effective_max_tokens, prompt_token_count
        )

        start_time = time.time()
        ttft = None
        accumulated_response = ""

        # Use llama-cpp-python's native chat completion when chat template
        if use_chat_template:
            stream = self.model.create_chat_completion(
                messages=messages,  # pyright: ignore[reportArgumentType]
                max_tokens=effective_max_tokens,
                temperature=temperature,
                top_p=top_p,
                repeat_penalty=repetition_penalty,
                stop=stop_words,
                stream=True,
            )
            # create_chat_completion yields {"choices": [{"delta": {"content": ...}}]}
            use_chat_api = True
        else:
            stream = self.model(
                text_prompt,
                max_tokens=effective_max_tokens,
                temperature=temperature,
                top_p=top_p,
                repeat_penalty=repetition_penalty,
                stop=stop_words,
                stream=True,
            )
            use_chat_api = False

        for output in stream:  # pyright: ignore[reportGeneralTypeIssues]
            if use_chat_api:
                delta = output["choices"][0].get("delta", {})  # pyright: ignore[reportArgumentType, reportAttributeAccessIssue]
                text = delta.get("content", "")
            else:
                text = output["choices"][0].get("text", "")  # pyright: ignore[reportArgumentType, reportAttributeAccessIssue]
            if not text:
                continue

            accumulated_response += text

            if ttft is None:
                ttft = time.time() - start_time

            # ---- Native stop-token check ----
            native_stop_tokens = self._stop_tokens or []
            for stop_token in native_stop_tokens:
                if stop_token in accumulated_response:
                    stop_pos = accumulated_response.find(stop_token)
                    text_before = accumulated_response[:stop_pos]
                    prev_len = len(accumulated_response) - len(text)
                    if len(text_before) > prev_len:
                        new_part = text_before[prev_len:]
                        if new_part:
                            yield from self._yield_with_reasoning(
                                new_part, reasoning_parser
                            )
                    if reasoning_parser:
                        yield from reasoning_parser.finalize()
                    yield self._make_metrics_from_response(
                        start_time, accumulated_response[:stop_pos], ttft
                    )
                    return

            # ---- Chat stop-token check (fallback) ----
            if use_chat_stop_tokens and self._chat_stop_tokens:
                for stop_token in self._chat_stop_tokens:
                    if stop_token in accumulated_response:
                        stop_pos = accumulated_response.find(stop_token)
                        text_before = accumulated_response[:stop_pos]
                        prev_len = len(accumulated_response) - len(text)
                        if len(text_before) > prev_len:
                            new_part = text_before[prev_len:]
                            if new_part:
                                yield from self._yield_with_reasoning(
                                    new_part, reasoning_parser
                                )
                        if reasoning_parser:
                            yield from reasoning_parser.finalize()
                        yield self._make_metrics_from_response(
                            start_time, text_before, ttft
                        )
                        return

            yield from self._yield_with_reasoning(text, reasoning_parser)

        # Finalize reasoning parser
        if reasoning_parser:
            yield from reasoning_parser.finalize()

        # Yield final metrics
        yield self._make_metrics_from_response(
            start_time, accumulated_response, ttft
        )

        if self.verbose:
            gen_time = time.time() - start_time
            tokens_generated = self.count_text_tokens(accumulated_response)
            tps = tokens_generated / gen_time if gen_time > 0 else 0
            print(
                f"\n\nGenerated {tokens_generated} tokens in "
                f"{gen_time:.1f}s ({tps:.1f} tokens/s)"
            )

    def _filter_end_tokens_from_response(
        self, response: str, use_chat_stop_tokens: bool = False
    ) -> str:
        """Filter end tokens from a complete response (batch mode).

        Applies the same filtering logic as streaming mode for
        consistent behaviour.

        Args:
            response: The complete generated response
            use_chat_stop_tokens: Whether to apply chat stop tokens

        Returns:
            Response with end tokens filtered out
        """
        # Native stop tokens first
        for stop_token in (self._stop_tokens or []):
            if stop_token in response:
                stop_pos = response.find(stop_token)
                if self.verbose:
                    print(
                        f"[DEBUG] Filtered stop token '{stop_token}' "
                        f"at position {stop_pos}"
                    )
                return response[:stop_pos].rstrip()

        # Chat stop tokens as fallback
        if use_chat_stop_tokens and self._chat_stop_tokens:
            for stop_token in self._chat_stop_tokens:
                if stop_token in response:
                    stop_pos = response.find(stop_token)
                    return response[:stop_pos]

        return response


    def _format_reasoning_response(self, response: str) -> str:
        """Format response from reasoning models for readability.

        For models that generate reasoning followed by a final answer,
        formats the output with clear section markers.
        """
        if not self._is_reasoning_model:
            return response

        if (
            self._reasoning_start
            and self._final_start
            and self._reasoning_start in response
            and self._final_start in response
        ):
            try:
                _before, after_start = response.split(
                    self._reasoning_start, 1
                )
                if self._reasoning_end and self._reasoning_end in after_start:
                    reasoning_content, after_reasoning = after_start.split(
                        self._reasoning_end, 1
                    )
                    if self._final_start in after_reasoning:
                        final_parts = after_reasoning.split(
                            self._final_start, 1
                        )
                        if len(final_parts) > 1:
                            final_answer = final_parts[1]
                            # Clean up channel markers
                            channel_marker = (
                                "\x3c|channel|\x3efinal\x3c|message|\x3e"
                            )
                            final_answer = final_answer.replace(
                                channel_marker, "", 1
                            )
                            parts = []
                            parts.append("\n**[Reasoning]**\n")
                            parts.append(reasoning_content.strip())
                            parts.append("\n\n---\n\n**[Answer]**\n")
                            parts.append(final_answer.strip())
                            return "\n".join(parts)
            except Exception:
                pass

        # Fallback: strip known control tokens
        cleaned = response
        control_tokens = [
            "\x3c|channel|\x3eanalysis\x3c|message|\x3e",
            "\x3c|end|\x3e",
            "\x3c|start|\x3eassistant",
            "\x3c|channel|\x3efinal\x3c|message|\x3e",
            "\x3c|return|\x3e",
        ]
        for marker in control_tokens:
            cleaned = cleaned.replace(marker, "")
        return cleaned.strip()


    def generate_batch(
        self,
        prompt: Union[str, list],
        max_tokens: int = 500,
        temperature: float = 0.7,
        top_p: float = 0.9,
        repetition_penalty: float = 1.1,
        # repetition_context_size: int = 20,
        use_chat_template: bool = True,
        interactive: bool = False,
    ) -> str:
        """Generate text in batch mode (non-streaming).

        Args:
            prompt: Input prompt
            max_tokens: Maximum tokens to generate
            temperature: Sampling temperature
            top_p: Top-p sampling parameter
            repetition_penalty: Penalty for repeated tokens
            repetition_context_size: Context size for repetition penalty
            use_chat_template: Apply chat template if available
            interactive: True for interactive mode

        Returns:
            Generated text
        """
        if not self.model:
            raise RuntimeError("Model not loaded. Call load_model() first.")

        effective_max_tokens = self.get_effective_max_tokens(
            max_tokens, interactive
        )
        stop_words = self._build_stop_words(use_chat_stop_tokens=False)

        if use_chat_template:
            if isinstance(prompt, list):
                messages = prompt
            else:
                messages = [{"role": "user", "content": prompt}]
            prompt_token_count = self.count_prompt_tokens(
                messages, use_chat_template=True
            )
        else:
            text_prompt = prompt if isinstance(prompt, str) else json.dumps(prompt)
            prompt_token_count = self.count_prompt_tokens(
                text_prompt, use_chat_template=False
            )

        effective_max_tokens = self._clamp_max_tokens_for_prompt(
            effective_max_tokens, prompt_token_count
        )

        start_time = time.time()

        # Use native chat completion for proper template handling
        if use_chat_template:
            output = self.model.create_chat_completion(
                messages=messages,  # pyright: ignore[reportArgumentType]
                max_tokens=effective_max_tokens,
                temperature=temperature,
                top_p=top_p,
                repeat_penalty=repetition_penalty,
                stop=stop_words,
                stream=False,
            )
            response = output["choices"][0]["message"].get("content", "")  # pyright: ignore[reportIndexIssue]
        else:
            output = self.model(
                text_prompt,
                max_tokens=effective_max_tokens,
                temperature=temperature,
                top_p=top_p,
                repeat_penalty=repetition_penalty,
                stop=stop_words,
                stream=False,
            )
            response = output["choices"][0].get("text", "")  # pyright: ignore[reportIndexIssue]

        # Apply end-token filtering (same as streaming)
        response = self._filter_end_tokens_from_response(
            response or "", use_chat_stop_tokens=False
        )

        # Format reasoning output
        response = self._format_reasoning_response(response)

        if self.verbose:
            gen_time = time.time() - start_time
            # Rough token count from response length
            print(f"\nGenerated in {gen_time:.1f}s")

        return response

    
    ## I have mostly focused on GPT streaming for Linux, 
    ## It is very much possible batch streaming might not be as robust
    ## Will focus on that next time.

    def generate_streaming_gpt(
        self,
        conversation,   # openai_harmony.Conversation
        max_tokens: int = 500,
        temperature: float = 0.7,
        top_p: float = 0.9,
        repetition_penalty: float = 1.1,
        # repetition_context_size: int = 20,
    ) -> Iterator[str | ToolCallStart | GenerationMetrics]:
        """Generate Harmony/GPT streaming output.

        Extracts messages from the Harmony conversation as text and
        uses llama-cpp-python's raw text generation with Harmony encoding.
        Control tokens (channel markers, start/end) are stripped and replaced
        with **[Reasoning]** / **[Answer]** headers matching the MLX backend.

        Yields:
            str chunks and tool-call metadata, then GenerationMetrics
        """
        if not self.model:
            raise RuntimeError("Model not loaded. Call load_model() first.")

        from openai_harmony import (
            HarmonyEncodingName,
            Role,
            StreamableParser,
            load_harmony_encoding,
        )

        effective_max_tokens = self.get_effective_max_tokens(
            max_tokens, False
        )

        encoding = load_harmony_encoding(HarmonyEncodingName.HARMONY_GPT_OSS)

        # Render the conversation for completion to get token IDs
        prompt_tokens = encoding.render_conversation_for_completion(
            conversation, Role.ASSISTANT
        )
        effective_max_tokens = self._clamp_max_tokens_for_prompt(
            effective_max_tokens, len(prompt_tokens)
        )

        start_time = time.time()
        tokens_generated = 0
        ttft = None

        stop_tokens = encoding.stop_tokens_for_assistant_actions()
        generator = self.model.generate(
            prompt_tokens,
            temp=temperature,
            top_p=top_p,
            repeat_penalty=repetition_penalty,
        )

        parser = StreamableParser(encoding, Role.ASSISTANT)
        is_analysis = None
        is_final = None
        is_commentary = None
        for token_id in generator:
            parser.process(token_id)

            if is_analysis is None and parser.current_channel == "analysis":
                is_analysis = True
                yield "**[Reasoning]**\n\n"

            if is_commentary is None and parser.current_channel == "commentary":
                is_commentary = True
                yield ToolCallStart(parser.current_recipient or "")

            if is_final is None and parser.current_channel == "final":
                is_final = True
                yield "\n---\n**[Answer]**\n\n"

            if ttft is None:
                ttft = time.time() - start_time

            if parser.last_content_delta:
                yield parser.last_content_delta

            tokens_generated += 1
            if token_id in stop_tokens or tokens_generated >= effective_max_tokens:
                break

        yield self._make_metrics(start_time, tokens_generated, ttft)

        if self.verbose:
            gen_time = time.time() - start_time
            tps = tokens_generated / gen_time if gen_time > 0 else 0
            print(
                f"\n\nGenerated {tokens_generated} tokens in "
                f"{gen_time:.1f}s ({tps:.1f} tokens/s)"
            )

    def generate_batch_gpt(
        self,
        conversation,   # openai_harmony.Conversation
        max_tokens: int = 500,
        temperature: float = 0.7,
        top_p: float = 0.9,
        repetition_penalty: float = 1.0,
        # repetition_context_size: int = 20,
        use_chat_template: bool = True,
        interactive: bool = False,
    ) -> str:
        """Generate Harmony/GPT output in batch mode.

        Extracts messages from the Harmony conversation as text,
        generates via llama-cpp-python's raw text generation with Harmony encoding,
        and applies reasoning formatting at the text level.
        """
        if not self.model:
            raise RuntimeError("Model not loaded. Call load_model() first.")

        from openai_harmony import HarmonyEncodingName, Role, load_harmony_encoding

        effective_max_tokens = self.get_effective_max_tokens(
            max_tokens, interactive
        )

        encoding = load_harmony_encoding(HarmonyEncodingName.HARMONY_GPT_OSS)

        # Render the conversation for completion to get token IDs
        prompt_tokens = encoding.render_conversation_for_completion(
            conversation, Role.ASSISTANT
        )
        effective_max_tokens = self._clamp_max_tokens_for_prompt(
            effective_max_tokens, len(prompt_tokens)
        )

        stop_token_ids = set(encoding.stop_tokens_for_assistant_actions())
        output_token_ids: list[int] = []
        generator = self.model.generate(
            prompt_tokens,
            temp=temperature,
            top_p=top_p,
            repeat_penalty=repetition_penalty,
        )
        for token_id in generator:
            output_token_ids.append(token_id)
            if (
                token_id in stop_token_ids
                or len(output_token_ids) >= effective_max_tokens
            ):
                break

        response = encoding.decode(output_token_ids)

        # Apply end-token filtering (same as streaming)
        response = self._filter_end_tokens_from_response(
            response or "", use_chat_stop_tokens=False
        )

        # Format reasoning output
        response = self._format_reasoning_response(response)

        if self.verbose:
            # Rough token count from response length
            print(f"\nGenerated in batch mode")

        return response

# private helpers

    def _make_metrics_from_response(
        self, start_time: float, response_text: str, ttft: float | None
    ) -> GenerationMetrics:
        """Build GenerationMetrics using actual BPE token counts."""
        filtered = self._filter_end_tokens_from_response(response_text)
        tokens_generated = self.count_text_tokens(filtered)
        return self._make_metrics(start_time, tokens_generated, ttft)

    @staticmethod
    def _make_metrics(
        start_time: float, tokens_generated: int, ttft: float | None
    ) -> GenerationMetrics:
        """Build a GenerationMetrics from timing data."""
        total_latency = time.time() - start_time
        tps = tokens_generated / total_latency if total_latency > 0 else 0
        ttft_ms = (ttft * 1000) if ttft is not None else 0
        return GenerationMetrics(
            ttft_ms=ttft_ms,
            total_tokens=tokens_generated,
            tokens_per_second=tps,
            total_latency_s=total_latency,
        )

    @staticmethod
    def _yield_with_reasoning(text, reasoning_parser):
        """Yield text through the reasoning parser if active, else raw."""
        if reasoning_parser:
            yield from reasoning_parser.process_token(text)
        else:
            yield text
