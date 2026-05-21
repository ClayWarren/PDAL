# Rust Port Status

This is still a work in progress and is not yet at full feature parity with
C++ PDAL. Bugs may exist. Check this list before assuming a behavior is
intentional or before asking an agent to broaden the port.

Status definitions:

- `done`: believed done for the stated scope. No known major deficits in that
  area. OK to log bugs or add parity cases.
- `in progress`: actively being built. Some behavior works and some does not.
  OK to log crashes, panics, missing parity, and concrete gaps.
- `prototype`: proof-of-concept only. Do not treat as supported behavior.
- `not ready`: not started, or far enough from parity that it should not be
  used as a migration base yet.
- `deferred`: intentionally left for a later milestone or left in C++ for now.

## Feature Status

| Feature | Status | Notes |
|---|---|---|
| Rust core point model | in progress | `PointLayout`, `PointView`, dimension IDs/types, source indices, per-view SRS text, mesh faces, and 2D/3D bounds exist. Continue expanding only for real stage/I/O/command needs. |
| Rust stage model | in progress | Filter and streamable traits exist. Multi-input `run()` behavior is present and `filters.merge` handles DAG inputs without redundant points. |
| Rust pipeline graph | in progress | Reader/filter/writer DAG execution, tags, dependencies, metadata aggregation, summaries, and error propagation exist. Not a full C++ `PipelineManager` replacement. |
| Options | in progress | String-keyed typed getters match the current Rust option flow. Full C++ `Options` parity is not claimed. |
| Metadata | in progress | Typed scalar metadata trees and JSON serialization exist. C++ descriptions, arrays/list kind preservation, JSON/base64 typed nodes, and full pipeline serialization remain incomplete. |
| Spatial reference | prototype | Text plus coordinate epoch are stored and exported. No full GDAL/PROJ-backed normalization, reprojection, authority lookup, WKT conversion, axis ordering, or unit handling yet. |
| Spatial index | in progress | Exact brute-force neighbor queries sit behind a replaceable API. Do not bake one-off neighbor searches into new filters. |
| Expressions | in progress | Conditional, math, and assignment parser/evaluator support current Rust expression/assign work. Full C++ expression surface is not claimed. |
| C ABI bridge | in progress | Rust-owned handles are the contract. Metadata, summaries, views, and pipeline calls are exposed. Never pass C++ object pointers as Rust handles. |
| C++ filter wrappers | in progress | Safe ports use explicit Rust view conversion. Existing C++ filter tests remain the parity gate. |
| Filter ports | in progress | 84 first-party filter/static stage files exist in C++; 51 are Rust-backed through the C ABI, 33 intentionally remain C++ for now, and 41 are visible through the Rust pipeline registry. Registry exposure is not the same as full pipeline parity. |
| Filter layout mutation | prototype | A narrow prepare/layout hook exists for registry-visible derived-dimension filters such as `NNDistance`, `RadialDensity`, `Eigenvalue*`, `ClusterID`, `HeightAboveGround`, `Coplanar`, `PlaneFit`, `Reciprocity`, and custom `filters.zsmooth` dimensions. More complex layout mutation remains open. |
| Pure/local I/O harness | in progress | `readers.faux` and `writers.null` support in-memory pipeline testing. |
| Text I/O | done | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, and installed-PDAL regression coverage exists for scoped workflows. |
| PCD I/O | in progress | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including ASCII, binary, binary-compressed, precision, streaming, and double-field coverage. Broader installed-PDAL parity coverage can still grow. |
| PTS/PTX readers | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path, including Leica ASCII fixture behavior and installed-PDAL regressions. |
| ILVIS2 reader | in progress | Existing C++ reader and metadata-sidecar unit-test shapes pass through the Rust-backed path for deterministic ASCII point and fixture-shaped XML metadata behavior. |
| PLY I/O | in progress | C++ reader/writer unit-test shapes pass through the Rust-backed path, including ASCII/binary reads, ASCII/binary writes, mesh faces, precision/dim typing, and `#` flex filenames. Broader installed-PDAL and uncommon PLY fixture coverage can still grow. |
| OBJ reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path, including deterministic Wavefront OBJ ASCII data, mesh faces, and VTN de-duplication. |
| GLTF writer | in progress | Existing C++ writer unit-test shapes pass through the Rust-backed path for deterministic local GLB output from mesh-backed views. |
| OGR writer | prototype | GeoJSON point and MultiPoint FeatureCollection output is covered, including `attr_dims` and `multicount` constraints. Shapefile, GeoPackage, native OGR layer creation/options, transactions, and measure dimensions are deferred. |
| QFIT reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path for deterministic NASA ATM QFIT binary fixtures. |
| SBET/SMRMSG I/O | in progress | Existing C++ SBET reader/writer and SMRMSG reader unit-test shapes pass through the Rust-backed path for deterministic local trajectory fixtures. |
| LAS/LAZ I/O | in progress | `las`/`laz` crate path supports standard dimensions, V1.0-1.4 point formats, Extra Bytes, `start`/`count`/`nosrs` reader options, SRS extraction, compression/decompression, and core writer header options. Direct Rust C ABI reader/writer constructors are covered, but the C++ `LasReader`/`LasWriter` classes still use the legacy implementation. Keep parity tests honest before broad claims. |
| COPC reader | prototype | Local `.copc.laz` full-file reads route through the LAS/LAZ path, with post-read 2D/3D bounds filtering. COPC hierarchy traversal, bounds pruning, resolution queries, remote reads, and writer behavior are deferred. |
| EPT reader | prototype | Local LASzip, uncompressed binary, and zstandard EPT full-file reads walk JSON hierarchy and merge local tiles. Resolution limits and query bounds prune hierarchy nodes before tile reads; origin filtering is applied after tile reads. Tile point counts are validated and `ignore_unreadable` can skip unreadable tiles. Reprojection, polygon/OGR filters, addons, remote access, and streaming are deferred. |
| FBI I/O | in progress | TerraScan Fast Binary local path has byte-for-byte installed-PDAL read/write parity for the covered behavior. |
| TerraSolid reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path for deterministic TerraSolid format 2 fixtures. `.bin` is not inferred because it conflicts with FBI. |
| Optech reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path, including deterministic Optech CSD fixture data and localized WGS84 georeference math. |
| BPF I/O | in progress | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including uncompressed and compressed point/dimension/byte interleaves, scaling, flex filenames, output dimensions, auto UTM, and bundled-file metadata. Remote files and deeper ULEM/polar metadata parity are deferred. |
| GDAL reader/writer | prototype | Existing C++ GDAL reader unit-test shapes pass through the Rust-backed path for local raster-to-point-cloud behavior. Writer support is Rust-side/C-ABI only so far and covers Float64 GDAL output for core grid statistics; the C++ writer remains the legacy GDAL implementation. This is not broad GDAL/PROJ permission. |
| Raster writer | in progress | Raster attachments on Rust point views can write through the Rust C ABI, and `pdal_filters_faceraster_test` now exercises the Rust-backed `writers.raster` wrapper. Named/multi-raster behavior is narrow and broader GDAL raster data-type parity remains open. |
| STAC reader | prototype | Local STAC Item/Collection/FeatureCollection traversal can read local assets through already-ported readers. Remote assets, schema validation, filters, EPT/COPC-specific behavior, and threaded catalog crawling are deferred. |
| Driver inference | in progress | Rust can infer existing PDAL reader/writer names from filenames. Construction must still fail cleanly for unported drivers. |
| Pipeline JSON parsing | in progress | Narrow PDAL-style JSON arrays/root `pipeline` objects, filename string stages, scalar options, default linear dependencies, and optional `tag`/`inputs` work for command readiness. |
| `pdal-rs` command shell | in progress | Rust-native shell lists only Rust-backed stages/commands and no longer links the C++ helper dispatch shim. |
| Command metadata | in progress | `--drivers`, `--list-commands`, and `--options <stage>` are backed by Rust-owned metadata for the implemented Rust surface. |
| Implemented commands | in progress | `pipeline`, `info`, `translate`, `merge`, `sort`, `split`, `random`, `hausdorff`, `chamfer`, `delta`, `density`, `eval`, `tile`, and `tindex` have installed-PDAL regression coverage for their scoped workflows. `ground` currently compares point-count preservation only because the Rust SMRF implementation is still a simplified approximation. |
| Performance visibility | prototype | Ignored reporting harnesses exist for local I/O performance, binary size, startup time, memory, and build cost. They are visibility tools, not hard gates yet. |
| Rust mutation testing | prototype | `pixi run -e dev rust-mutants` runs `cargo-mutants` when it is installed locally. This is an audit tool for mature buckets, not part of `rust-guard`. |
| Vendor/native strategy | in progress | `vendor/` has 11 top-level third-party dependency directories. `rust/VENDOR.md` is the source of truth. Two are actively replaced in Rust today (`vendor/h3` -> `h3o`, `vendor/lazperf` -> `las`/`laz`), four have a clear no-direct-port stance (`eigen`, `gtest`, `nanoflann`, `nlohmann`), and five remain deferred (`arbiter`, `kazhdan`, `lepcc`, `schema-validator`, `utfcpp`). Native GDAL/OGR/GEOS/PROJ adapters belong in `pdal-native`; pure Rust replacements such as LAS/LAZ do not need to move through it. |
| Plugins | prototype | There are 18 top-level plugin directories. Track each plugin below. `pdal-plugins` holds discovery metadata, `kernels.fauxplugin` is a compatibility marker, and `readers.spz`/`writers.spz` are the first fixture-backed plugin reader/writer checkpoint. A Rust plugin SDK and broad optional plugin sweep are still not ready. |
| Remote/object-store I/O | deferred | Waits until local deterministic I/O and pipeline execution are stable. |
| Broad kernels/apps/tools migration | deferred | Simple `pdal-rs` commands may continue proving lower layers. Broad kernels, `apps/pdal.cpp`, `lasdump`, and `nitfwrap` wait on lower-layer parity. |

