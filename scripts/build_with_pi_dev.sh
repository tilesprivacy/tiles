#!/usr/bin/env bash
set -euo pipefail

PI_TAR_DIR="/Users/tiles/Downloads"
# cargo build

# Build the pi binary

# sh "${TILES_PI_DIR}/scripts/build-binaries.sh" --skip-deps --platform darwin-arm64

rm -rf .tiles_dev/tiles/pi

cp "${PI_TAR_DIR}/pi-darwin-arm64.tar.gz" ".tiles_dev/tiles/"

cd .tiles_dev/tiles

tar -xvf pi-darwin-arm64.tar.gz

