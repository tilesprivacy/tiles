"""Spawn and manage a llama-server subprocess."""

from __future__ import annotations

import ctypes
import json
import logging
import os
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any

import httpx

from ...config import LLAMA_SERVER_HOST, LLAMA_SERVER_PORT
from .gguf import find_mtp_gguf_file

logger = logging.getLogger("app")

_process: subprocess.Popen[bytes] | None = None
_loaded_gguf: Path | None = None
_loaded_config_key: str | None = None
# Serializes ensure_running so concurrent requests can't double-start the server.
_ensure_lock = threading.Lock()
# Warnings recorded while starting/restarting llama-server (e.g. MTP
# requested but no MTP GGUF on disk). ensure_running drains them inside
# _ensure_lock and returns them to its caller so the CLI can surface
# them to the user.
_startup_warnings: list[str] = []


def _record_warning(message: str, *args: Any) -> None:
    logger.warning(message, *args)
    _startup_warnings.append(message % args if args else message)


def take_warnings() -> list[str]:
    """Return and clear the collected startup warnings.

    Only safe to call while holding _ensure_lock (ensure_running does);
    a bare call could race a concurrent spawn's recording.
    """
    warnings = list(_startup_warnings)
    _startup_warnings.clear()
    return warnings


def resolve_llama_server_binary() -> str:
    env_bin = os.environ.get("TILES_LLAMA_SERVER_BIN")
    if env_bin and Path(env_bin).is_file():
        return env_bin

    server_root = Path(__file__).resolve().parents[2]
    for candidate in (
        server_root / "bin" / "llama-server",
        server_root.parent / "bin" / "llama-server",
    ):
        if candidate.is_file():
            return str(candidate)

    path_bin = shutil.which("llama-server")
    if path_bin:
        return path_bin

    raise FileNotFoundError(
        "llama-server binary not found. Set TILES_LLAMA_SERVER_BIN, place a binary at "
        "server/bin/llama-server, or install llama-server on PATH. "
        "See scripts/fetch_llama_server.sh."
    )


# best first; each is a dir next to llama-server with the ggml backend and,
# for cuda, its runtime libs
_GPU_VARIANTS = ("cuda_v13", "cuda_v12", "vulkan")
_DEVICE_LINE = re.compile(r"^\s*(CUDA|Vulkan)\d+:", re.MULTILINE)
_selected_backend: dict[str, Path | None] = {}


def _cuda_driver_version() -> int | None:
    """CUDA API version of the installed driver (e.g. 13040), None without one."""
    try:
        libcuda = ctypes.CDLL("libcuda.so.1")
    except OSError:
        return None
    version = ctypes.c_int()
    try:
        if libcuda.cuDriverGetVersion(ctypes.byref(version)) != 0:
            return None
    except AttributeError:
        return None
    return version.value


def _backend_library(variant_dir: Path) -> Path | None:
    backend = "cuda" if variant_dir.name.startswith("cuda") else variant_dir.name
    candidate = variant_dir / f"libggml-{backend}.so"
    return candidate if candidate.is_file() else None