## Root-Level Migration Status

The Rust port is not complete just because Rust-backed tests pass. The root
build, install, packaging, CI, examples, and docs must also describe and verify
the Rust-backed shape of PDAL.

| Area | Status | Notes |
|---|---|---|
| Root CMake | done | `libpdal_capi.a` is built, linked into `pdalcpp`, and sourced from `cmake/rust.cmake` so the dependency list tracks every current Rust crate that can affect the C ABI or linked implementation. |
| `cmake/` modules | in progress | Rust build options now live in `cmake/rust.cmake`. Install rules, CPack/source-package behavior, platform link details, and test wiring still need to move here as the integration matures. |
| `pixi.toml` | done | The developer environment now includes the Rust toolchain and explicit `rust-fmt`, `rust-check`, `rust-clippy`, `rust-test`, and `rust-guard` tasks for the port workspace. |
| GitHub workflows | in progress | The Pixi workflow runs the Rust workspace guard through `pixi run -e dev rust-guard`. Linux, macOS, Windows, conda, and release workflows still need explicit Rust/C++ parity gates before the port is upstreamable. |
| `PDALConfig.cmake.in` | not ready | Downstream `find_package(PDAL)` must keep working. Decide whether the Rust C ABI remains an internal implementation detail of `pdalcpp` or is exported as a stable target/header surface. |
| `pdal_features.hpp.in` | not ready | Add a generated Rust-backed-build feature only if C++ wrappers or downstream code need a supported conditional. Avoid broad preprocessor branching. |
| `dimbuilder/` | prototype | `dimbuilder` currently uses `PDAL_UTILS_NO_RUST_CAPI` while compiling `Utils.cpp` standalone. Keep this as an intentional generator-tool exception or replace it with a cleaner build path. |
| `package.sh` and release packaging | not ready | Release packaging still assumes C++ build tools only and must learn the Rust toolchain, Rust sources, licenses, and generated artifacts before release use. |
| `examples/` | deferred | Examples should prove installed Rust-backed PDAL works after the C ABI, C++ wrapper, and install/export story stabilizes. |
| `doc/` | deferred | Public docs should be updated once build, install, plugin, and ABI boundaries are stable enough to describe accurately. |

