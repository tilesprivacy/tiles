#!/usr/bin/env bash
# Provision the llama-server binary that powers Tiles inference.
#   - Linux: download a prebuilt CUDA or Vulkan binary.
#     CUDA defaults to the latest ai-dock release; Vulkan defaults to b9867.
#   - macOS: download the official prebuilt Metal release from llama.cpp.
#
# Linux (CUDA): the downloaded binary links against CUDA 12.8 runtime
# libraries, which must be present on the host for llama-server to start.
#
# Environment variables:
#   TILES_LLAMA_BACKEND  Linux backend: cuda (default) or vulkan.
#   LLAMA_CPP_TAG        Pin a specific release tag (e.g. b10276).
#   TILES_CUDA_VERSION   CUDA version suffix for the asset (default: 12.8).
#                        Only used when LLAMA_CPP_TAG is pinned; the auto-
#                        latest path reads the version from the asset name.
#   FORCE_LLAMA_FETCH    Set to 1 to re-download even if a binary is present.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${ROOT}/server/bin"
OS="$(uname -s)"

if [[ "${OS}" == "Darwin" ]]; then
  BACKEND="${TILES_LLAMA_BACKEND:-metal}"
  [[ "${BACKEND}" == "metal" ]] || { echo "Unsupported macOS llama backend: ${BACKEND}" >&2; exit 1; }
elif [[ "${OS}" == "Linux" ]]; then
  BACKEND="${TILES_LLAMA_BACKEND:-cuda}"
  [[ "${BACKEND}" == "cuda" || "${BACKEND}" == "vulkan" ]] || { echo "Unsupported Linux llama backend: ${BACKEND}" >&2; exit 1; }
else
  echo "Unsupported OS: ${OS}" >&2
  exit 1
fi

mkdir -p "${OUT_DIR}"

# Reuse an already-provisioned binary. Set FORCE_LLAMA_FETCH=1 to re-download.
if [[ -x "${OUT_DIR}/llama-server" && -f "${OUT_DIR}/.llama-backend" \
  && "$(<"${OUT_DIR}/.llama-backend")" == "${BACKEND}" \
  && "${FORCE_LLAMA_FETCH:-}" != "1" ]]; then
  echo "llama-server already present at ${OUT_DIR}/llama-server; skipping fetch (set FORCE_LLAMA_FETCH=1 to force)."
  exit 0
fi

# ---------------------------------------------------------------------------
# macOS: official prebuilt Metal release from ggml-org/llama.cpp.
# --------------------------------------------------------------------------
if [[ "${OS}" == "Darwin" ]]; then
  LLAMA_CPP_TAG="${LLAMA_CPP_TAG:-b9867}"
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
  # cp -a preserves the .dylib -> .dylib.N -> .dylib.N.M.K symlink chain;
  # a plain cp dereferences it into 3 real copies of every library.
  cp -a "${BIN_DIR}/"*.dylib "${OUT_DIR}/" 2>/dev/null || true
  cp -a "${BIN_DIR}/"*.metal* "${OUT_DIR}/" 2>/dev/null || true
  chmod +x "${OUT_DIR}/llama-server"
  printf '%s\n' "${BACKEND}" > "${OUT_DIR}/.llama-backend"
  rm -rf "${TMP}"
  echo "Installed ${OUT_DIR}/llama-server (macOS Metal, ${LLAMA_CPP_TAG})"
  exit 0
fi

# ---------------------------------------------------------------------------
# Linux: prebuilt CUDA or Vulkan binary.
# --------------------------------------------------------------------------
ARCH="$(uname -m)"
case "${ARCH}" in
  x86_64)  ASSET_ARCH="amd64"; OFFICIAL_ARCH="x64" ;;
  aarch64|arm64) ASSET_ARCH="arm64"; OFFICIAL_ARCH="arm64" ;;
  *) echo "Unsupported Linux architecture: ${ARCH}" >&2; exit 1 ;;
