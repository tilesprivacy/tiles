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

# Backend precedence: --backend flag, then TILES_LLAMA_BACKEND, then the OS default.
BACKEND_FLAG=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --backend)
      [[ $# -ge 2 ]] || { echo "--backend requires a value" >&2; exit 1; }
      BACKEND_FLAG="$2"
      shift 2
      ;;
    *) echo "Usage: $0 [--backend cuda|vulkan]" >&2; exit 1 ;;
  esac
done

# Fetching the tiles binary version from its cargo.toml version
VERSION=$(grep '^version' tiles/Cargo.toml | head -1 | awk -F'"' '{print $2}')
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "${OS}" in
  darwin) STACK_SPEC="server/stack/macos/venvstacks.toml"; LLAMA_BACKEND="${BACKEND_FLAG:-${TILES_LLAMA_BACKEND:-metal}}" ;;
  linux) STACK_SPEC="server/stack/linux/venvstacks.toml"; LLAMA_BACKEND="${BACKEND_FLAG:-${TILES_LLAMA_BACKEND:-cuda}}" ;;
  *) echo "Unsupported OS for venvstack bundle: ${OS}" >&2; exit 1 ;;
esac
[[ "${OS}:${LLAMA_BACKEND}" == "darwin:metal" \
  || "${OS}:${LLAMA_BACKEND}" == "linux:cuda" \
  || "${OS}:${LLAMA_BACKEND}" == "linux:vulkan" ]] \
  || { echo "Unsupported llama backend ${LLAMA_BACKEND} on ${OS}" >&2; exit 1; }

# fetch_llama_server.sh reads the backend from the environment.
export TILES_LLAMA_BACKEND="${LLAMA_BACKEND}"
case "${ARCH}" in
  x86_64) PI_ARCH="x64" ;;
  aarch64|arm64) PI_ARCH="arm64" ;;
  *) echo "Unsupported architecture for Pi bundle: ${ARCH}" >&2; exit 1 ;;
esac
PI_TARBALL="pi-${OS}-${PI_ARCH}.tar.gz"

# Name for final tar.gz 

# Linux ships one tarball per inference backend, so the backend is part of the
# name. macOS stays unsuffixed: metal is the only backend there.
OUT_NAME="${BINARY_NAME}-v${VERSION}-${ARCH}-${OS}"
if [[ "${OS}" == "linux" ]]; then
  OUT_NAME="${OUT_NAME}-${LLAMA_BACKEND}"
fi

echo "🚀 Building ${BINARY_NAME} (${TARGET} mode, ${LLAMA_BACKEND} backend)..."

cargo build -p tiles --${TARGET}


# Destination where the release binary is generated
CLI_BIN_PATH="target/${TARGET}/${BINARY_NAME}"
  
chmod +x "${CLI_BIN_PATH}"

if [[ "${OS}" == "darwin" ]]; then
  echo "Signing the Tiles binary..."

  codesign --force \
    --sign "$DEVELOPER_ID_APPLICATION"\
    --options runtime \
    --timestamp \
    --strict \
    "${CLI_BIN_PATH}"
fi


mkdir -p "${DIST_DIR}/tmp"

cp "${CLI_BIN_PATH}"  "${DIST_DIR}/tmp/"

echo "Embedding Pi"
if [[ ! -f "${PI_TARBALL}" ]]; then
  echo "Missing Pi artifact: ${PI_TARBALL}" >&2
  echo "Download or build it before bundling." >&2
  exit 1
fi

cp "${PI_TARBALL}" "${DIST_DIR}/tmp/"

tar -xvf "${DIST_DIR}/tmp/${PI_TARBALL}" -C "${DIST_DIR}/tmp"

rm "${DIST_DIR}/tmp/${PI_TARBALL}"

# removing unnecessary files
# rm -rf "${DIST_DIR}/tmp/pi/examples"

# Signing the pi binary

if [[ "${OS}" == "darwin" ]]; then
  echo "Signing Pi binary..."

  codesign --force \
    --sign "$DEVELOPER_ID_APPLICATION" \
    --options runtime \
    --timestamp \
    --entitlements entitleme.plist \
    --strict \
    "${DIST_DIR}/tmp/pi/pi"
fi


echo "🧩 Provisioning llama-server binary into ${SERVER_DIR}/bin..."
./scripts/fetch_llama_server.sh

if [[ "${OS}" == "darwin" ]]; then
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
fi

# flushing this folder, else the final zip will have previous app-server zips too (#84)
rm -rf "${SERVER_DIR}/stack_export_prod"

echo "🔒 Locking the venvstack...."

venvstacks lock "${STACK_SPEC}"

echo "🛠️ Building the venvstack...."

venvstacks build "${STACK_SPEC}"

echo "📦 Publishing the venvstack...."

venvstacks publish --tag-outputs --output-dir ../../stack_export_prod "${STACK_SPEC}"

cp -r "${SERVER_DIR}" "${DIST_DIR}/tmp/"

rm -rf "${DIST_DIR}/tmp/server/__pycache__"
# rm -rf "${DIST_DIR}/tmp/server/mem_agent/__pycache__"
# rm -rf "${DIST_DIR}/tmp/server/backend/__pycache__"
rm -rf "${DIST_DIR}/tmp/server/.venv"
rm -rf "${DIST_DIR}/tmp/server/stack"

cp -r "${MODELFILE_DIR}" "${DIST_DIR}/tmp/"

echo "📦 Creating ${OUT_NAME}.tar.gz..."
tar --exclude-from=scripts/tar.exclude -czf "${DIST_DIR}/${OUT_NAME}.tar.gz" -C "${DIST_DIR}/tmp" .

rm -rf "${DIST_DIR}/tmp"

echo "✅ Bundle created: ${DIST_DIR}/${OUT_NAME}.tar.gz"
