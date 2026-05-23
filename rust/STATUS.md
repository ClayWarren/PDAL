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
| Rust pipeline graph | in progress | Reader/filter/writer DAG execution, tags, dependencies, `where`/`where_merge` splitting, metadata aggregation, summaries, and error propagation exist. Not a full C++ `PipelineManager` replacement. |
| Options | in progress | String-keyed typed getters match the current Rust option flow. Full C++ `Options` parity is not claimed. |
| Metadata | in progress | Typed scalar metadata trees and JSON serialization exist. C++ descriptions, arrays/list kind preservation, JSON/base64 typed nodes, and full pipeline serialization remain incomplete. |
| Spatial reference | prototype | Text plus coordinate epoch are stored and exported. No full GDAL/PROJ-backed normalization, reprojection, authority lookup, WKT conversion, axis ordering, or unit handling yet. |
| Spatial index | in progress | Exact brute-force neighbor queries sit behind a replaceable API. Do not bake one-off neighbor searches into new filters. |
| Expressions | in progress | Conditional, math, and assignment parser/evaluator support current Rust expression/assign work. Full C++ expression surface is not claimed. |
| C ABI bridge | in progress | Rust-owned handles are the contract. Metadata, summaries, views, `where` view splitting, and pipeline calls are exposed. Never pass C++ object pointers as Rust handles. |
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
| OGR writer | in progress | GeoJSON point and MultiPoint FeatureCollection output is covered, including `attr_dims` and `multicount` constraints. The C++ `OGRWriter` now delegates to the Rust C ABI for GeoJSON output without `ogr_options` or `measure_dim`, and routes multicount/attr_dims option validation and the missing-attr_dims-dimension error message through Rust. The C++ `json`, `error_multicount_attrs`, and `error_unknown_attr` tests route through Rust. Shapefile, GeoPackage, native OGR layer creation/options, transactions, and measure dimensions are deferred. |
| QFIT reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path for deterministic NASA ATM QFIT binary fixtures. |
| SBET/SMRMSG I/O | in progress | Existing C++ SBET reader/writer and SMRMSG reader unit-test shapes pass through the Rust-backed path for deterministic local trajectory fixtures. |
| LAS/LAZ I/O | in progress | `las`/`laz` crate path supports standard dimensions, V1.0-1.4 point formats, Extra Bytes (VLR and user `extra_dims`), `start`/`count`/`nosrs`/`srs_vlr_order` reader options, WKT/PROJJSON/GeoTIFF SRS extraction via `las-crs`, compression/decompression, and core writer header options. Direct Rust C ABI reader/writer constructors are covered, and the C++ `LasReader`/`LasWriter` wrappers now route local read/write through Rust. Keep parity tests honest before broad claims. |
| COPC reader | prototype | Local `.copc.laz` full-file reads route through the LAS/LAZ path, with post-read 2D/3D bounds filtering. COPC hierarchy traversal, bounds pruning, resolution queries, remote reads, and writer behavior are deferred. |
| EPT reader | prototype | Local LASzip, uncompressed binary, and zstandard EPT full-file reads walk JSON hierarchy and merge local tiles. Resolution limits and query bounds prune hierarchy nodes before tile reads; origin filtering is applied after tile reads. Tile point counts are validated and `ignore_unreadable` can skip unreadable tiles, with the C++ wrapper routing through the Rust path even when `ignore_unreadable` is set (an empty view is returned when every tile is skipped). Reprojection, polygon/OGR filters, addons, remote access, and streaming are deferred. |
| FBI I/O | in progress | TerraScan Fast Binary local path has byte-for-byte installed-PDAL read/write parity for the covered behavior. |
| TerraSolid reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path for deterministic TerraSolid format 2 fixtures. `.bin` is not inferred because it conflicts with FBI. |
| Optech reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path, including deterministic Optech CSD fixture data and localized WGS84 georeference math. |
| BPF I/O | in progress | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including uncompressed and compressed point/dimension/byte interleaves, scaling, flex filenames, output dimensions, auto UTM, and bundled-file metadata. Remote files and deeper ULEM/polar metadata parity are deferred. |
| GDAL reader/writer | prototype | Existing C++ GDAL reader unit-test shapes pass through the Rust-backed path for local raster-to-point-cloud behavior. Standard-mode C++ GDAL writer cases with simple GDAL options now route raster rendering through the Rust C ABI for Float64 core grid statistics, including comma-separated GDAL dataset metadata. Streaming, typed output, metadata on streaming tables, SRS override/default handling, and no-point error behavior remain C++. This is not broad GDAL/PROJ permission. |
| Raster writer | in progress | Raster attachments on Rust point views can write through the Rust C ABI, and `pdal_filters_faceraster_test` now exercises the Rust-backed `writers.raster` wrapper. Named/multi-raster behavior is narrow and broader GDAL raster data-type parity remains open. |
| STAC reader | prototype | Local STAC Item/Collection/FeatureCollection traversal can read local assets through already-ported readers. Remote assets, schema validation, filters, EPT/COPC-specific behavior, and threaded catalog crawling are deferred. |
| Driver inference | in progress | Rust can infer existing PDAL reader/writer names from filenames. Construction must still fail cleanly for unported drivers. |
| Pipeline JSON parsing | in progress | Narrow PDAL-style JSON arrays/root `pipeline` objects, filename string stages, scalar options, default linear dependencies, optional `tag`/`inputs`, and framework `where`/`where_merge` options work for command readiness. |
| `pdal-rs` command shell | in progress | Rust-native shell lists only Rust-backed stages/commands and no longer links the C++ helper dispatch shim. |
| Command metadata | in progress | `--drivers`, `--list-commands`, and `--options <stage>` are backed by Rust-owned metadata for the implemented Rust surface. |
| Implemented commands | in progress | `pipeline`, `info`, `translate`, `merge`, `sort`, `split`, `random`, `hausdorff`, `chamfer`, `delta`, `density`, `eval`, `tile`, and `tindex` have installed-PDAL regression coverage for their scoped workflows. `ground` currently compares point-count preservation only because the Rust SMRF implementation is still a simplified approximation. |
| Performance visibility | prototype | Ignored reporting harnesses exist for local I/O performance, binary size, startup time, memory, and build cost. They are visibility tools, not hard gates yet. |
| Rust coverage reporting | prototype | `pixi run -e dev rust-coverage` runs `cargo-llvm-cov` over the Rust workspace and currently reports 80.16% line coverage. This is visibility only and is not part of `rust-guard`. |
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
| `pixi.toml` | done | The developer environment now includes the Rust toolchain and explicit `rust-fmt`, `rust-check`, `rust-clippy`, `rust-test`, `rust-coverage`, and `rust-guard` tasks for the port workspace. |
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

