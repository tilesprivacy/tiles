#!/usr/bin/env bash
# Provision the llama-server binary that comes with Tiles, from the official llama.cpp
# release pinned in toolchain.toml.
#   - Linux: the CPU build. Tiles downloads the GPU engine on first run.
#   - macOS: the Metal build.
#
# Environment variables:
#   LLAMA_CPP_TAG        The llama.cpp build (default: the pin in toolchain.toml).
#   FORCE_LLAMA_FETCH    Set to 1 to re-download even if a binary is present.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${ROOT}/server/bin"
OS="$(uname -s)"
LLAMA_CPP_TAG="${LLAMA_CPP_TAG:-$(grep '^llama.cpp' "${ROOT}/toolchain.toml" | head -1 | awk -F'"' '{print $2}')}"

case "${OS}:$(uname -m)" in
  Darwin:arm64)  ASSET="llama-${LLAMA_CPP_TAG}-bin-macos-arm64.tar.gz"; BACKEND="metal" ;;
  Darwin:x86_64) ASSET="llama-${LLAMA_CPP_TAG}-bin-macos-x64.tar.gz"; BACKEND="metal" ;;
  Linux:x86_64)  ASSET="llama-${LLAMA_CPP_TAG}-bin-ubuntu-x64.tar.gz"; BACKEND="cpu" ;;
  Linux:aarch64) ASSET="llama-${LLAMA_CPP_TAG}-bin-ubuntu-arm64.tar.gz"; BACKEND="cpu" ;;
  *) echo "Unsupported platform: ${OS} $(uname -m)" >&2; exit 1 ;;
esac

# Reuse an already-provisioned binary. Set FORCE_LLAMA_FETCH=1 to re-download.
if [[ -x "${OUT_DIR}/llama-server" && -f "${OUT_DIR}/.llama-backend" \
  && "$(<"${OUT_DIR}/.llama-backend")" == "${BACKEND} ${LLAMA_CPP_TAG}" \
  && "${FORCE_LLAMA_FETCH:-}" != "1" ]]; then
  echo "llama-server already present at ${OUT_DIR}/llama-server; skipping fetch (set FORCE_LLAMA_FETCH=1 to force)."
  exit 0
fi

mkdir -p "${OUT_DIR}"
# keep the download on disk rather than in a RAM-backed /tmp
mkdir -p "${ROOT}/.cache"
TMP="$(mktemp -d "${ROOT}/.cache/llama.XXXXXX")"
trap 'rm -rf "${TMP}"' EXIT

echo "Downloading ${ASSET}"
curl -fsSL --retry 3 "https://github.com/ggml-org/llama.cpp/releases/download/${LLAMA_CPP_TAG}/${ASSET}" \
  | tar -xz -C "${TMP}"

SERVER_BIN="$(find "${TMP}" -type f -name llama-server | head -1)"
if [[ -z "${SERVER_BIN}" ]]; then
  echo "llama-server not found inside ${ASSET}" >&2
  exit 1
fi
BIN_DIR="$(dirname "${SERVER_BIN}")"
# an older build or layout leaves files behind
find "${OUT_DIR}" -mindepth 1 -delete
cp "${SERVER_BIN}" "${OUT_DIR}/llama-server"
# llama-server loads its ggml libraries from its own folder.
# cp -a keeps the shared library symlinks as links instead of copying each one
if [[ "${OS}" == "Darwin" ]]; then
  cp -a "${BIN_DIR}/"*.dylib "${OUT_DIR}/" 2>/dev/null || true
  cp -a "${BIN_DIR}/"*.metal* "${OUT_DIR}/" 2>/dev/null || true
else
  cp -a "${BIN_DIR}/"*.so* "${OUT_DIR}/"
fi
chmod +x "${OUT_DIR}/llama-server"
printf '%s\n' "${BACKEND} ${LLAMA_CPP_TAG}" > "${OUT_DIR}/.llama-backend"
echo "Installed ${OUT_DIR}/llama-server (${BACKEND}, ${LLAMA_CPP_TAG})"
