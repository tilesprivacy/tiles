from unittest.mock import Mock, patch

import httpx
import pytest

from server.config import LLAMA_CONFIG_FIELDS, LlamaConfig, get_llama_config


@pytest.fixture(autouse=True)
def _reset_llama_config_cache():
    # The TTL cache in config.py would otherwise persist values across tests.
    import server.config

    server.config._reset_llama_config_cache()
    yield
    server.config._reset_llama_config_cache()


def test_get_llama_config_handles_null_llama_config():
    response = Mock()
    response.json.return_value = {"llama": None}

    # Daemon reachable with null/empty llama means "no overrides" — do not
    # fall through to ~/.config/tiles/config.toml.
    with (
        patch("server.config.httpx.get", return_value=response),
        patch(
            "server.config._read_llama_config_from_toml",
            side_effect=AssertionError("must not fall back when daemon answers"),
        ),
    ):
        assert get_llama_config() == {}


def test_get_llama_config_returns_present_llama_values():
    response = Mock()
    response.json.return_value = {
        "llama": {
            "context_length": 20000,
            "gpu_layers": 12,
            "offload_kqv": False,
            "batch_size": 128,
            "n_cpu_moe": 12,
            "flash_attn": True,
            "load_mode": "auto",
        }
    }

    with patch("server.config.httpx.get", return_value=response):
        assert get_llama_config() == {
            "context_length": 20000,
            "gpu_layers": 12,
            "offload_kqv": False,
            "batch_size": 128,
            "n_cpu_moe": 12,
            "flash_attn": True,
            "load_mode": "auto",
        }


def test_get_llama_config_falls_back_to_toml_when_daemon_down():
    with (
        patch("server.config.httpx.get", side_effect=httpx.ConnectError("down")),
        patch(
            "server.config._read_llama_config_from_toml",
            return_value={"gpu_layers": 4},
        ),
    ):
        assert get_llama_config() == {"gpu_layers": 4}


def test_canonical_field_list_matches_python_model():
    assert set(LLAMA_CONFIG_FIELDS) == set(LlamaConfig.model_fields.keys())


def test_llama_config_fields_match_rust_struct():
    """Guard against drift between the Python LlamaConfig and the Rust twin in
    tiles/src/utils/config.rs. Adding/removing a field on either side without
    the other will fail this test.
    """
    import re
    from pathlib import Path

    rust_path = Path(__file__).resolve().parents[2] / "tiles/src/utils/config.rs"
    src = rust_path.read_text(encoding="utf-8")
    match = re.search(r"struct LlamaConfig\b.*?\{(.*?)\}", src, re.DOTALL)
    assert match, "LlamaConfig struct not found in tiles/src/utils/config.rs"
    rust_fields = set(re.findall(r"pub (\w+)\s*:", match.group(1)))
    assert rust_fields, "no pub fields parsed from Rust LlamaConfig"
    assert rust_fields == set(LLAMA_CONFIG_FIELDS)
    assert rust_fields == set(LlamaConfig.model_fields.keys())