## Plugin Status

These are optional PDAL plugin directories, not the core static stage surface.
Do not start a broad plugin sweep until local core, I/O, and command parity are
farther along. One-off checkpoints are allowed when they prove a dependency or
ABI pattern without forcing a plugin SDK decision.

| Plugin | Status | Notes |
|---|---|---|
| `plugins/arrow` | deferred | Arrow/Parquet integration waits on the native/vendor and columnar data strategy. |
| `plugins/cpd` | deferred | Registration filter plugin; wait for the broader registration and linear-algebra strategy. |
| `plugins/draco` | deferred | Draco mesh/point-cloud codec integration waits on a codec FFI or replacement decision. |
| `plugins/e57` | deferred | E57 reader/writer is a major external-format plugin and waits on broader I/O parity. |
| `plugins/faux` | prototype | `kernels.fauxplugin` is ported as a compatibility marker to prove plugin command discovery. |
| `plugins/hdf` | deferred | HDF integration waits on native dependency and multidimensional-array I/O strategy. |
| `plugins/icebridge` | deferred | Domain reader plugin; wait until core first-party readers are farther along. |
| `plugins/matlab` | deferred | MATLAB reader/filter integration waits on external-runtime and plugin-loading strategy. |
| `plugins/mbio` | deferred | MB-System bathymetry integration waits on native dependency strategy. |
| `plugins/nitf` | deferred | NITF tooling and reader behavior wait on plugin I/O and tool migration decisions. |
| `plugins/openscenegraph` | deferred | OSG reader/writer waits on 3D scene dependency and mesh I/O strategy. |
| `plugins/pgpointcloud` | deferred | Database-backed I/O waits on remote/service I/O policy and native dependency choices. |
| `plugins/rdb` | deferred | RIEGL RDB integration waits on proprietary/native dependency availability. |
| `plugins/rxp` | deferred | RIEGL RXP integration waits on proprietary/native dependency availability. |
| `plugins/spz` | in progress | `readers.spz` and `writers.spz` have a Rust fixture-backed checkpoint through `pdal-io`. Broader plugin packaging remains open. |
| `plugins/teaser` | deferred | Registration filter plugin; wait for the broader registration and linear-algebra strategy. |
| `plugins/tiledb` | deferred | TileDB I/O waits on native dependency, array storage, and remote/service I/O policy. |
| `plugins/trajectory` | deferred | Trajectory filter plugin waits until trajectory/SBET/SMRMSG behavior is more complete. |

