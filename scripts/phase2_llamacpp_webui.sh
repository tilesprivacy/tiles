#!/usr/bin/env bash
# Run ggml-org/llama.cpp SvelteKit web UI with the dev proxy pointed at the Tiles server.
#
# Prerequisites: Node.js (npm), git. Start Tiles separately (e.g. just serve on :6969).
#
# Environment:
#   LLAMA_CPP_ROOT   Clone path (default: sibling of this repo: ../llama.cpp)
#   TILES_BACKEND    Proxy target (default: http://127.0.0.1:6969)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LLAMA_CPP="${LLAMA_CPP_ROOT:-$ROOT/../llama.cpp}"
BACKEND="${TILES_BACKEND:-http://127.0.0.1:6969}"
WEBUI="$LLAMA_CPP/tools/server/webui"
VITE_CFG="$WEBUI/vite.config.ts"

if [ ! -d "$LLAMA_CPP" ]; then
  echo "Cloning llama.cpp -> $LLAMA_CPP"
  git clone --depth 1 https://github.com/ggml-org/llama.cpp "$LLAMA_CPP"
fi

if [ ! -f "$VITE_CFG" ]; then
  echo "Expected file missing: $VITE_CFG"
  exit 1
fi

if ! grep -qF "$BACKEND" "$VITE_CFG" 2>/dev/null; then
  echo "Patching $VITE_CFG to proxy API routes to $BACKEND"
  cp "$VITE_CFG" "${VITE_CFG}.tiles.bak"
  perl -pi -e "s|http://localhost:8080|${BACKEND}|g" "$VITE_CFG"
fi

echo "Backend proxy: $BACKEND (run Tiles with just serve). Load model via POST /start, session file, TILES_BOOTSTRAP_*, or TILES_MODEL_CACHE_PATH + UI load."
echo ""

cd "$WEBUI"
if [ -f package-lock.json ]; then
  npm ci
else
  npm install
fi
exec npm run dev
