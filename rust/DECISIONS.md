# Rust Port Decision Ledger

This file records the decisions that close open-ended porting questions. It is
the tie-breaker for future agent work: if a choice is listed here, do not reopen
it unless a concrete parity failure proves the decision wrong.

## Global Decisions

| Area | Decision | Reason |
|---|---|---|
| C ABI | Keep the C ABI as the cross-language contract. | C++, Python, CLI, and future bindings need one stable boundary. Rust and C++ objects must not be shared directly. |
| C++ public API | Keep exported C++ classes as compatibility wrappers. | Existing downstream users include PDAL headers and link C++ symbols. Removing those symbols is not part of this port. |
| C++ tests | Keep pre-existing C++ tests as the behavioral contract. | The target is behavior-preserving replacement, not a test rewrite. |
| Rust tests | Keep Rust unit/integration tests as implementation coverage, not as a replacement for C++ parity tests. | Rust tests catch local behavior; C++ tests catch compatibility. |
| Vendor code | Do not port vendored source line-by-line. | Vendor directories are third-party dependencies, not first-party PDAL implementation. |
| Optional plugins | Keep broad optional plugin implementation late or native-adapter-bound. | Plugin packaging, dynamic loading, metadata, and SDK boundaries need a stable first-party stack first. |
| Unsafe Rust | Keep unsafe at C ABI, native FFI, and tests for those boundaries. | Unsafe spread into ordinary stage logic would defeat the purpose of the port. |
| Formatting-only churn | Keep formatting out of port commits except where touched code needs local formatting. | The port diff is already large; formatting belongs in its own branch/PR. |

## Release Gate Decisions

| Gate | Decision | Reason |
|---|---|---|
| Rust default guard | Release-blocking. `rust-guard` must pass. | This covers formatting, type checking, clippy, Rust tests, coverage threshold, license audit, unsafe accounting, and C ABI header sync. |
| C++ compatibility audits | Release-blocking. `rust-cpp-test-parity` and `rust-cpp-port-audit` must pass before upstreaming. | The port is only valid if the pre-port C++ behavioral contract remains Rust C ABI-backed and the implementation backlog stays classified. |
| Install/export/package smoke | Release-blocking. `rust-capi-install-smoke` and `rust-source-package-smoke` must pass. | Downstream consumers need `find_package(PDAL)` / `PDAL::CAPI` and source releases to work, not just the build tree. |
| Rust mutation testing | Visibility gate, not release-blocking yet. | Run deliberately on mature buckets; require investigation of meaningful survivors before promoting to a threshold. |
| Performance, memory, binary size, startup, compile time, and full-suite timing | Visibility gates. Record comparable data and explain major regressions; do not block on fixed ratios yet. | Current harnesses are useful but still platform/build-config-sensitive. Hard thresholds need a controlled same-config baseline. |
| C++ coverage | Not a Rust-port release gate. | The pre-existing C++ tests are the compatibility contract; raising legacy C++ coverage is lower value than finishing and guarding the Rust-backed replacement. |

## Mainline Source Areas

| Area | Decision | What remains |
|---|---|---|
| `apps/` | Done as a thin C++ executable peer over the Rust C ABI. | Keep CLI argument plumbing/log setup as C++ glue unless Rust becomes the installed executable. |
| `tools/` | Done for `lasdump` and `nitfwrap` as C ABI-backed launchers. | No broad tool work unless a new tool has a parity test and a Rust implementation path. |
| `kernels/` | Keep exported C++ kernel classes as API shells; Rust owns command dispatch for first-party commands through the C ABI. | Broaden command parity by workflow/output shape, not by deleting C++ classes. XML pipeline fallback remains explicit until Rust XML pipeline support is chosen. |
| `filters/` | Pure first-party filter implementation backlog is closed. | Remaining C++ is wrapper/compatibility surface plus documented native/private adapters. New filter work should be parity-gap-driven only. |
| `pdal/` core | Keep C++ compatibility shells; Rust owns behavior where a C ABI-backed replacement exists. | Expand Rust core only when a stage/I/O/command needs it. Do not rewrite by C++ source file. |
| `io/` | First-party I/O implementation backlog is closed for the current scope. | Local deterministic readers/writers, covered remote/VSI paths, and C ABI-backed wrappers are complete; broader OGR/cloud/vendor-heavy behavior remains native-adapter or parity-gap-driven work. |
| `examples/` | Keep examples as downstream install/export checks. | Plugin-authoring examples wait for plugin SDK/export decisions. |
| `doc/` | Keep Rust docs developer-facing until the port is upstreamable. | Public user docs wait for final build/install/plugin policy. |
| `scripts/` | Treat scripts as QA and migration support, not product surface. | Add scripts only when they guard a named milestone. |