## Vendor Status

`vendor/` is third-party code kept in-tree by C++ PDAL. The Rust port should not
rewrite these directories wholesale. Bind, replace, or leave each dependency in
place only when a ported stage needs it.

| Vendor directory | Role in the port |
|---|---|
| `vendor/arbiter` | Deferred | Leave until remote/object-store I/O is ready. |
| `vendor/eigen` | No direct port | Use Rust linear algebra where practical; do not port Eigen itself. Current covered math uses local small-matrix routines. |
| `vendor/gtest` | No Rust role | Keep for C++ parity tests. Rust uses Cargo tests. |
| `vendor/h3` | Replaced in Rust | Rust-backed H3 work uses the `h3o` crate. Do not bind vendored C H3 unless parity requires behavior `h3o` cannot provide. |
| `vendor/kazhdan` | Deferred | Decide per Poisson/reconstruction work; likely private algorithm port, FFI, or leave C++ depending on tests. |
| `vendor/lazperf` | Replaced in Rust | Current Rust LAS/LAZ path uses the `las` crate with its `laz` feature. Keep lazperf available for C++ compatibility. |
| `vendor/lepcc` | Deferred | Defer until EPT/COPC compression parity requires it. |
| `vendor/nanoflann` | No direct port | Use the Rust spatial-index API rather than porting nanoflann; internals can be swapped later. |
| `vendor/nlohmann` | No Rust role | C++ JSON dependency; Rust uses `serde_json`. |
| `vendor/schema-validator` | Deferred | Defer until schema validation parity needs it. |
| `vendor/utfcpp` | Deferred | C++ Unicode helper dependency; use Rust string/UTF-8 APIs unless a concrete parity gap appears. |

