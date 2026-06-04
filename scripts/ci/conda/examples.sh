#!/bin/bash
set -euo pipefail

export BASE="$(pwd)"
export PDAL_DRIVER_PATH="${PDAL_DRIVER_PATH:-}"
export _CONDA_SET_PDAL_DRIVER_PATH="${_CONDA_SET_PDAL_DRIVER_PATH:-}"

set +u
conda activate test
conda install cmake ninja compilers -y
set -u

./rust/scripts/check_installed_capi_consumer.sh --prefix "$CONDA_PREFIX"

PDAL_CMAKE_DIR="$CONDA_PREFIX/lib/cmake/PDAL"
if [[ ! -d "$PDAL_CMAKE_DIR" &&
    -d "$CONDA_PREFIX/Library/lib/cmake/PDAL" ]]; then
    PDAL_CMAKE_DIR="$CONDA_PREFIX/Library/lib/cmake/PDAL"
fi

if ! grep -R -q "PDAL::PDAL" "$PDAL_CMAKE_DIR" 2>/dev/null; then
    echo "PDAL::PDAL target not exported by installed package at $CONDA_PREFIX; skipping example/plugin CMake smoke."
    return 0 2>/dev/null || exit 0
fi

if [ "${PDAL_PLATFORM:-}" == "windows-latest" ]; then

export CC=cl.exe
export CXX=cl.exe
where cl
fi

for EXAMPLE in writing writing-filter writing-kernel \
    writing-reader writing-writer
do
    cd "$BASE/examples/$EXAMPLE"
    mkdir -p _build || exit 1
    cd _build || exit 1
    cmake -G "Ninja" .. -DPDAL_DIR="$CONDA_PREFIX/lib/cmake/PDAL" && ninja
done
