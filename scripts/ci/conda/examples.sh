#!/bin/bash
set -euo pipefail

export BASE="$(pwd)"
export PDAL_DRIVER_PATH="${PDAL_DRIVER_PATH:-}"
export _CONDA_SET_PDAL_DRIVER_PATH="${_CONDA_SET_PDAL_DRIVER_PATH:-}"

conda activate test
conda install cmake ninja compilers -y

./rust/scripts/check_installed_capi_consumer.sh --prefix "$CONDA_PREFIX"

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
