#!/usr/bin/env bash
set -euo pipefail

rm -rf .tiles_dev/tiles/pi

VERSION=$(grep '^pi' toolchain.toml | head -1 | awk -F'"' '{print $2}')



# Detect OS
case "$(uname -s)" in
    Darwin) OS="darwin" ;;
    Linux)  OS="linux" ;;
    *)      echo "Unsupported OS: $(uname -s)"; exit 1 ;;
esac

# Detect architecture
case "$(uname -m)" in
    x86_64)       ARCH="x64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *)            echo "Unsupported architecture: $(uname -m)"; exit 1 ;;
esac

PLATFORM="${OS}-${ARCH}"
TARBALL="pi-${PLATFORM}.tar.gz"

TAR_URL="https://github.com/badlogic/pi-mono/releases/download/${VERSION}/${TARBALL}"

echo "Downloading Pi ${VERSION} for ${PLATFORM}..."
curl -fL -o "$TARBALL" "$TAR_URL"

cp "$TARBALL" ".tiles_dev/tiles/"

cd .tiles_dev/tiles

tar -xvf "$TARBALL"

rm "$TARBALL"
