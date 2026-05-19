# Vendor Boundary

`vendor/` is not a Rust porting source area. Vendored code should not be copied
into Rust crates as a way to make progress. Treat it as one of three things:

1. an external dependency with a maintained Rust crate,
2. an external dependency reached through explicit FFI, or
3. C++ implementation detail that stays behind an existing C++ stage until that
   stage has a dedicated port-versus-FFI decision.

This file exists so agents do not have to infer the vendor plan from scattered
includes.

Rust native-library bindings should be centralized through `pdal-native` or an
equally explicit adapter crate. Feature crates such as `pdal-core`,
`pdal-filters`, and `pdal-io` should depend on PDAL-level wrappers, not raw
vendor APIs, unless a focused migration step explicitly proves that boundary is
too coarse.

## When To Touch Vendor Compatibility

Vendor compatibility is not a standalone milestone. Work on it only when the
current Rust milestone reaches a user-visible behavior that depends on a
vendored or external library boundary.

- During filter work, make a vendor decision only for the filter family being
  ported, such as linear algebra, H3, GDAL/PROJ/SRS, GEOS geometry, or a
  private reconstruction/segmentation algorithm.
- During I/O work, make a vendor decision only when the selected reader/writer
  needs it, such as LAS/LAZ compression, LEPCC, remote/object-store access, or
  JSON schema validation.
- During app/kernel work, use only vendor decisions already proven by the
  lower-layer pipeline, reader, writer, and stage parity tests.

Every vendor decision must name the stage, reader, writer, or core behavior it
unlocks. It should also name the parity gate that will compare Rust behavior to
the existing C++ implementation. Do not create broad compatibility crates or
copy vendor source just to reduce the apparent amount of remaining work.

## Current Mapping

| Vendor path | Current C++ role | Rust-port stance |
| --- | --- | --- |
| `vendor/arbiter` | Remote/local file access support | Do not port early. I/O should first prove deterministic local readers/writers. Later choose a Rust async/object-store stack or keep C++/FFI for remote access. |
| `vendor/eigen` | Linear algebra for geometry, statistics, registration, and filters | Do not vendor Eigen into Rust. Existing Rust filter ports currently use small local math where parity needs are narrow. If broader linear algebra is needed, prefer a Rust crate such as `nalgebra` after a concrete filter/core requirement. |
| `vendor/gtest` | C++ test framework | No Rust-port role. Existing C++ tests remain the behavioral contract. Rust uses Cargo tests. |
| `vendor/h3` | H3 indexing support | Already replaced on the Rust side by the `h3o` crate. Do not bind to the vendored C H3 library unless parity requires behavior `h3o` cannot provide. |
| `vendor/kazhdan` | Poisson reconstruction private algorithm | Defer. For Poisson-related stages, make a stage-level decision: port the algorithm, bind the C++ implementation through FFI, or leave the stage in C++. Do not begin with a broad Kazhdan rewrite. |
| `vendor/lazperf` | LAS/LAZ compression and LAS point stream helpers | Defer until the I/O vertical slice reaches LAS/LAZ. Prefer a maintained Rust crate or explicit FFI; do not hand-port compression code casually. |
| `vendor/lepcc` | Esri LEPCC compression support | Defer until the relevant I/O path is selected. Likely FFI or leave C++ unless a mature Rust alternative exists. |
| `vendor/nanoflann` | KD-tree nearest-neighbor support | Do not port directly. Rust spatial filters already use a shared `pdal-core::spatial` API that can swap internals later. If performance requires it, choose a Rust spatial index crate behind that API. |
| `vendor/nlohmann` | JSON support in C++ code | No direct Rust-port role. Rust code should use `serde_json` where JSON is part of the contract. |
| `vendor/schema-validator` | JSON schema validation for pipeline/config surfaces | Defer until Rust owns pipeline JSON validation. Prefer a Rust JSON-schema crate or explicit compatibility adapter after parity tests identify the required draft/features. |
| `vendor/utfcpp` | UTF conversion helpers in C++ paths | No early Rust-port role. Use Rust string/UTF-8 APIs unless a specific C++ parity issue requires an adapter. |

## Rules

- Do not add vendored source under `rust/`.
- Do not create a `pdal-vendor` crate just to mirror `vendor/`.
- Use `pdal-native` as the default home for GDAL/OGR, GEOS, PROJ,
  LASzip/laz-perf, and similar native adapter work.
- Prefer dependency crates for stable external libraries when they match PDAL
  behavior.
- Use explicit FFI when the external library is the behavior contract or the
  Rust ecosystem does not have a credible replacement.
- Leave plugin/vendor-heavy stages in C++ until the stage has tests strong
  enough to validate a Rust replacement.
- Every vendor decision should name the stage or core behavior that needs it.

## Already Chosen Rust Replacements

- H3: `h3o`
- GEOS geometry operations: `geos` through `pdal-native`
- PROJ transformations: `proj`
- GDAL raster/vector access: `gdal-sys` through `pdal-native`
- JSON parsing: `serde_json`

These are choices made for the current spike, not a permanent promise. If a
crate cannot match PDAL's existing behavior, keep the C++ path or add an FFI
adapter instead of changing user-visible semantics.
