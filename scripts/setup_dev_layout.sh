#!/usr/bin/env bash
# Symlink repo assets into .tiles_dev/tiles for `cargo run` development.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEV_DIR="${ROOT}/.tiles_dev/tiles"

mkdir -p "${DEV_DIR}"

link_dir() {
  local name="$1"
  local src="${ROOT}/${name}"
  local dst="${DEV_DIR}/${name}"
  if [[ ! -d "${src}" ]]; then
    echo "Missing ${src}"
    exit 1
  fi
  rm -rf "${dst}"
  ln -sfn "${src}" "${dst}"
  echo "linked ${dst} -> ${src}"
}

link_dir modelfiles
link_dir server
link_dir vendor
link_dir plugins

if [[ ! -d "${DEV_DIR}/pi" ]]; then
  echo "Pi binary missing. Run: just build_w_pi"
fi

echo "Dev layout ready under ${DEV_DIR}"