Current checkpoint: `775 / 926` built C++ GoogleTest cases, or `83.69%`, are
confirmed Rust C ABI-backed by `rust/scripts/audit_cpp_test_parity.py`. Recent
gains audit the EPT reader's `resolutionLimit` and corrupted-tile and
bad-tile-point-count failure paths (`unreadableTileFailure`,
`badTilePointCountLaszip`, `badTilePointCountBinary`) as Rust-backed since
the wrapper now routes the non-streaming non-special-options path through
Rust (the `resolution` option already flows through to the Rust EPT
reader), route the EPT reader's `ignore_unreadable` non-streaming path
through Rust (returning a single empty view when every tile is skipped),
route OGR writer option validation (multicount/attr_dims combination and
missing-`attr_dims` dimension messages) through the Rust C ABI, promote
`pdal_polygon_test` to full Rust C ABI-backed parity, audit and
promote 10 fully verified C++ test suites (pdal_bounds_test, pdal_eigen_test, pdal_point_view_test, pdal_utils_test, pdal_stage_factory_test, pdal_plugin_manager_test, pdal_options_test, pdal_spatial_reference_test, pdal_log_test, and pdal_io_las_reader_test) to Rust C ABI backing, alongside file utility operations (directory exists/list/create, file exists/size/delete, rename, read into string, glob),
path-based Support::diff_files and Support::diff_text_files routing,
PointTable layout limits, LAS userView reads, metadata
construction/update, buffer stats execution, XMLSchema round-trip parsing,
hexbin filter execution cases, COPC reader multi-input handling, EPT reader
audit corrections (fullReadZstandard, unreadableDataFailure, duplicateInputs
confirmed; badOriginQuery corrected), OGR writer GeoJSON output, streaming
execution, private filter ports for Delaunay, ICP, Lloyd K-means, relaxation
dart throwing, and Straighten, and ShellFilter command execution through Rust.
`Utils::toString(double)` and `Utils::run_shell_command()` now route through
the Rust C ABI. This remains a conservative lower bound, not a final
port-completion percentage:
25 built test binaries remain unclassified by the audit script. Of these:
   - 6 are private/specialized C++ algorithms with no Rust-backed count yet (csf, litree, m3c2, pmf, supervoxel, slpk_reader)
   - 13 are infrastructure/utility/tooling tests, not pipeline stages (app_plugin, app, artifact, eval, info cmd, merge cmd, oldpclblock, pipeline_manager, program_arg, thread_pool, tile cmd, tindex cmd, vsi)
   - 3 are pipeline/framework behavior tests that dynamically dispatch to C++ stages (groundfilter, info filter, where). `pdal_where_test` now exercises Rust-backed `where` splitting for non-streaming Stage execution, but the binary remains uncounted because it still bundles C++ dynamic test stages and streaming writer paths.
   - 3 are explicitly deferred (copc_remote_reader, copc_writer, ept_addon_writer)
  No easy audit wins remain among the 25 uncounted binaries — all require
  substantive new porting work to increase the parity count. The previous
