#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="tiles"
# Folder where final tar.gz will be created
DIST_DIR="dist"
# Folder where we store the modelfiles, which will be copied to installer
MODELFILE_DIR="modelfiles"
# Py server folder, which will be copied to installer
SERVER_DIR="server"
# cargo build mode for production
TARGET="release"

# Fetching the tiles binary version from its cargo.toml version 
VERSION=$(grep '^version' tiles/Cargo.toml | head -1 | awk -F'"' '{print $2}')
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Final tar.gz name
OUT_NAME="${BINARY_NAME}-v${VERSION}-${ARCH}-${OS}"

echo "🚀 Building ${BINARY_NAME} (${TARGET} mode)..."

cargo build -p tiles --${TARGET}


# Destination where the release build is generated
CLI_BIN_PATH="target/${TARGET}/${BINARY_NAME}"
  
chmod +x "${CLI_BIN_PATH}"

# echo "Signing the Tiles binary..."

# # Signing the tiles binary
# codesign --force \
#   --sign "$DEVELOPER_ID_APPLICATION"\
#   --options runtime \
#   --timestamp \
#   --strict \
#   "${CLI_BIN_PATH}"

# # echo "Notarizing Tiles binary..."

# # notarizing the tiles binary
# xcrun notarytool submit --force "${CLI_BIN_PATH}" \
#   --keychain-profile "tiles-notary-profile" \
#   --wait


mkdir -p "${DIST_DIR}/tmp"

cp "${CLI_BIN_PATH}"  "${DIST_DIR}/tmp/"

echo "Embedding Pi"
# Copying pi artifacts into extracted pi folder
cp pi-darwin-arm64.tar.gz "${DIST_DIR}/tmp/"

tar -xvf "${DIST_DIR}/tmp/pi-darwin-arm64.tar.gz" -C "${DIST_DIR}/tmp"

rm "${DIST_DIR}/tmp/pi-darwin-arm64.tar.gz"

# removing unnecessary files
rm -rf "${DIST_DIR}/tmp/pi/examples"

# Signing the pi binary
# codesign --force \
#   --sign "$DEVELOPER_ID_APPLICATION"\
#   --options runtime \
#   --timestamp \
#   --strict \
#   "${DIST_DIR}/tmp/pi/pi" 

# echo "Notarizing Pibinary..."

# # notarizing the pi binary
# xcrun notarytool submit --force "${DIST_DIR}/tmp/pi/pi" \
#   --keychain-profile "tiles-notary-profile" \
#   --wait

# flushing this folder, else the final zip will have previous app-server zips too (#84)
# rm -rf "${SERVER_DIR}/stack_export_prod"

# echo "🔒 Locking the venvstack...."

# venvstacks lock server/stack/venvstacks.toml

# echo "🛠️ Building the venvstack...."

# venvstacks build server/stack/venvstacks.toml

# echo "📦 Publishing the venvstack...."

# venvstacks publish --tag-outputs --output-dir ../stack_export_prod server/stack/venvstacks.toml

cp -r "${SERVER_DIR}" "${DIST_DIR}/tmp/"

rm -rf "${DIST_DIR}/tmp/server/__pycache__"
rm -rf "${DIST_DIR}/tmp/server/mem_agent/__pycache__"
rm -rf "${DIST_DIR}/tmp/server/backend/__pycache__"
rm -rf "${DIST_DIR}/tmp/server/.venv"
rm -rf "${DIST_DIR}/tmp/server/stack"

cp -r "${MODELFILE_DIR}" "${DIST_DIR}/tmp/"

echo "📦 Creating ${OUT_NAME}.tar.gz..."
tar --exclude-from=scripts/tar.exclude -czf "${DIST_DIR}/${OUT_NAME}.tar.gz" -C "${DIST_DIR}/tmp" .

rm -rf "${DIST_DIR}/tmp"

echo "✅ Bundle created: ${DIST_DIR}/${OUT_NAME}.tar.gz"