## Native Adapter Decisions

| Boundary | Decision | Notes |
|---|---|---|
| GDAL raster/vector | Use `pdal-native` with `gdal-sys`; keep C++ GDAL compatibility where Rust does not yet own parity. | Applies to GDAL reader/writer, OGR writer, TIndex/OGR paths, raster helpers, and covered GeoJSON/Shapefile/GPKG cases. |
| PROJ/SRS | Use `pdal-native`/Rust C ABI for SRS and transforms where already covered; retain C++ GDAL/OGR consumers that still require C++ objects. | `SrsTransform::get()` consumers remain C++ compatibility pointers; LAS GeoTIFF VLR decoding and encoding use the Rust/native libgeotiff path. |
| GEOS | Use Rust GEOS only through isolated `pdal-native` paths. Do not call it from legacy C++ `Geometry`/`Polygon` methods. | Direct mixed GEOS hooks were unstable on macOS; C++ geometry stays on existing GDAL/OGR path. |
| Nitro | Use `pdal-native` Nitro adapter for NITF wrap/unwrap and NITF reader/writer parity. | Do not port Nitro itself. |
| Arbiter/connector | Keep C++ Arbiter connector as a native adapter. Rust remote reads use GDAL VSI for covered paths. | Do not build a Rust Arbiter mirror unless a concrete parity case requires it. |
| libgeotiff | Use the `pdal-native` libgeotiff adapter for LAS GeoTIFF GeoKey decoding and encoding. | Added when user-defined GeoTIFF SRS VLRs were needed for LAS/STAC parity, then extended to pre-1.4 LAS writer SRS VLR parity. |
| LEPCC/I3S/SLPK | Keep ESRI reader family as LEPCC-backed native adapter for now. | `readers.slpk`/`readers.i3s` may be inferred but must fail cleanly in the Rust registry until a real port/FFI decision lands. |
| E57 bundled library | Treat `plugins/e57/libE57Format` as vendor/native adapter, not first-party code to rewrite. | A future E57 milestone may choose FFI or a Rust E57 crate, but not a line-by-line port. |
| Arrow/Parquet | Keep optional Arrow plugin as native adapter until a deliberate Arrow Rust/FFI strategy is chosen. | No broad Arrow port in the first-party milestone. |
| TileDB | Keep optional TileDB plugin as native adapter. | Storage/database-style plugin, not first-party local I/O. |
| HDF5/IceBridge | Keep optional HDF/IceBridge plugins as native adapters. | A Rust HDF strategy would be a dedicated plugin milestone. |
| PostgreSQL PointCloud | Keep optional PGPointCloud plugin as native/service adapter. | Requires database/service parity work, not a core port shortcut. |
| RIEGL RDB/RXP | Keep optional proprietary/native integrations as native adapters. | Do not port without dependency access and parity tests. |
| CPD/TEASER/trajectory | Keep registration/trajectory plugins as native math/solver adapters. | Rust replacements require deliberate algorithm decisions and benchmark/parity gates. |
| Draco/OpenSceneGraph/MBIO/MATLAB | Keep optional codec/runtime/domain plugins as native adapters. | These are not needed to finish first-party PDAL replacement. |

## Vendor Decisions

