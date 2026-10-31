import atexit
import signal
import sys

import uvicorn

from .api import app
import logging

from .config import PORT, remove_pid_file, write_pid_file
from fastapi import Request
from . import runtime

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)],
)
logging.getLogger("httpx").setLevel(logging.WARNING)
logger = logging.getLogger("app")

@app.middleware("http")
async def log_requests(request: Request, call_next):
    try:
        body = await request.json()
    except Exception:
        body = None

    logger.info(
        {
            "method": request.method,
            "url": str(request.url),
            "client": request.client.host,
            "body": body,
        }
    )

    response = await call_next(request)
    logger.info(f"<-- {request.method} {request.url.path} {response.status_code}")
    return response

def get_backend():
    """
    Return the inference backend. Tiles uses llama-server on all platforms.
    """
    from .backend import llama_server

    logger.info("Using llama-server backend")
    return llama_server

runtime.backend = get_backend()


def _stop_backend_server() -> None:
    """Best-effort shutdown hook for backend-managed subprocesses."""
    try:
        backend = getattr(runtime, "backend", None)
        process_mod = getattr(backend, "process", None)
        stop_fn = getattr(process_mod, "stop", None)
        if callable(stop_fn):
            stop_fn()
    except Exception:  # noqa: BLE001 - shutdown path should be resilient
        logger.exception("Failed to stop backend subprocess during shutdown")


def _handle_termination(_signum: int, _frame) -> None:
    """Ensure llama-server exits when the API process is terminated."""
    _stop_backend_server()
    sys.exit(0)


atexit.register(_stop_backend_server)
signal.signal(signal.SIGTERM, _handle_termination)
signal.signal(signal.SIGINT, _handle_termination)

def run():
    config = uvicorn.Config(app, host="127.0.0.1", port=PORT)
    # get the port first, so the pid file never points at a server that didn't get it
    sock = config.bind_socket()
    write_pid_file()
    atexit.register(remove_pid_file)
    uvicorn.Server(config).run(sockets=[sock])


if __name__ == "__main__":
    run()
