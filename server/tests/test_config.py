import os
from unittest.mock import Mock, patch

import httpx
import pytest

from server.config import (
    PORT,
    get_daemon_config,
    get_llama_config,
    get_llama_server,
    pid_path,
    remove_pid_file,
    write_pid_file,
)


@pytest.fixture(autouse=True)
def _reset_daemon_config_cache():
    # The TTL cache in config.py would otherwise persist values across tests.
    import server.config

    server.config._reset_daemon_config_cache()
    yield
    server.config._reset_daemon_config_cache()


def _daemon_answers(config: dict) -> Mock:
    response = Mock()
    response.json.return_value = config
    return response


def test_get_llama_config_handles_null_llama_config():
    with patch("server.config.httpx.get", return_value=_daemon_answers({"llama": None})):
        assert get_llama_config() == {}
        assert get_daemon_config().llama_server_args == ()


def test_get_llama_config_returns_what_is_set():
    llama = {
        "context_length": 20000,
        "gpu_layers": 12,
        "offload_kqv": False,
        "batch_size": None,
        "load_mode": "auto",
    }

    with patch("server.config.httpx.get", return_value=_daemon_answers({"llama": llama})):
        assert get_llama_config() == {
            "context_length": 20000,
            "gpu_layers": 12,
            "offload_kqv": False,
            "load_mode": "auto",
        }


def test_a_setting_this_server_has_never_heard_of_comes_through():
    config = {
        "llama": {"some_new_setting": 3},
        "llama_server_args": ["--some-new-flag", "3"],
    }

    with patch("server.config.httpx.get", return_value=_daemon_answers(config)):
        assert get_llama_config() == {"some_new_setting": 3}
        assert get_daemon_config().llama_server_args == ("--some-new-flag", "3")


def test_the_daemon_names_the_engines_llama_server():
    config = {"llama": None, "llama_server": "/engines/vulkan-b1/llama-server"}

    with patch("server.config.httpx.get", return_value=_daemon_answers(config)):
        assert get_llama_server() == "/engines/vulkan-b1/llama-server"


def test_without_the_daemon_nothing_is_set():
    with patch("server.config.httpx.get", side_effect=httpx.ConnectError("down")):
        assert get_llama_config() == {}
        assert get_daemon_config().llama_server_args == ()
        # and the bundled llama-server
        assert get_llama_server() is None


def test_a_failed_config_read_keeps_the_daemons_last_answer():
    import server.config

    config = {
        "llama": {"context_length": 8192},
        "llama_server_args": ["-c", "8192"],
        "llama_server": "/engines/cuda-13-b1/llama-server",
    }
    with patch("server.config.httpx.get", return_value=_daemon_answers(config)):
        assert get_llama_server() == "/engines/cuda-13-b1/llama-server"

    # the cached answer runs out, and the daemon doesn't answer this time
    server.config._daemon_config_cache = None
    with patch("server.config.httpx.get", side_effect=httpx.ReadTimeout("busy")):
        assert get_llama_server() == "/engines/cuda-13-b1/llama-server"
        assert get_llama_config() == {"context_length": 8192}
        assert get_daemon_config().llama_server_args == ("-c", "8192")


def test_pid_file_written_and_removed(tmp_path, monkeypatch):
    monkeypatch.setenv("XDG_STATE_HOME", str(tmp_path))
    write_pid_file()
    path = tmp_path / "tiles" / f"inference-{PORT}.pid"
    assert path.read_text() == str(os.getpid())

    remove_pid_file()
    assert not path.exists()


def test_remove_pid_file_leaves_another_servers_pid(tmp_path, monkeypatch):
    monkeypatch.setenv("XDG_STATE_HOME", str(tmp_path))
    path = pid_path()
    path.parent.mkdir(parents=True)
    path.write_text("424242")

    remove_pid_file()
    assert path.read_text() == "424242"
