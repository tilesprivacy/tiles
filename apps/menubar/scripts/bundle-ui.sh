#!/usr/bin/env bash
# builds the chat ui into the root of the menu bar app's assets, so tauri embeds
# it alongside the panel. the chat ui takes the root because it is a router-based
# spa and the webview falls back to the root index.html for unknown paths, which
# is what makes a deep link resolve. runs after vite build, which owns dist/panel
set -euo pipefail

root=$(cd "$(dirname "$0")/../../.." && pwd)
ui="$root/vendor/tiles-ui"
out="$root/apps/menubar/dist"

if [ ! -f "$ui/package.json" ]; then
  echo "vendor/tiles-ui is empty, run: git submodule update --init" >&2
  exit 1
fi

cd "$ui"
[ -d node_modules ] || npm ci

# the adapter writes into dist without clearing it, so a worker from an earlier
# build would survive
rm -rf dist

# no service worker in a desktop shell, it takes over navigation
VITE_PUBLIC_NO_PWA=1 \
VITE_PUBLIC_API_ORIGIN="${TILES_DAEMON_ORIGIN:-http://127.0.0.1:1729}" \
  npm run build

# clear the last chat ui without touching the panel vite just wrote
find "$out" -mindepth 1 -maxdepth 1 ! -name panel -exec rm -rf {} +
cp -R dist/. "$out/"