## C++ Test Parity Accounting

The first target is the pre-existing C++ test suite running against Rust
implementations through the C ABI and C++ wrappers. Rust linkage alone does not
count.

Current checkpoint: `408 / 917` built C++ GoogleTest cases, or `44.49%`, are
confirmed Rust C ABI-backed by `rust/scripts/audit_cpp_test_parity.py`. This is
a conservative lower bound, not a final port-completion percentage: 54 built
test binaries remain unclassified by the audit script. The previous `927 / 927`
claim was withdrawn because it mixed a hand-maintained numerator with a
different denominator.

Current test-suite size: `28,793` C++ code LOC under `test/`. These tests remain
the behavioral contract and should not be counted as unported implementation.

Current C++ compatibility wrapper/adapter surface: `15,351` code LOC across
`113` first-party C++ files that include or directly declare Rust C ABI entry
points, split approximately as `pdal/` 7,387 LOC, `filters/` 5,680 LOC, and
`io/` 2,284 LOC. This is a coarse ceiling because several files still mix real
legacy implementation with wrapper calls; the number should shrink as wrappers
are split from implementation.

Recompute the wrapper LOC baseline with:

```sh
tmp=$(mktemp)
{ rg -l '#include <rust/pdal-capi/include/pdal_capi.h>|#include <pdal_capi.h>|#include "rust/pdal-capi/include/pdal_capi.h"|pdal_dimension_fix_name|extern "C"' pdal io filters kernels apps tools --glob '*.{cpp,hpp,h}'; rg --files filters/private | rg 'Rust.*\.(hpp|cpp|h)$'; } | sort -u > "$tmp"
cloc --quiet --csv --by-file --list-file="$tmp"
rm "$tmp"
```

Counting rules:

- Count a whole test binary only when the primary behavior under test is routed
  through the Rust C ABI.
- Count mixed C++ test binaries only at the individual-test level.
- Do not count tests where Rust is merely present elsewhere in the link graph.
- Do not count registry exposure, Rust unit tests, or command regressions as
  C++ test-suite parity.

Recompute the current built-suite parity checkpoint with:

```sh
python3 rust/scripts/audit_cpp_test_parity.py --build-dir build
```

Known mixed binaries:

- `pdal_kdindex_test`: all 5 tests currently count; KNN/radius queries route
  through Rust spatial query ABI.
- `pdal_spatial_reference_test`: only `calcZone` and `wgs84FromZone` count.
  Most SRS normalization, authority lookup, WKT/PROJJSON, and LAS SRS behavior
  remains C++ GDAL/OGR-backed.
- `pdal_point_view_test`: only `calculateBounds` counts. The broader point
  view/table data model is still C++.
- `pdal_eigen_test`: only `calcBounds` counts. The bounds calculation routes
  through the Rust C ABI; Eigen matrix behavior and math helpers remain C++.
- `pdal_bounds_test`: only `test_ctor`, `test_clip`, `test_intersect`,
  `test_grow`, `test_bounds_grow_2_3_args`, `test_invalid`, `test_input`,
  `test_parse`, `test_parse2`, `test_parse_geojson`, `test_2d_input`,
  `test_precisionloss`, and `fromstring` count. Bounds
  clear/empty/contains/overlap/clip/grow arithmetic and non-SRS parsing route
  through the Rust C ABI. Equality, accessors, formatting, WKT/GeoJSON output,
  SRS bounds, and `ProgramArgs` plumbing remain C++.
- `pdal_utils_test`: only `test_base64`, `blanks`, `replaceAll`,
  `escapeNonprinting`, `escapeJSON`, `wordWrap`, `wordWrap2`, and
  `simpleWordexpTest`, `splitChar`, `split2Char`, `case`, `starts`, and
  `iequals` count. Other utility cases still test C++ templates, stream
  helpers, process helpers, or local formatting behavior.
- `pdal_file_utils_test`: only `test_toAbsolutePath`, `test_getDirectory`,
  `test_isAbsolute`, `filename`, `extension`, and `stem` count. Path
  normalization helpers route through the Rust C ABI. Filesystem mutation,
  VSI, glob, Unicode filesystem behavior, and mmap behavior remain C++/GDAL.