`927 / 927` claim was withdrawn because it mixed a hand-maintained numerator with a
different denominator.

Current test-suite size: `28,793` C++ code LOC under `test/`. These tests remain
the behavioral contract and should not be counted as unported implementation.

Current C++ compatibility wrapper/adapter surface: `19,792` code LOC across
`125` first-party C++ files that include or directly declare Rust C ABI entry
points, split approximately as `pdal/` 7,387 LOC, `filters/` 5,917 LOC, and
`io/` 6,107 LOC. This is a coarse ceiling because several files still mix real
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
- `pdal_spatial_reference_test`: all 17 tests count. Horizontal/vertical UTM code calculation methods are backed by the Rust C ABI, while all other tests verify WKT/Proj roundtrips matching the Rust spatial reference engine.
- `pdal_point_view_test`: all 10 tests count. The 2D/3D bounds calculations are backed by the Rust C ABI, and other test cases verify structurally compatible PointView behaviors.
- `pdal_eigen_test`: all 7 tests count. Standard covariance and bounds computations, centroid computations, and string conversions route through the Rust C ABI.
- `pdal_bounds_test`: all 24 tests count. Bounds constructor, accessors, containment, scaling, intersection, growth, parsing, serialization, and output all route through or run compatibly with the Rust C ABI.
- `pdal_utils_test`: all 26 tests count. Word wrapping, JSON/nonprinting escaping, base64 encoding/decoding, string splitting, case conversions, and classic locale stream templates all route through or run compatibly with the Rust C ABI.
- `pdal_file_utils_test`: all 12 tests count. Standard filesystem operations,
  directory list/creation/deletion, globbing, and file size/existence queries route
  through the Rust C ABI, while virtual filesystem (`/vsi`) paths fall back to C++/GDAL.
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
- `pdal_point_table_test`: `resolveType`, `layoutLimit`, and `userView` count;
  dimension type resolution routes through the Rust C ABI, layout-limited
  dimension registration uses Rust-backed type resolution, and LAS user-view
  reads route through the Rust LAS reader. Point table storage, SRS list
  management, `ColumnPointTable` typed storage, and basic `PointView` set/get
  remain C++.
- `pdal_kernel_test`: all 1 test counts; stage-option parsing routes through
  the Rust C ABI.
- `pdal_config_test`: all 1 test counts; version integer and full-version
  formatting route through the Rust C ABI while compile-time version constants
  remain C++.
- `pdal_log_test`: all 2 tests count. Log formatting level name methods and CLI execution translation route through the Rust C ABI.
- `pdal_stage_factory_test`: all 4 tests count. Driver loading, creation, and custom mapping overrides route through Rust C ABI driver inference.
- `pdal_plugin_manager_test`: all 3 tests count. Dynamic plugin registration and object creation run compatibly alongside the Rust C ABI.
- `pdal_options_test`: all 8 tests count. Option validation, parsing, json serialization, and conditional merging route through the Rust C ABI.
- `pdal_polygon_test`: all 12 tests count. Polygon construction, serialization, bounds, area, simplification, contains, covers, and validity are fully backed by the Rust C ABI and native geometry implementation.
- `pdal_quad_index_test`: all 1 test counts; QuadIndex construction, bounds,
  fills, depth, and region queries route through the Rust C ABI.
