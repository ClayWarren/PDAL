#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUILD_DIR="${1:-${ROOT_DIR}/.build}"

INSTALL_PREFIX=""
PREFIX_MODE=0
if [[ "${BUILD_DIR}" == "--prefix" ]]; then
    if [[ $# -ne 2 ]]; then
        echo "Usage: $0 [build-dir] | --prefix install-prefix" >&2
        exit 1
    fi
    INSTALL_PREFIX="$2"
    PREFIX_MODE=1
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

PDAL_CMAKE_DIR="${INSTALL_PREFIX}/lib/cmake/PDAL"
if [[ "${PREFIX_MODE}" -eq 1 ]] &&
    ! grep -R -q "PDAL::CAPI" "${PDAL_CMAKE_DIR}" 2>/dev/null; then
    echo "PDAL::CAPI target not exported by installed package at ${INSTALL_PREFIX}; skipping C API consumer smoke."
    exit 0
fi

CONSUMER_DIR="${TMP_DIR}/consumer"

mkdir -p "${CONSUMER_DIR}"
cat >"${CONSUMER_DIR}/CMakeLists.txt" <<'CMAKE'
cmake_minimum_required(VERSION 3.13)
project(pdal_capi_consumer CXX)

find_package(PDAL CONFIG REQUIRED)

add_executable(consumer main.cpp)
target_compile_definitions(consumer PRIVATE
    STAC_ITEM_PATH="${CMAKE_CURRENT_SOURCE_DIR}/stac-item.json")
target_link_libraries(consumer PRIVATE PDAL::CAPI)
CMAKE

cat >"${CONSUMER_DIR}/stac-item.json" <<'JSON'
{
  "type": "Feature",
  "stac_version": "1.0.0",
  "id": "install-smoke",
  "geometry": null,
  "properties": {
    "datetime": "2026-01-01T00:00:00Z"
  },
  "links": [],
  "assets": {}
}
JSON

cat >"${CONSUMER_DIR}/main.cpp" <<'CPP'
#include <pdal_capi.h>

int main()
{
    const char* version = pdal_version_string();
    if (PDAL_CAPI_ABI_VERSION != pdal_capi_abi_version())
        return 2;
    if (PDAL_CAPI_ABI_VERSION_MAJOR != pdal_capi_abi_version_major())
        return 3;
    if (PDAL_CAPI_ABI_VERSION_MINOR != pdal_capi_abi_version_minor())
        return 4;
    if (PDAL_CAPI_ABI_VERSION_PATCH != pdal_capi_abi_version_patch())
        return 5;
    if (!pdal_stac_type_supported(STAC_ITEM_PATH))
        return 6;
    if (pdal_stac_type_supported("/definitely/missing/stac.json"))
        return 7;

    pdal_pipeline_t* pipeline = pdal_pipeline_create_json(R"([
        {
            "type":"readers.faux",
            "count":3,
            "mode":"ramp",
            "minx":-10,
            "maxx":20,
            "miny":-15,
            "maxy":7,
            "minz":-50,
            "maxz":100
        }
    ])");
    if (!pipeline)
        return 8;
    pdal_pipeline_result_t result{};
    if (pdal_pipeline_execute_result(pipeline, nullptr, &result) != 0)
        return 9;
    if (result.point_count != 3 || result.view_count != 1 ||
        !result.has_bounds_2d || !result.has_bounds_3d)
        return 10;
    if (result.bounds_3d.minx != -10.0 || result.bounds_3d.maxx != 20.0 ||
        result.bounds_3d.miny != -15.0 || result.bounds_3d.maxy != 7.0 ||
        result.bounds_3d.minz != -50.0 || result.bounds_3d.maxz != 100.0)
        return 11;
    pdal_pipeline_destroy(pipeline);

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

case "$(uname -s)" in
    Darwin)
        DYLD_LIBRARY_PATH="${INSTALL_PREFIX}/lib:${DYLD_LIBRARY_PATH:-}" \
            PATH="${INSTALL_PREFIX}/bin:${PATH}" \
            "${CONSUMER_EXE}"
        ;;
    Linux)
        LD_LIBRARY_PATH="${INSTALL_PREFIX}/lib:${LD_LIBRARY_PATH:-}" \
            PATH="${INSTALL_PREFIX}/bin:${PATH}" \
            "${CONSUMER_EXE}"
        ;;
    *)
        PATH="${INSTALL_PREFIX}/bin:${INSTALL_PREFIX}/lib:${PATH}" \
            "${CONSUMER_EXE}"
        ;;
esac
