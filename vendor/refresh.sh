#!/usr/bin/env bash
# Rebuilds vendor/node_modules from the lockfile.
# Run after changing pi-mcp-adapter version in package.json:
#   npm install --package-lock-only --legacy-peer-deps && ./refresh.sh
set -euo pipefail

npm ci --legacy-peer-deps

# prune: platform recheck backends (pure JS fallback is kept), readme banner
rm -rf node_modules/recheck-jar node_modules/recheck-*/ node_modules/pi-mcp-adapter/banner.png

# linux release target needs its keyring binary; npm only installs the local platform
mkdir -p node_modules/@napi-rs/keyring-linux-x64-gnu
curl -sL https://registry.npmjs.org/@napi-rs/keyring-linux-x64-gnu/-/keyring-linux-x64-gnu-1.3.0.tgz \
  | tar -xz --strip-components=1 -C node_modules/@napi-rs/keyring-linux-x64-gnu

du -sh node_modules
