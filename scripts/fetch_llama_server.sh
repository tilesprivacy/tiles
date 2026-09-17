#!/usr/bin/env bash
# provision the llama-server binary for tiles inference
#   macOS: prebuilt metal release from llama.cpp
#   linux: prebuilt llama.cpp release, one shared core plus a directory per
#          gpu backend (cuda_v12, cuda_v13, vulkan) with its runtime libs.
#          the launcher picks one at runtime via GGML_BACKEND_PATH
#
#   LLAMA_CPP_TAG          pin a llama.cpp release tag
#   TILES_LLAMA_VARIANTS   linux variants to fetch (default: cuda_v12 cuda_v13 vulkan)
#   TILES_LLAMA_BACKEND    macOS only: metal
#   FORCE_LLAMA_FETCH=1    re-download an existing binary
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${ROOT}/server/bin"
OS="$(uname -s)"
ARCH="$(uname -m)"

mkdir -p "${OUT_DIR}"

if [[ "${OS}" == "Darwin" ]]; then
  BACKEND="${TILES_LLAMA_BACKEND:-metal}"
  [[ "${BACKEND}" == "metal" ]] || { echo "Unsupported macOS llama backend: ${BACKEND}" >&2; exit 1; }

  if [[ -x "${OUT_DIR}/llama-server" && -f "${OUT_DIR}/.llama-backend" \
    && "$(<"${OUT_DIR}/.llama-backend")" == "${BACKEND}" \
    && "${FORCE_LLAMA_FETCH:-}" != "1" ]]; then
    echo "llama-server already present at ${OUT_DIR}/llama-server; skipping fetch (set FORCE_LLAMA_FETCH=1 to force)."
    exit 0
  fi

  LLAMA_CPP_TAG="${LLAMA_CPP_TAG:-b9867}"
  case "${ARCH}" in
    arm64)  ASSET="llama-${LLAMA_CPP_TAG}-bin-macos-arm64.tar.gz" ;;
    x86_64) ASSET="llama-${LLAMA_CPP_TAG}-bin-macos-x64.tar.gz" ;;
    *) echo "Unsupported macOS architecture: ${ARCH}" >&2; exit 1 ;;
  esac

  URL="https://github.com/ggml-org/llama.cpp/releases/download/${LLAMA_CPP_TAG}/${ASSET}"
  TMP="$(mktemp -d)"
  trap 'rm -rf "${TMP}"' EXIT
  echo "Downloading ${ASSET} (Metal prebuilt)"
  curl -fL -o "${TMP}/${ASSET}" "${URL}"
  tar -xzf "${TMP}/${ASSET}" -C "${TMP}"

  SERVER_BIN="$(find "${TMP}" -type f -name llama-server | head -1)"
  if [[ -z "${SERVER_BIN}" ]]; then
    echo "llama-server not found inside ${ASSET}" >&2
    exit 1
  fi
  BIN_DIR="$(dirname "${SERVER_BIN}")"
  cp "${SERVER_BIN}" "${OUT_DIR}/llama-server"
  # cp -a keeps the dylib symlink chain
  cp -a "${BIN_DIR}/"*.dylib "${OUT_DIR}/" 2>/dev/null || true
  cp -a "${BIN_DIR}/"*.metal* "${OUT_DIR}/" 2>/dev/null || true
  chmod +x "${OUT_DIR}/llama-server"
  printf '%s\n' "${BACKEND}" > "${OUT_DIR}/.llama-backend"
  echo "Installed ${OUT_DIR}/llama-server (macOS Metal, ${LLAMA_CPP_TAG})"
  exit 0
fi

if [[ "${OS}" != "Linux" ]]; then
  echo "Unsupported OS: ${OS}" >&2
  exit 1
fi

LLAMA_CPP_TAG="${LLAMA_CPP_TAG:-b11005}"
VARIANTS="${TILES_LLAMA_VARIANTS-cuda_v12 cuda_v13 vulkan}"
LAYOUT="linux ${LLAMA_CPP_TAG} [${VARIANTS}]"

if [[ -x "${OUT_DIR}/llama-server" && -f "${OUT_DIR}/.llama-backend" \
  && "$(<"${OUT_DIR}/.llama-backend")" == "${LAYOUT}" \
  && "${FORCE_LLAMA_FETCH:-}" != "1" ]]; then
  echo "llama-server already present at ${OUT_DIR}/llama-server; skipping fetch (set FORCE_LLAMA_FETCH=1 to force)."
  exit 0
fi

case "${ARCH}" in
  x86_64) ASSET_ARCH="x64" ;;
  aarch64|arm64) ASSET_ARCH="arm64" ;;
  *) echo "Unsupported Linux architecture: ${ARCH}" >&2; exit 1 ;;
