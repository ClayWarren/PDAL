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

# Pass 1: Run standard tests
cargo llvm-cov \
    --manifest-path rust/Cargo.toml \
    --workspace \
    --all-targets \
    --exclude-from-report pdal-capi \
    --no-report \
    -- "${test_args[@]}"

# Pass 2: Run ignored tests and report combined metrics
exec cargo llvm-cov \
    --manifest-path rust/Cargo.toml \
    --workspace \
    --all-targets \
    --exclude-from-report pdal-capi \
    --no-clean \
    --ignore-run-fail \
    "${cov_args[@]}" \
    -- "${test_args[@]}" --ignored

