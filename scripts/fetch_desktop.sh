#!/usr/bin/env bash
# Puts the Tiles desktop app in desktop/: Tiles.app on macOS, the tiles-desktop binary
# with its launcher entry and icon on Linux. It comes with each tiles-ui release, so the
# version is the tiles-ui one in toolchain.toml.
#
# Environment variables:
#   TILES_DESKTOP_BUILD  A local folder laid out like the release, to use instead of it.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${ROOT}/desktop"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# desktop/ is replaced only once the new app is complete
STAGE="$(mktemp -d "${ROOT}/.desktop.XXXXXX")"
trap 'rm -rf "${STAGE}"' EXIT

if [[ -n "${TILES_DESKTOP_BUILD:-}" ]]; then
  echo "Using the desktop app at ${TILES_DESKTOP_BUILD}"
  cp -R "${TILES_DESKTOP_BUILD}"/. "${STAGE}/"
else
  VERSION=$(grep '^tiles-ui' "${ROOT}/toolchain.toml" | head -1 | awk -F'"' '{print $2}')
  RELEASE="https://github.com/tilesprivacy/tiles-ui/releases/download/${VERSION}"
  echo "Fetching the desktop app ${VERSION}"
  case "${OS}:${ARCH}" in
    darwin:*)
      curl -fsSL --retry 3 -o "${STAGE}/app.zip" "${RELEASE}/tiles-desktop-${VERSION}-macos.zip"
      ditto -x -k "${STAGE}/app.zip" "${STAGE}"
      rm "${STAGE}/app.zip"
      ;;
    linux:x86_64)
      curl -fsSL --retry 3 "${RELEASE}/tiles-desktop-${VERSION}-x86_64-linux.tar.gz" \
        | tar -xz -C "${STAGE}" --strip-components=1
      ;;
    *)
      echo "No desktop app for ${OS} on ${ARCH} yet, the bundle goes without it"
      rm -rf "${OUT_DIR}"
      exit 0
      ;;
  esac
fi

case "${OS}" in
  darwin) [[ -d "${STAGE}/Tiles.app" ]] || { echo "No Tiles.app in the desktop app" >&2; exit 1; } ;;
  *)
    [[ -x "${STAGE}/tiles-desktop" ]] || { echo "No tiles-desktop in the desktop app" >&2; exit 1; }
    # install.sh needs the launcher entry and icon too
    for file in tiles.desktop tiles.png; do
      [[ -f "${STAGE}/${file}" ]] || { echo "No ${file} in the desktop app" >&2; exit 1; }
    done
    ;;
esac
chmod 755 "${STAGE}"
rm -rf "${OUT_DIR}"
mv "${STAGE}" "${OUT_DIR}"