esac

BASE_URL="https://github.com/ggml-org/llama.cpp/releases/download/${LLAMA_CPP_TAG}"

cuda_toolkit() {
  case "$1" in
    cuda_v12) echo "12.8" ;;
    cuda_v13) echo "13.3" ;;
    *) return 1 ;;
  esac
}

backend_lib() {
  case "$1" in
    cuda_v12|cuda_v13) echo "libggml-cuda.so" ;;
    vulkan) echo "libggml-vulkan.so" ;;
    *) echo "Unsupported Linux llama variant: $1" >&2; return 1 ;;
  esac
}

for v in ${VARIANTS}; do backend_lib "${v}" >/dev/null; done

TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

fetch() {
  local asset="$1" dest="$2"
  mkdir -p "${dest}"
  echo "Downloading ${asset}"
  curl -fL -o "${TMP}/${asset}" "${BASE_URL}/${asset}"
  tar -xzf "${TMP}/${asset}" -C "${dest}"
  rm -f "${TMP}/${asset}"
}

STAGE="${TMP}/stage"
mkdir -p "${STAGE}"
CORE_DIR=""

for variant in ${VARIANTS}; do
  if [[ "${variant}" == cuda_* ]]; then
    toolkit="$(cuda_toolkit "${variant}")"
    asset="llama-${LLAMA_CPP_TAG}-bin-ubuntu-cuda-${toolkit}-${ASSET_ARCH}.tar.gz"
    runtime="cudart-llama-${LLAMA_CPP_TAG}-bin-ubuntu-cuda-${toolkit}-${ASSET_ARCH}.tar.gz"
  else
    asset="llama-${LLAMA_CPP_TAG}-bin-ubuntu-${variant}-${ASSET_ARCH}.tar.gz"
    runtime=""
  fi

  if ! fetch "${asset}" "${TMP}/${variant}"; then
    # cuda 12.8 is x64 only upstream
    echo "Skipping ${variant}: ${asset} not available" >&2
    continue
  fi
  bin_dir="$(dirname "$(find "${TMP}/${variant}" -type f -name llama-server | head -1)")"
  lib="$(backend_lib "${variant}")"
  [[ -f "${bin_dir}/${lib}" ]] || { echo "${lib} not found inside ${asset}" >&2; exit 1; }

  mkdir -p "${STAGE}/${variant}"
  cp "${bin_dir}/${lib}" "${STAGE}/${variant}/"
  if [[ -n "${runtime}" ]]; then
    fetch "${runtime}" "${TMP}/${variant}-runtime"
    find "${TMP}/${variant}-runtime" -name 'lib*.so*' -exec cp -a {} "${STAGE}/${variant}/" \;
  fi
  # the non-backend files are identical across variants
  [[ -n "${CORE_DIR}" ]] || CORE_DIR="${bin_dir}"
done

if [[ -z "${CORE_DIR}" ]]; then
  asset="llama-${LLAMA_CPP_TAG}-bin-ubuntu-${ASSET_ARCH}.tar.gz"
  fetch "${asset}" "${TMP}/cpu"
  CORE_DIR="$(dirname "$(find "${TMP}/cpu" -type f -name llama-server | head -1)")"
fi

rm -rf "${OUT_DIR}/llama-server" "${OUT_DIR}/"*.so* "${OUT_DIR}/.llama-backend" \
  "${OUT_DIR}/cuda_v12" "${OUT_DIR}/cuda_v13" "${OUT_DIR}/vulkan"
cp "${CORE_DIR}/llama-server" "${OUT_DIR}/llama-server"
chmod +x "${OUT_DIR}/llama-server"
find "${CORE_DIR}" -maxdepth 1 -name 'lib*.so*' \
  ! -name 'libggml-cuda*' ! -name 'libggml-vulkan*' ! -name 'libggml-rpc*' \
  -exec cp -a {} "${OUT_DIR}/" \;
for variant in ${VARIANTS}; do
  [[ -d "${STAGE}/${variant}" ]] && cp -a "${STAGE}/${variant}" "${OUT_DIR}/${variant}"
done

printf '%s\n' "${LAYOUT}" > "${OUT_DIR}/.llama-backend"
echo "Installed ${OUT_DIR}/llama-server (Linux prebuilt, ${LLAMA_CPP_TAG})"
for variant in ${VARIANTS}; do
  [[ -d "${OUT_DIR}/${variant}" ]] && echo "  ${variant}: $(du -sh "${OUT_DIR}/${variant}" | cut -f1)"
done
echo "GPU variant is chosen at runtime; a host without a usable GPU runs on CPU."
