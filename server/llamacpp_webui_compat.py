"""
Minimal responses for ggml-org/llama.cpp tools/server/webui (SvelteKit).

The Web UI expects GET /props (llama-server shape) and enriched /v1/models entries.
OpenAI-only clients can ignore these fields.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def _default_params() -> dict[str, Any]:
    return {
        "n_predict": -1,
        "seed": -1,
        "temperature": 0.8,
        "dynatemp_range": 0.0,
        "dynatemp_exponent": 1.0,
        "top_k": 40,
        "top_p": 0.95,
        "min_p": 0.05,
        "top_n_sigma": -1.0,
        "xtc_probability": 0.0,
        "xtc_threshold": 0.1,
        "typ_p": 1.0,
        "repeat_last_n": 64,
        "repeat_penalty": 1.0,
        "presence_penalty": 0.0,
        "frequency_penalty": 0.0,
        "dry_multiplier": 0.0,
        "dry_base": 1.75,
        "dry_allowed_length": 2,
        "dry_penalty_last_n": -1,
        "dry_sequence_breakers": [],
        "mirostat": 0,
        "mirostat_tau": 5.0,
        "mirostat_eta": 0.1,
        "stop": [],
        "max_tokens": 8192,
        "n_keep": 0,
        "n_discard": 0,
        "ignore_eos": False,
        "stream": True,
        "logit_bias": [],
        "n_probs": 0,
        "min_keep": 0,
        "grammar": "",
        "grammar_lazy": False,
        "grammar_triggers": [],
        "preserved_tokens": [],
        "chat_format": "Content-only",
        "reasoning_format": "none",
        "reasoning_in_content": False,
        "generation_prompt": "",
        "samplers": "penalties;dry;top_n_sigma;top_k;typ_p;top_p;min_p;xtc;temperature",
        "backend_sampling": False,
        "speculative.n_max": 16,
        "speculative.n_min": 0,
        "speculative.p_min": 0.75,
        "timings_per_token": False,
        "post_sampling_probs": False,
        "lora": [],
    }


def _read_n_ctx(model_cache_path: str | None) -> int:
    if not model_cache_path:
        return 8192
    p = Path(model_cache_path) / "config.json"
    if not p.is_file():
        return 8192
    try:
        with open(p, encoding="utf-8") as f:
            cfg = json.load(f)
        if not isinstance(cfg, dict):
            return 8192
        for key in (
            "max_position_embeddings",
            "n_positions",
            "context_length",
            "max_sequence_length",
        ):
            v = cfg.get(key)
            if isinstance(v, int) and v > 0:
                return min(v, 262144)
    except (OSError, json.JSONDecodeError, TypeError, AttributeError):
        pass
    return 8192


def tiles_props_for_webui(
    model_cache_path: str | None,
    *,
    build_info: str = "tiles-python-server",
) -> dict[str, Any]:
    """Shape compatible with ApiLlamaCppServerProps in llama.cpp webui."""
    n_ctx = _read_n_ctx(model_cache_path)
    path = model_cache_path or ""
    params = _default_params()
    return {
        "default_generation_settings": {
            "id": 0,
            "id_task": 0,
            "n_ctx": n_ctx,
            "speculative": False,
            "is_processing": False,
            "params": params,
            "prompt": "",
            "next_token": {
                "has_next_token": False,
                "has_new_line": False,
                "n_remain": 0,
                "n_decoded": 0,
                "stopping_word": "",
            },
        },
        "total_slots": 1,
        "model_path": path,
        "role": "model",
        "modalities": {"vision": False, "audio": False},
        "chat_template": "",
        "bos_token": "",
        "eos_token": "",
        "build_info": build_info,
        "webui_settings": {},
    }


def openai_model_entry_with_llama_fields(
    model_id: str,
    created: int,
    *,
    model_cache_path: str | None,
) -> dict[str, Any]:
    """Single /v1/models row with fields the llama webui maps as ApiModelDataEntry."""
    path = model_cache_path or ""
    return {
        "id": model_id,
        "object": "model",
        "created": created,
        "owned_by": "tiles",
        "in_cache": bool(model_cache_path),
        "path": path,
        "status": {"value": "loaded"},
    }
