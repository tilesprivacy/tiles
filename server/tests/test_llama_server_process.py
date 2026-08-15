from pathlib import Path
from unittest.mock import Mock, patch

from server.backend.llama_server import process
from server.backend.llama_server.process import (
    build_llama_server_command,
    ensure_running,
    is_server_ready,
)


def _reset_process_state():
    process._process = None
    process._loaded_gguf = None
    process._loaded_config_key = None


def test_is_server_ready_requires_health_ok():
    loading = Mock(status_code=503)
    ready = Mock(status_code=200, json=Mock(return_value={"status": "ok"}))
    not_ready = Mock(status_code=200, json=Mock(return_value={"status": "loading model"}))

    with patch("server.backend.llama_server.process.httpx.get", return_value=loading):
        assert is_server_ready() is False
    with patch("server.backend.llama_server.process.httpx.get", return_value=not_ready):
        assert is_server_ready() is False
    with patch("server.backend.llama_server.process.httpx.get", return_value=ready):
        assert is_server_ready() is True


def test_build_llama_server_command_omits_unset_flags(tmp_path: Path):
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")

    with patch(
        "server.backend.llama_server.process.resolve_llama_server_binary",
        return_value="/usr/bin/llama-server",
    ):
        cmd = build_llama_server_command(gguf, {})

    assert cmd == [
        "/usr/bin/llama-server",
        "--host",
        "127.0.0.1",
        "--port",
        "18080",
        "-m",
        str(gguf),
        "--jinja",
    ]


def test_build_llama_server_command_includes_optional_flags(tmp_path: Path):
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")
    config = {
        "context_length": 32768,
        "gpu_layers": 12,
        "offload_kqv": False,
        "batch_size": 128,
        "n_cpu_moe": 12,
        "flash_attn": True,
        "no_mmap": True,
        "mtp": False,
    }

    with patch(
        "server.backend.llama_server.process.resolve_llama_server_binary",
        return_value="/usr/bin/llama-server",
    ):
        cmd = build_llama_server_command(gguf, config)

    assert cmd[0] == "/usr/bin/llama-server"
    assert "-m" in cmd and str(gguf) in cmd
    assert "-c" in cmd and "32768" in cmd
    assert "-ngl" in cmd and "12" in cmd
    assert "--no-kv-offload" in cmd
    assert "--n-cpu-moe" in cmd and "12" in cmd
    assert "--flash-attn" in cmd and "on" in cmd
    assert "--no-mmap" in cmd
    assert "--jinja" in cmd
    assert "--spec-type" not in cmd


def test_mtp_auto_enables_when_head_present(tmp_path: Path):
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")
    (tmp_path / "mtp-gemma-4-12b-it.gguf").write_bytes(b"x")

    with patch(
        "server.backend.llama_server.process.resolve_llama_server_binary",
        return_value="/usr/bin/llama-server",
    ):
        cmd = build_llama_server_command(gguf, {})

    assert "--spec-type" in cmd and "draft-mtp" in cmd
    draft_idx = cmd.index("--spec-draft-model")
    assert cmd[draft_idx + 1] == str(tmp_path / "mtp-gemma-4-12b-it.gguf")


def test_mtp_stays_off_without_head(tmp_path: Path):
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")

    with patch(
        "server.backend.llama_server.process.resolve_llama_server_binary",
        return_value="/usr/bin/llama-server",
    ):
        cmd = build_llama_server_command(gguf, {})

    assert "--spec-type" not in cmd
    assert "--spec-draft-model" not in cmd


def test_mtp_explicit_true_without_head_warns(tmp_path: Path):
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")

    with (
        patch(
            "server.backend.llama_server.process.resolve_llama_server_binary",
            return_value="/usr/bin/llama-server",
        ),
        patch("server.backend.llama_server.process.logger") as logger,
    ):
        cmd = build_llama_server_command(gguf, {"mtp": True})

    assert "--spec-type" not in cmd
    assert logger.warning.called


def test_mtp_explicit_false_overrides_head(tmp_path: Path):
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")
    (tmp_path / "mtp-gemma-4-12b-it.gguf").write_bytes(b"x")

    with patch(
        "server.backend.llama_server.process.resolve_llama_server_binary",
        return_value="/usr/bin/llama-server",
    ):
        cmd = build_llama_server_command(gguf, {"mtp": False})

    assert "--spec-type" not in cmd
    assert "--spec-draft-model" not in cmd


def test_ensure_running_reuses_running_server(tmp_path: Path):
    _reset_process_state()
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")
    config = {"gpu_layers": 8}

    # A fake already-running process that is alive and reports ready.
    fake_proc = Mock()
    fake_proc.poll.return_value = None

    with (
        patch("server.backend.llama_server.process.subprocess.Popen", return_value=fake_proc) as popen,
        patch("server.backend.llama_server.process.is_server_ready", return_value=True),
        patch("server.backend.llama_server.process.build_llama_server_command", return_value=["llama-server"]),
        patch("server.backend.llama_server.process.resolve_llama_server_binary", return_value="llama-server"),
    ):
        ensure_running(gguf, config)
        ensure_running(gguf, config)

    # Second call must not spawn again — same gguf + config, server already ready.
    assert popen.call_count == 1


def test_ensure_running_restarts_on_config_change(tmp_path: Path):
    _reset_process_state()
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")

    fake_proc = Mock()
    fake_proc.poll.return_value = None

    with (
        patch("server.backend.llama_server.process.subprocess.Popen", return_value=fake_proc) as popen,
        patch("server.backend.llama_server.process.is_server_ready", return_value=True),
        patch("server.backend.llama_server.process.build_llama_server_command", return_value=["llama-server"]),
        patch("server.backend.llama_server.process.resolve_llama_server_binary", return_value="llama-server"),
        patch("server.backend.llama_server.process.stop") as stop,
    ):
        ensure_running(gguf, {"gpu_layers": 8})
        ensure_running(gguf, {"gpu_layers": 99})

    # Different config key -> the server is respawned (stop is called before
    # every spawn; the first call is a no-op since no prior process exists).
    assert popen.call_count == 2
    assert stop.call_count == 2
