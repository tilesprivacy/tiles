#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="tiles"
DIST_DIR="dist"
MODELFILE_DIR="modelfiles"
SERVER_DIR="server"
TARGET="release"

VERSION=$(grep '^version' tiles/Cargo.toml | head -1 | awk -F'"' '{print $2}')
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

OUT_NAME="${BINARY_NAME}-v${VERSION}-${ARCH}-${OS}"

echo "🚀 Building ${BINARY_NAME} (${TARGET} mode)..."

cargo build -p tiles --${TARGET}

CLI_BIN_PATH="target/${TARGET}/${BINARY_NAME}"

PKG_CLI_BIN_PATH="pkgroot/usr/local/bin"

PKG_LIBS_PATH="pkgroot/usr/local/share/tiles"

# CLI binary pkg install path
mkdir -p "${CLI_BIN_PATH}"

# Other libs pkg install path
mkdir -p "${PKG_LIBS_PATH}"

# move cli to bin path

cp "${CLI_BIN_PATH}" "${PKG_CLI_BIN_PATH}"

chmod +x "${PKG_CLI_BIN_PATH}/tiles"

# Signing the tiles binary
codesign --force \
  --sign "$DEVELOPER_ID_APPLICATION"\
  --options runtime \
  --timestamp \
  --strict \
  "${PKG_CLI_BIN_PATH}/tiles"

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

cp -r "${SERVER_DIR}" "${PKG_LIBS_PATH}"

rm -rf "${PKG_LIBS_PATH}/server/__pycache__"
rm -rf "${PKG_LIBS_PATH}/server/mem_agent/__pycache__"
rm -rf "${PKG_LIBS_PATH}/server/backend/__pycache__"
rm -rf "${PKG_LIBS_PATH}/server/.venv"
rm -rf "${PKG_LIBS_PATH}/server/stack"

cp -r "${MODELFILE_DIR}" "${PKG_LIBS_PATH}"


# Creating .pkg
pkgbuild --root pkgroot --scripts pkg/scripts --identifier com.tilesprivacy.tiles --version "$VERSION" pkg/tiles-unsigned.pkg

