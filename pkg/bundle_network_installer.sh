#!/usr/bin/env bash

set -euo pipefail

VERSION=$(grep '^version' tiles/Cargo.toml | head -1 | awk -F'"' '{print $2}')
# the name the release file has
OUT="pkg/tiles-${VERSION}-signed.pkg"

productbuild \
  --distribution pkg/distribution_network.xml \
  --resources pkg/resources \
  --package-path pkg/  \
  pkg/tiles-dist-unsigned.pkg


# signing
productsign \
  --sign "$DEVELOPER_ID_INSTALLER" \
  pkg/tiles-dist-unsigned.pkg \
  "${OUT}"

# notarizing
xcrun notarytool submit "${OUT}" \
  --keychain-profile "tiles-notary-profile" \
  --wait

# staple the approval ticket to pkg
xcrun stapler staple "${OUT}"

rm pkg/tiles-unsigned.pkg
rm pkg/tiles-dist-unsigned.pkg

echo "✅ Installer created: ${OUT}"
