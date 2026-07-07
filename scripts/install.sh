#!/usr/bin/env bash
set -euo pipefail

REPO="tilesprivacy/tiles"

VERSION="0.4.15"
DEV="false"

for arg in "$@"; do
  case "$arg" in
    --dev|-dev)
      DEV="true"
      ;;
    -h|--help)
      echo "Usage: scripts/install.sh [--dev]"
      echo ""
      echo "  --dev   Install from a local dist/*.tar.gz instead of GitHub"
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Usage: scripts/install.sh [--dev]" >&2
      exit 1
      ;;
  esac
done

INSTALL_DIR="/usr/local/bin"           # CLI install location

SERVER_DIR="/usr/local/share/tiles/server"         # Python server folder
MODELFILE_DIR="/usr/local/share/tiles/modelfiles"  # Modelfile server folder

PI_DIR="/usr/local/share/tiles/pi"

TMPDIR="$(mktemp -d)"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [[ "${OS}" == "linux" && "$(id -u)" != "0" ]]; then
  INSTALL_DIR="${HOME}/.local/bin"
  LIB_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/tiles"
else
  INSTALL_DIR="/usr/local/bin"
  LIB_DIR="/usr/local/share/tiles"
fi

SERVER_DIR="${LIB_DIR}/server"         # Python server folder
MODELFILE_DIR="${LIB_DIR}/modelfiles"  # Modelfile server folder
PI_DIR="${LIB_DIR}/pi"

TMPDIR="$(mktemp -d)"

log() { echo -e "\033[1;36m$*\033[0m"; }
err() { echo -e "\033[1;31m$*\033[0m" >&2; exit 1; }
warn() {
  printf "\033[1;33m%s\033[0m\n" "$*"
}

if [[ "${DEV}" == "true" ]]; then
  log "⬇️  Installing local Tiles build for ${ARCH}-${OS}..."
else
  log "⬇️  Downloading Tiles (${VERSION}) for ${ARCH}-${OS}..."
fi

if [[ "${DEV}" == "false" ]]; then
  TAR_URL="https://github.com/${REPO}/releases/download/${VERSION}/tiles-v${VERSION}-${ARCH}-${OS}.tar.gz"
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

log "🔧 Setting up Python environment..."
cd "${SERVER_DIR}/stack_export_prod"

for f in *.tar.xz; do
  tar -xvf "$f"
done

rm -rf *.tar.xz

cpython3.13/bin/python cpython3.13/postinstall.py
if [[ -x framework-mlx/bin/python ]]; then
  framework-mlx/bin/python framework-mlx/postinstall.py
fi
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
