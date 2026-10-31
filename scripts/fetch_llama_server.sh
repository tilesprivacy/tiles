#!/usr/bin/env bash
# Provision the llama-server binary that powers Tiles inference, from the official
# llama.cpp release. Every platform uses the same release, so a flag that works on one
# build works on all of them.
#   - Linux: the CUDA or Vulkan build. CUDA comes with its runtime libraries, so the
#     host only needs the NVIDIA driver.
#   - macOS: the Metal build.
#
# Environment variables:
#   TILES_LLAMA_BACKEND  Linux backend: cuda (default) or vulkan.
#   LLAMA_CPP_TAG        The llama.cpp build (default: b11960, the build of v0.8.0).
#   TILES_CUDA_VERSION   CUDA build: 12.8 (default, x64 only) or 13.4.
#   FORCE_LLAMA_FETCH    Set to 1 to re-download even if a binary is present.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${ROOT}/server/bin"
OS="$(uname -s)"
LLAMA_CPP_TAG="${LLAMA_CPP_TAG:-b11960}"
CUDA_VERSION="${TILES_CUDA_VERSION:-12.8}"

if [[ "${OS}" == "Darwin" ]]; then
  BACKEND="${TILES_LLAMA_BACKEND:-metal}"
  [[ "${BACKEND}" == "metal" ]] || { echo "Unsupported macOS llama backend: ${BACKEND}" >&2; exit 1; }
elif [[ "${OS}" == "Linux" ]]; then
  BACKEND="${TILES_LLAMA_BACKEND:-cuda}"
  [[ "${BACKEND}" == "cuda" || "${BACKEND}" == "vulkan" ]] || { echo "Unsupported Linux llama backend: ${BACKEND}" >&2; exit 1; }
  [[ "${BACKEND}" == "vulkan" ]] || BACKEND="cuda ${CUDA_VERSION}"
else
  echo "Unsupported OS: ${OS}" >&2
  exit 1
fi

mkdir -p "${OUT_DIR}"
# the downloads are large, keep them on disk rather than in a RAM-backed /tmp
mkdir -p "${ROOT}/.cache"
TMP="$(mktemp -d "${ROOT}/.cache/llama.XXXXXX")"
trap 'rm -rf "${TMP}"' EXIT

# Reuse an already-provisioned binary. Set FORCE_LLAMA_FETCH=1 to re-download.
if [[ -x "${OUT_DIR}/llama-server" && -f "${OUT_DIR}/.llama-backend" \
  && "$(<"${OUT_DIR}/.llama-backend")" == "${BACKEND} ${LLAMA_CPP_TAG}" \
  && "${FORCE_LLAMA_FETCH:-}" != "1" ]]; then
  echo "llama-server already present at ${OUT_DIR}/llama-server; skipping fetch (set FORCE_LLAMA_FETCH=1 to force)."
  exit 0
fi

# ---------------------------------------------------------------------------
# macOS: official prebuilt Metal release from ggml-org/llama.cpp.
# --------------------------------------------------------------------------
if [[ "${OS}" == "Darwin" ]]; then
  ARCH="$(uname -m)"
  case "${ARCH}" in
    arm64)  ASSET="llama-${LLAMA_CPP_TAG}-bin-macos-arm64.tar.gz" ;;
    x86_64) ASSET="llama-${LLAMA_CPP_TAG}-bin-macos-x64.tar.gz" ;;
    *) echo "Unsupported macOS architecture: ${ARCH}" >&2; exit 1 ;;
  esac

  URL="https://github.com/ggml-org/llama.cpp/releases/download/${LLAMA_CPP_TAG}/${ASSET}"
  echo "Downloading ${ASSET} (Metal prebuilt)"
  curl -fL -o "${TMP}/${ASSET}" "${URL}"
  tar -xzf "${TMP}/${ASSET}" -C "${TMP}"

  SERVER_BIN="$(find "${TMP}" -type f -name llama-server | head -1)"
  if [[ -z "${SERVER_BIN}" ]]; then
    echo "llama-server not found inside ${ASSET}" >&2
    exit 1
  fi
  BIN_DIR="$(dirname "${SERVER_BIN}")"
  find "${OUT_DIR}" -mindepth 1 -delete
  cp "${SERVER_BIN}" "${OUT_DIR}/llama-server"
  # cp -a preserves the .dylib -> .dylib.N -> .dylib.N.M.K symlink chain;
  # a plain cp dereferences it into 3 real copies of every library.
  cp -a "${BIN_DIR}/"*.dylib "${OUT_DIR}/" 2>/dev/null || true
  cp -a "${BIN_DIR}/"*.metal* "${OUT_DIR}/" 2>/dev/null || true
  chmod +x "${OUT_DIR}/llama-server"
  printf '%s\n' "${BACKEND} ${LLAMA_CPP_TAG}" > "${OUT_DIR}/.llama-backend"
  echo "Installed ${OUT_DIR}/llama-server (macOS Metal, ${LLAMA_CPP_TAG})"
  exit 0
fi

# ---------------------------------------------------------------------------
# Linux: prebuilt CUDA or Vulkan binary.
# --------------------------------------------------------------------------
case "$(uname -m)" in
  x86_64) ARCH="x64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  *) echo "Unsupported Linux architecture: $(uname -m)" >&2; exit 1 ;;
esac

BASE_URL="https://github.com/ggml-org/llama.cpp/releases/download/${LLAMA_CPP_TAG}"

fetch() {
  echo "Downloading $1"
  mkdir -p "$2"
  curl -fsSL --retry 3 "${BASE_URL}/$1" | tar -xz -C "$2"
}

if [[ "${BACKEND}" == "vulkan" ]]; then
  fetch "llama-${LLAMA_CPP_TAG}-bin-ubuntu-vulkan-${ARCH}.tar.gz" "${TMP}/build"
else
  fetch "llama-${LLAMA_CPP_TAG}-bin-ubuntu-cuda-${CUDA_VERSION}-${ARCH}.tar.gz" "${TMP}/build"
  # libcudart and libcublas, otherwise llama.cpp quietly runs on the CPU
  fetch "cudart-llama-${LLAMA_CPP_TAG}-bin-ubuntu-cuda-${CUDA_VERSION}-${ARCH}.tar.gz" "${TMP}/runtime"
fi

SERVER_BIN="$(find "${TMP}/build" -type f -name llama-server | head -1)"
if [[ -z "${SERVER_BIN}" ]]; then
  echo "llama-server not found in the ${BACKEND} build" >&2
  exit 1
fi
BIN_DIR="$(dirname "${SERVER_BIN}")"
# a different backend or an older layout leaves files behind
find "${OUT_DIR}" -mindepth 1 -delete
cp "${SERVER_BIN}" "${OUT_DIR}/llama-server"
# llama-server loads libggml-cuda or libggml-vulkan from its own folder.
# cp -a preserves the shared-library symlink chain.
cp -a "${BIN_DIR}/"*.so* "${OUT_DIR}/"
[[ -d "${TMP}/runtime" ]] && find "${TMP}/runtime" -name 'lib*.so*' -exec cp -a {} "${OUT_DIR}/" \;
chmod +x "${OUT_DIR}/llama-server"
printf '%s\n' "${BACKEND} ${LLAMA_CPP_TAG}" > "${OUT_DIR}/.llama-backend"
echo "Installed ${OUT_DIR}/llama-server (Linux ${BACKEND}, ${LLAMA_CPP_TAG})"
