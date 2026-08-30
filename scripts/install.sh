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

VERSION="0.4.19"
DEV="false"
NIGHTLY="false"
BACKEND="auto"
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
  echo "  --nightly            Install the latest nightly GitHub release"
  echo "                       (e.g. tiles-v0.4.17-x86_64-linux-cuda.tar.gz)"
  echo "  --backend BACKEND    Linux inference backend: auto (default), cuda, or vulkan"
  echo "  --install-dir PATH   Override the binary installation directory"
  echo "  --lib-dir PATH       Override the runtime installation directory"
}

# Resolve the newest GitHub nightly release that has a tarball for this platform.
# The asset version may differ from the tag, e.g. tag 0.4.17-nightly.2 shipping
# a nightly.1 Linux tarball.
# Sets RELEASE_TAG, VERSION (for logs), and RELEASE_ASSET (exact filename).
resolve_nightly_version() {
  local api_url="https://api.github.com/repos/${REPO}/releases?per_page=30"
  local releases_json tag tags release_json asset
  local platform="${ARCH}-${OS}${ASSET_SUFFIX}"

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

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [[ "${OS}" == "linux" ]]; then
  [[ "${BACKEND}" == "auto" || "${BACKEND}" == "cuda" || "${BACKEND}" == "vulkan" ]] \
    || err "Unsupported Linux backend: ${BACKEND}."

  # checks for NVIDIA userspace drivers and enumerates GPU availability
  # then checks if CUDA runtime is availabile
  if [[ "${BACKEND}" == "auto" ]]; then
    CUDA_RUNTIME="false"
    if command -v nvidia-smi >/dev/null 2>&1 && nvidia-smi -L >/dev/null 2>&1; then
      if command -v ldconfig >/dev/null 2>&1 \
        && ldconfig -p 2>/dev/null | grep -E 'libcudart\.so\.12([[:space:]]|$)' >/dev/null; then
        CUDA_RUNTIME="true"
      # ldconfig covers only the system cache, not pip/conda/tarball installs.
      # manually check paths
      elif [[ -n "${LD_LIBRARY_PATH:-}" ]]; then
        IFS=':' read -r -a LIBRARY_PATHS <<< "${LD_LIBRARY_PATH}"
        for LIBRARY_PATH in "${LIBRARY_PATHS[@]}"; do
          [[ -n "${LIBRARY_PATH}" ]] || LIBRARY_PATH="."
          if [[ -e "${LIBRARY_PATH}/libcudart.so.12" ]]; then
            CUDA_RUNTIME="true"
            break
          fi
        done
      fi
    fi

    if [[ "${CUDA_RUNTIME}" == "true" ]]; then
      BACKEND="cuda"
    else
      BACKEND="vulkan"
    fi
    log "Auto-selected ${BACKEND} inference backend."
  fi
elif [[ "${OS}" == "darwin" ]]; then
  [[ "${BACKEND}" == "auto" ]] || err "Backend ${BACKEND} is not supported on ${OS}."
  BACKEND="metal"
else
  err "Unsupported OS: ${OS}."
fi

if [[ "${BACKEND}" == "vulkan" ]] \
  && ! ldconfig -p 2>/dev/null | grep 'libvulkan\.so\.1' >/dev/null; then
  warn "⚠️  No Vulkan loader found (libvulkan.so.1)."
  warn "    Install your distro's vulkan-loader package."
fi

# Linux assets carry the backend in their name; macOS assets do not (metal is
# the only backend there). BACKEND is already resolved from "auto" by here.
ASSET_SUFFIX=""
[[ "${OS}" == "linux" ]] && ASSET_SUFFIX="-${BACKEND}"

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

TMPDIR="$(mktemp -d)"
RELEASE_TAG="${VERSION}"
RELEASE_ASSET="tiles-v${VERSION}-${ARCH}-${OS}${ASSET_SUFFIX}.tar.gz"

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
  # Unflagged install always uses the hardcoded VERSION above (never nightly).
  # Nightly uses RELEASE_ASSET from resolve_nightly_version (may differ from tag).
  TAR_URL="https://github.com/${REPO}/releases/download/${RELEASE_TAG}/${RELEASE_ASSET}"
  curl -fL -o "${TMPDIR}/tiles.tar.gz" "$TAR_URL"
else
  if [[ -n "${TILES_DEV_TARBALL:-}" ]]; then
    LOCAL_TARBALL="${TILES_DEV_TARBALL}"
  else
    LOCAL_TARBALL=""
    shopt -s nullglob
    LOCAL_TARBALLS=(dist/tiles-v*-"${ARCH}"-"${OS}${ASSET_SUFFIX}".tar.gz)
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
