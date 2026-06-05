#!/usr/bin/env bash
set -euo pipefail

if ! cargo llvm-cov --version >/dev/null 2>&1; then
    cat >&2 <<'EOF'
cargo-llvm-cov is required for Rust coverage reporting.

Install it through the Pixi dev environment, or manually with:
  cargo install cargo-llvm-cov

Then rerun:
  pixi run -e dev rust-coverage
EOF
    exit 127
fi

if [[ -n "${CONDA_PREFIX:-}" ]]; then
    export DYLD_FALLBACK_LIBRARY_PATH="${CONDA_PREFIX}/lib:/usr/lib${DYLD_FALLBACK_LIBRARY_PATH:+:${DYLD_FALLBACK_LIBRARY_PATH}}"
    export LD_LIBRARY_PATH="${CONDA_PREFIX}/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
fi

cov_args=()
test_args=()
seen_dash_dash=false

for arg in "$@"; do
    if [[ "$arg" == "--" ]]; then
        seen_dash_dash=true
        continue
    fi
    if [[ "$seen_dash_dash" == true ]]; then
        test_args+=("$arg")
    else
        cov_args+=("$arg")
    fi
done

if ((${#cov_args[@]} == 0)); then
    cov_args=(--summary-only)
fi
test_args+=(--test-threads=1)

# The native adapter crate exercises GDAL/PROJ/libgeotiff directly. It is
# already covered by the regular Rust test gate; running it under llvm-cov can
# crash at native-library teardown on CI. Keep it tested, but out of coverage.
cargo test \
    --manifest-path rust/Cargo.toml \
    -p pdal-native \
    --no-default-features \
    -- "${test_args[@]}"

# Pass 1: Run standard tests
cargo llvm-cov \
    --manifest-path rust/Cargo.toml \
    --workspace \
    --all-targets \
    --no-default-features \
    --exclude-from-report pdal-capi \
    --exclude pdal-native \
    --no-report \
    -- "${test_args[@]}"

# Pass 2: Run ignored installed-PDAL parity tests only when the baseline binary
# exists. CI coverage must remain self-contained, while local/Homebrew parity
# runs can still include the extra ignored tests.
if ! command -v pdal >/dev/null 2>&1; then
    exec cargo llvm-cov \
        --manifest-path rust/Cargo.toml \
        --workspace \
        --all-targets \
        --no-default-features \
        --exclude-from-report pdal-capi \
        --exclude pdal-native \
        --no-clean \
        "${cov_args[@]}"
fi

exec cargo llvm-cov \
    --manifest-path rust/Cargo.toml \
    --workspace \
    --all-targets \
    --no-default-features \
    --exclude-from-report pdal-capi \
    --exclude pdal-native \
    --no-clean \
    --ignore-run-fail \
    "${cov_args[@]}" \
    -- "${test_args[@]}" --ignored
