#!/usr/bin/env bash
set -euo pipefail

ENV="prod" # prod is another env, try taking it from github env
REPO="tilesprivacy/tiles" 
# VERSION="${TILES_VERSION:-latest}"       
VERSION="0.4.0-rc.1"       
INSTALL_DIR="$HOME/.local/bin"           # CLI install location
SERVER_DIR="$HOME/.local/share/tiles/server"         # Python server folder
TMPDIR="$(mktemp -d)"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)


log() { echo -e "\033[1;36m$*\033[0m"; }
err() { echo -e "\033[1;31m$*\033[0m" >&2; exit 1; }

# echo "🔍 Checking Python..."
# if ! command -v python3 >/dev/null 2>&1; then
#   log "⚠️  Python 3.10+ not found."
#   if [[ "$OS" == "darwin" ]]; then
#     log "Installing via Homebrew..."
#     brew install python || err "Could not install Python automatically. Please install manually."
#   elif [[ -f /etc/debian_version ]]; then
#     log "Installing via apt..."
#     sudo apt-get update -y && sudo apt-get install -y python3 python3-venv
#   else
#     err "Please install Python manually: https://www.python.org/downloads/"
#   fi
# fi

# echo "🔍 Checking uv..."
# if ! command -v uv >/dev/null 2>&1; then
#   log "⬇️  Installing uv..."
#   curl -LsSf https://astral.sh/uv/install.sh | sh
#   export PATH="$HOME/.local/bin:$PATH"
# fi

log "⬇️  Downloading Tiles (${VERSION}) for ${ARCH}-${OS}..."


if [[ "$ENV" == "prod" ]]; then
  TAR_URL="https://github.com/${REPO}/releases/download/${VERSION}/tiles-v${VERSION}-${ARCH}-${OS}.tar.gz"
  curl -fsSL -o "${TMPDIR}/tiles.tar.gz" "$TAR_URL"
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

# Ensure Python 3.13 is available
# if ! command -v python3.13 >/dev/null 2>&1; then
#   if [[ "$OS" == "darwin" ]]; then
#     log "Installing Python 3.13 via Homebrew..."
#     brew install python@3.13 || err "Failed to install Python 3.13"
#   else
#     err "Python 3.13 is required but not found. Please install it manuallyv and retry installing tiles"
#   fi
# fi

# # Force uv to use Python 3.13
# uv venv --python python3.13
# uv sync --frozen || err "Dependency setup failed."
rm -rf "${TMPDIR}"

log "✅ Tiles installed successfully!"
log ""
log "👉 Make sure ${INSTALL_DIR} is in your PATH."
