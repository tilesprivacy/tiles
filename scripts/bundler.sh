#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="tiles"
DIST_DIR="dist"
SERVER_DIR="server"
TARGET="release"

VERSION=$(grep '^version' tiles/Cargo.toml | head -1 | awk -F'"' '{print $2}')
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
OUT_NAME="${BINARY_NAME}-v${VERSION}-${ARCH}-${OS}"

echo "🚀 Building ${BINARY_NAME} (${TARGET} mode)..."

cargo build -p tiles --${TARGET}

mkdir -p "${DIST_DIR}/tmp"
cp "target/${TARGET}/${BINARY_NAME}" "${DIST_DIR}/tmp/"

echo "🔒 Locking the venvstack...."

venvstacks lock server/stack/venvstacks.toml

echo "🛠️ Building the venvstack...."

venvstacks build server/stack/venvstacks.toml

echo "📦 Publishing the venvstack...."

venvstacks publish --tag-outputs --output-dir ../stack_export_prod server/stack/venvstacks.toml

cp -r "${SERVER_DIR}" "${DIST_DIR}/tmp/"

rm -rf "${DIST_DIR}/tmp/server/__pycache__"
rm -rf "${DIST_DIR}/tmp/server/.venv"
rm -rf "${DIST_DIR}/tmp/server/stack"

echo "📦 Creating ${OUT_NAME}.tar.gz..."
tar --exclude-from=scripts/tar.exclude -czf "${DIST_DIR}/${OUT_NAME}.tar.gz" -C "${DIST_DIR}/tmp" .

rm -rf "${DIST_DIR}/tmp"

echo "✅ Bundle created: ${DIST_DIR}/${OUT_NAME}.tar.gz"