| Vendor | Decision |
|---|---|
| `vendor/arbiter` | Keep C++ compatibility adapter; Rust covered remote paths use GDAL VSI. |
| `vendor/eigen` | Do not port Eigen. Use Rust math crates/local math behind Rust APIs when needed. |
| `vendor/gtest` | Keep for C++ parity tests only. |
| `vendor/h3` | Replace with `h3o` in Rust. |
| `vendor/kazhdan` | Deferred private algorithm decision for Poisson/reconstruction only. |
| `vendor/lazperf` | Replace with `las`/`laz` crates for current Rust LAS/LAZ; keep lazperf for C++ compatibility. |
| `vendor/lepcc` | Keep as ESRI reader native-adapter dependency until that family is active. |
| `vendor/nanoflann` | Do not port; use Rust spatial-index abstraction. |
| `vendor/nlohmann` | No Rust role; use `serde_json`. |
| `vendor/schema-validator` | Defer until Rust pipeline/config schema validation becomes an active milestone. |
| `vendor/utfcpp` | No early Rust role; use Rust UTF-8/string APIs unless parity proves otherwise. |

## Plugin Decisions

| Plugin | Decision |
|---|---|
| `plugins/faux` | Done as C ABI-backed plugin command checkpoint. |
| `plugins/nitf` | Done for reader/writer/wrap workflows through Nitro native adapter; remaining helper code is native adapter/glue. |
| `plugins/spz` | Done as fixture-backed Rust reader/writer checkpoint. |
| `plugins/e57` | Native adapter/deferred; do not rewrite bundled E57 library. |
| `plugins/arrow` | Native adapter/deferred. |
| `plugins/tiledb` | Native adapter/deferred. |
| `plugins/trajectory` | Native adapter/deferred. |
| `plugins/pgpointcloud` | Native/service adapter/deferred. |
| `plugins/draco` | Native codec adapter/deferred. |
| `plugins/hdf` | Native HDF adapter/deferred. |
| `plugins/icebridge` | Native domain/HDF adapter/deferred. |
| `plugins/rdb` | Native/proprietary adapter/deferred. |
| `plugins/rxp` | Native/proprietary adapter/deferred. |
| `plugins/matlab` | External runtime adapter/deferred. |
| `plugins/mbio` | Native/domain adapter/deferred. |
| `plugins/openscenegraph` | Native scene adapter/deferred. |
| `plugins/cpd` | Native solver adapter/deferred. |
| `plugins/teaser` | Native solver adapter/deferred. |

## Holdout Decisions

| Holdout family | Decision |
|---|---|
| Exported C++ base classes and typedefs | Keep as compatibility/API shell. |
| `ProgramArgs` C++ argument binding | Keep as C++ app/kernel compatibility glue until Rust CLI fully replaces installed `pdal`. |
| `PipelineManager` / `PipelineExecutor` | Keep as compatibility facade over existing C++ SDK expectations. |
| `PointRef`, `PointTable`, `Reader`, `Writer`, `FlexWriter`, DB base classes | Keep as exported C++ compatibility surfaces. |
| Public endian streams/extractor/inserter/null ostream helpers | Keep as C++ compatibility helpers. |
| Deprecated LAS/BPF header/VLR compatibility APIs | Keep exported C++ compatibility APIs. |
| `StreamCallbackFilter` | Keep as C++ callback ABI holdout until a deliberate callback ABI is designed. |
| `dimbuilder` Rust-free Utils fallback | Keep permanently for standalone generator build behavior. |

## What To Work On Next

After these decisions, do not hunt for generic "remaining C++." The useful next
work is:

1. keep the release-blocking gates green: `rust-guard`,
   `rust-cpp-test-parity`, `rust-cpp-port-audit`,
   `rust-capi-install-smoke`, and `rust-source-package-smoke`;
2. broaden upstream/platform confidence, especially Linux and Windows CI
   behavior for Rust build, install/export, package, and C ABI consumer checks;
3. refresh visibility evidence when behavior or build shape changes:
   performance, memory, binary size, startup time, compile time, full-suite
   timing, and targeted mutation testing;
4. strengthen parity/regression coverage only for concrete user-visible deltas
   in existing Rust-backed workflows;
5. select a native-adapter/plugin family only as a dedicated milestone with
   fixtures, dependency policy, and a regression target.
