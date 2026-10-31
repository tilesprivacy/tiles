#!/usr/bin/env bash
# Puts the built web UI in ui/. The bundle installs it as <lib dir>/ui and the daemon
# serves it on port 1730. The version comes from toolchain.toml.
#
# Environment variables:
#   TILES_UI_BUILD  A local tiles-ui build folder to use instead of the release.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${ROOT}/ui"

# ui/ is replaced only once the new build is complete
STAGE="$(mktemp -d "${ROOT}/.ui.XXXXXX")"
trap 'rm -rf "${STAGE}"' EXIT

if [[ -n "${TILES_UI_BUILD:-}" ]]; then
  echo "Using the UI build at ${TILES_UI_BUILD}"
  cp -r "${TILES_UI_BUILD}"/. "${STAGE}/"
else
  VERSION=$(grep '^tiles-ui' "${ROOT}/toolchain.toml" | head -1 | awk -F'"' '{print $2}')
  URL="https://github.com/tilesprivacy/tiles-ui/releases/download/${VERSION}/tiles-ui-${VERSION}.tar.gz"
  echo "Fetching the UI ${VERSION}"
  curl -fsSL --retry 3 "${URL}" | tar -xz -C "${STAGE}"
fi

[[ -f "${STAGE}/index.html" ]] || { echo "No index.html in the UI build" >&2; exit 1; }
# mktemp makes it private, the installed UI must be readable by everyone
chmod 755 "${STAGE}"
rm -rf "${OUT_DIR}"
mv "${STAGE}" "${OUT_DIR}"
