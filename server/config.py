import os

import httpx
from pydantic import BaseModel

PORT = 6969
DAEMON_PORT = 1729
MODEL_ID = "driaforall/mem-agent"

MEMORY_PATH = os.path.expanduser("~") + "/tiles_memory"


class LlamaConfig(BaseModel):
    context_length: int | None = None
    gpu_layers: int | None = None
    offload_kqv: bool | None = None
    batch_size: int | None = None


def get_llama_config() -> dict:
    try:
        response = httpx.get(f"http://127.0.0.1:{DAEMON_PORT}/config", timeout=5)
        response.raise_for_status()
        config = response.json()
    except httpx.HTTPError:
        return {}

    return LlamaConfig(**config.get("llama", {})).model_dump(exclude_none=True)
