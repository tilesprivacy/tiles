#!/bin/sh
# Support `curl ... | sh` by re-executing under bash (this script uses bash features).
if [ -z "${BASH_VERSION:-}" ]; then
  if ! command -v bash >/dev/null 2>&1; then
    echo "bash is required to install Tiles" >&2
    exit 1
  fi
  # File invocation: re-exec the same path with bash.
  if [ -f "$0" ] && [ -r "$0" ]; then
    case "$0" in
      sh|-sh|*/sh) ;;
      *) exec bash "$0" "$@" ;;
    esac
  fi
  # Piped invocation (`curl | sh`): remaining stdin is the rest of this script.
  # Avoid `exec` + EXIT trap (that can delete the temp file before bash starts).
  _tiles_install_tmp="$(mktemp)"
  cat > "${_tiles_install_tmp}"
  bash "${_tiles_install_tmp}" "$@"
  _tiles_install_status=$?
  rm -f "${_tiles_install_tmp}"
  exit "${_tiles_install_status}"
fi

set -euo pipefail

REPO="tilesprivacy/tiles"

# the latest release unless --version picks one
VERSION=""
DEV="false"
NIGHTLY="false"
BACKEND=""
HEADLESS="false"
INSTALL_DIR_OVERRIDE=""
LIB_DIR_OVERRIDE=""

log() { echo -e "\033[1;36m$*\033[0m"; }
err() { echo -e "\033[1;31m$*\033[0m" >&2; exit 1; }
warn() {
  printf "\033[1;33m%s\033[0m\n" "$*"
}

usage() {
  echo "Usage: curl -LsSf https://www.tiles.run/install.sh | sh -s -- [OPTIONS]"
  echo "   or: scripts/install.sh [OPTIONS]"
  echo ""
  echo "  --dev                Install from a local dist/*.tar.gz instead of GitHub"
  echo "  --version VERSION    Install that release, like an rc (default: the latest release)"
  echo "  --nightly            Install the latest nightly GitHub release"
  echo "                       (e.g. tiles-v0.4.17-x86_64-linux.tar.gz)"
  echo "  --headless           Leave out the Tiles app and the web UI, for servers and Tilekit"
  echo "  --install-dir PATH   Override the binary installation directory"
  echo "  --lib-dir PATH       Override the runtime installation directory"
}

# github.com redirects /releases/latest to the newest release that isn't a pre-release
latest_version() {
  local location
  location="$(curl -fsSI "https://github.com/${REPO}/releases/latest" | tr -d '\r' | grep -i '^location:' || true)"
  [[ "${location}" == */releases/tag/* ]] || err "Couldn't find the latest Tiles release on GitHub."
  echo "${location##*/}"
}

# Resolve the newest GitHub nightly release that has a tarball for this platform.
# The asset version may differ from the tag, e.g. tag 0.4.17-nightly.2 shipping
# a nightly.1 Linux tarball.
# Sets RELEASE_TAG, VERSION (for logs), and RELEASE_ASSET (exact filename).
resolve_nightly_version() {
  local api_url="https://api.github.com/repos/${REPO}/releases?per_page=30"
  local releases_json tag tags release_json asset
  local platform="${ARCH}-${OS}"

  releases_json="$(curl -fsSL "${api_url}")" || err "Failed to query GitHub releases for ${REPO}."

  tags="$(
    printf '%s' "${releases_json}" \
      | grep -oE '"tag_name"[[:space:]]*:[[:space:]]*"[^"]+"' \
      | sed -E 's/.*"([^"]+)".*/\1/' \
      | grep -i 'nightly' \
      || true
  )"

  # Walk nightly tags (API returns newest first) and inspect that release's assets.
  while IFS= read -r tag; do
    [ -z "${tag}" ] && continue
    release_json="$(
      curl -fsSL "https://api.github.com/repos/${REPO}/releases/tags/${tag}" 2>/dev/null || true
    )"
    [ -z "${release_json}" ] && continue

    asset="$(
      printf '%s' "${release_json}" \
        | grep -oE "\"name\"[[:space:]]*:[[:space:]]*\"tiles-v[^\"]*-${platform}\\.tar\\.gz\"" \
        | sed -E 's/.*"([^"]+)".*/\1/' \
        | head -n 1 \
        || true
    )"
    [ -z "${asset}" ] && continue

    RELEASE_TAG="${tag}"
    VERSION="${tag#v}"
    RELEASE_ASSET="${asset}"
    return 0
  done <<EOF
