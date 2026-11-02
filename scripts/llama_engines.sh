#!/usr/bin/env bash
# Writes the llama.cpp builds Tiles downloads on Linux to crates/tiles-engine/src/engines.json,
# for the llama.cpp release pinned in toolchain.toml. Run it after changing that pin.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${ROOT}/crates/tiles-engine/src/engines.json"
TAG=$(grep '^llama.cpp' "${ROOT}/toolchain.toml" | head -1 | awk -F'"' '{print $2}')
API_JSON="$(curl -fsSL "https://api.github.com/repos/ggml-org/llama.cpp/releases/tags/${TAG}")"

# a gzip file ends with its unpacked size, so 4 bytes are enough to learn it
unpacked() {
  local url="$1" size="$2"
  curl -fsSL -r "$((size - 4))-$((size - 1))" "${url}" | od -An -tu4 | tr -d ' '
}

file_json() {
  local asset
  asset="$(jq -c --arg name "$1" '.assets[] | select(.name == $name)' <<< "${API_JSON}")"
  [[ -n "${asset}" ]] || return 1
  local url size
  url="$(jq -r .browser_download_url <<< "${asset}")"
  size="$(jq -r .size <<< "${asset}")"
  jq -c --arg unpacked "$(unpacked "${url}" "${size}")" \
    '{name, size, unpacked: ($unpacked | tonumber), sha256: (.digest | ltrimstr("sha256:"))}' <<< "${asset}"
}

entries=()
for arch in x64 arm64; do
  rust_arch="x86_64"
  [[ "${arch}" == "arm64" ]] && rust_arch="aarch64"

  # the newest toolkit of each CUDA major in this release
  for major in 12 13; do
    version="$(jq -r --arg arch "${arch}" --arg major "${major}" '
      [.assets[].name
        | capture("^llama-.*-bin-ubuntu-cuda-(?<v>[0-9]+\\.[0-9]+)-" + $arch + "\\.tar\\.gz$")?.v
        | select(startswith($major + "."))]
      | sort_by(split(".") | map(tonumber)) | last // empty' <<< "${API_JSON}")"
    [[ -n "${version}" ]] || continue
    build="$(file_json "llama-${TAG}-bin-ubuntu-cuda-${version}-${arch}.tar.gz")"
    runtime="$(file_json "cudart-llama-${TAG}-bin-ubuntu-cuda-${version}-${arch}.tar.gz")"
    entries+=("$(jq -nc --arg engine "cuda-${major}" --arg arch "${rust_arch}" --arg version "${version}" \
      --argjson build "${build}" --argjson runtime "${runtime}" \
      '{engine: $engine, arch: $arch, version: $version, files: [$build, $runtime]}')")
  done

  for engine in vulkan cpu; do
    name="llama-${TAG}-bin-ubuntu-${arch}.tar.gz"
    [[ "${engine}" == "vulkan" ]] && name="llama-${TAG}-bin-ubuntu-vulkan-${arch}.tar.gz"
    build="$(file_json "${name}")" || continue
    entries+=("$(jq -nc --arg engine "${engine}" --arg arch "${rust_arch}" --argjson build "${build}" \
      '{engine: $engine, arch: $arch, files: [$build]}')")
  done
done

printf '%s\n' "${entries[@]}" | jq -s --arg tag "${TAG}" '{tag: $tag, engines: .}' > "${OUT}"
echo "Wrote ${#entries[@]} builds for ${TAG} to ${OUT}"
