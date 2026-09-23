"""What this machine can give a model.

llama-server reports every device it can run on, with total and free memory,
for whichever backend is in use: CUDA, Vulkan or Metal. That is the number a
model has to fit in, so ask it rather than each vendor's own tool.
"""

from __future__ import annotations

import logging
import os
import re
import subprocess
import sys

from . import estimate, process
from .process import resolve_llama_server_binary, select_gpu_backend

logger = logging.getLogger("app")

_DEVICE = re.compile(r"^\s*(\w+?\d+):\s+(.+?)\s+\((\d+) MiB, (\d+) MiB free\)\s*$")

# a gpu that borrows system memory can run a model, but its "free" figure is
# system ram, not vram. amd reports its apus under the cpu's own name
_INTEGRATED = re.compile(r"Ryzen|Processor|Radeon\(TM\) Graphics|Intel\(R\) (UHD|Iris|Graphics)|Arc\(TM\) Graphics|llvmpipe", re.I)

MiB = 1 << 20


def parse_devices(output: str) -> list[dict]:
    devices = []
    for line in output.splitlines():
        match = _DEVICE.match(line)
        if not match:
            continue
        name, description, total, free = match.groups()
        # unified memory is the whole point on a mac, the gpu is not "integrated" there
        integrated = bool(_INTEGRATED.search(description)) and sys.platform != "darwin"
        devices.append(
            {
                "id": name,
                "name": description,
                "total_bytes": int(total) * MiB,
                "free_bytes": int(free) * MiB,
                "integrated": integrated,
            }
        )
    return devices


def system_memory() -> dict:
    """Total and available ram, for when no gpu can take the model."""
    total = os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_PHYS_PAGES")
    available = total
    try:
        with open("/proc/meminfo", encoding="utf-8") as meminfo:
            for line in meminfo:
                if line.startswith("MemAvailable:"):
                    available = int(line.split()[1]) * 1024
    except OSError:
        pass
    return {"total_bytes": total, "available_bytes": available}


def devices() -> list[dict]:
    try:
        binary = resolve_llama_server_binary()
    except FileNotFoundError as exc:
        logger.warning("No llama-server to ask about devices: %s", exc)
        return []

    env = os.environ.copy()
    backend = select_gpu_backend(binary)
    if backend is not None:
        env["GGML_BACKEND_PATH"] = str(backend)
    try:
        result = subprocess.run(
            [binary, "--list-devices"], env=env, capture_output=True, text=True, timeout=60
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        logger.warning("Could not list devices: %s", exc)
        return []
    return parse_devices(result.stdout + result.stderr)


def reclaimable() -> int:
    """VRAM our own llama-server holds, which switching models gives back."""
    proc, gguf = process._process, process._loaded_gguf
    if proc is None or proc.poll() is not None or gguf is None:
        return 0
    try:
        with open(gguf, "rb") as model:
            header = model.read(128 * MiB)
        meta, tensors = estimate.parse(header)
        from ...config import get_llama_config

        context = get_llama_config().get("context_length") or estimate.DEFAULT_CONTEXT
        return estimate.need(meta, tensors, gguf.stat().st_size, int(context))["vram_bytes"]
    except (OSError, ValueError, KeyError, estimate.Truncated) as exc:
        logger.warning("Could not size the loaded model: %s", exc)
        return 0


def hardware() -> dict:
    found = devices()
    held = reclaimable()
    for device in found:
        # the model tiles itself has loaded is on the discrete gpu; its memory
        # is what a new model would get
        device["reclaimable_bytes"] = held if not device["integrated"] else 0
    return {"devices": found, "memory": system_memory()}
