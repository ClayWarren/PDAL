#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUILD_DIR="${1:-${ROOT_DIR}/.build}"

INSTALL_PREFIX=""
if [[ "${BUILD_DIR}" == "--prefix" ]]; then
    if [[ $# -ne 2 ]]; then
        echo "Usage: $0 [build-dir] | --prefix install-prefix" >&2
        exit 1
    fi
    INSTALL_PREFIX="$2"
elif [[ ! -d "${BUILD_DIR}" ]]; then
    echo "Build directory does not exist: ${BUILD_DIR}" >&2
    exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/pdal-capi-install.XXXXXX")"
trap 'rm -rf "${TMP_DIR}"' EXIT

if [[ -z "${INSTALL_PREFIX}" ]]; then
    INSTALL_PREFIX="${TMP_DIR}/install"
    cmake --install "${BUILD_DIR}" --prefix "${INSTALL_PREFIX}" >/dev/null
fi

CONSUMER_DIR="${TMP_DIR}/consumer"

mkdir -p "${CONSUMER_DIR}"
cat >"${CONSUMER_DIR}/CMakeLists.txt" <<'CMAKE'
cmake_minimum_required(VERSION 3.13)
project(pdal_capi_consumer CXX)

find_package(PDAL CONFIG REQUIRED)

add_executable(consumer main.cpp)
target_link_libraries(consumer PRIVATE PDAL::CAPI)
CMAKE

cat >"${CONSUMER_DIR}/main.cpp" <<'CPP'
#include <pdal_capi.h>

int main()
{
    const char* version = pdal_version_string();
    pdal_capi_free(nullptr);
    return version ? 0 : 1;
}
CPP

cmake -S "${CONSUMER_DIR}" -B "${CONSUMER_DIR}/build" -G Ninja \
    -DCMAKE_PREFIX_PATH="${INSTALL_PREFIX}" >/dev/null
cmake --build "${CONSUMER_DIR}/build" >/dev/null

CONSUMER_EXE="${CONSUMER_DIR}/build/consumer"
if [[ -f "${CONSUMER_EXE}.exe" ]]; then
    CONSUMER_EXE="${CONSUMER_EXE}.exe"
fi

PATH="${INSTALL_PREFIX}/bin:${INSTALL_PREFIX}/lib:${PATH}" "${CONSUMER_EXE}"
