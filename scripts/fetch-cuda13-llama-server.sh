#!/usr/bin/env bash
# Provision the llama-server binary that powers Tiles inference.
#   - Linux: build from source with CUDA when nvcc is available, CPU otherwise.
#   - macOS: download the official prebuilt Metal release from llama.cpp.
# Override the pinned release with LLAMA_CPP_TAG=bXXXX.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${ROOT}/server/bin"
BUILD_DIR="${ROOT}/.cache/llama.cpp-build"
REPO_DIR="${ROOT}/.cache/llama.cpp"

LLAMA_CPP_TAG="${LLAMA_CPP_TAG:-b9867}"

mkdir -p "${OUT_DIR}"

# Reuse an already-provisioned binary. Set FORCE_LLAMA_FETCH=1 to rebuild or
# re-download (e.g. when bumping LLAMA_CPP_TAG).
if [[ -x "${OUT_DIR}/llama-server" && "${FORCE_LLAMA_FETCH:-}" != "1" ]]; then
  echo "llama-server already present at ${OUT_DIR}/llama-server; skipping fetch (set FORCE_LLAMA_FETCH=1 to force)."
  exit 0
fi

OS="$(uname -s)"

if [[ "${OS}" == "Darwin" ]]; then
  ARCH="$(uname -m)"
  case "${ARCH}" in
    arm64)  ASSET="llama-${LLAMA_CPP_TAG}-bin-macos-arm64.tar.gz" ;;
    x86_64) ASSET="llama-${LLAMA_CPP_TAG}-bin-macos-x64.tar.gz" ;;
    *) echo "Unsupported macOS architecture: ${ARCH}" >&2; exit 1 ;;
  esac

  URL="https://github.com/ggml-org/llama.cpp/releases/download/${LLAMA_CPP_TAG}/${ASSET}"
  TMP="$(mktemp -d)"
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
  # cp -a preserves the .dylib symlink chain (see fetch_llama_server.sh)
  cp -a "${BIN_DIR}/"*.dylib "${OUT_DIR}/" 2>/dev/null || true
  cp -a "${BIN_DIR}/"*.metal* "${OUT_DIR}/" 2>/dev/null || true
  chmod +x "${OUT_DIR}/llama-server"
  rm -rf "${TMP}"
  echo "Installed ${OUT_DIR}/llama-server (macOS Metal, ${LLAMA_CPP_TAG})"
  exit 0
fi

if [[ "${OS}" != "Linux" ]]; then
  echo "Unsupported OS: ${OS}" >&2
  exit 1
fi

# Linux: build from source. CUDA when the toolkit is present, CPU otherwise.
if [[ ! -d "${REPO_DIR}/.git" ]]; then
  git clone --depth 1 --branch "${LLAMA_CPP_TAG}" \
    https://github.com/ggml-org/llama.cpp "${REPO_DIR}"
else
  git -C "${REPO_DIR}" fetch --depth 1 origin tag "${LLAMA_CPP_TAG}" || true
  git -C "${REPO_DIR}" checkout "${LLAMA_CPP_TAG}" || true
fi

CMAKE_ARGS=(-B "${BUILD_DIR}")
if command -v nvcc >/dev/null 2>&1; then
  echo "nvcc detected -> building llama-server with CUDA"
  CMAKE_ARGS+=(-DGGML_CUDA=ON)
else
  echo "nvcc not found -> building CPU-only llama-server"
fi

cmake -S "${REPO_DIR}" "${CMAKE_ARGS[@]}"
cmake --build "${BUILD_DIR}" --target llama-server -j"$(nproc 2>/dev/null || echo 4)"

cp "${BUILD_DIR}/bin/llama-server" "${OUT_DIR}/llama-server"
# cp -a preserves the .so symlink chain (see fetch_llama_server.sh)
cp -a "${BUILD_DIR}/bin/"lib*.so* "${OUT_DIR}/" 2>/dev/null || true
chmod +x "${OUT_DIR}/llama-server"
echo "Installed ${OUT_DIR}/llama-server (Linux, ${LLAMA_CPP_TAG})"