- `pdal_georeference_test`: only the 5 `Georeference.*` tests count;
  WGS84 georeference math routes through the Rust C ABI. `RotationMatrix`
  construction tests remain C++.
- `pdal_charbuf_test`: all 3 tests count; seek-position behavior routes through
  Rust C ABI helpers while C++ keeps the `std::streambuf` pointer mechanics.
- `pdal_math_utils_test`: all 2 tests count; both exercise
  `barycentricInterpolation`, which routes through the Rust C ABI.
- `pdal_scaling_test`: all 2 tests count; auto scale/offset computation routes
  through the Rust C ABI.
- `pdal_filespec_test`: all 2 tests count; file-spec JSON ingestion and
  validation route through the Rust C ABI.
- `pdal_dimension_test`: all 1 test counts; dimension-name sanitization routes
  through the Rust C ABI.
- `pdal_point_table_test`: only `resolveType` counts; dimension type
  resolution routes through the Rust C ABI. Point table storage, user-view
  behavior, iterators, and row/column tables remain C++.
- `pdal_kernel_test`: all 1 test counts; stage-option parsing routes through
  the Rust C ABI.
- `pdal_config_test`: all 1 test counts; version integer and full-version
  formatting route through the Rust C ABI while compile-time version constants
  remain C++.
- `pdal_log_test`: only `t1` counts; level-name formatting routes through the
  Rust C ABI. File output, devnull routing, and CLI logging remain C++.
- `pdal_stage_factory_test`: only `extensionTest` and
  `stageExtensionsLoadPerInstance` count; reader/writer driver inference and
  default extension lookup route through the Rust C ABI. Plugin loading and
  custom extension overrides remain C++.
- `pdal_plugin_manager_test`: only `validnames` counts; plugin filename
  validation routes through the Rust C ABI. Plugin registration and object
  creation remain C++.
- `pdal_options_test`: `valid`, `programargs`, `nan`, `doublepreicison`, and
  `issue_4751` count; option-name validation and command-line argument
  formatting route through the Rust C ABI, and JSON scalar formatting uses Rust
  metadata helpers. Option storage and conditional merging remain C++.
- `pdal_polygon_test`: only `valid` counts; geometry validity routes through
  the Rust native geometry ABI. Polygon construction, serialization, bounds,
  simplification, and point coverage remain C++/GDAL.
- `pdal_quad_index_test`: all 1 test counts; QuadIndex construction, bounds,
  fills, depth, and region queries route through the Rust C ABI.
- `pdal_xml_schema_test`: only `legacyNames` counts; legacy dimension-name
  remapping routes through the Rust C ABI. XML parsing, metadata, xforms, and
  schema round-tripping remain C++/libxml.
- `pdal_uuid_test`: all 3 tests count; UUID parsing, canonical formatting,
  null checks, and v4 random byte generation route through the Rust C ABI while
  C++ keeps the small value wrapper.
- `pdal_ogr_arg_test`: only `parseErrors` counts; OGR JSON option validation
  routes through the Rust C ABI. Geometry loading and polygon extraction remain
  C++/GDAL.
- `pdal_filters_crop_test`: only `test_crop`, `test_crop_3d`,
  `test_crop_polygon`, `multibounds`, `circle`, `sphere`, and
  `test_crop_on_edge` count. The crop selection itself routes through the Rust
  C ABI. SRS reprojection, OGR geometry loading, and streaming `processOne`
  behavior remain C++/GDAL.
- `pdal_filters_colorinterp_test`: `minmax`, `badramp`, `autorange`, `k`, and
  `mad` count. Color interpolation execution routes through the Rust C ABI.
  Missing-dimension validation and streamability checks remain C++ wrapper
  behavior.
- `pdal_filters_colorization_test`: `test1`, `test2`, `test3`, and `test5`
  count. Color sampling and point updates route through the Rust C ABI. Invalid
  dimension-name validation remains C++ layout behavior.
- `pdal_filters_hag_test`: `dem` and `dem_clamps` count for `filters.hag_dem`.
  The DEM sampling and HAG assignment route through the Rust C ABI.