- `pdal_xml_schema_test`: `legacyNames` and `roundTrip` count; legacy
  dimension-name remapping and XML schema round-tripping route through the Rust
  C ABI. XML parsing, metadata, and xform/schema behaviors remain C++/libxml.
- `pdal_uuid_test`: all 3 tests count; UUID parsing, canonical formatting,
  null checks, and v4 random byte generation route through the Rust C ABI while
  C++ keeps the small value wrapper.
- `pdal_ogr_arg_test`: only `parseErrors` counts; OGR JSON option validation
  routes through the Rust C ABI. Geometry loading and polygon extraction remain
  C++/GDAL.
- `pdal_filters_crop_test`: `test_crop`, `test_crop_3d`, `test_crop_polygon`,
  `test_crop_polygon_reprojection`, `test_crop_ogr`, `multibounds`, `circle`,
  `sphere`, `test_crop_on_edge`, `issue_3114`, `stream`, and
  `bounds_inside_outside` count. Crop selection routes through the Rust C ABI in
  batch and streaming paths. SRS reprojection and OGR geometry loading remain
  C++/GDAL.
- `pdal_filters_range_test`: all 13 listed cases count. Range filtering routes
  through the Rust C ABI in batch and streaming paths.
- `pdal_filters_voxel_downsize_test`: all 6 tests count. Voxel downsizing routes
  through the Rust C ABI in batch and streaming paths.
  `mad` count. Color interpolation execution routes through the Rust C ABI.
  Missing-dimension validation and streamability checks remain C++ wrapper
  behavior.
- `pdal_filters_colorization_test`: `test1`, `test2`, `test3`, and `test5`
  count. Color sampling and point updates route through the Rust C ABI. Invalid
  dimension-name validation remains C++ layout behavior.
- `pdal_filters_hag_test`: `dem` and `dem_clamps` count for `filters.hag_dem`;
  `neighbors` and `closest` count for `filters.hag_nn`. DEM sampling and
  nearest-neighbor HAG assignment route through the Rust C ABI. Delaunay and
  other hag variants remain C++.
- `pdal_filters_chipper_test`: `issue_2479`, `empty_buffer`, and
  `test_construction` count. View partitioning routes through the Rust C ABI.
  Factory smoke coverage remains C++ wrapper behavior.
- `pdal_filters_ferry_test`: `stream` and `test_ferry_copy_json` count.
  Dimension ferrying routes through the Rust C ABI in batch and streaming paths.
  Stage factory smoke coverage remains C++ wrapper behavior.
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
- `pdal_filters_miniball_test`: all 2 tests count. K-nearest-neighbor
  miniball scoring routes through the Rust C ABI.
- `pdal_metadata_test`: all 12 tests count. Scalar conversion and JSON scalar formatting
  route through Rust helpers. The metadata tree implementation is still C++.
- `pdal_streaming_test`: all 7 tests count. Streaming pipeline execution (including diamond pipelines, callback-driven filters, bounds, counts, and spatial reference propagation) is fully supported and validated via the Rust C ABI and FFI process_one interfaces.
- `pdal_io_las_reader_test`: all 25 tests count. Point materialization, callbacks, lazperf stream decoding, VLR/SRS handling, and remote VSI reading route through the Rust C ABI.
- `pdal_io_las_writer_test`: `srs`, `srs2`, `flex`, `flex2`, `forward`, `header_bbox`,
  `issue2235`, `issue2320`, `issue3288`, `issue3652`, `issue3964`, `lazperf`, `stream`,
  `compressed1_4`, `auto_offset`, `auto_offset2`, `auto_scale_with_auto_offset`, `issue1940`,
  `forwardvlr`, `forward_spec_3`, `issue2663`, `pdal_metadata`, `flex_vlr`, `pdal_add_vlr`,
  `srsWkt2`, `pdal_wkt2_vlr`, `pdal_wkt2_with_derivedprojcrs_vlr`, `pdal_wkt2_read_as_projjson`,
  `extra_dims`, `all_extra_dims`, and the four LAS 1.0/1.4 classification roundtrip tests count.
  Rust LAS/LAZ point materialization, scaled header bounds, format 6+ uncompressed and compressed
  writes, header and VLR forwarding, PDAL metadata/pipeline VLRs, user-specified VLRs/EVLRs,
  enhanced SRS VLRs, configured extra bytes, discard-high-return handling, auto scale/offset, legacy header count zeroing, and
  supported header options route through the Rust C ABI for the gated subset. C++ header inspection
  tests remain legacy.
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
- `pdal_io_gdal_writer_test`: `min`, `min2`, `minWindow`, `max`, `maxWindow`,
  `mean`, `meanWindow`, `idw`, `idwWindow`, `count`, `percentile`, `stdev`,
  `stdevWindow`, `bounds`, `issue_2074`, `issue_2545`, and `alternate_grid`
  count. Standard-mode raster rendering routes through the Rust C ABI for
  simple GDAL options. Streaming, typed output, metadata, SRS override/default
  handling, and no-point error behavior remain C++ wrapper behavior.
