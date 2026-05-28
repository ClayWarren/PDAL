#!/bin/bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUILD_DIR="${1:-${ROOT_DIR}/build}"
BUILD_DIR="$(cd "${BUILD_DIR}" && pwd)"
COMPILE_COMMANDS="${BUILD_DIR}/compile_commands.json"

if [ ! -f "${COMPILE_COMMANDS}" ]; then
    echo "compile_commands.json not found in ${BUILD_DIR}" >&2
    echo "Configure with -DCMAKE_EXPORT_COMPILE_COMMANDS=ON first." >&2
    exit 1
fi

CLANG_TIDY="${CLANG_TIDY:-clang-tidy}"
RUN_CLANG_TIDY="${RUN_CLANG_TIDY:-run-clang-tidy}"

if ! command -v "${CLANG_TIDY}" >/dev/null 2>&1; then
    echo "clang-tidy not found: ${CLANG_TIDY}" >&2
    exit 1
fi

if ! command -v "${RUN_CLANG_TIDY}" >/dev/null 2>&1; then
    echo "run-clang-tidy not found: ${RUN_CLANG_TIDY}" >&2
    exit 1
fi

DIMENSION_HEADER="${BUILD_DIR}/include/pdal/Dimension.hpp"
if [ ! -f "${DIMENSION_HEADER}" ]; then
    cmake --build "${BUILD_DIR}" --target generate_dimension_hpp
fi

FILE_LIST="$(mktemp "${TMPDIR:-/tmp}/pdal-clang-tidy-files.XXXXXX")"
trap 'rm -f "${FILE_LIST}"' EXIT

python3 - "${ROOT_DIR}" "${COMPILE_COMMANDS}" > "${FILE_LIST}" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
compile_commands = Path(sys.argv[2])

excluded = (
    "vendor/",
    "plugins/e57/libE57Format/",
    "filters/private/csf/",
    "filters/private/miniball/",
    "plugins/nitf/io/nitflib.h",
)

seen = set()
for entry in json.loads(compile_commands.read_text()):
    path = Path(entry["file"]).resolve()
    try:
        rel = path.relative_to(root).as_posix()
    except ValueError:
        continue

    if any(rel.startswith(item) if item.endswith("/") else rel == item
           for item in excluded):
        continue

    if rel in seen:
        continue

    seen.add(rel)
    sys.stdout.write(rel + "\0")
PY

if [ ! -s "${FILE_LIST}" ]; then
    echo "No first-party source files found in ${COMPILE_COMMANDS}" >&2
    exit 1
fi

EXTRA_ARGS=()
if [ "$(uname -s)" = "Darwin" ] && command -v xcrun >/dev/null 2>&1; then
    SDKROOT_PATH="$(xcrun --show-sdk-path)"
    EXTRA_ARGS=(-extra-arg=-isysroot -extra-arg="${SDKROOT_PATH}")
fi

cd "${ROOT_DIR}"
xargs -0 "${RUN_CLANG_TIDY}" \
    -quiet \
    -p "${BUILD_DIR}" \
    -clang-tidy-binary "${CLANG_TIDY}" \
    -warnings-as-errors='*' \
    "${EXTRA_ARGS[@]}" < "${FILE_LIST}"
