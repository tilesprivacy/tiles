#!/usr/bin/env bash
# Puts tiles-ui's web UI in ui/ and its desktop app in desktop/, from the release
# toolchain.toml names, or built from tiles-ui main until that release is out.
# `latest` there follows tiles-ui's newest release, and never builds main.
#
# Environment variables:
#   TILES_UI_BUILD  A local folder laid out like the release file, to use instead of it.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
REPO="https://github.com/tilesprivacy/tiles-ui"
VERSION=$(grep '^tiles-ui' "${ROOT}/toolchain.toml" | head -1 | awk -F'"' '{print $2}')
LATEST="false"
# a local build needs no lookup
if [[ "${VERSION}" == "latest" && -z "${TILES_UI_BUILD:-}" ]]; then
  # GitHub's latest link lands on the newest release's tag
  LANDED=$(curl -fsSLI --retry 3 -o /dev/null -w '%{url_effective}' "${REPO}/releases/latest" || true)
  if [[ "${LANDED}" != */releases/tag/* ]]; then
    echo "Couldn't find tiles-ui's latest release" >&2
    exit 1
  fi
  VERSION="${LANDED##*/}"
  LATEST="true"
fi
RELEASE="${REPO}/releases/download/${VERSION}"

case "${OS}:${ARCH}" in
  darwin:*) ASSET="tiles-desktop-${VERSION}-macos.zip" APP="Tiles.app" ;;
  linux:x86_64) ASSET="tiles-desktop-${VERSION}-x86_64-linux.tar.gz" APP="tiles-desktop" ;;
  *) ASSET="tiles-ui-${VERSION}.tar.gz" APP="" ;;
esac

STAGE="$(mktemp -d "${ROOT}/.tiles-ui.XXXXXX")"
trap 'rm -rf "${STAGE}"' EXIT

# 404 means the release isn't out yet
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

# outside the repo, or cargo takes the app for part of this workspace
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
elif [[ "${LATEST}" == "true" ]]; then
  # the release exists before its desktop files are uploaded
  echo "tiles-ui ${VERSION} has no ${ASSET} yet, try again in a few minutes" >&2
  exit 1
else
  echo "tiles-ui ${VERSION} isn't released yet, building its main branch"
  build_main
fi

[[ -f "${STAGE}/ui/index.html" ]] || { echo "No index.html in the tiles-ui build" >&2; exit 1; }
case "${APP}" in
  Tiles.app) [[ -d "${STAGE}/Tiles.app" ]] || { echo "No Tiles.app in the tiles-ui build" >&2; exit 1; } ;;
  tiles-desktop)
    for file in tiles-desktop tiles.desktop tiles.png; do
      [[ -f "${STAGE}/${file}" ]] || { echo "No ${file} in the tiles-ui build" >&2; exit 1; }
    done
    ;;
esac

rm -rf "${ROOT}/ui" "${ROOT}/desktop"
mv "${STAGE}/ui" "${ROOT}/ui"
if [[ -n "${APP}" ]]; then
  # mktemp makes it private
  chmod 755 "${STAGE}"
  mv "${STAGE}" "${ROOT}/desktop"
else
  echo "No desktop app for ${OS} on ${ARCH} yet, the bundle goes without it"
fi
