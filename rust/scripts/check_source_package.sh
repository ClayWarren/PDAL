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
    "rust/pdal-capi/include/pdal_capi.h"
    "rust/pdal-core/Cargo.toml"
    "rust/pdal-filters/Cargo.toml"
    "rust/pdal-io/Cargo.toml"
    "rust/pdal-native/Cargo.toml"
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
    )
    for path in "${forbidden[@]}"; do
        if grep -Fq "${path}" <<<"${listing}"; then
            echo "${archive} contains forbidden path fragment ${path}" >&2
            exit 1
        fi
    done
done
