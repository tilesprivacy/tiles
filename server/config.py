import logging
import os
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import httpx

logger = logging.getLogger("app")

PORT = 6969
if os.environ.get("TILES_PY_ENV") == "dev":
    DAEMON_PORT = os.environ.get("TILES_DAEMON_DEV_PORT", "1729")
else:
    DAEMON_PORT = 1729
LLAMA_SERVER_HOST = os.environ.get("TILES_LLAMA_SERVER_HOST", "127.0.0.1")
LLAMA_SERVER_PORT = int(os.environ.get("TILES_LLAMA_SERVER_PORT", "18080"))
MODEL_ID = "driaforall/mem-agent"

MEMORY_PATH = os.path.expanduser("~") + "/tiles_memory"

# Short-lived cache for get_llama_config(). The daemon is authoritative but we
# avoid a localhost round-trip on every request; config changes propagate within
# this TTL. Keep small so user edits to config.toml pick up quickly.
_DAEMON_CONFIG_TTL_S = 1.0


@dataclass(frozen=True)
class DaemonConfig:
    llama: dict[str, Any] = field(default_factory=dict)
    llama_server_args: tuple[str, ...] = ()
    llama_server: str | None = None


_daemon_config_cache: DaemonConfig | None = None
_daemon_config_cache_ts: float = 0.0
# the daemon's last answer, kept when it can't be asked. forgetting it would move a
# running model onto the bundled build for as long as /config fails
_daemon_config_seen = DaemonConfig()


def get_daemon_config() -> DaemonConfig:
    global _daemon_config_cache, _daemon_config_cache_ts
    now = time.monotonic()
    if (
        _daemon_config_cache is not None
        and (now - _daemon_config_cache_ts) < _DAEMON_CONFIG_TTL_S
    ):
        return _daemon_config_cache

    config = _fetch_daemon_config()
    _daemon_config_cache = config
    # Capture the timestamp when the fetch completes, not when the call began,
    # so the TTL window reflects how long ago the cached value was obtained.
    _daemon_config_cache_ts = time.monotonic()
    return config


def get_llama_config() -> dict:
    return get_daemon_config().llama


def get_llama_server() -> str | None:
    """The llama-server of the engine the daemon installed, None for the one bundled with Tiles."""
    return get_daemon_config().llama_server


def _reset_daemon_config_cache() -> None:
    """Forget the daemon's config (used by tests)."""
    global _daemon_config_cache, _daemon_config_cache_ts, _daemon_config_seen
    _daemon_config_cache = None
    _daemon_config_cache_ts = 0.0
    _daemon_config_seen = DaemonConfig()


def _fetch_daemon_config() -> DaemonConfig:
    global _daemon_config_seen
    try:
        response = httpx.get(f"http://127.0.0.1:{DAEMON_PORT}/config", timeout=2)
        response.raise_for_status()
        config = response.json()
    except httpx.HTTPError:
        return _daemon_config_seen

    llama = config.get("llama") or {}
    logger.info("Loaded llama config from Tiles daemon")
    _daemon_config_seen = DaemonConfig(
        llama={name: value for name, value in llama.items() if value is not None},
        llama_server_args=tuple(config.get("llama_server_args") or ()),
        llama_server=config.get("llama_server"),
    )
    return _daemon_config_seen


# pid file the server writes for itself once it has PORT. the daemon reads it to
# stop the server, no matter who started it. keep the path in sync with
# `server_pid_path` in tiles/src/core/server.rs.
def pid_path() -> Path:
    state = os.environ.get("XDG_STATE_HOME") or str(Path.home() / ".local" / "state")
    return Path(state) / "tiles" / f"inference-{PORT}.pid"


def write_pid_file() -> None:
    path = pid_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(str(os.getpid()))


def remove_pid_file() -> None:
    """Delete the pid file, unless a newer server has written its own pid since."""
    path = pid_path()
    try:
        if path.read_text().strip() == str(os.getpid()):
            path.unlink()
    except OSError:
        pass
