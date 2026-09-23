import struct

from server.backend.llama_server import estimate, hardware


def _string(text: str) -> bytes:
    raw = text.encode()
    return struct.pack("<Q", len(raw)) + raw


def _kv(key: str, kind: int, payload: bytes) -> bytes:
    return _string(key) + struct.pack("<I", kind) + payload


def _u32(value: int) -> bytes:
    return struct.pack("<I", value)


def _array(item_kind: int, items: list[bytes]) -> bytes:
    return struct.pack("<I", item_kind) + struct.pack("<Q", len(items)) + b"".join(items)


def _tensor(name: str, dims: list[int], kind: int) -> bytes:
    return _string(name) + _u32(len(dims)) + b"".join(struct.pack("<Q", d) for d in dims) + _u32(kind) + struct.pack("<Q", 0)


def _gguf() -> bytes:
    """Two layers, one full attention and one sliding window, like gemma 4."""
    kvs = [
        _kv("general.architecture", 8, _string("gemma4")),
        _kv("gemma4.block_count", 4, _u32(2)),
        _kv("gemma4.attention.head_count_kv", 9, _array(4, [_u32(1), _u32(8)])),
        _kv("gemma4.attention.sliding_window_pattern", 9, _array(7, [b"\x00", b"\x01"])),
        _kv("gemma4.attention.sliding_window", 4, _u32(1024)),
        _kv("gemma4.attention.key_length", 4, _u32(512)),
        _kv("gemma4.attention.value_length", 4, _u32(512)),
        _kv("gemma4.attention.key_length_swa", 4, _u32(256)),
        _kv("gemma4.attention.value_length_swa", 4, _u32(256)),
        _kv("tokenizer.ggml.tokens", 9, _array(8, [_string("a"), _string("b")])),
    ]
    tensors = [
        _tensor("per_layer_token_embd.weight", [256, 1024], 12),  # q4_k: 144 bytes per 256
        _tensor("blk.0.ffn_gate_exps.weight", [256, 512], 12),
    ]
    return b"GGUF" + _u32(3) + struct.pack("<Q", len(tensors)) + struct.pack("<Q", len(kvs)) + b"".join(kvs) + b"".join(tensors)


def test_the_header_gives_the_same_split_llama_server_allocates():
    meta, tensors = estimate.parse(_gguf())
    result = estimate.need(meta, tensors, file_bytes=10 * estimate.MiB, context=32768)

    full = 32768 * 1 * (512 + 512) * 2
    sliding = estimate.SLOTS * (1024 + estimate.UBATCH // 4) * 8 * (256 + 256) * 2
    assert result["kv_bytes"] == full + sliding
    # per-layer embeddings stay on the cpu, so they are not vram
    assert result["weights_bytes"] == 10 * estimate.MiB - 1024 * 144
    assert result["expert_bytes"] == 512 * 144
    assert result["vram_bytes"] == result["weights_bytes"] + full + sliding + estimate.OVERHEAD


def test_a_short_read_asks_for_more_rather_than_guessing():
    whole = _gguf()
    try:
        estimate.parse(whole[: len(whole) // 2])
    except estimate.Truncated:
        pass
    else:
        raise AssertionError("a truncated header must not parse")


def test_devices_are_read_off_list_devices():
    output = (
        "Available devices:\n"
        "  CUDA0: NVIDIA GeForce RTX 5060 Ti (15918 MiB, 14508 MiB free)\n"
        "  Vulkan1: AMD Ryzen 5 7600 6-Core Processor (RADV RAPHAEL_MENDOCINO) (8050 MiB, 8020 MiB free)\n"
    )
    devices = hardware.parse_devices(output)
    assert [d["id"] for d in devices] == ["CUDA0", "Vulkan1"]
    assert devices[0]["free_bytes"] == 14508 * hardware.MiB
    assert devices[0]["integrated"] is False
    assert devices[1]["integrated"] is True


def test_concurrent_estimates_all_survive_in_the_cache(tmp_path, monkeypatch):
    import asyncio
    import json

    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("XDG_DATA_HOME", str(tmp_path / "xdg"))
    monkeypatch.setattr(estimate, "_cache", None)

    async def header(url):
        await asyncio.sleep(0.01)
        return estimate.parse(_gguf())

    monkeypatch.setattr(estimate, "_fetch_header", header)

    async def run():
        await asyncio.gather(*(estimate.estimate(f"https://x/{i}.gguf", 10 * estimate.MiB) for i in range(5)))

    asyncio.run(run())
    saved = json.loads((tmp_path / "xdg" / "tiles" / "cache" / "estimates.json").read_text())
    assert len(saved) == 5
