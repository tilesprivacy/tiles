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
    process._startup_warnings.clear()


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


def test_mtp_off_by_default_even_when_head_present(tmp_path: Path):
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")
    (tmp_path / "mtp-gemma-4-12b-it.gguf").write_bytes(b"x")

    with patch(
        "server.backend.llama_server.process.resolve_llama_server_binary",
        return_value="/usr/bin/llama-server",
    ):
        cmd = build_llama_server_command(gguf, {})

    assert "--spec-type" not in cmd
    assert "--spec-draft-model" not in cmd


def test_mtp_explicit_true_enables_when_head_present(tmp_path: Path):
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")
    (tmp_path / "mtp-gemma-4-12b-it.gguf").write_bytes(b"x")

    with patch(
        "server.backend.llama_server.process.resolve_llama_server_binary",
        return_value="/usr/bin/llama-server",
    ):
        cmd = build_llama_server_command(gguf, {"mtp": True})

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


def _hf_cache_layout(tmp_path: Path) -> tuple[Path, Path]:
    blobs = tmp_path / "blobs"
    blobs.mkdir()
    snapshot = tmp_path / "snapshots" / "fc034cfff751157913579611efad8462ac1be606"
    snapshot.mkdir(parents=True)

    links = []
    for blob_sha, filename in (
        ("0a270ec9fe6b34f4a0d33992b6135117b484ebc4766ab76b51d4ae8c457e4c42", "gemma-4-12b-it-Q4_K_M.gguf"),
        ("145db9094bc0f85f1701e255a2ed216dcc9800fc8bc8631ad00905b456bd451b", "mtp-gemma-4-12b-it.gguf"),
    ):
        (blobs / blob_sha).write_bytes(b"x")
        link = snapshot / filename
        link.symlink_to(Path("../../blobs") / blob_sha)
        links.append(link)

    return links[0], links[1]


def test_mtp_off_by_default_through_hf_cache_symlinks(tmp_path: Path):
    _reset_process_state()
    gguf, _ = _hf_cache_layout(tmp_path)

    fake_proc = Mock()
    fake_proc.poll.return_value = None

    with (
        patch("server.backend.llama_server.process.subprocess.Popen", return_value=fake_proc) as popen,
        patch("server.backend.llama_server.process.is_server_ready", return_value=True),
        patch("server.backend.llama_server.process.resolve_llama_server_binary", return_value="llama-server"),
    ):
        ensure_running(gguf, {})

    cmd = popen.call_args.args[0]
    assert "--spec-type" not in cmd
    assert "--spec-draft-model" not in cmd


def test_mtp_explicit_true_enables_through_hf_cache_symlinks(tmp_path: Path):
    _reset_process_state()
    gguf, mtp = _hf_cache_layout(tmp_path)

    fake_proc = Mock()
    fake_proc.poll.return_value = None

    with (
        patch("server.backend.llama_server.process.subprocess.Popen", return_value=fake_proc) as popen,
        patch("server.backend.llama_server.process.is_server_ready", return_value=True),
        patch("server.backend.llama_server.process.resolve_llama_server_binary", return_value="llama-server"),
    ):
        ensure_running(gguf, {"mtp": True})

    cmd = popen.call_args.args[0]
    assert "--spec-type" in cmd and "draft-mtp" in cmd
    assert cmd[cmd.index("--spec-draft-model") + 1] == str(mtp)
    assert cmd[cmd.index("-m") + 1] == str(gguf)


def test_missing_mtp_head_warns_and_is_collectable(tmp_path: Path):
    """`mtp = true` with no head on disk must return a warning from
    ensure_running — this is what the CLI surfaces as a yellow WARNING
    line after model load.
    """
    _reset_process_state()
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")

    fake_proc = Mock()
    fake_proc.poll.return_value = None

    with (
        patch("server.backend.llama_server.process.subprocess.Popen", return_value=fake_proc),
        patch("server.backend.llama_server.process.is_server_ready", return_value=True),
        patch("server.backend.llama_server.process.resolve_llama_server_binary", return_value="llama-server"),
    ):
        warnings = ensure_running(gguf, {"mtp": True})

    assert len(warnings) == 1
    assert "no MTP GGUF found" in warnings[0]


def test_reused_server_reports_no_stale_warnings(tmp_path: Path):
    """A warning from one spawn must not replay on later calls that reuse
    the already-running server.
    """
    _reset_process_state()
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")

    fake_proc = Mock()
    fake_proc.poll.return_value = None

    with (
        patch("server.backend.llama_server.process.subprocess.Popen", return_value=fake_proc),
        patch("server.backend.llama_server.process.is_server_ready", return_value=True),
        patch("server.backend.llama_server.process.resolve_llama_server_binary", return_value="llama-server"),
    ):
        first = ensure_running(gguf, {"mtp": True})
        assert first != []

        # Same config: server is reused, no fresh warnings expected.
        second = ensure_running(gguf, {"mtp": True})
        assert second == []


def test_concurrent_loads_each_get_own_warnings(tmp_path: Path):
    """Two threads load the same model at once (both /start and /v1/responses
    can race in production): one spawns the server and must receive the MTP
    warning, the other reuses the server and must receive none. Warnings
    must never leak across requests or get lost.
    """
    import threading

    _reset_process_state()
    gguf = tmp_path / "model.gguf"
    gguf.write_bytes(b"x")

    fake_proc = Mock()
    fake_proc.poll.return_value = None
    barrier = threading.Barrier(2)
    results: dict[int, list[str]] = {}

    def load(index: int) -> None:
        barrier.wait()
        results[index] = ensure_running(gguf, {"mtp": True})

    with (
        patch("server.backend.llama_server.process.subprocess.Popen", return_value=fake_proc) as popen,
        patch("server.backend.llama_server.process.is_server_ready", return_value=True),
        patch("server.backend.llama_server.process.resolve_llama_server_binary", return_value="llama-server"),
    ):
        threads = [threading.Thread(target=load, args=(i,)) for i in range(2)]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()

    # exactly one spawn; the spawner got the warning, the reuser got none
    assert popen.call_count == 1
    all_warnings = [w for ws in results.values() for w in ws]
    assert len(all_warnings) == 1
    assert "no MTP GGUF found" in all_warnings[0]
    assert sorted(len(ws) for ws in results.values()) == [0, 1]


def test_ensure_running_dedupes_unresolved_and_resolved_paths(tmp_path: Path):
    _reset_process_state()
    gguf, _ = _hf_cache_layout(tmp_path)

    fake_proc = Mock()
    fake_proc.poll.return_value = None

    with (
        patch("server.backend.llama_server.process.subprocess.Popen", return_value=fake_proc) as popen,
        patch("server.backend.llama_server.process.is_server_ready", return_value=True),
        patch("server.backend.llama_server.process.resolve_llama_server_binary", return_value="llama-server"),
    ):
        ensure_running(gguf, {})
        ensure_running(gguf.resolve(), {})

    assert popen.call_count == 1


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
