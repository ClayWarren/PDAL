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

args=("$@")
if (($# == 0)); then
    args=(--summary-only)
fi

has_test_args=false
for arg in "${args[@]}"; do
    if [[ "$arg" == "--" ]]; then
        has_test_args=true
        break
    fi
done

if [[ "$has_test_args" == false ]]; then
    args+=(-- --test-threads=1)
else
    args+=(--test-threads=1)
fi

exec cargo llvm-cov \
    --manifest-path rust/Cargo.toml \
    --workspace \
    --all-targets \
    --exclude-from-report pdal-capi \
    "${args[@]}"