esac

API_URL="https://api.github.com/repos/ai-dock/llama.cpp-cuda/releases/latest"

if [[ "${BACKEND}" == "vulkan" ]]; then
  TAG="${LLAMA_CPP_TAG:-b9867}"
  ASSET="llama-${TAG}-bin-ubuntu-vulkan-${OFFICIAL_ARCH}.tar.gz"
  URL="https://github.com/ggml-org/llama.cpp/releases/download/${TAG}/${ASSET}"
elif [[ -n "${LLAMA_CPP_TAG:-}" ]]; then
  # Pinned tag: construct the URL directly. CUDA version comes from the
  # override (default 12.8), since the API isn't queried.
  CUDA_VERSION="${TILES_CUDA_VERSION:-12.8}"
  TAG="${LLAMA_CPP_TAG}"
  ASSET="llama.cpp-${TAG}-cuda-${CUDA_VERSION}-${ASSET_ARCH}.tar.gz"
  URL="https://github.com/ai-dock/llama.cpp-cuda/releases/download/${TAG}/${ASSET}"
else
  # Auto-latest: query the GitHub API. jq picks the asset matching our arch.
  echo "Querying ai-dock/llama.cpp-cuda latest release..."
  API_JSON="$(curl -fsSL "${API_URL}")"
  TAG="$(printf '%s' "${API_JSON}" | jq -r '.tag_name')"
  if [[ -z "${TAG}" || "${TAG}" == "null" ]]; then
    echo "Failed to determine latest ai-dock release tag" >&2
    exit 1
  fi
  URL="$(printf '%s' "${API_JSON}" | jq -r --arg arch "${ASSET_ARCH}" \
    '.assets[] | select(.name | endswith("-" + $arch + ".tar.gz")) | .browser_download_url')"
  if [[ -z "${URL}" || "${URL}" == "null" ]]; then
    echo "No asset matching arch ${ASSET_ARCH} in ai-dock release ${TAG}" >&2
    exit 1
  fi
  ASSET="$(basename "${URL}")"
  # Asset name is llama.cpp-<tag>-cuda-<version>-<arch>.tar.gz; extract the
  # CUDA version so the runtime note below reports the actual selected value.
  if [[ "${ASSET}" =~ -cuda-([0-9.]+)- ]]; then
    CUDA_VERSION="${BASH_REMATCH[1]}"
  else
    CUDA_VERSION="unknown"
  fi
fi

TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT
echo "Downloading ${ASSET} (prebuilt ${BACKEND}, ${TAG})"
curl -fL -o "${TMP}/${ASSET}" "${URL}"
tar -xzf "${TMP}/${ASSET}" -C "${TMP}"

SERVER_BIN="$(find "${TMP}" -type f -name llama-server | head -1)"
if [[ -z "${SERVER_BIN}" ]]; then
  echo "llama-server not found inside ${ASSET}" >&2
  exit 1
fi
BIN_DIR="$(dirname "${SERVER_BIN}")"
rm -f "${OUT_DIR}/llama-server" "${OUT_DIR}/"*.so* "${OUT_DIR}/.llama-backend"
cp "${SERVER_BIN}" "${OUT_DIR}/llama-server"
# Bring along bundled shared libs (libggml*.so etc.) if shipped next to it.
# cp -a preserves the shared-library symlink chain.
cp -a "${BIN_DIR}/"*.so* "${OUT_DIR}/" 2>/dev/null || true
chmod +x "${OUT_DIR}/llama-server"
printf '%s\n' "${BACKEND}" > "${OUT_DIR}/.llama-backend"
echo "Installed ${OUT_DIR}/llama-server (Linux ${BACKEND} prebuilt, ${TAG})"
if [[ "${BACKEND}" == "cuda" ]]; then
  echo "Note: requires CUDA ${CUDA_VERSION} runtime libraries on the host to run."
fi
