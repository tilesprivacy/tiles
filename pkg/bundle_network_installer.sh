#!/usr/bin/env bash

set -euo pipefail

VERSION=$(grep '^version' tiles/Cargo.toml | head -1 | awk -F'"' '{print $2}')

productbuild \
  --distribution pkg/distribution_network.xml \
  --resources pkg/resources \
  --package-path pkg/  \
  pkg/tiles-dist-unsigned.pkg


# signing
productsign \
  --sign "$DEVELOPER_ID_INSTALLER" \
  --entitlements entitleme.plist \
  pkg/tiles-dist-unsigned.pkg \
  pkg/tiles.pkg

# notarizing
xcrun notarytool submit pkg/tiles.pkg \
  --keychain-profile "tiles-notary-profile" \
  --wait

# staple the approval ticket to pkg
xcrun stapler staple pkg/tiles.pkg

rm pkg/tiles-unsigned.pkg
rm pkg/tiles-dist-unsigned.pkg
