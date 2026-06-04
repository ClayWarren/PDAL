#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUST_DIR="${ROOT_DIR}/rust"
ITERATIONS="${PDAL_RUST_PERF_ITERS:-5}"
COLD_BUILD=0
TEST_SUITES=0

# Locate the configured C++ build tree (used by --test-suites' ctest run).
# The pixi dev environment configures into .build; a plain CMake build may use
# build. Honor an explicit PDAL_BUILD_DIR, otherwise auto-detect.
BUILD_DIR="${PDAL_BUILD_DIR:-}"
if [[ -z "${BUILD_DIR}" ]]; then
    for candidate in "${ROOT_DIR}/.build" "${ROOT_DIR}/build"; do
        if [[ -f "${candidate}/CTestTestfile.cmake" ]]; then
            BUILD_DIR="${candidate}"
            break
        fi
    done
fi
BUILD_LIB_DIR=""
if [[ -n "${BUILD_DIR}" && -d "${BUILD_DIR}/lib" ]]; then
    BUILD_LIB_DIR="${BUILD_DIR}/lib"
fi

REFERENCE_PDAL="${PDAL_REFERENCE_PDAL:-}"
if [[ -z "${REFERENCE_PDAL}" ]]; then
    for candidate in /opt/homebrew/bin/pdal /usr/local/bin/pdal; do
        if [[ -x "${candidate}" ]]; then
            REFERENCE_PDAL="${candidate}"
            break
        fi
    done
fi
if [[ -z "${REFERENCE_PDAL}" ]]; then
    REFERENCE_PDAL="$(command -v pdal || true)"
fi
if [[ -z "${REFERENCE_PDAL}" ]]; then
    echo "error: installed/reference pdal not found; set PDAL_REFERENCE_PDAL." >&2
    exit 2
fi

if [[ -n "${CONDA_PREFIX:-}" ]]; then
    export DYLD_FALLBACK_LIBRARY_PATH="${CONDA_PREFIX}/lib:${DYLD_FALLBACK_LIBRARY_PATH:-/usr/lib}"
    export LD_LIBRARY_PATH="${CONDA_PREFIX}/lib:${LD_LIBRARY_PATH:-}"
fi

usage() {
    cat <<'EOF'
Usage: rust/scripts/measure_guardrails.sh [--cold-build] [--test-suites]

Reports opt-in Rust port guardrail measurements:
  - installed PDAL local I/O pipeline wall time and peak RSS
  - Rust local I/O performance harness wall time and peak RSS
  - Rust workspace incremental build wall time and peak RSS
  - full C++ and Rust test-suite wall time and peak RSS when --test-suites is set

Set PDAL_REFERENCE_PDAL=<path> to choose the installed/reference pdal binary.
Set PDAL_RUST_PERF_ITERS=<n> to control perf harness iterations.
Use --cold-build only when you intentionally want to run cargo clean first.
Use --test-suites only when you intentionally want the slower full-suite timing.
EOF
}

while (($#)); do
    case "$1" in
        --cold-build)
            COLD_BUILD=1
            ;;
        --test-suites)
            TEST_SUITES=1
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/pdal-rust-guardrails.XXXXXX")"
trap 'rm -rf "${TMP_DIR}"' EXIT

escape_json_path() {
    python3 -c 'import json, sys; print(json.dumps(sys.argv[1])[1:-1])' "$1"
}

write_pipeline() {
    local reader="$1"
    local input="$2"
    local writer="$3"
    local output="$4"
    local precision="$5"
    local pipeline="$6"

    local writer_options
    if [[ "${writer}" == "writers.text" ]]; then
        writer_options="\"filename\":\"$(escape_json_path "${output}")\",\"order\":\"X,Y,Z\",\"quote_header\":false,\"precision\":${precision}"
    else
        writer_options="\"filename\":\"$(escape_json_path "${output}")\",\"order\":\"X,Y,Z\",\"precision\":${precision}"
    fi

    cat >"${pipeline}" <<EOF
[
  {"type":"${reader}","filename":"$(escape_json_path "${input}")"},
  {"type":"filters.decimation","step":2},
  {"type":"${writer}",${writer_options}}
]
EOF
}

