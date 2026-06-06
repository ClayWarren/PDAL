# Rust Port Handoff Packet

This is the short reviewer entrypoint for the Rust-backed PDAL draft PR. The
long-form rules and ledgers remain in `PORTING.md`, `STATUS.md`,
`DECISIONS.md`, `VENDOR.md`, and `BENCHMARKS.md`.

## Current Claim

The current branch is not a declaration of full PDAL feature parity. It is a
Rust-backed replacement candidate whose first-party implementation backlog is
closed for the current scope, with explicit C++ compatibility shells,
native-adapter boundaries, and holdouts documented.

The main proof points are:

- Pre-port C++ GoogleTest parity: `819 / 819` counted baseline cases route
  through Rust C ABI-backed paths.
- C++ implementation backlog: `0` port-candidate files in mainline and
  plugin-inclusive scans.
- Workflow parity: `25` installed-PDAL workflow checks pass against the
  Rust-backed build, with `3` byte-exact contracts and `22` semantic contracts.
- Rust coverage: line coverage is currently `91.05%`.
- Full release gate: `pixi run -e dev rust-release-gate` passes locally on
  macOS, including Rust checks, audits, workflow parity, install/source smokes,
  and `187 / 187` CTest binaries.

## Reproduce The Evidence

```sh
pixi run -e dev rust-release-gate
python3 rust/scripts/compare_pdal_workflows.py \
    --rust .build/bin/pdal \
    --json-report /tmp/pdal-workflow-parity.json
python3 rust/scripts/audit_cpp_port_backlog.py \
    --include-plugins \
    --json-report /tmp/pdal-cpp-backlog.json
pixi run -e dev rust-bench
pixi run -e dev rust-bench-memory
pixi run -e dev rust-bench-size
```

The workflow report is intentionally split into exact and semantic contracts.
Byte-for-byte is required only where PDAL has deterministic textual/artifact
output. PCD/PLY/LAS/info and several command workflows are compared
semantically because binary headers, metadata timestamps, point-format defaults,
randomized samples, classification tie-breaks, and floating-point formatting are
not stable byte contracts.

## Remaining Review Focus

- Platform confidence: local evidence is strongest on macOS. Linux, Windows,
  and package-manager CI should be treated as real handoff checks, especially
  Rust/CMake linkage and shared-library paths.
- Windows STAC execution: Rust STAC preview/probing remains active, but full
  STAC execution is currently held on the C++ native path after the nested Rust
  reader path hit an MSVC stack overflow in CI. Non-Windows full STAC execution
  stays Rust-backed.
- Native adapters: GDAL/PROJ/GEOS/Nitro/Arrow/TileDB/etc. are not rewritten
  line-by-line. They remain native dependencies behind explicit adapter
  boundaries.
- C++ holdouts: exported C++ SDK compatibility surfaces, callback/debug
  boundaries, stream/endian helpers, and build-tool exceptions are deliberately
  retained. Reopen one only with a concrete replacement API and parity coverage.
- Performance evidence: benchmark harnesses exist and show no major local
  regression, but current numbers are macOS-only and not a controlled
  same-flags pure-C++ build comparison.
- Public product policy: user-facing docs, plugin SDK/versioning, and final
  release positioning still need upstream decisions.

## Where To Look

- `PORTING.md`: migration rules and finish-line criteria.
- `STATUS.md`: live feature/platform status ledger and accepted-boundary notes.
- `PARITY.md`: C++ test-parity accounting, implementation-replacement backlog,
  mixed-test notes, and audit commands.
- `DECISIONS.md`: settled architecture and release-gate decisions.
- `VENDOR.md`: vendor/dependency policy.
- `BENCHMARKS.md`: parity, performance, memory, binary footprint, and build-cost
  evidence.
