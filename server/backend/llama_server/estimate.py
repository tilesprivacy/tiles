"""How much memory a GGUF needs in llama-server, from its header alone.

The metadata and tensor table sit at the front of the file, so a range request
for a few megabytes is enough to size a model before downloading it. Checked
against llama-server's own buffers on Gemma 4 E2B, E4B and 12B: within 1%, and
the attention cache exactly.
"""

from __future__ import annotations

import asyncio
import json
import os
import struct
from pathlib import Path

import httpx

MiB = 1 << 20

# the context and slot count Tiles runs llama-server with
DEFAULT_CONTEXT = 32768
SLOTS = 4
UBATCH = 512
# compute buffer plus the cuda or metal context
OVERHEAD = 300 * MiB

_SCALAR = {0: "<B", 1: "<b", 2: "<H", 3: "<h", 4: "<I", 5: "<i", 6: "<f", 7: "<?", 10: "<Q", 11: "<q", 12: "<d"}
# ggml type -> (elements per block, bytes per block)
_GGML = {
    0: (1, 4), 1: (1, 2), 30: (1, 2), 2: (32, 18), 3: (32, 20), 6: (32, 22), 7: (32, 24),
    8: (32, 34), 10: (256, 84), 11: (256, 110), 12: (256, 144), 13: (256, 176),
    14: (256, 210), 15: (256, 292),
}


class Truncated(Exception):
    pass


def parse(buf: bytes) -> tuple[dict, dict[str, int]]:
    """Metadata and tensor byte sizes. Raises Truncated when `buf` is too short."""
    off = 0

    def take(fmt: str):
        nonlocal off
        size = struct.calcsize(fmt)
        if off + size > len(buf):
            raise Truncated
        (value,) = struct.unpack_from(fmt, buf, off)
        off += size
        return value

    def string() -> str:
        nonlocal off
        n = take("<Q")
        if off + n > len(buf):
            raise Truncated
        text = buf[off : off + n].decode("utf-8", "replace")
        off += n
        return text

    def value(kind: int):
        if kind in _SCALAR:
            return take(_SCALAR[kind])
        if kind == 8:
            return string()
        item, count = take("<I"), take("<Q")
        if item == 8:
            for _ in range(count):
                string()
            return None
        return [value(item) for _ in range(count)]

    if buf[:4] != b"GGUF":
        raise ValueError("not a GGUF file")
    off = 4
    take("<I")
    n_tensors, n_kv = take("<Q"), take("<Q")
    meta = {}
    for _ in range(n_kv):
        key = string()
        meta[key] = value(take("<I"))
    tensors = {}
    for _ in range(n_tensors):
        name = string()
        dims = [take("<Q") for _ in range(take("<I"))]
        kind = take("<I")
        take("<Q")
        elements = 1
        for dim in dims:
            elements *= dim
        per_block, block_bytes = _GGML.get(kind, (1, 2))
        tensors[name] = elements // per_block * block_bytes
    return meta, tensors


def need(meta: dict, tensors: dict[str, int], file_bytes: int, context: int = DEFAULT_CONTEXT) -> dict:
    arch = meta["general.architecture"]

    def get(key, default=None):
        return meta.get(f"{arch}.{key}", default)

    layers = get("block_count")
    kv_heads = get("attention.head_count_kv")
    kv_heads = kv_heads if isinstance(kv_heads, list) else [kv_heads] * layers
    sliding = get("attention.sliding_window_pattern") or [False] * layers
    window = get("attention.sliding_window", 0) or 0
    # the last `shared` layers read an earlier layer's cache instead of their own
    shared = get("attention.shared_kv_layers", 0) or 0

    full_kv = swa_kv = 0
    for layer in range(layers - shared):
        if sliding[layer]:
            dims = get("attention.key_length_swa") + get("attention.value_length_swa")
            swa_kv += SLOTS * (window + UBATCH // 4) * kv_heads[layer] * dims * 2
        else:
            dims = get("attention.key_length") + get("attention.value_length")
            full_kv += context * kv_heads[layer] * dims * 2

    # llama.cpp keeps per-layer embeddings on the cpu
    cpu_only = sum(size for name, size in tensors.items() if name.startswith("per_layer_token_embd"))
    experts = sum(size for name, size in tensors.items() if "_exps" in name)
    weights = file_bytes - cpu_only
    return {
        "vram_bytes": weights + full_kv + swa_kv + OVERHEAD,
        "weights_bytes": weights,
        "kv_bytes": full_kv + swa_kv,
        # what can move to the cpu with --n-cpu-moe, for mixture-of-experts models
        "expert_bytes": experts,
    }


def _cache_path() -> Path:
    dev = Path.cwd() / ".tiles_dev" / "tiles"
    if dev.is_dir():
        return dev / "cache" / "estimates.json"
    xdg = os.environ.get("XDG_DATA_HOME")
    data_home = Path(xdg) if xdg else Path.home() / ".local" / "share"
    return data_home / "tiles" / "cache" / "estimates.json"


# estimates for a whole lineup arrive at once, and each writing back only what
# it read would drop the others' results
_cache: dict[str, dict] | None = None
_cache_lock = asyncio.Lock()


def _load_cache() -> dict[str, dict]:
    global _cache
    if _cache is None:
        try:
            _cache = json.loads(_cache_path().read_text(encoding="utf-8"))
        except (OSError, ValueError):
            _cache = {}
    return _cache


async def _fetch_header(url: str) -> tuple[dict, dict[str, int]]:
    length = 8 * MiB
    async with httpx.AsyncClient(follow_redirects=True, timeout=60) as client:
        while True:
            response = await client.get(url, headers={"Range": f"bytes=0-{length - 1}"})
            response.raise_for_status()
            try:
                return parse(response.content)
            except Truncated:
                if length >= 128 * MiB:
                    raise
                length *= 2


async def estimate(url: str, file_bytes: int, context: int = DEFAULT_CONTEXT) -> dict:
    """Memory `url` needs, read from as little of its header as parses."""
    key = f"{url}|{file_bytes}|{context}"
    if key in _load_cache():
        return _load_cache()[key]

    meta, tensors = await _fetch_header(url)
    result = need(meta, tensors, file_bytes, context)

    async with _cache_lock:
        cache = _load_cache()
        cache[key] = result
        path = _cache_path()
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            tmp = path.with_suffix(".tmp")
            tmp.write_text(json.dumps(cache), encoding="utf-8")
            tmp.replace(path)
        except OSError:
            pass
    return result
