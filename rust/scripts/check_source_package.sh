#!/usr/bin/env bash
set -euo pipefail

BUILD_DIR="${1:-.build}"

shopt -s nullglob
archives=("${BUILD_DIR}"/PDAL-*-src.tar.gz "${BUILD_DIR}"/PDAL-*-src.tar.bz2)
if [[ ${#archives[@]} -eq 0 ]]; then
    echo "No PDAL source archives found in ${BUILD_DIR}" >&2
    exit 1
fi

required=(
    "rust/Cargo.toml"
    "rust/Cargo.lock"
    "rust/DECISIONS.md"
    "rust/PARITY.md"
    "rust/PORTING.md"
    "rust/STATUS.md"
    "rust/VENDOR.md"
    "rust/pdal-capi/include/pdal_capi.h"
    "rust/pdal-core/Cargo.toml"
    "rust/pdal-filters/Cargo.toml"
    "rust/pdal-io/Cargo.toml"
    "rust/pdal-kernels/Cargo.toml"
    "rust/pdal-cli/Cargo.toml"
    "rust/pdal-native/Cargo.toml"
    "rust/pdal-plugins/Cargo.toml"
    "rust/scripts/audit_capi_header.py"
    "rust/scripts/check_installed_capi_consumer.sh"
    "rust/scripts/check_source_package.sh"
)

for archive in "${archives[@]}"; do
    listing="$(tar -tf "${archive}")"

    for path in "${required[@]}"; do
        if ! grep -Eq "^[^/]+-src/${path}$" <<<"${listing}"; then
            echo "${archive} is missing ${path}" >&2
            exit 1
        fi
    done

    forbidden=(
        "/rust/target/"
        "/.build/"
        "/.pixi/"
        "/.mull-build/"
        "/__pycache__/"
        ".pyc"
    )
    for path in "${forbidden[@]}"; do
        if grep -Fq "${path}" <<<"${listing}"; then
            echo "${archive} contains forbidden path fragment ${path}" >&2
            exit 1
        fi
    done
done
