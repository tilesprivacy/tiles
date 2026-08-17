#!/usr/bin/env bash
# Builds the offline-model pkg: bundles the default gemma 4 12b GGUF (Q4_K_M)
# + MTP head so Tiles works without any network access.
#
# Since pkgbuild can't package very large files, the model dir is tarballed
# and split into 2 GB parts; pkg/scripts/gguf/postinstall rejoins + extracts
# them into /usr/local/share/tiles/models/huggingface/hub on install.
#
# Run when the bundled model changes, or a local copy is needed for the
# full/offline installer (`just bundle_pkg_full`).
set -euo pipefail

MODELS_VERSION=2.0

REPO="unsloth/gemma-4-12b-it-GGUF"
GGUF_FILE="${TILES_GEMMA_QUANT:-gemma-4-12b-it-Q4_K_M.gguf}"
MTP_FILE="${TILES_GEMMA_MTP:-mtp-gemma-4-12b-it.gguf}"

TARBALL="gemma-4-12b-it-GGUF.tar.gz"

STAGING="pkg/staging_gguf"
PKGROOT="pkgroot_gguf"
MODELS_HUB_DIR="${PKGROOT}/usr/local/share/tiles/models/huggingface/hub"
# HF hub cache dir name: org/repo -> models--org--repo
MODEL_DIR_NAME="models--$(echo "${REPO}" | tr '/' '-')"
STAGING_MODEL_DIR="${STAGING}/${MODEL_DIR_NAME}"

download() {
    local file="$1"
    local url="https://huggingface.co/${REPO}/resolve/main/${file}"
    if [[ -f "downloads/${file}" ]]; then
        echo "${file} already downloaded, skipping"
    else
        echo "Downloading ${url}"
        # Download to a .part temp and rename only on success, so an
        # interrupted download can never satisfy the cached-file check.
        # The .part file persists for -C - to resume on the next run.
        curl -fL --retry 3 -C - -o "downloads/${file}.part" "${url}"
        mv "downloads/${file}.part" "downloads/${file}"
    fi
}

mkdir -p downloads "${STAGING_MODEL_DIR}" "${MODELS_HUB_DIR}"

download "${GGUF_FILE}"
download "${MTP_FILE}"

cp "downloads/${GGUF_FILE}" "downloads/${MTP_FILE}" "${STAGING_MODEL_DIR}/"

echo "Creating ${TARBALL}..."
tar -czf "${STAGING}/${TARBALL}" -C "${STAGING}" "${MODEL_DIR_NAME}"

echo "Splitting into 2 GB parts..."
rm -f "${MODELS_HUB_DIR}/${TARBALL}.part."*
split -b 2048m "${STAGING}/${TARBALL}" "${MODELS_HUB_DIR}/${TARBALL}.part."

rm -rf "${STAGING}"

pkgbuild \
    --root "${PKGROOT}" \
    --scripts pkg/scripts/gguf \
    --identifier com.tilesprivacy.tiles_models_gguf \
    --version "${MODELS_VERSION}" \
    pkg/tiles-model_gguf.pkg

rm -rf "${PKGROOT}"
