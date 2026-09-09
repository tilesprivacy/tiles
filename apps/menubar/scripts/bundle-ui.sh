#!/usr/bin/env bash
# builds the chat ui into the root of the menu bar app's assets, so tauri embeds
# it alongside the panel. the chat ui takes the root because it is a router-based
# spa and the webview falls back to the root index.html for unknown paths, which
# is what makes a deep link resolve. runs after vite build, which owns dist/panel
set -euo pipefail

root=$(cd "$(dirname "$0")/../../.." && pwd)
out="$root/apps/menubar/dist"
cache="$root/apps/menubar/.ui-src"

if [ -n "${TILES_UI_DIR:-}" ]; then
  # working on both at once, build whatever is in the checkout
  ui="$TILES_UI_DIR"
else
  # shellcheck source=../ui.pin
  . "$root/apps/menubar/ui.pin"
  ui="$cache"

  if [ ! -d "$ui/.git" ]; then
    rm -rf "$ui"
    git clone -q "$repo" "$ui"
  fi

  # a bare sha is not a ref, so ask for it directly and fall back to everything
  git -C "$ui" fetch -q origin "$commit" 2>/dev/null || git -C "$ui" fetch -q origin
  git -C "$ui" checkout -q --detach "$commit"
fi

cd "$ui"

if [ ! -d node_modules ] || [ package-lock.json -nt node_modules ]; then
  npm ci
fi

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
