#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="tiles"
DIST_DIR="dist"
MODELFILE_DIR="modelfiles"
SERVER_DIR="server"
TARGET="release"

VERSION=$(grep '^version' tiles/Cargo.toml | head -1 | awk -F'"' '{print $2}')
STACK_SPEC="server/stack/macos/venvstacks.toml"

echo "🚀 Building ${BINARY_NAME} (${TARGET} mode)..."

cargo build -p tiles --${TARGET}

CLI_BIN_PATH="target/${TARGET}/${BINARY_NAME}"

PKG_CLI_BIN_PATH="pkgroot/usr/local/bin"

PKG_LIBS_PATH="pkgroot/usr/local/share/tiles"

# CLI binary pkg install path
mkdir -p "${PKG_CLI_BIN_PATH}"

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


echo "Embedding Pi"
# Copying pi artifacts into extracted pi folder
cp pi-darwin-arm64.tar.gz "${PKG_LIBS_PATH}"

tar -xvf "${PKG_LIBS_PATH}/pi-darwin-arm64.tar.gz" -C "${PKG_LIBS_PATH}"

rm "${PKG_LIBS_PATH}/pi-darwin-arm64.tar.gz"

# removing unnecessary files
# rm -rf "${DIST_DIR}/tmp/pi/examples"

# Signing the pi binary

echo "Signing Pi binary..."

codesign --force \
  --sign "$DEVELOPER_ID_APPLICATION" \
  --options runtime \
  --timestamp \
  --entitlements entitleme.plist \
  --strict \
  "${PKG_LIBS_PATH}/pi/pi"

echo "Signing Pi native modules..."

find "${PKG_LIBS_PATH}/pi" -name '*.node' -type f | while read -r node_bin; do
  codesign --force \
    --sign "$DEVELOPER_ID_APPLICATION" \
    --options runtime \
    --timestamp \
    --strict \
    "$node_bin"
done

  
# Build venvstack and move to /usr/local/share/tiles
# 
# flushing this folder, else the final zip will have previous app-server zips too (#84)

rm -rf "${SERVER_DIR}/stack_export_prod"

echo "🔒 Locking the venvstack...."

venvstacks lock "${STACK_SPEC}"

echo "🛠️ Building the venvstack...."

venvstacks build "${STACK_SPEC}"

echo "📦 Publishing the venvstack...."

venvstacks publish --tag-outputs --output-dir ../../stack_export_prod "${STACK_SPEC}"

echo "🧩 Provisioning llama-server binary into ${SERVER_DIR}/bin..."
./scripts/fetch_llama_server.sh

echo "Signing llama-server binaries..."
for f in "${SERVER_DIR}/bin/llama-server" "${SERVER_DIR}/bin/"*.dylib; do
  [[ -e "$f" ]] || continue
  codesign --force \
    --sign "$DEVELOPER_ID_APPLICATION" \
    --options runtime \
    --timestamp \
    --entitlements entitleme.plist \
    --strict \
    "$f"
done

cp -r "${SERVER_DIR}" "${PKG_LIBS_PATH}"

rm -rf "${PKG_LIBS_PATH}/server/__pycache__"
rm -rf "${PKG_LIBS_PATH}/server/mem_agent/__pycache__"
rm -rf "${PKG_LIBS_PATH}/server/backend/__pycache__"
rm -rf "${PKG_LIBS_PATH}/server/.venv"
rm -rf "${PKG_LIBS_PATH}/server/stack"

cp -r "${MODELFILE_DIR}" "${PKG_LIBS_PATH}"


# Creating .pkg
pkgbuild --root pkgroot --scripts pkg/scripts --identifier com.tilesprivacy.tiles --version "$VERSION" pkg/tiles-unsigned.pkg