def _probe_backend(binary: str, backend_lib: Path) -> bool:
    """True when llama-server sees a GPU through this backend."""
    env = os.environ.copy()
    env["GGML_BACKEND_PATH"] = str(backend_lib)
    try:
        result = subprocess.run(
            [binary, "--list-devices"],
            env=env,
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        logger.warning("Probing %s failed: %s", backend_lib.parent.name, exc)
        return False
    return _DEVICE_LINE.search(result.stdout + result.stderr) is not None


def select_gpu_backend(binary: str) -> Path | None:
    """Pick the ggml GPU backend to load, or None for CPU.

    cuda variants need a driver of that major; every candidate is confirmed
    with --list-devices. TILES_LLAMA_VARIANT forces one, or cpu.
    """
    if sys.platform != "linux":
        return None
    if binary in _selected_backend:
        return _selected_backend[binary]

    bin_dir = Path(binary).resolve().parent
    forced = os.environ.get("TILES_LLAMA_VARIANT", "").strip()
    if forced == "cpu":
        logger.info("TILES_LLAMA_VARIANT=cpu: not loading a GPU backend")
        _selected_backend[binary] = None
        return None
    candidates = [forced] if forced else list(_GPU_VARIANTS)

    driver_version: int | None = None
    driver_checked = False
    selected: Path | None = None
    for variant in candidates:
        backend_lib = _backend_library(bin_dir / variant)
        if backend_lib is None:
            if forced:
                logger.warning("TILES_LLAMA_VARIANT=%s but %s has no backend library", variant, bin_dir / variant)
            continue
        if not forced and variant.startswith("cuda_v"):
            if not driver_checked:
                driver_version = _cuda_driver_version()
                driver_checked = True
            required = int(variant[len("cuda_v"):]) * 1000
            if driver_version is None or driver_version < required:
                logger.info(
                    "Skipping %s: NVIDIA driver supports CUDA %s",
                    variant,
                    "none" if driver_version is None else f"{driver_version // 1000}.{driver_version % 1000 // 10}",
                )
                continue
        if _probe_backend(binary, backend_lib):
            selected = backend_lib
            break
        logger.warning("GPU backend %s found no usable device; trying the next one", variant)

    if selected is None and any((bin_dir / v).is_dir() for v in _GPU_VARIANTS):
        logger.warning("No usable GPU backend; llama-server will run on CPU")
    _selected_backend[binary] = selected
    return selected


def _config_key(llama_config: dict[str, Any]) -> str:

    return json.dumps(llama_config, sort_keys=True, default=str)


def build_llama_server_command(
    gguf_path: Path, llama_config: dict[str, Any]
) -> list[str]:
    """Build the llama-server argv from Tiles config.

    Only flags explicitly set in ``llama_config`` are forwarded; unset values
    are left to llama-server's own defaults.
    """
    binary = resolve_llama_server_binary()
    # Always bind to Tiles' llama-server port (default 18080) so we don't
    # collide with a stock llama-server on 8080.
    cmd = [
        binary,
        "--host",
        LLAMA_SERVER_HOST,
        "--port",
        str(LLAMA_SERVER_PORT),
        "-m",
        str(gguf_path),
        "--jinja",
    ]

    context_length = llama_config.get("context_length")
    if context_length is not None:
        cmd.extend(["-c", str(int(context_length))])

    batch_size = llama_config.get("batch_size")
    if batch_size is not None:
        cmd.extend(["-b", str(int(batch_size))])

    gpu_layers = llama_config.get("gpu_layers")
    if gpu_layers is not None:
        cmd.extend(["-ngl", str(int(gpu_layers))])

    offload_kqv = llama_config.get("offload_kqv")
    if offload_kqv is True:
        cmd.append("--kv-offload")
    elif offload_kqv is False:
        cmd.append("--no-kv-offload")

    n_cpu_moe = llama_config.get("n_cpu_moe")
    if n_cpu_moe is not None:
        cmd.extend(["--n-cpu-moe", str(int(n_cpu_moe))])

    flash_attn = llama_config.get("flash_attn")
    if flash_attn is True:
        cmd.extend(["--flash-attn", "on"])
    elif flash_attn is False:
        cmd.extend(["--flash-attn", "off"])

    no_mmap = llama_config.get("no_mmap")
    if no_mmap is True:
        cmd.append("--no-mmap")

    # MTP speculative decoding: opt-in. Enabled only when `mtp = true` is
    # set in config.toml (or passed via `tiles run --mtp`); the presence
    # of an MTP head GGUF on disk alone does not enable it. `mtp = true`
    # with no file on disk warns and runs without it.
    mtp_config = llama_config.get("mtp")
    if mtp_config is True:
        mtp_path = find_mtp_gguf_file(gguf_path)
        if mtp_path is not None:
            cmd.extend(
                [
                    "--spec-type",
                    "draft-mtp",
                    "--spec-draft-model",
                    str(mtp_path),
                ]
            )
            logger.info("MTP speculative decoding enabled with %s", mtp_path)
        else:
            _record_warning(
                "MTP enabled but no MTP GGUF found next to %s. "
                "Re-run model download or set mtp = false in config.",
                gguf_path,
            )

    return cmd


def _health_url() -> str:
    return f"http://{LLAMA_SERVER_HOST}:{LLAMA_SERVER_PORT}/health"


def _resolve_log_dir() -> Path:
    """Where Tiles keeps its logs.

    Mirrors the Rust side's data dir so llama-server logs sit next to the
    daemon and server logs. They used to go under the user data dir, which
    mixed app output into user content and kept them out of reach of the
    uninstaller, since that deliberately preserves user data.
    """
    dev_dir = Path.cwd() / ".tiles_dev" / "tiles" / "logs"
    if dev_dir.is_dir():
        return dev_dir

    xdg_data_home = os.environ.get("XDG_DATA_HOME")
    data_home = Path(xdg_data_home) if xdg_data_home else Path.home() / ".local" / "share"
    return data_home / "tiles" / "logs"


def _llama_server_log_hint() -> str:
    return str(_resolve_log_dir() / "llama-server.err.log")


def is_server_ready() -> bool:
    """True when llama-server /health reports the model is loaded."""
    try:
        response = httpx.get(_health_url(), timeout=2.0)
    except httpx.HTTPError:
        return False
    if response.status_code != 200:
        return False
    try:
        payload = response.json()
    except ValueError:
        return True
    status = payload.get("status")
    if status is None:
        return True
    return status == "ok"


def _tail_llama_server_log(max_lines: int = 8) -> str:
    log_path = Path(_llama_server_log_hint())
    if not log_path.is_file():
        return ""
    try:
        lines = log_path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return ""
    if not lines:
        return ""
    return "\n".join(lines[-max_lines:])


def wait_until_ready(proc: subprocess.Popen[bytes], timeout_s: float = 600.0) -> None:
    """Poll /health until the model finishes loading. 503 while loading is normal."""
    deadline = time.time() + timeout_s
    started = time.time()
    last_progress_log = 0.0
    httpx_logger = logging.getLogger("httpx")
    previous_httpx_level = httpx_logger.level

    logger.info("Waiting for llama-server to finish loading the model...")
    httpx_logger.setLevel(logging.WARNING)
    try:
        while time.time() < deadline:
            if proc.poll() is not None:
                detail = _tail_llama_server_log()
                hint = _llama_server_log_hint()
                message = (
                    f"llama-server exited during startup (code {proc.returncode}). "
                    f"Check {hint}."
                )
                if detail:
                    message = f"{message}\n{detail}"
                raise RuntimeError(message)
            if is_server_ready():
                elapsed = time.time() - started
                logger.info("llama-server ready (%.0fs)", elapsed)
                return

            now = time.time()
            elapsed = now - started
            if elapsed >= 5 and now - last_progress_log >= 30:
                logger.info("Still loading model (%.0fs elapsed)...", elapsed)
                last_progress_log = now
            time.sleep(1.0)
    finally:
        httpx_logger.setLevel(previous_httpx_level)

    if proc.poll() is not None:
        raise RuntimeError(
            f"llama-server exited before becoming ready (code {proc.returncode}). "
            f"Check {_llama_server_log_hint()}."
        )
    raise TimeoutError(
        f"llama-server did not finish loading within {timeout_s:.0f}s. "
        f"Check {_llama_server_log_hint()}."
    )


def stop() -> None:
    global _process, _loaded_gguf, _loaded_config_key
    if _process is None:
        _loaded_gguf = None
        _loaded_config_key = None
        return

    proc = _process
    _process = None
    _loaded_gguf = None
    _loaded_config_key = None

    if proc.poll() is not None:
        return

    proc.send_signal(signal.SIGTERM)
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)


