#!/usr/bin/env bash
set -euo pipefail

ENV="prod" # prod is another env, try taking it from github env
REPO="tilesprivacy/tiles"
# VERSION="${TILES_VERSION:-latest}"
VERSION="0.4.3"
# INSTALL_DIR="$HOME/.local/bin"           # CLI install location
INSTALL_DIR="/usr/local/bin"           # CLI install location
# SERVER_DIR="$HOME/.local/lib/tiles/server"         # Python server folder
# MODELFILE_DIR="$HOME/.local/lib/tiles/modelfiles"  # Python server folder

SERVER_DIR="/usr/local/share/tiles/server"         # Python server folder
MODELFILE_DIR="/usr/local/share/tiles/modelfiles"  # Python server folder

TMPDIR="$(mktemp -d)"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)


log() { echo -e "\033[1;36m$*\033[0m"; }
err() { echo -e "\033[1;31m$*\033[0m" >&2; exit 1; }
warn() {
  printf "\033[1;33m%s\033[0m\n" "$*"
}

log "⬇️  Downloading Tiles (${VERSION}) for ${ARCH}-${OS}..."


if [[ "$ENV" == "prod" ]]; then
  TAR_URL="https://github.com/${REPO}/releases/download/${VERSION}/tiles-v${VERSION}-${ARCH}-${OS}.tar.gz"
  curl -fL -o "${TMPDIR}/tiles.tar.gz" "$TAR_URL"
else
  # Installer suppose to ran from tiles root folder after running the bundler
  mv "dist/tiles-v${VERSION}-${ARCH}-${OS}.tar.gz" "${TMPDIR}/tiles.tar.gz"
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
framework-mlx/bin/python framework-mlx/postinstall.py
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