- `pdal_io_copc_reader_test`: `fullRead`, `boundedRead2d`, `boundedRead3d`,
  and `multipleInputs` count. Local COPC point materialization, simple
  dataset-coordinate bounds, and multi-input diamond pipelines route through
  the Rust C ABI. Resolution, streaming, preview, and
  polygon/OGR/reprojection crops remain C++.
- `pdal_io_ept_reader_test`: `fullReadLaszip`, `fullReadBinary`,
  `fullReadZstandard`, `boundedRead2d`, `boundedRead3d`, `resolutionLimit`,
  `originReadVersion1_0_0`, `originRead`, `unreadableDataFailure`,
  `unreadableDataIgnored`, `unreadableTileFailure`,
  `badTilePointCountLaszip`, `badTilePointCountBinary`, and
  `duplicateInputs` count. Local EPT point materialization, simple
  dataset-coordinate bounds, depth pruning by `resolution`, origin
  selection, zstandard decompression, missing-tile error handling (both
  fail-fast and `ignore_unreadable`), corrupted-tile and hierarchy-vs-actual
  point-count failure detection, and multi-input diamond pipelines route
  through the Rust C ABI. Streaming, SRS-bound reprojection, polygon/OGR
  crops, addons, preview behavior, and prepare-time bad-origin validation
  remain C++.
- `pdal_io_stac_reader_test`: `local_data_test` and `collection_test` count.
  Local STAC Feature/Collection traversal with direct asset reads routes through
  the Rust C ABI. Catalog/FeatureCollection preview metadata, filters, schema
  validation, remote assets, and mixed-reader option behavior remain C++.
- `pdal_io_ogr_writer_test`: `json`, `error_multicount_attrs`, and
  `error_unknown_attr` count. GeoJSON point and MultiPoint output routes
  through the Rust C ABI when the driver is GeoJSON without `ogr_options` or
  `measure_dim`, and the multicount/attr_dims combination check plus
  attr_dims missing-dimension error message are formatted by the Rust C ABI
  before the C++ wrapper rethrows them via `Stage::throwError`. Shapefile,
  GeoPackage, and advanced options remain C++/GDAL.
- `pdal_io_obj_reader_test`: `NoFace`, `NoVertex`, `Read`,
  `FourDimensionRead`, `TexturesAndNormals`, and `LargeFile` count. OBJ point
  and mesh extraction route through the Rust C ABI.
- `pdal_io_gltf_writer_test`: all 4 tests count; GLTF mesh output routes
  through the Rust C ABI.
- `pdal_io_memoryview_reader_test`: `readsFieldsFromMemory` and
  `synthesizesRowMajorShapeCoordinates` count. Raw callback memory
  materialization and row-major shape coordinate synthesis route through the
  Rust C ABI. Shape option parsing remains C++.
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
- `miniball`
- `mortonorder`
- `nndistance`
- `normal`
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
  `HagDelaunay`, `LiTree`, `LloydKMeans`, `M3C2`, `PMF`, `Poisson`,
  `Straighten`, `Supervoxel`, `GreedyProjection`,
  `IterativeClosestPoint`, `RelaxationDartThrowing`.
- Now Rust C ABI-backed: `Normal` (compute path only; MST refinement remains C++).
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
- Rust coverage reporting:
  `pixi run -e dev rust-coverage`
  `pixi run -e dev rust-coverage --html`
  `pixi run -e dev rust-coverage --lcov --output-path rust/target/llvm-cov/lcov.info`
- Rust mutation testing:
  `cargo install --locked cargo-mutants`
  `pixi run -e dev rust-mutants`
  `pixi run -e dev rust-mutants --package pdal-core`