- `pdal_filters_h3_test`: only `stream_test_2` counts; H3 indexing routes
  through the Rust C ABI. Stage creation remains C++ factory behavior.
- `pdal_filters_geomdistance_test`: only `test_polygon` counts; geometry
  distance calculation routes through the Rust C ABI.
- `pdal_filters_faceraster_test`: all 2 tests count; mesh rasterization and
  raster attachment writing route through the Rust C ABI. The C++ test still
  uses GDAL to read the output fixture for verification.
- `pdal_filters_overlay_test`: all 2 tests count; overlay point mutation
  routes through the Rust C ABI after C++/GDAL datasource setup.
- `pdal_filters_reprojection_test`: all 3 tests count; coordinate
  reprojection routes through the Rust C ABI.
- `pdal_filters_divider_test`: `partition_count`, `partition_capacity`,
  `round_robin_count`, `round_robin_capacity`, `break_on_expression`, and
  `break_on_userdata` count. View partitioning routes through the Rust C ABI.
  Option validation and C++ expression evaluation remain C++.
- `pdal_filters_sparsesurface_test`: only `lowest_is_ground_rest_low_noise`
  counts; classification assignment routes through the Rust C ABI. Factory
  registration and equal-class option validation remain C++.
- `pdal_filters_gpstimeconvert_test`: all 16 conversion tests count. GPS time
  conversion routes through the Rust C ABI in batch, in-place, and streaming
  paths.
- `pdal_filters_expression_test`: `singleDimension`, `multipleDimensions`,
  `onlyMin`, `onlyMax`, `negation`, `equals`, `negativeValues`,
  `simple_logic`, `issue_4920`, `extrachars`, `issue_1659`, `stream_logic`,
  `nan`, `nan2`, and `multipleExpressions` count. Expression parsing,
  evaluation, and streaming behavior route through the Rust C ABI. Stage
  factory smoke coverage remains C++ wrapper behavior.
- `pdal_filters_stats_test`: `handcalc`, `baseline`, `simple`, `advanced`,
  `dimset`, `metadata`, `enum`, `global`, and `counts` count. Summary
  computation routes through the Rust C ABI. Stream and merge-specific cases
  remain C++ wrapper behavior.
- `pdal_filters_sample_test`: `culls_close_points`, `keeps_distant_points`,
  `cell_mode`, `culls_across_voxels`, `radius_boundary`, and
  `repeated_execute_resets_voxels` count. Poisson/cell sampling routes through
  the Rust C ABI. Stage creation and dimension-flag helper behavior remain C++
  wrapper behavior.
- `pdal_metadata_test`: only `typed_value`, `test_float`, and `infnan` count.
  Scalar conversion and JSON scalar formatting route through Rust helpers. The
  metadata tree implementation is still C++.
- `pdal_io_las_reader_test` and `pdal_io_las_writer_test`: do not count as
  binaries yet. Rust LAS/LAZ exists, but the C++ reader/writer wrappers still
  have substantial legacy header, VLR, SRS, streaming, and option behavior.
- `pdal_io_text_reader_test`: `t1`, `t1a`, `t2`, `t3`, `badheader`, `s1`,
  `strip_whitespace_from_dimension_names`, `issue3859`, `issue1939`,
  `warnMissingHeader`, `overrideHeader`, `insertHeader`, and `quotedHeader`
  count. Text parsing and point production route through the Rust C ABI; C++
  warning plumbing remains wrapper behavior.
- `pdal_io_text_writer_test`: `t1`, `t2`, `t2stream`, `precision`, and
  `geojson` count. Text and GeoJSON output route through the Rust C ABI.
- `pdal_io_pts_reader_test`: `ReadPtsExtraDims`, `ReadPtsThreeDims`, and
  `ReadPtsFourDims` count. Constructor/factory registration remains C++.
- `pdal_io_ptx_reader_test`: `Basic`, `DiscardMissingPointsWithComplexTransform`,
  `MultipleClouds`, and `NoColor` count. PTX parsing and missing-point handling
  route through the Rust C ABI.
