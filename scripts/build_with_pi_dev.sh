#!/usr/bin/env bash
set -euo pipefail

rm -rf .tiles_dev/tiles/pi

VERSION=$(grep '^tiles-pi' toolchain.toml | head -1 | awk -F'"' '{print $2}')

# TAR_URL="https://github.com/badlogic/pi-mono/releases/download/${VERSION}/pi-darwin-arm64.tar.gz"

TAR_URL="https://github.com/tilesprivacy/tiles-pi/releases/download/${VERSION}/pi-darwin-arm64.tar.gz"

curl -fL -o "pi-darwin-arm64.tar.gz" "$TAR_URL"

cp "pi-darwin-arm64.tar.gz" ".tiles_dev/tiles/"

cd .tiles_dev/tiles

tar -xvf pi-darwin-arm64.tar.gz

rm pi-darwin-arm64.tar.gz
