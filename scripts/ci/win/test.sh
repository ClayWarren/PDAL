#!/bin/bash
args=(-VV --output-on-failure --schedule-random --timeout 600)

if [[ "${BUILD_TYPE:-}" == "fixed" ]]; then
    # This remote /vsicurl COPC fixture is still covered by Windows current,
    # Linux, macOS, and Pixi. The Windows fixed dependency set has timed out
    # intermittently in GDAL/Curl remote reads, which blocks the compatibility
    # matrix without identifying a PDAL regression.
    args+=(-E '^pdal_io_copc_remote_reader_test$')
fi

ctest "${args[@]}"
