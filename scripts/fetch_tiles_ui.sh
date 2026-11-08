#!/usr/bin/env bash
# Puts tiles-ui in place for the bundle: the web UI in ui/, which the bundle installs as
# <lib dir>/ui for the daemon to serve on port 1730, and the desktop app in desktop/:
# Tiles.app on macOS, the tiles-desktop binary with its launcher entry and icon on Linux.
# Both come in one file of the tiles-ui release that toolchain.toml pins. Platforms
# without the app get only the web UI.
#
# Before that release is out, tiles-ui main is built instead. Its checkout lives in
# ~/.cache/tiles/tiles-ui, not in this repo, as cargo would take the app for part of its
# workspace. That needs git and npm, and the app needs Rust and Tauri's build
# dependencies too.
#
# Environment variables:
#   TILES_UI_BUILD  A local folder laid out like the release file, to use instead of it.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
REPO="https://github.com/tilesprivacy/tiles-ui"
VERSION=$(grep '^tiles-ui' "${ROOT}/toolchain.toml" | head -1 | awk -F'"' '{print $2}')
RELEASE="${REPO}/releases/download/${VERSION}"

case "${OS}:${ARCH}" in
  darwin:*) ASSET="tiles-desktop-${VERSION}-macos.zip" APP="Tiles.app" ;;
  linux:x86_64) ASSET="tiles-desktop-${VERSION}-x86_64-linux.tar.gz" APP="tiles-desktop" ;;
  *) ASSET="tiles-ui-${VERSION}.tar.gz" APP="" ;;
esac

# ui/ and desktop/ are replaced only once the new ones are complete
STAGE="$(mktemp -d "${ROOT}/.tiles-ui.XXXXXX")"
trap 'rm -rf "${STAGE}"' EXIT

# a missing file means the release isn't out, any other failure stops the bundle
is_released() {
  local status
  status=$(curl -sL --retry 3 -r 0-0 -o /dev/null -w '%{http_code}' "$1" || true)
  case "${status}" in
    200 | 206) return 0 ;;
    404) return 1 ;;
    *) echo "Couldn't reach $1 (HTTP ${status})" >&2; exit 1 ;;
  esac
}

download() {
  echo "Fetching tiles-ui ${VERSION}"
  curl -fsSL --retry 3 -o "${STAGE}/${ASSET}" "${RELEASE}/${ASSET}"
  case "${ASSET}" in
    *.zip) ditto -x -k "${STAGE}/${ASSET}" "${STAGE}" ;;
    tiles-desktop-*) tar -xzf "${STAGE}/${ASSET}" -C "${STAGE}" --strip-components=1 ;;
    *)
      mkdir "${STAGE}/ui"
      tar -xzf "${STAGE}/${ASSET}" -C "${STAGE}/ui"
      ;;
  esac
  rm "${STAGE}/${ASSET}"
}

# lays out main's build like the release file. one checkout is kept so cargo builds
# incrementally, and only the latest commit's build next to it
build_main() {
  local base="${XDG_CACHE_HOME:-${HOME}/.cache}/tiles/tiles-ui" commit
  commit=$(git ls-remote "${REPO}" refs/heads/main | cut -f1)
  [[ -n "${commit}" ]] || { echo "Couldn't find tiles-ui's main branch" >&2; exit 1; }
  local src="${base}/src" built="${base}/${commit}"

  [[ -d "${src}/.git" ]] || git init --quiet "${src}"
  if [[ "$(git -C "${src}" rev-parse -q --verify HEAD || true)" != "${commit}" ]]; then
    echo "Getting tiles-ui main at ${commit:0:7}"
    git -C "${src}" fetch --quiet --depth 1 "${REPO}" "${commit}"
    git -C "${src}" checkout --quiet --force FETCH_HEAD
    (cd "${src}" && npm ci)
    find "${base}" -mindepth 1 -maxdepth 1 ! -name src ! -name "${commit}" -exec rm -rf {} +
  fi

  if [[ ! -d "${built}" ]]; then
    echo "Building tiles-ui main"
    local out="${base}/.${commit}"
    rm -rf "${out}"
    mkdir -p "${out}"
    (
      cd "${src}"
      case "${APP}" in
        Tiles.app)
          npx tauri build --bundles app
          ditto desktop/target/release/bundle/macos/Tiles.app "${out}/Tiles.app"
          ;;
        tiles-desktop)
          npx tauri build --no-bundle
          cp desktop/target/release/tiles-desktop desktop/linux/tiles.desktop "${out}/"
          cp desktop/icons/128x128@2x.png "${out}/tiles.png"
          ;;
        *) npm run build ;;
      esac
      # the Mac menu bar's panel builds next to the UI
      cp -R build "${out}/ui"
      rm -rf "${out}/ui/panel"
    )
    mv "${out}" "${built}"
  fi
  cp -R "${built}"/. "${STAGE}/"
}

if [[ -n "${TILES_UI_BUILD:-}" ]]; then
  echo "Using the tiles-ui build at ${TILES_UI_BUILD}"
  cp -R "${TILES_UI_BUILD}"/. "${STAGE}/"
elif is_released "${RELEASE}/${ASSET}"; then
  download
else
  echo "tiles-ui ${VERSION} isn't released yet, building its main branch"
  build_main
fi

[[ -f "${STAGE}/ui/index.html" ]] || { echo "No index.html in the tiles-ui build" >&2; exit 1; }
case "${APP}" in
  Tiles.app) [[ -d "${STAGE}/Tiles.app" ]] || { echo "No Tiles.app in the tiles-ui build" >&2; exit 1; } ;;
  tiles-desktop)
    # install.sh needs the launcher entry and icon too
    for file in tiles-desktop tiles.desktop tiles.png; do
      [[ -f "${STAGE}/${file}" ]] || { echo "No ${file} in the tiles-ui build" >&2; exit 1; }
    done
    ;;
esac

rm -rf "${ROOT}/ui" "${ROOT}/desktop"
mv "${STAGE}/ui" "${ROOT}/ui"
if [[ -n "${APP}" ]]; then
  # mktemp makes it private, the installed app must be readable by everyone
  chmod 755 "${STAGE}"
  mv "${STAGE}" "${ROOT}/desktop"
else
  echo "No desktop app for ${OS} on ${ARCH} yet, the bundle goes without it"
fi