time_command() {
    local label="$1"
    shift

    local out="${TMP_DIR}/${label//[^A-Za-z0-9_.-]/_}.out"
    local err="${TMP_DIR}/${label//[^A-Za-z0-9_.-]/_}.err"

    if [[ "$(uname -s)" == "Darwin" && "${label}" == rust-* ]]; then
        python3 - "$label" "${out}" "${err}" "$@" <<'PY'
import os
import resource
import subprocess
import sys
import time

label, out_path, err_path, *cmd = sys.argv[1:]
env = os.environ.copy()
if cmd and cmd[0] == "env":
    cmd = cmd[1:]
    while cmd and "=" in cmd[0] and not cmd[0].startswith("-"):
        key, value = cmd.pop(0).split("=", 1)
        env[key] = value
with open(out_path, "wb") as out, open(err_path, "wb") as err:
    start = time.monotonic()
    result = subprocess.run(cmd, stdout=out, stderr=err, env=env, check=False)
    elapsed = time.monotonic() - start
if result.returncode:
    sys.stderr.write(open(err_path, encoding="utf-8", errors="replace").read())
    raise SystemExit(result.returncode)

rss_bytes = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
print(f"{label},{elapsed:.2f},{rss_bytes / (1024 * 1024):.2f}")
PY
        return
    fi

    if /usr/bin/time -l true >/dev/null 2>&1; then
        /usr/bin/time -l "$@" >"${out}" 2>"${err}"
        python3 - "$label" "${err}" <<'PY'
import re
import sys

label, path = sys.argv[1], sys.argv[2]
text = open(path, encoding="utf-8", errors="replace").read()
rss = re.search(r"^\s*(\d+)\s+maximum resident set size", text, re.M)
real = re.search(r"^\s*([0-9.]+)\s+real", text, re.M)
rss_mib = int(rss.group(1)) / (1024 * 1024) if rss else None
real_s = float(real.group(1)) if real else None
print(f"{label},{real_s if real_s is not None else ''},{rss_mib:.2f}" if rss_mib is not None else f"{label},{real_s if real_s is not None else ''},")
PY
    elif /usr/bin/time -v true >/dev/null 2>&1; then
        /usr/bin/time -v "$@" >"${out}" 2>"${err}"
        python3 - "$label" "${err}" <<'PY'
import re
import sys

label, path = sys.argv[1], sys.argv[2]
text = open(path, encoding="utf-8", errors="replace").read()
rss = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", text)
elapsed = re.search(r"Elapsed \(wall clock\) time.*:\s*([0-9:.]+)", text)
rss_mib = int(rss.group(1)) / 1024 if rss else None
real_s = ""
if elapsed:
    parts = [float(p) for p in elapsed.group(1).split(":")]
    if len(parts) == 3:
        real_s = parts[0] * 3600 + parts[1] * 60 + parts[2]
    elif len(parts) == 2:
        real_s = parts[0] * 60 + parts[1]
    else:
        real_s = parts[0]
print(f"{label},{real_s},{rss_mib:.2f}" if rss_mib is not None else f"{label},{real_s},")
PY
    else
        local start end
        start="$(python3 -c 'import time; print(time.monotonic())')"
        "$@" >"${out}" 2>"${err}"
        end="$(python3 -c 'import time; print(time.monotonic())')"
        python3 - "$label" "$start" "$end" <<'PY'
import sys
label, start, end = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
print(f"{label},{end - start},")
PY
    fi
}

run_installed_case() {
    local name="$1"
    local reader="$2"
    local input="$3"
    local writer="$4"
    local precision="$5"
    local case_dir="${TMP_DIR}/${name}"
    mkdir -p "${case_dir}"
    local output="${case_dir}/installed.out"
    local pipeline="${case_dir}/pipeline.json"
    write_pipeline "${reader}" "${input}" "${writer}" "${output}" "${precision}" "${pipeline}"
    time_command "installed-${name}" "${REFERENCE_PDAL}" pipeline "${pipeline}"
}

echo "metric,wall_seconds,peak_rss_mib"

run_installed_case "text-decimation" "readers.text" "${ROOT_DIR}/test/data/text/utm17_1.txt" "writers.text" 2
run_installed_case "pcd-decimation" "readers.pcd" "${ROOT_DIR}/test/data/pcd/utm17_space.pcd" "writers.pcd" 2
run_installed_case "pts-decimation" "readers.pts" "${ROOT_DIR}/test/data/pts/site_56_8.pts" "writers.pcd" 6
run_installed_case "ptx-decimation" "readers.ptx" "${ROOT_DIR}/test/data/ptx/1.2-with-color.ptx" "writers.pcd" 6

time_command "rust-local-io-perf-harness" env PDAL_RUST_PERF_ITERS="${ITERATIONS}" cargo test --manifest-path "${RUST_DIR}/Cargo.toml" -p pdal-io --test perf_regression -- --ignored --nocapture

if [[ "${COLD_BUILD}" == "1" ]]; then
    cargo clean --manifest-path "${RUST_DIR}/Cargo.toml" >/dev/null
    time_command "rust-workspace-cold-build" cargo build --manifest-path "${RUST_DIR}/Cargo.toml" --workspace
else
    time_command "rust-workspace-incremental-build" cargo build --manifest-path "${RUST_DIR}/Cargo.toml" --workspace
fi

if [[ "${TEST_SUITES}" == "1" ]]; then
    if [[ -z "${BUILD_DIR}" || ! -f "${BUILD_DIR}/CTestTestfile.cmake" ]]; then
        echo "error: --test-suites needs a configured C++ build tree with registered tests;" >&2
        echo "       set PDAL_BUILD_DIR or build into .build/ or build/ first." >&2
        exit 2
    fi
    if [[ -n "${BUILD_LIB_DIR}" ]]; then
        time_command "cpp-full-test-suite" env \
            "DYLD_LIBRARY_PATH=${BUILD_LIB_DIR}:${DYLD_LIBRARY_PATH:-}" \
            "LD_LIBRARY_PATH=${BUILD_LIB_DIR}:${LD_LIBRARY_PATH:-}" \
            ctest --test-dir "${BUILD_DIR}" --output-on-failure
    else
        time_command "cpp-full-test-suite" ctest --test-dir "${BUILD_DIR}" --output-on-failure
    fi
    time_command "rust-full-test-suite" cargo test --manifest-path "${RUST_DIR}/Cargo.toml" --workspace -- --test-threads=1
fi
