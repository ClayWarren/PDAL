#!/usr/bin/env bash
set -euo pipefail

if ! cargo mutants --version >/dev/null 2>&1; then
    cat >&2 <<'EOF'
cargo-mutants is required for Rust mutation testing.

Install it with:
  cargo install --locked cargo-mutants

Then rerun this command. Mutation testing is intentionally not part of
rust-guard; run it deliberately on mature port areas.
EOF
    exit 127
fi

exec cargo mutants --manifest-path rust/Cargo.toml "$@"
