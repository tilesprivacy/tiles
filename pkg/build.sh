#!/usr/bin/env bash

set -euo pipefail

VERSION=$(grep '^version' tiles/Cargo.toml | head -1 | awk -F'"' '{print $2}')

TARGET="release"
MODELFILE_DIR="modelfiles"
SERVER_DIR="server"
BINARY_NAME="tiles"

VERSION=$(grep '^version' tiles/Cargo.toml | head -1 | awk -F'"' '{print $2}')
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
OUT_NAME="${BINARY_NAME}-v${VERSION}-${ARCH}-${OS}"

echo "🚀 Building ${BINARY_NAME} (${TARGET} mode)..."

cargo build -p tiles --${TARGET}

CLI_BIN_PATH="pkgroot/usr/local/bin"
LIBS_PATH="pkgroot/usr/local/share/tiles"

# CLI binary install path

mkdir -p "${CLI_BIN_PATH}"

mkdir -p "${LIBS_PATH}"


# move cli to bin path

cp "target/${TARGET}/${BINARY_NAME}" "${CLI_BIN_PATH}"
chmod +x "${CLI_BIN_PATH}/tiles"


# Build venvstack and move to /usr/local/share/tiles
# 
# flushing this folder, else the final zip will have previous app-server zips too (#84)

rm -rf "${SERVER_DIR}/stack_export_prod"

echo "🔒 Locking the venvstack...."

venvstacks lock server/stack/venvstacks.toml

echo "🛠️ Building the venvstack...."

venvstacks build server/stack/venvstacks.toml

echo "📦 Publishing the venvstack...."

venvstacks publish --tag-outputs --output-dir ../stack_export_prod server/stack/venvstacks.toml

cp -r "${SERVER_DIR}" "${LIBS_PATH}"

rm -rf "${LIBS_PATH}/server/__pycache__"
rm -rf "${LIBS_PATH}/server/mem_agent/__pycache__"
rm -rf "${LIBS_PATH}/server/backend/__pycache__"
rm -rf "${LIBS_PATH}/server/.venv"
rm -rf "${LIBS_PATH}/server/stack"

cp -r "${MODELFILE_DIR}" "${LIBS_PATH}"


# Creating .pkg
pkgbuild --root pkgroot --scripts pkg/scripts --identifier com.tilesprivacy.tiles --version "$VERSION" "tiles-${VERSION}".pkg"


# signing


# notarizing