${tags}
EOF

  err "No nightly found for ${platform} (no matching tiles-v*-${platform}.tar.gz on GitHub)."
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dev|-dev)
      DEV="true"
      ;;
    --nightly|-nightly)
      NIGHTLY="true"
      ;;
    --version)
      [[ $# -ge 2 && -n "$2" ]] || err "--version requires a value."
      VERSION="${2#v}"
      [[ -n "${VERSION}" ]] || err "--version requires a value."
      shift
      ;;
    --headless|-headless)
      HEADLESS="true"
      ;;
    --backend)
      [[ $# -ge 2 ]] || err "--backend requires a value."
      BACKEND="$2"
      shift
      ;;
    --install-dir)
      [[ $# -ge 2 ]] || err "--install-dir requires a value."
      [[ -n "$2" ]] || err "--install-dir requires a non-empty value."
      INSTALL_DIR_OVERRIDE="$2"
      shift
      ;;
    --lib-dir)
      [[ $# -ge 2 ]] || err "--lib-dir requires a value."
      [[ -n "$2" ]] || err "--lib-dir requires a non-empty value."
      LIB_DIR_OVERRIDE="$2"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
  shift
done

if [[ "${DEV}" == "true" && "${NIGHTLY}" == "true" ]]; then
  err "--dev and --nightly cannot be used together."
fi
if [[ -n "${VERSION}" && ( "${DEV}" == "true" || "${NIGHTLY}" == "true" ) ]]; then
  err "--version cannot be used with --dev or --nightly."
fi

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

[[ "${OS}" == "linux" || "${OS}" == "darwin" ]] || err "Unsupported OS: ${OS}."

if [[ -n "${BACKEND}" ]]; then
  warn "⚠️  --backend is no longer needed. Tiles asks to download the GPU engine the first time it runs a model."
fi

if [[ "${OS}" == "linux" && "$(id -u)" != "0" ]]; then
  INSTALL_DIR="${HOME}/.local/bin"
  LIB_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/tiles"
else
  INSTALL_DIR="/usr/local/bin"
  LIB_DIR="/usr/local/share/tiles"
fi

[[ -n "${INSTALL_DIR_OVERRIDE}" ]] && INSTALL_DIR="${INSTALL_DIR_OVERRIDE}"
[[ -n "${LIB_DIR_OVERRIDE}" ]] && LIB_DIR="${LIB_DIR_OVERRIDE}"

SERVER_DIR="${LIB_DIR}/server"         # Python server folder
MODELFILE_DIR="${LIB_DIR}/modelfiles"  # Modelfile server folder
PI_DIR="${LIB_DIR}/pi"
UI_DIR="${LIB_DIR}/ui"                 # web UI the daemon serves on port 1730

# the Tiles app. on Linux its launcher entry and icon go where the desktop looks for them
MAC_APP="/Applications/Tiles.app"
DESKTOP_BIN="${LIB_DIR}/tiles-desktop"
if [[ "$(id -u)" == "0" ]]; then
  SHARE_DIR="/usr/local/share"
else
  SHARE_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}"
fi
DESKTOP_ENTRY="${SHARE_DIR}/applications/tiles.desktop"
DESKTOP_ICON="${SHARE_DIR}/icons/hicolor/256x256/apps/tiles.png"

if [[ "${DEV}" == "false" && "${NIGHTLY}" == "false" && -z "${VERSION}" ]]; then
  VERSION="$(latest_version)"
fi

TMPDIR="$(mktemp -d)"
RELEASE_TAG="${VERSION}"
RELEASE_ASSET="tiles-v${VERSION}-${ARCH}-${OS}.tar.gz"

if [[ "${NIGHTLY}" == "true" ]]; then
  resolve_nightly_version
  log "⬇️  Downloading Tiles nightly (${VERSION}) [${RELEASE_ASSET}] for ${ARCH}-${OS}..."
elif [[ "${DEV}" == "true" ]]; then
  log "⬇️  Installing local Tiles build for ${ARCH}-${OS}..."
else
  log "⬇️  Downloading Tiles (${VERSION}) for ${ARCH}-${OS}..."
fi

if [[ "${DEV}" == "false" ]]; then
  # Stable (default) and --nightly both download from GitHub.
  # Unflagged install gets the latest release, or the one --version picks.
  # Nightly uses RELEASE_ASSET from resolve_nightly_version (may differ from tag).
  TAR_URL="https://github.com/${REPO}/releases/download/${RELEASE_TAG}/${RELEASE_ASSET}"
  curl -fL -o "${TMPDIR}/tiles.tar.gz" "$TAR_URL"
else
  if [[ -n "${TILES_DEV_TARBALL:-}" ]]; then
    LOCAL_TARBALL="${TILES_DEV_TARBALL}"
  else
    LOCAL_TARBALL=""
    shopt -s nullglob
    LOCAL_TARBALLS=(dist/tiles-v*-"${ARCH}"-"${OS}".tar.gz)
    shopt -u nullglob
    for candidate in "${LOCAL_TARBALLS[@]}"; do
      if [[ -z "${LOCAL_TARBALL}" || "${candidate}" -nt "${LOCAL_TARBALL}" ]]; then
        LOCAL_TARBALL="${candidate}"
      fi
    done
  fi

  if [[ -z "${LOCAL_TARBALL}" || ! -f "${LOCAL_TARBALL}" ]]; then
    err "No local Tiles tarball found. Run just bundle first, or set TILES_DEV_TARBALL=/path/to/tar.gz."
  fi

  log "📦 Using local Tiles tarball ${LOCAL_TARBALL}"
  cp "${LOCAL_TARBALL}" "${TMPDIR}/tiles.tar.gz"
fi

# a daemon left running would keep serving the old version, so stop it first
stop_running_daemon() {
  local url="http://127.0.0.1:1729"
  curl -fsS -m 2 -o /dev/null "${url}/" 2>/dev/null || return 0

  log "⏹️ Stopping the running Tiles daemon..."
  curl -fsS -m 5 -o /dev/null -X POST -H "Content-Type: application/json" "${url}/shutdown" 2>/dev/null \
    || curl -fsS -m 5 -o /dev/null "${url}/shutdown" 2>/dev/null \
    || true

  for _ in $(seq 40); do
    curl -fsS -m 1 -o /dev/null "${url}/" 2>/dev/null || return 0
    sleep 0.25
  done
  warn "⚠️  The Tiles daemon is still running. Run \`tiles daemon stop\` after the update."
}

stop_running_daemon

echo "⬇️ Installing tiles..."
# Lets point to tile repo
tar -xzf "${TMPDIR}/tiles.tar.gz" -C "${TMPDIR}"

log "📦 Installing tiles binary to ${INSTALL_DIR}..."
mkdir -p "${INSTALL_DIR}"
install -m 755 "${TMPDIR}/tiles" "${INSTALL_DIR}/tiles"

log "Unpacking libs ..."
rm -rf "${MODELFILE_DIR}"

mkdir -p "${MODELFILE_DIR}"

cp -r "${TMPDIR}/modelfiles"/* "${MODELFILE_DIR}/"


log "Installing PI artifacts ..."

rm -rf "${PI_DIR}"

mkdir -p "${PI_DIR}"

cp -r "${TMPDIR}/pi"/* "${PI_DIR}/"

# releases from before the web UI have no ui folder
if [[ "${HEADLESS}" == "true" ]]; then
  rm -rf "${UI_DIR}"
elif [[ -d "${TMPDIR}/ui" ]]; then
  log "Installing the web UI ..."
  rm -rf "${UI_DIR}"
  mkdir -p "${UI_DIR}"
  cp -r "${TMPDIR}/ui"/. "${UI_DIR}/"
fi

# remembered for `tiles update`, which runs this script again without the flag
if [[ "${HEADLESS}" == "true" ]]; then
  touch "${LIB_DIR}/.headless"
else
  rm -f "${LIB_DIR}/.headless"
fi

APP_INSTALLED="false"
# a headless install also takes away an app an earlier install left
if [[ "${HEADLESS}" == "true" ]]; then
  log "Leaving out the Tiles app and the web UI"
  [[ "${OS}" == "darwin" ]] && rm -rf "${MAC_APP}"
  rm -f "${DESKTOP_BIN}" "${DESKTOP_ENTRY}" "${DESKTOP_ICON}"
elif [[ -d "${TMPDIR}/desktop" ]]; then
  log "Installing the Tiles app ..."
  if [[ "${OS}" == "darwin" ]]; then
    rm -rf "${MAC_APP}"
    cp -R "${TMPDIR}/desktop/Tiles.app" "${MAC_APP}"
  else
    install -m 755 "${TMPDIR}/desktop/tiles-desktop" "${DESKTOP_BIN}"
    mkdir -p "$(dirname "${DESKTOP_ENTRY}")" "$(dirname "${DESKTOP_ICON}")"
    install -m 644 "${TMPDIR}/desktop/tiles.png" "${DESKTOP_ICON}"
    # an icon cache another app left would hide the new icon
    HICOLOR_DIR="${SHARE_DIR}/icons/hicolor"
    touch "${HICOLOR_DIR}"
    gtk-update-icon-cache -q -f -t "${HICOLOR_DIR}" >/dev/null 2>&1 || true
    sed "s|^Exec=.*|Exec=\"${DESKTOP_BIN}\"|" "${TMPDIR}/desktop/tiles.desktop" > "${DESKTOP_ENTRY}"
    update-desktop-database "$(dirname "${DESKTOP_ENTRY}")" >/dev/null 2>&1 || true
    if command -v ldconfig >/dev/null 2>&1 \
      && ! ldconfig -p 2>/dev/null | grep 'libwebkit2gtk-4\.1\.so\.0' >/dev/null; then
      warn "⚠️  The Tiles app needs WebKitGTK 4.1, and it isn't installed."
      warn "    Install libwebkit2gtk-4.1-0 (apt), webkit2gtk4.1 (dnf) or webkit2gtk-4.1 (pacman)."
      warn "    Until then \`tiles ui\` opens Tiles in your browser."
    fi
  fi
  APP_INSTALLED="true"
fi


log "📦 Installing Python server to ${SERVER_DIR}..."
rm -rf "${SERVER_DIR}"

mkdir -p "${SERVER_DIR}"

cp -r "${TMPDIR}/server"/* "${SERVER_DIR}/"

chmod +x "${SERVER_DIR}/bin/llama-server" 2>/dev/null || true

log "🔧 Setting up Python environment..."
cd "${SERVER_DIR}/stack_export_prod"

for f in *.tar.xz; do
  tar -xvf "$f"
done

rm -rf *.tar.xz

cpython3.13/bin/python cpython3.13/postinstall.py
app-server/bin/python app-server/postinstall.py

rm -rf "${TMPDIR}"

log "✅ Tiles installed successfully!"
log ""
[[ "${APP_INSTALLED}" == "true" ]] && echo "🪟 Open Tiles from your apps, or run \"tiles ui\""

case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    echo "🚀 Start Tiles by running \"tiles\""
    ;;
  *)
    echo ""
    warn "⚠️  $INSTALL_DIR is not in your PATH."
    echo ""
    echo "ℹ️  To use Tiles, add this line to your shell configuration(ex: ~/.bashrc, ~/.zshrc)"
    echo ""
    echo "  export PATH=$INSTALL_DIR:\$PATH"
    echo ""
    echo "🚀 Then restart your terminal..."
    ;;
esac