def ensure_running(gguf_path: Path, llama_config: dict[str, Any]) -> list[str]:
    """Start or restart llama-server for the given GGUF and config.
    Returns the warnings recorded during this call's spawn (e.g. MTP
    requested but no head GGUF found).
    """
    global _process, _loaded_gguf, _loaded_config_key

    gguf_path = Path(os.path.abspath(gguf_path))
    resolved_gguf = gguf_path.resolve()
    key = _config_key(llama_config)
    with _ensure_lock:
        # Fresh call: drop warnings from any previous spawn so callers only
        # ever see warnings from this invocation.
        _startup_warnings.clear()
        if (
            _process is not None
            and _process.poll() is None
            and _loaded_gguf == resolved_gguf
            and _loaded_config_key == key
        ):
            if is_server_ready():
                return []
            wait_until_ready(_process)
            return []

        stop()

        gpu_layers = llama_config.get("gpu_layers")
        if gpu_layers is not None and int(gpu_layers) <= 0:
            logger.warning(
                "gpu_layers=%s — running on CPU. Set [llama].gpu_layers in config.toml "
                "for GPU offload.",
                gpu_layers,
            )

        cmd = build_llama_server_command(gguf_path, llama_config)

        logger.info("Starting llama-server: %s", " ".join(cmd))
        env = os.environ.copy()
        binary = cmd[0]
        lib_dir = str(Path(binary).resolve().parent)
        if sys.platform == "darwin":
            for var in ("DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH"):
                prev = env.get(var, "")
                env[var] = f"{lib_dir}:{prev}" if prev else lib_dir
        else:
            prev = env.get("LD_LIBRARY_PATH", "")
            env["LD_LIBRARY_PATH"] = f"{lib_dir}:{prev}" if prev else lib_dir
            # not on LD_LIBRARY_PATH too, ggml would register the gpu twice
            backend_lib = select_gpu_backend(binary)
            if backend_lib is not None:
                env["GGML_BACKEND_PATH"] = str(backend_lib)
                logger.info("Using %s GPU backend (%s)", backend_lib.parent.name, backend_lib)
        log_dir = _resolve_log_dir()
        log_dir.mkdir(parents=True, exist_ok=True)
        stdout_log = open(log_dir / "llama-server.out.log", "ab")
        stderr_log = open(log_dir / "llama-server.err.log", "ab")
        try:
            # Popen dups the fds, so our copies can be closed immediately.
            _process = subprocess.Popen(
                cmd,
                stdout=stdout_log,
                stderr=stderr_log,
                env=env,
            )
        finally:
            stdout_log.close()
            stderr_log.close()
        _loaded_gguf = resolved_gguf
        _loaded_config_key = key
        wait_until_ready(_process)
        return take_warnings()