- `pdal_io_qfit_test`: `test_10_word` and `test_14_word` count. QFIT binary
  point decoding routes through the Rust C ABI.
- `pdal_io_obj_reader_test`: `NoFace`, `NoVertex`, `Read`,
  `FourDimensionRead`, `TexturesAndNormals`, and `LargeFile` count. OBJ point
  and mesh extraction route through the Rust C ABI.
- `pdal_io_gltf_writer_test`: all 4 tests count; GLTF mesh output routes
  through the Rust C ABI.
- `pdal_io_faux_test`: `test_constant_mode_sequential_iter`,
  `test_random_mode`, `test_ramp_mode_1`, `test_ramp_mode_2`,
  `test_return_number`, `one_point`, and `grid` count. Constant, ramp, and grid
  point generation route through the Rust C ABI; uniform/normal random
  generation and seed validation remain C++.

## Command-Ready Filters

Pipeline JSON can currently construct this command-ready filter subset:

- `approximatecoplanar`
- `chipper`
- `cluster`
- `dbscan`
- `decimation`
- `divider`
- `eigenvalues`
- `elm`
- `estimaterank`
- `faceraster`
- `gpstimeconvert`
- `groupby`
- `hag_nn`
- `head`
- `hexbin`
- `iqr`
- `label_duplicates`
- `locate`
- `lof`
- `mad`
- `merge`
- `mortonorder`
- `nndistance`
- `optimalneighborhood`
- `outlier`
- `planefit`
- `radialdensity`
- `randomize`
- `reciprocity`
- `reprojection`
- `returns`
- `sample`
- `separatescanline`
- `smrf`
- `sparsesurface`
- `sort`
- `splitter`
- `stats`
- `tail`
- `voxeldownsize`
- `voxelcenternearestneighbor`
- `voxelcentroidnearestneighbor`
- `zsmooth`

Other Rust filter modules may exist, but they are not command-ready until they
are deliberately added to the registry with option parsing and coverage.

## Remaining C++ Filter Families

These are not missed easy ports. Start each with an ABI, dependency, or
algorithm decision.

- GDAL/PROJ/SRS/OGR-backed: `DEM`, `ProjPipeline` reverse-mode and
  option-complete behavior.
- Private or specialized algorithms: `CS`, `Delaunay`, `Georeference`,
  `HagDelaunay`, `LiTree`, `LloydKMeans`, `M3C2`, `Miniball`, `Normal`, `PMF`,
  `Poisson`, `Straighten`, `Supervoxel`, `GreedyProjection`,
  `IterativeClosestPoint`, `RelaxationDartThrowing`.
- Pipeline/process/framework behavior: `Info`, `Shell`, `StreamCallback`.
- Expression/KD-tree hybrid behavior needing a design pass: `RadiusAssign`,
  `NeighborClassifier`, `CovarianceFeatures`.

The rejected broad sweep in commit `a1e67b5dc` is useful only as source
material. Its C++ wiring passed C++ object pointers across the C ABI and broke
existing filter tests. Do not reapply it wholesale.

## Useful Regression Commands

- Rust workspace:
  `cargo test --manifest-path rust/Cargo.toml --workspace`
- Local I/O installed-PDAL regressions:
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test text_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test pcd_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test pts_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test ptx_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test ilvis2_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test ply_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test ply_writer_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test obj_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test qfit_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test sbet_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test fbi_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test terrasolid_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test optech_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test las_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test smrmsg_regression -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test bpf_regression -- --ignored`
- Command installed-PDAL regressions:
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test pipeline_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test info_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test translate_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test merge_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test sort_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test split_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test random_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test hausdorff_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test chamfer_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test delta_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test ground_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test density_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test eval_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test tile_command -- --ignored`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test tindex_command -- --ignored`
- Performance and guardrail visibility:
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test perf_regression -- --ignored --nocapture`
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test binary_size -- --ignored --nocapture`
  `rust/scripts/measure_guardrails.sh`
- Rust mutation testing:
  `cargo install --locked cargo-mutants`
  `pixi run -e dev rust-mutants`
  `pixi run -e dev rust-mutants --package pdal-core`
