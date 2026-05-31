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
| Spatial reference | in progress | `SpatialReference::set` (non-WKT user input and WKT1 to WKT2 normalization), `prettyWkt`, `getWKT1`, `getProj4`, `getPROJJSON`, `equals` (semantic IsSame fallback), `identifyHorizontalEPSG`, `identifyVerticalEPSG`, `getUTMZone`, `getHorizontal`, `getVertical`, `getHorizontalUnits`, `getVerticalUnits`, `isGeographic`, `isGeocentric`, `isProjected`, `getAxisOrdering`, and `valid` route through a Rust GDAL/OSR adapter (`pdal_srs_user_input_to_wkt`, `pdal_srs_wkt_to_wkt1`, `pdal_srs_wkt_to_wkt2`, `pdal_srs_pretty_wkt`, `pdal_srs_wkt_to_proj4`, `pdal_srs_wkt_to_projjson`, `pdal_srs_is_same`, `pdal_srs_identify_horizontal_epsg`, `pdal_srs_identify_vertical_epsg`, `pdal_srs_get_utm_zone`, `pdal_srs_get_horizontal_wkt`, `pdal_srs_get_vertical_wkt`, `pdal_srs_get_horizontal_units`, `pdal_srs_get_vertical_units`, `pdal_srs_is_geographic`, `pdal_srs_is_geocentric`, `pdal_srs_is_projected`, `pdal_srs_axis_ordering`, `pdal_srs_valid`). `SrsTransform` default-axis, explicit-axis, point, and array transforms now call the Rust C ABI transform handle. Vertical extraction uses a Rust WKT bracket-matching parser because GDAL's C API has no `OGR_SRSNode` equivalent. `SrsTransform::get()` consumers and GeoTIFF VLR encoding are still C++ GDAL/OGR-backed. |
| Spatial index | in progress | Rust metrics use an `rstar`-backed nearest-neighbor path, while the C++ `KD*Index` facade intentionally remains on the existing nanoflann implementation until a persistent Rust index handle can replace it without rebuilding/converting the view on every query. Do not bake one-off neighbor searches into new filters. |
| Thread pool | in progress | `pdal::ThreadPool` now delegates scheduling, stop/restart, await, queue clearing, and resize behavior through the Rust C ABI while keeping the existing C++ facade. |
| Expressions | in progress | Conditional, math, and assignment parser/evaluator support current Rust expression/assign work. Full C++ expression surface is not claimed. |
| C ABI bridge | in progress | Rust-owned handles are the contract. Metadata, summaries, views, `where` view splitting, and pipeline calls are exposed. Never pass C++ object pointers as Rust handles. |
| C++ filter wrappers | in progress | Safe ports use explicit Rust view conversion. Existing C++ filter tests remain the parity gate. |
| Filter ports | in progress | 84 first-party filter/static stage files exist in C++; the portable filter implementation backlog is now at 0 port-candidate LOC. Remaining C++ under `filters/` is wrapper/compatibility surface plus the exported `filters/private/Point` OGR geometry adapter used by C++ option parsing (`filters.crop` centers and `filters.normal` viewpoint). Registry exposure is not the same as full pipeline parity. |
| Filter layout mutation | prototype | A narrow prepare/layout hook exists for registry-visible derived-dimension filters such as `NNDistance`, `RadialDensity`, `Eigenvalue*`, `ClusterID`, `HeightAboveGround`, `Coplanar`, `PlaneFit`, `Reciprocity`, and custom `filters.zsmooth` dimensions. More complex layout mutation remains open. |
| Pure/local I/O harness | in progress | `readers.faux` and `writers.null` support in-memory pipeline testing. |
| Text I/O | done | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, and installed-PDAL regression coverage exists for scoped workflows. |
| PCD I/O | in progress | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including ASCII, binary, binary-compressed, precision, streaming, and double-field coverage. Broader installed-PDAL parity coverage can still grow. |
| PTS/PTX readers | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path, including Leica ASCII fixture behavior and installed-PDAL regressions. |
| ILVIS2 reader | in progress | Existing C++ reader and metadata-sidecar unit-test shapes pass through the Rust-backed path for deterministic ASCII point and fixture-shaped XML metadata behavior. |
| PLY I/O | in progress | C++ reader/writer unit-test shapes pass through the Rust-backed path, including ASCII/binary reads, ASCII/binary writes, mesh faces, precision/dim typing, and `#` flex filenames. Broader installed-PDAL and uncommon PLY fixture coverage can still grow. |
| OBJ reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path, including deterministic Wavefront OBJ ASCII data, mesh faces, and VTN de-duplication. |
| GLTF writer | in progress | Existing C++ writer unit-test shapes pass through the Rust-backed path for deterministic local GLB output from mesh-backed views. |
| OGR writer | in progress | GeoJSON point and MultiPoint FeatureCollection output is covered, including `attr_dims`, `multicount`, and the GeoJSON `WRITE_BBOX`/`COORDINATE_PRECISION` creation options used by the C++ tests. Plain Shapefile and GeoPackage point output, attribute fields, Shapefile MultiPoint grouping, and Shapefile measured point output now go through a Rust native GDAL/OGR adapter. The C++ `OGRWriter` delegates those GeoJSON, Shapefile, and GeoPackage cases to the Rust C ABI and routes multicount/attr_dims option validation, missing-attr_dims-dimension errors, and the RFC7946 unsupported-SRS error through Rust. All pre-port `pdal_io_ogr_writer_test` cases route through Rust. Native OGR creation options beyond the covered subset and transactions are deferred. The `pdal-native` `VectorPointWriter` also supports polygon layers (`create_polygon`, `write_polygon`, `write_geometry_wkt`), and `filters.hexbin`/`kernels.density` use it for non-GeoJSON density/boundary output. |
| QFIT reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path for deterministic NASA ATM QFIT binary fixtures. |
| SBET/SMRMSG I/O | in progress | Existing C++ SBET reader/writer and SMRMSG reader unit-test shapes pass through the Rust-backed path for deterministic local trajectory fixtures. |
| LAS/LAZ I/O | in progress | `las`/`laz` crate path supports standard dimensions, V1.0-1.4 point formats, Extra Bytes (VLR and user `extra_dims`), `start`/`count`/`nosrs`/`srs_vlr_order` reader options, WKT/PROJJSON/GeoTIFF SRS extraction via `las-crs`, compression/decompression, full-file GDAL VSI URL reads, and core writer header options. Direct Rust C ABI reader/writer constructors are covered, and the C++ `LasReader`/`LasWriter` wrappers now route local read/write through Rust. Keep parity tests honest before broad claims. |
| COPC reader | in progress | Local `.copc.laz` full-file and streaming reads plus no-filter `inspect()` metadata route through the LAS/LAZ path, with post-read 2D/3D bounds, same-SRS polygon filtering, polygon reprojection for the covered EPSG:4326 crop, and GeoJSON OGR polygon crop support for the existing fixture shape. A first-party COPC hierarchy walker (`pdal-io::copc_hierarchy`) parses the COPC info VLR and walks hierarchy/sub-hierarchy pages over either local files or the `pdal-native::vsi::VsiFile` byte-range adapter, applying 2D/3D bounds and `resolution` pruning that matches the C++ `depthEnd = max(1, ceil(log2(spacing/resolution)) + 1)` math. Resolution-limited execution materializes only the kept LAZ chunks by reading the LAZ chunk table to map each hierarchy entry's file offset to a `(start_point_idx, count)` range and streaming the LAS reader past unwanted records (the `laz` crate's variable-chunk seek is buggy, so we deliberately stay on a sequential read). The C++ `CopcReader::inspect()` now routes bounds/resolution previews (no polygons/OGR) through the Rust `pdal_copc_preview` C ABI, which is what makes `pdal_io_copc_remote_reader_test.vsi` count. Addons, writer behavior, and broad OGR datasource coverage remain deferred. |
| EPT reader | prototype | Local LASzip, uncompressed binary, and zstandard EPT full-file reads walk JSON hierarchy and merge local tiles. Remote LASzip EPT JSON/hierarchy/tile reads work through GDAL VSI for the covered STAC mixed-reader workflow. Resolution limits and query bounds prune hierarchy nodes before tile reads; origin, same-SRS polygon, SRS-bound polygon reprojection, transformed 3D bounds filters, and GeoJSON OGR polygon crops are applied after tile reads. Tile point counts are validated and `ignore_unreadable` can skip unreadable tiles, with the C++ wrapper routing through the Rust path even when `ignore_unreadable` is set (an empty view is returned when every tile is skipped). Local binary EPT addon overlays are read through Rust for the existing addon round-trip checks, and the C++ `EptAddonWriter::writeOne` delegates its per-addon binary chunk writes, hierarchy JSON emission, and `ept-addon.json` metadata write to the Rust `pdal_ept_addon_write` C ABI. Remote binary/zstandard EPT and spatial-filter preview are deferred. |
| FBI I/O | in progress | TerraScan Fast Binary local path has byte-for-byte installed-PDAL read/write parity for the covered behavior. Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including reader header summary, dimension discovery, point reads, and basic writer round-trip coverage. |
| TerraSolid reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path for deterministic TerraSolid format 2 fixtures. `.bin` is not inferred because it conflicts with FBI. |
| Optech reader | in progress | Existing C++ reader unit-test shapes pass through the Rust-backed path, including deterministic Optech CSD fixture data and localized WGS84 georeference math. |
| BPF I/O | in progress | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including uncompressed and compressed point/dimension/byte interleaves, preview point count/SRS/header bounds/dimension labels, scaling, flex filenames, output dimensions, auto UTM, and bundled-file metadata. Remote files and deeper ULEM/polar metadata parity are deferred. |
| GDAL reader/writer | prototype | Existing C++ GDAL reader unit-test shapes pass through the Rust-backed path for local raster-to-point-cloud behavior. Standard-mode C++ GDAL writer cases with simple GDAL options now route raster rendering through the Rust C ABI for Float64 core grid statistics, including comma-separated GDAL dataset metadata. Streaming, typed output, metadata on streaming tables, SRS override/default handling, and no-point error behavior remain C++. This is not broad GDAL/PROJ permission. |
| Raster writer | in progress | Raster attachments on Rust point views can write through the Rust C ABI, and `pdal_filters_faceraster_test` now exercises the Rust-backed `writers.raster` wrapper. Named/multi-raster behavior is narrow and broader GDAL raster data-type parity remains open. |
| TIndex reader | prototype | Local GeoJSON tile-index reads can route through the Rust C ABI and the C++ wrapper uses that path for unfiltered `.json`/`.geojson` indexes when `t_srs` is explicitly empty. OGR datasources, SQL, attribute/spatial filters, reprojection, SRS-column handling, and reader-args remain on the C++ path. |
| STAC reader | prototype | Local STAC Item/Catalog/Collection/FeatureCollection traversal can read local and covered remote assets through already-ported readers. Preview supports item/catalog/date/bounds pruning, local structural validation for the schema-flag fixture shapes, and the GeoJSON OGR-boundary shape covered by existing C++ tests. Execution supports item/catalog/bounds/OGR/property filters, collection filtering, and reader-specific args for local supported STAC documents, including the existing mixed EPT/COPC catalog workflow. Mixed-reader merges in Rust normalize to a union layout before appending views. Remote ranged COPC reads now use GDAL VSI-backed byte sources for hierarchy, chunk-table, and ranged point materialization. Full remote JSON-schema resolution, broad remote traversal, and threaded catalog crawling are deferred. |
| Driver inference | in progress | Rust can infer existing PDAL reader/writer names from filenames. Construction must still fail cleanly for unported drivers. |
| Pipeline JSON parsing | in progress | Narrow PDAL-style JSON arrays/root `pipeline` objects, filename string stages, scalar options, default linear dependencies, optional `tag`/`inputs`, and framework `where`/`where_merge` options work for command readiness. |
| `pdal-rs` command shell | in progress | Rust-native shell lists Rust-backed stages/commands and owns the Rust command implementations. It is not yet the installed `pdal` executable. |
| Command metadata | in progress | `--drivers`, `--list-commands`, and `--options <stage>` are backed by Rust-owned metadata for the implemented Rust surface. |
| C++ `pdal` app shell | done | `apps/` is a single file (`apps/pdal.cpp`, ~345 LOC) and the port backlog audit reports it at **0 port-candidate LOC** -- it is a thin entry-point peer over the Rust C ABI with no portable implementation left. Every piece of behavior/data routes through Rust: version (`pdal_version_string`), driver listing (`pdal_stage_list_json` -> `pdal_rust_stage_list_json`), command listing (`pdal_kernel_list_json` -> `pdal_rust_kernel_list_json`), stage option metadata (`pdal_stage_options_text`/`_json`), kernel dispatch (`pdal_kernel_run`, with a Rust dispatch guard for every first-party command), the unknown-command message (`pdal_app_unknown_command_message`), and log line prefixes (`pdal_log_format_prefix`). What remains in C++ is the intentional entry-point glue PORTING.md designates as the thin executable peer: `ProgramArgs` CLI parsing, terminal-table layout for `--drivers`/`--list-commands` (formatted from Rust JSON), log-sink selection (`Log::makeLog`), and the debug `SIGSEGV` backtrace handler. All 4 `pdal_app_test` cases (`option_file`, `load`, `log`, `listCommands`) pass through Rust, and a built-binary smoke confirms `--version`/`--drivers` (124 stages)/`--list-commands` (16 command entries, including `kernels.fauxplugin`)/dispatch/unknown-command parity. Reopen only if a command's deeper output shape is found to diverge from installed PDAL. |
| Implemented commands | in progress | All 15 first-party C++ kernel commands (`chamfer`, `delta`, `density`, `eval`, `ground`, `hausdorff`, `info`, `merge`, `pipeline`, `random`, `sort`, `split`, `tile`, `tindex`, `translate`) are Rust-dispatchable through the C ABI and listed in Rust command metadata. They have installed-PDAL regression coverage for scoped workflows. `info` owns summary, metadata, stdin pipeline JSON, point lookup including lists/ranges, 2D/3D nearest query, stats with `--dimensions`/`--enumerate`/`--breakout`, schema, boundary via Rust `filters.hexbin`, all-mode schema/stat/metadata/boundary/STAC output, pipeline serialization, STAC `pc_type`, the existing STAC app guard, and remote LAS header/VLR/EVLR extraction through the Rust C ABI `pointless_las` helper. Full STAC feature geometry/projection parity remains limited. `tile` owns the existing app tests, including globbed input, text/LAS output, per-source reprojection to `out_srs`, and writer text options. `tindex` owns the existing local GeoJSON create + bounds/polygon-filtered merge workflow with `filters.crop`, merge-time reprojection to `--t_srs`, stdin-fed create workflow, filelist create workflow, input-source conflict guard, invalid forwarded-filter diagnostic, GeoJSON stdout layer-description option, `--t_srs` layer SRS, `--a_srs` assignment/override behavior, fast bbox boundaries, SRS mismatch warning/skip behavior, and exact hexer-driven boundary generation for `--threshold`/`--resolution`/`--simplify` with optional `--where` point-expression filtering. GEOS topology-preserving simplification is applied through `pdal-native`. `density` now routes file inputs, JSON pipeline files, and stdin JSON pipelines through the Rust kernel by appending a Rust-backed `filters.hexbin` stage; XML pipeline input remains on the C++ `PipelineManager` fallback because the Rust parser does not own XML. `ground` compares per-point classification against installed PDAL (>=99.8% agreement on `interesting.las` with `cell=10`) after the Rust SMRF implementation gained the low-outlier mask, net cutting, KD-tree inpainting, and full validation. The Rust runner now owns the full `GroundKernel` option surface (max_window_size/slope/cell_size/scalar/threshold/cut/returns/ignore mapped onto `filters.smrf`, `--reset` -> `filters.assign`, `--denoise` -> `filters.outlier`, `--extract` -> `filters.range`, accept-and-ignore max_distance/initial_distance and the basic switches) and never returns the -1 C++ fallback sentinel, so the behavior runs through Rust. The exported C++ `ChamferKernel`, `DeltaKernel`, `EvalKernel`, `GroundKernel`, `HausdorffKernel`, `InfoKernel`, `MergeKernel`, `PipelineKernel`, `RandomKernel`, `SortKernel`, `SplitKernel`, `TIndexKernel`, `TileKernel`, and `TranslateKernel` classes are **retained** as API/ABI shells; direct `execute()` paths for those shells now dispatch through `pdal_rust_kernel_run` where their lower-layer behavior is Rust-backed, matching the app-path Rust behavior without deleting public symbols. `pdal delta` now supports the direct C++ shell's `--detail` and `--alldims` options through `pdal_delta_ex`. The direct `PipelineKernel` shell now routes normal execute, metadata, progress-file events, serialization, and validate JSON output through Rust; hidden PointCloudSchema behavior and true streamability enforcement remain explicit gaps. The direct `TranslateKernel` shell now routes normal reader/filter/writer execution, JSON filter-extraction (`--json`), metadata output, and pipeline serialization through Rust; serialized pipeline byte shape is Rust JSON rather than exact C++ `PipelineWriter` output. The direct `TIndexKernel` shell now routes parsed create/merge execution through Rust; broad OGR datasource/update parity and non-GeoJSON tile-index workflows remain deferred to the Rust tindex runner's documented scope. `pdal ground` smoke-tested across basic/extract/reset/denoise/combined/ignore/smrf-passthrough and the direct exported class path is now Rust-dispatched too. `tools.lasdump` and `tools.nitfwrap` have Rust command paths for their scoped fixture-backed workflows. |
| Performance visibility | prototype | Ignored reporting harnesses exist for local I/O performance, binary size, startup time, memory, build cost, and opt-in full C++ vs Rust test-suite timing. They are visibility tools, not hard gates yet. |
| Rust coverage reporting | done | `pixi run -e dev rust-coverage` runs `cargo-llvm-cov` over the Rust workspace. The line-coverage threshold is enforced by `rust-coverage-check` inside `rust-guard`; keep the percentage in `pixi.toml` synced with the latest measured coverage. |
| Rust mutation testing | prototype | `pixi run -e dev rust-mutants` runs `cargo-mutants` when it is installed locally. This is an audit tool for mature buckets, not part of `rust-guard`. |
| Unsafe Rust footprint | in progress | Current first-party Rust count, excluding `rust/target`, is 248 `unsafe { ... }` blocks, 412 `unsafe extern "C" fn` exports, 35 non-extern `unsafe fn` helpers, two unsafe extern callback type aliases, no unsafe extern blocks, and one `unsafe impl`. Unsafe remains concentrated in `pdal-capi`, `pdal-native`, and Rust callers of the C ABI; keep new unsafe at C/native boundaries or tests that exercise those boundaries. |
| Vendor/native strategy | in progress | `vendor/` has 11 top-level third-party dependency directories. `rust/VENDOR.md` is the source of truth. Two are actively replaced in Rust today (`vendor/h3` -> `h3o`, `vendor/lazperf` -> `las`/`laz`), four have a clear no-direct-port stance (`eigen`, `gtest`, `nanoflann`, `nlohmann`), and five remain deferred or adapter-bound (`arbiter`, `kazhdan`, `lepcc`, `schema-validator`, `utfcpp`). Native GDAL/OGR/GEOS/PROJ/Nitro adapters belong in `pdal-native`; pure Rust replacements such as LAS/LAZ do not need to move through it. |
| Plugins | prototype | There are 18 top-level plugin directories. Track each plugin below. `pdal-plugins` holds discovery metadata, `kernels.fauxplugin` is a compatibility marker, and `readers.spz`/`writers.spz` are the first fixture-backed plugin reader/writer checkpoint. A Rust plugin SDK and broad optional plugin sweep are still not ready. |
| Remote/object-store I/O | in progress | `pdal-native::vsi::VsiFile` opens local, URL, and `/vsicurl/` paths through GDAL VSI and now implements `std::io::Read + Seek` so byte-range readers can stream over it. The Rust COPC hierarchy walker consumes the adapter end-to-end: `pdal_io_copc_remote_reader_test.vsi` (autzen-classified.copc.laz over both https and `/vsicurl/`) now counts as Rust C ABI-backed. STAC remote JSON traversal, remote LASzip EPT reads, and `pdal info` remote pointless-LAS header/VLR/EVLR extraction consume the same adapter. Broader object-store option parity remains open. |
| Broad kernels/apps/tools migration | in progress | Simple `pdal-rs` commands may continue proving lower layers. **`apps/` is complete** (0 port-candidate LOC -- `apps/pdal.cpp` is a thin entry-point peer; see the `C++ \`pdal\` app shell` row). The standalone tools have C ABI-backed dispatch shells, and broad `kernels/` command parity still depends on lower-layer kernel coverage. The C++ `pdal pipeline`, `pdal translate`, `pdal random`, `pdal density`, `pdal ground`, `pdal split`, `pdal sort`, `pdal merge`, `pdal delta`, `pdal tindex`, and simple `pdal tile` app paths now execute through Rust for local reader/filter/writer workflows. `pdal density` also owns JSON pipeline and stdin JSON pipeline inputs through Rust; XML remains the explicit C++ fallback. Direct exported C++ `DeltaKernel`, `EvalKernel`, `GroundKernel`, `MergeKernel`, `PipelineKernel`, `RandomKernel`, `SortKernel`, `SplitKernel`, `TIndexKernel`, and `TileKernel` execution now reaches the same Rust C ABI runner, reducing the remaining kernel implementation backlog while preserving the public C++ classes. `pdal info` supports stdin pipeline JSON through Rust. `pdal translate` supports `filters.range` option files for the existing app guard. Standalone `lasdump` and `nitfwrap` dispatch through the Rust C ABI; `lasdump` covers LAS/LAZ header, VLR/EVLR, and point checksum output, and `nitfwrap` uses the Nitro native adapter for LIDARA DES wrap/unwrap with LAS/BPF fixture parity. |

## Root-Level Migration Status

The Rust port is not complete just because Rust-backed tests pass. The root
build, install, packaging, CI, examples, and docs must also describe and verify
the Rust-backed shape of PDAL.

| Area | Status | Notes |
|---|---|---|
| Root CMake | done | `libpdal_capi.a` is built, linked into `pdalcpp`, and sourced from `cmake/rust.cmake` so the dependency list tracks every current Rust crate that can affect the C ABI or linked implementation. |
| `cmake/` modules | in progress | Rust build options now live in `cmake/rust.cmake`; source packaging excludes generated Rust build output. Platform link details and test wiring still need to move here as the integration matures. |
| `pixi.toml` | done | The developer environment now includes the Rust toolchain and explicit `rust-fmt`, `rust-check`, `rust-clippy`, `rust-test`, `rust-coverage`, and `rust-guard` tasks for the port workspace. |
| GitHub workflows | in progress | The Pixi workflow runs the Rust workspace guard through `pixi run -e dev rust-guard`. Linux, macOS, Windows, conda, and release workflows still need explicit Rust/C++ parity gates before the port is upstreamable. |
| `PDALConfig.cmake.in` | in progress | Downstream `find_package(PDAL)` keeps the C++ target as the primary link surface while exposing `PDAL_CAPI_INCLUDE_DIRS` for the stable C ABI header. A dedicated installed C ABI target/library export is still deferred until the archive/shared-library shape is finalized. |
| `pdal_features.hpp.in` | done | Decision: no Rust-backed-build feature macro is added. The Rust C ABI is an unconditional, mandatory part of the build -- `cargo` is required (fatal if missing), `pdal_rust_capi`/`libpdal_capi.a` is always a dependency of `pdalcpp`, and there is no `option()` to disable it -- so a `PDAL_HAVE_RUST`-style macro would always be defined and provide no supported conditional to branch on (and the guidance is to avoid preprocessor branching). The one Rust-related compile define, `PDAL_UTILS_NO_RUST_CAPI`, is a private switch for the standalone `dimbuilder` generator only (see `dimbuilder/`), not a feature-availability macro for the installed library. Revisit only if a future supported build mode actually makes the Rust C ABI optional. |
| `dimbuilder/` | done | Intentional generator-tool exception. `dimbuilder` is a standalone code generator (`Dimension.json` -> `Dimension.hpp`) that, by existing PDAL design, compiles `Utils.cpp` directly into the executable rather than linking `pdalcpp` (so Linux packagers who disable rpath can still run it during the build). It therefore builds with `PDAL_UTILS_NO_RUST_CAPI`, which selects the pure-C++ fallback for each Rust-C-ABI-backed `Utils` function. Verified: a clean standalone build compiles `Utils.cpp` + `DimBuilder.cpp` with no Rust capi linked, and the generated `Dimension.hpp` is byte-identical to the build-tree header (modulo the input-path comment). The tool only uses `Utils::split`/`toupper`/`trim`/`wordWrap`, all of which have correct C++ fallbacks under the guard. This is the accepted permanent shape, not a half-port; the guarded C++ fallbacks must stay behavior-equal to the Rust path. |
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
| `plugins/nitf` | in progress | `tools.nitfwrap` has a Nitro-backed native adapter for byte-preserving LAS/BPF wrap and unwrap workflows. `readers.nitf` and `writers.nitf` Rust stages run behind the C ABI: the reader uses `pdal_nitf_lidar_segment` plus a shifted `LasReader` (via `start_offset`) for the embedded LAS payload and exposes NITF header/TRE metadata through `pdal_nitf_read_metadata`; the writer plumbs `ftitle`/`fsclas`/`oname`/`ophone`/`idatim`/`iid2`/`aimidb`/`acftb`/security through `pdal_nitf_write`, defers LAS payload generation to `LasWriter` (writing to a temp file that gets wrapped), and supports `#` multi-view filename templating. The C++ plugin wrappers in `plugins/nitf/io/NitfReader.cpp` and `NitfWriter.cpp` are now thin shims over those C ABI entries; `pdal_io_nitf_reader_test` and `pdal_io_nitf_writer_test` pass through Rust. The legacy in-tree `NitfFileReader`/`NitfFileWriter` C++ classes still exist as compile-time peers for option storage (`m_nitf.m_fileTitle`, etc.) but no longer do the wrap/unwrap themselves. |
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
| `vendor/arbiter` | Adapter-bound | C++ `io/private/connector` remains an Arbiter-backed native adapter for remote/local compatibility while Rust remote paths use the GDAL VSI adapter. Do not broaden Arbiter work until a concrete I/O parity case needs it. |
| `vendor/eigen` | No direct port | Use Rust linear algebra where practical; do not port Eigen itself. Current covered math uses local small-matrix routines. |
| `vendor/gtest` | No Rust role | Keep for C++ parity tests. Rust uses Cargo tests. |
| `vendor/h3` | Replaced in Rust | Rust-backed H3 work uses the `h3o` crate. Do not bind vendored C H3 unless parity requires behavior `h3o` cannot provide. |
| `vendor/kazhdan` | Deferred | Decide per Poisson/reconstruction work; likely private algorithm port, FFI, or leave C++ depending on tests. |
| `vendor/lazperf` | Replaced in Rust | Current Rust LAS/LAZ path uses the `las` crate with its `laz` feature. Keep lazperf available for C++ compatibility. |
| `vendor/lepcc` | Adapter-bound | The ESRI/I3S/SLPK reader family (`io/EsriReader*`, `io/I3SReader*`, `io/SlpkReader*`, `io/private/esri/*`) remains a LEPCC-backed native adapter bucket. Port, FFI, or leave C++ only when ESRI reader parity is the active milestone. |
| `vendor/nanoflann` | No direct port | Use the Rust spatial-index API rather than porting nanoflann; internals can be swapped later. |
| `vendor/nlohmann` | No Rust role | C++ JSON dependency; Rust uses `serde_json`. |
| `vendor/schema-validator` | Deferred | Defer until schema validation parity needs it. |
| `vendor/utfcpp` | Deferred | C++ Unicode helper dependency; use Rust string/UTF-8 APIs unless a concrete parity gap appears. |

## Tools Status

| Tool | Status | Notes |
|---|---|---|
| `tools/lasdump` | done | Standalone `lasdump` is a thin C++ launcher over the Rust C ABI. Rust covers LAS/LAZ header, VLR/EVLR, and point checksum output; command tests and a LAZ smoke pass. |
| `tools/nitfwrap` | done | Standalone `nitfwrap` is a thin C++ launcher over the Rust C ABI. Rust wraps and unwraps LAS/BPF through Nitro, preserves embedded bytes, unwraps the existing NITF fixture, and passes the existing `nitfwrap_test`. Full NITF reader/writer stage parity is tracked under `plugins/nitf` and I/O. |

## C++ Test Parity Accounting

The first target was the pre-existing C++ test suite running against Rust
implementations through the C ABI and C++ wrappers. Rust linkage alone does not
count.

**This milestone is complete.** `820 / 820` baseline C++ GoogleTest cases
(`100.00%`) are confirmed Rust C ABI-backed by
`rust/scripts/audit_cpp_test_parity.py`. The audit defaults to the test set
from `3df1668e0^`, before both the local C++ guard-test additions and the Rust
port, so newly added guard tests do not move the headline denominator. Keep this
audit green as a regression gate, but it no longer measures remaining work.

This proves the current C++ compatibility layer can satisfy the pre-port
behavioral contract; it is not the finish line. The active goal is now the
**implementation-replacement backlog** below: reduce the real implementation
still living in C++ to glue/wrappers plus documented holdouts, broaden
first-party I/O/filter/core replacement, and make
install/export/CI/regression/performance evidence strong enough for an
upstreamable port.

## C++ Implementation-Replacement Backlog

With test parity at 100%, the next-goal metric is how much first-party C++ is
*real implementation* still to be ported, versus glue/wrappers and documented
holdouts. Measure it with:

```sh
python3 rust/scripts/audit_cpp_port_backlog.py
python3 rust/scripts/audit_cpp_port_backlog.py --area io --top 40
python3 rust/scripts/audit_cpp_port_backlog.py --show holdout
```

The script classifies each mainline C++ file (`pdal/`, `filters/`, `io/`,
`kernels/`, `apps/`, `tools/`) by `cloc` code LOC into:

- `c-abi-backed`: includes the Rust C ABI header (or is a known Rust bridge
  header). A header inherits its sibling `.cpp`'s category, so interface headers
  over a Rust-backed implementation are not counted as backlog.
- `native-adapter`: C++ bindings over an external native library, platform API,
  or remote/vendor adapter path whose Rust home is `pdal-native`/FFI or a later
  remote I/O adapter (e.g. `pdal/private/gdal/`, GDAL VSI, GDAL grid/density OGR
  adapters, lazperf compatibility, Arbiter-backed STAC traversal, dynamic
  library loading, and backtrace adapters), not a from-scratch port.
- `holdout`: a documented intentional C++ holdout (libgeotiff GeoKey encoding,
  the `StreamCallback` callback ABI, and the `ProgramArgs` compatibility glue).
  Keep this list small and cited in the script.
- `port-candidate`: pure C++ with no C ABI reference — the actionable backlog.

Current snapshot (mainline, excluding `test/`, `vendor/`, and deferred
`plugins/`):

| category | LOC | files |
|---|---:|---:|
| port-candidate | 5,054 | 42 |
| c-abi-backed | 41,943 | 374 |
| native-adapter | 5,529 | 52 |
| holdout | 5,799 | 53 |
| total | 58,325 | 521 |

Port-candidate backlog by area: `io` 3,604 · `pdal` 1,450. `filters`,
`kernels`, `apps` and `tools` are now at 0 (apps is a thin entry-point
peer; the only `tools` entry the audit had been counting was the in-tree
GoogleTest `tools/nitfwrap/NitfWrapTest.cpp`, which is behavioral contract, not
implementation — the audit now excludes files including `pdal_test_main.hpp`).
(The latest kernel sweeps moved direct C++ `DeltaKernel`/`EvalKernel`/
`GroundKernel`/`MergeKernel`/`PipelineKernel`/`RandomKernel`/`SortKernel`/
`SplitKernel`/`TIndexKernel`/`TileKernel`/`TranslateKernel`, and supported
`InfoKernel` modes execute through
`pdal_rust_kernel_run`, reducing kernel port-candidate backlog while retaining
the exported classes as compatibility shells.)
(A dead-code sweep removed the orphaned C++
delaunator, CSF, miniball, straighten, mongoexpression, and DisjointSet
implementations once their filters routed through Rust, plus the genuinely-dead
non-exported `io/PcdHeader.cpp`, `io/FbiHeader.cpp` dumper, and the non-exported
BPF header (de)serialization helpers. A later sweep removed the private C++
hexer grid/density OGR subsystem after `filters.hexbin` standard and streaming
paths both routed through the Rust hexbin stage and `kernels.tindex` preserved
the historic exact-boundary numeric behavior. The last `filters/` remainder,
`filters/private/Point`, is intentionally counted as a native adapter because
it is an exported OGR-backed compatibility helper for C++ option parsing, not
portable filter algorithm logic. Platform backtrace/endian helpers are likewise
counted as native adapters, not domain-port backlog. Public C++ compatibility
headers for export macros, plugin registration, deprecated JSON forwarding,
and the intentionally-disabled `PointContainer` include are holdouts. The
empty `io/OptechRotationMatrix.hpp` compatibility include and the PCL
`io/point_types.hpp` header are also tracked as compatibility holdouts, not
implementation to port. Small
format helper headers that only serve Rust-backed C++ reader/writer wrappers
(`PcdHeader`, `FbiHeader`, `SbetCommon`, `OptechCommon`, `HeaderVal`) count
with their Rust-backed owners rather than as standalone portable
implementation. The compression base and gzip headers count with the
Rust-backed deflate implementation: `DeflateCompression.cpp` owns the exported
`GzipDecompressor` methods, routes zlib-format deflate through Rust, and keeps
only the gzip auto-detect zlib fallback in C++. Public C++ typedef, exception,
log-level, `std::istream`/`std::ostream`, endian extractor, endian inserter, and
null ostream headers are compatibility holdouts: Rust owns equivalent domain
logic where needed, but these exported C++ APIs must remain until the C++ SDK
itself goes away. The private STAC C++ traversal files are counted as native/
remote adapter work: local supported STAC paths route through Rust, while those
files preserve Arbiter/remote/schema fallback behavior for the later remote I/O
milestone. The COPC writer private `Common.hpp` option/header shell counts with
the Rust-backed `CopcWriter` wrapper rather than as standalone algorithmic
backlog. Exported deprecated `LasHeader`/`LasVLR` and exported `BpfHeader`
classes are compatibility holdouts; their private parser/data helpers remain
backlog unless they route through Rust or are proved dead. Obvious exported C++
SDK shells under `pdal/` (point/table handles, reader/writer bases, DB bases,
dimension/mesh/quick-info/artifact helpers, subcommand shells, and
`Utils::Random` with its `std::mt19937&` API) are likewise compatibility
holdouts; the remaining `pdal/` backlog is now concentrated in
`PipelineManager`, JSON pipeline parsing support, and raster data ownership.
Rerun the audit after each change.
**API-parity note:** removing dead code is
only legitimate when nothing references it AND it is not part of the exported
public API. Under `-fvisibility=hidden`, `PDAL_EXPORT` *is* the public ABI, so
exported classes/methods must be preserved (hollowed to delegate to Rust, not
deleted) until the whole port is done — that is what makes this a port and not a
rewrite. On this basis the exported symbols that earlier deletions had removed
were RESTORED: `kernels/GroundKernel`, the metric/merge/sort/split/random kernel
classes (`Chamfer/Hausdorff/Delta/Eval/Merge/Sort/Split/Random`), the exported
`BpfHeader::read`/`readDimensions`, and `PDALUtils::compute{Chamfer,Hausdorff,
HausdorffPair}` — all kept as exported API while behavior runs through the Rust
runners. The retired copcwriter subsystem files carried no `PDAL_EXPORT` symbols
(internal `io/private`), and the Pcd/Fbi header removals dropped only
non-exported code, so those stay removed. A branch-vs-master sweep found no
other exported file net-deleted and no header that net-lost exported symbols.

**Exported-symbol audit (nm vs released 2.10.1).** Diffed
`nm -gjU build/lib/libpdalcpp.20.1.0.dylib` against Homebrew's `pdal` 2.10.1
(identical upstream version, zero version skew). 167 `pdal::` symbols are in the
released library but not the branch — and on triage every one is a
`pdal::Class::method` whose class still exists (e.g. `BpfReader::readByteMajor`,
`GltfWriter::writeBinHeader`, `GpsTimeConvert::*`, `Ilvis2MetadataReader::*`),
i.e. internal implementation methods removed as those stages' implementations
moved to Rust. No public class lost its vtable/typeinfo (only `PlyReader`'s
nested `ListProperty`/`SimpleProperty` impl structs), and zero free functions,
operators, or core-namespace API (`Utils`/`FileUtils`/`Stage`/`PointView`/
`Options`/`Metadata`/`gdal`/…) were removed. So the public class/method/free-
function surface is intact; the only exported-symbol delta is implementation
internals, which is the expected footprint of a behind-the-C-ABI port. Re-run
after future delegation work: `nm -gjU <lib> | c++filt | sort -u` on both and
`comm -23`, then confirm any `pdal::` diff is an internal method of a
still-present class, never a vtable/typeinfo/ctor or a public method.) This is a heuristic ceiling, not a precise
backlog: header-only files that still hold C++ data structures count even when
some of their behavior already routes through Rust, and the holdout list is
deliberately conservative. Drive the number down by porting the ranked
`port-candidate` files (or by reclassifying one with a cited holdout/backed
entry when that is the honest answer).

The branch-wide visibility metric, including guard tests added before and during
the port, is `958 / 1056` currently built C++ GoogleTest cases, or `90.72%`;
compute that with `--include-added-tests`. That number is informational because
many added guard tests intentionally still exercise C++ compatibility seams or
non-port holdouts; do not use it as the headline parity denominator.

When the NITF plugin is built (`-DBUILD_PLUGIN_NITF=ON`), `pdal_io_nitf_reader_test`
and `pdal_io_nitf_writer_test` (6 tests total) route through the Rust C ABI
via `pdal_nitf_lidar_segment`, `pdal_nitf_read_metadata`, and `pdal_nitf_write`.
The pre-port baseline above does not include them because the baseline build
did not enable the NITF plugin; rerun the audit with `BUILD_PLUGIN_NITF=ON`
and `--include-added-tests` to see them counted when that plugin is present.

Recent gains route `SpatialReference` user-input normalization, PROJ4 export,
semantic `IsSame` equality, horizontal/vertical EPSG helpers, Polygon
WKT/GeoJSON parsing/output, root-array pipeline execution, large point-view
storage, point row add/mutate/swap behavior, checked typed writes, typed
point-view reads, default spatial-reference behavior, basic point storage,
Chamfer/Hausdorff app metric tests,
option JSON canonicalization, EPT preview and selected non-streaming EPT paths,
OGR writer option validation, file utilities, Support diff helpers, PointTable
layout limits, LAS userView reads, metadata construction/update, buffer stats
execution, XMLSchema round-trip parsing, selected COPC/EPT/OGR/streaming
execution paths, COPC writer scale/offset, UTM SRS VLR, and extra-dimension
readback through the Rust LAS/LAZ compatibility writer path, private filter ports,
`pdal::ThreadPool` behavior,
ShellFilter command execution, `Utils::toString(double)`, `kernels.fauxplugin`,
installed-app `sort`/`merge`/simple `tile`/metric commands/`info --summary`/`tindex`, `Utils::run_shell_command()`,
`filters.pmf`, `filters.litree`, `filters.m3c2`, `filters.info` point
selection/query/bounds/schema reporting, the `filters.csf` construction,
empty-input, and option-validation guard paths, writer-side `where`, and
SpatialReference bbox transform checks, artifact-manager storage semantics,
ProgramArgs parser behavior, local SLPK package summary behavior, local VSI
tell/seek stream behavior, and the EPT addon writer input invariant through the
Rust C ABI.
This remains a conservative lower bound, not a final port-completion
percentage:
`writers.copc` is now a real first-party Rust COPC writer
(`pdal-io::copcwriter`), not the old plain-LAZ delegation. It ports the C++
`io/private/copcwriter/` subsystem: `VoxelKey`/`Grid` octree sizing,
`OctantInfo`/`CellManager` point storage, `VoxelInfo` node state, the
`Processor` occupancy-grid subsampling, the bottom-up `Pyramid` build driver,
per-node LAZ variable-chunk encoding via the `laz` crate (`chunk_writer`), the
hierarchy EVLR page emission (`hierarchy`), and the file assembly (`output`)
producing a `copc` info VLR (record 1) + laszip VLR + SRS/eb VLRs + per-node
LAZ chunks + hierarchy EVLR. `pdal_writer_create_copc` drives it from a
`PointView` through `copcwriter::writer::CopcWriter`. `pdal_io_copc_writer_test`
(`scaling`, `srsUTM`, `srsWkt2`, `extradim`) passes against it, and the output
round-trips through the Rust COPC reader and `las::Reader`. Parity note: the
C++ `sample()` shuffles with `std::mt19937` before selecting one point per
occupancy cell, so exact per-node membership is not byte-reproducible in Rust;
the algorithm and observable COPC contract (all points retained, valid octree,
resolution/bounds queryable) are preserved. Faithful bottom-up pyramid
threading and deeper hierarchy sub-paging beyond the covered shapes can still
grow. The C++ `writers.copc` *stage* (`io/CopcWriter.cpp`) now delegates the
whole write to this Rust writer through the C ABI (the same pattern
`LasWriter` uses): it parses options, resolves the forward list / SRS, and
collects forwarded/user/pipeline/metadata VLRs, then converts the view via
`rust_view_converter::toRust` and passes scale/offset/ids/SRS/extra_dims plus
its VLRs (encoded as `user_vlr_*` options) to `pdal_writer_create_copc`. The
Rust writer gained the matching stage options: `system_id`, `software_id`,
`project_id` (→ LAS GUID) and ingestion of user/forward/PDAL metadata+pipeline
VLRs. With this, the entire ~1,800 LOC C++ octree/sampling/output subsystem in
`io/private/copcwriter/` (BuPyramid, Grid, CellManager, Processor,
PyramidManager, Reprocessor, Output, VoxelInfo/Key, …) is dead and has been
deleted; only a slimmed `Common.hpp` (the stage's option/VLR storage) remains.
`LasWriterTest.issue2235` (COPC written via the C++ stage, asserts extended
per-return counts) and the full `CopcReaderTest` confirm the delegated stage.
The CSF cloth-simulation algorithm is now a first-class Rust port
(`pdal_filters::csf_algorithm`) and the C++ `CSFilter::run` routes its
classification step through `pdal_filter_csf_classify`, which is what makes
`pdal_filters_groundfilter_test` count for all four parameterized
filter types (csf, pmf, skewnessbalancing, smrf).
  No easy audit wins remain among the uncounted binaries — all require
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

Next implementation-replacement checkpoint: all portable first-party C++
implementation has moved behind the Rust C ABI, leaving only compatibility
glue/wrappers and documented intentional C++ holdouts. This is separate from
final port completion: the C++ tests can pass before every portable
implementation is replaced, and final completion still requires packaging,
install/export, CI, performance, platform, and plugin decisions.

Current remaining C++ port-candidate ceiling for that checkpoint, excluding
C++ tests and vendor, is `10,524` code LOC for the main first-party
surface (`pdal/`, `filters/`, `io/`, `kernels/`, `apps/`, and `tools`). That is
still a ceiling, not a precise backlog: mixed files count as wrapper when they
include the C ABI even if they still contain legacy implementation, and
intentional C++ holdouts have not all been subtracted. Optional `plugins/` add
another deferred ceiling of about `36,476` code LOC; plugin work should stay
separate from the mainline implementation-replacement checkpoint until the
plugin ABI/support decision is made.

Using the same simple nonblank/noncomment line counter, the current first-party
C++ implementation ceilings by area are approximately:

| Area | First-party C++ LOC | Wrapper/adapter LOC | Port-candidate ceiling |
|---|---:|---:|---:|
| `pdal/` | 24,072 | 8,736 | 15,336 |
| `filters/` | 20,695 | 8,917 | 11,778 |
| `io/` | 25,171 | 8,457 | 16,714 |
| `kernels/` | 3,306 | 0 | 3,306 |
| `apps/` | 340 | 340 | 0 |
| `tools/` | 12 | 12 | 0 |
| Mainline total | 73,596 | 26,462 | 47,134 |
| `plugins/` deferred | 36,738 | 262 | 36,476 |

The numbers in this section are the older coarse manual estimate. The
`rust/scripts/audit_cpp_port_backlog.py` audit (see "C++
Implementation-Replacement Backlog" above) is now the authoritative,
repeatable measurement; it folds interface headers into their backed `.cpp`
and separates native adapters and documented holdouts. The legacy recompute
recipe below is kept only for cross-checking that coarse ceiling:

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
python3 rust/scripts/audit_cpp_test_parity.py --build-dir build --include-added-tests
```

Known mixed binaries:

- `pdal_kdindex_test`: all 5 tests currently count; KNN/radius queries route
  through Rust spatial query ABI.
- `pdal_spatial_reference_test`: `test_ctor`, `calcZone`, `wgs84FromZone`,
  `test_proj4_roundtrip`, `test_userstring_roundtrip`, `test_read_srs`,
  `test_io`, `test_vertical_and_horizontal`, `readerOptions`, `merge`,
  `test_writing_vlr`, `identifyEPSG`, `issue_1989`, and `test_bounds` count.
  User-input normalization (`OSRSetFromUserInput`
  + WKT1 and WKT2_2018 export), `getProj4`, semantic equality (`OSRIsSame`
  fallback), horizontal-EPSG identification, vertical-EPSG identification,
  `getUTMZone`, `getHorizontal`, `getVertical` (WKT bracket-matching subtree
  extraction because GDAL's C API has no `OGR_SRSNode` equivalent),
  `getHorizontalUnits`, `getVerticalUnits`, `getPROJJSON`, `getWKT1`,
  `prettyWkt`, WKT1-to-WKT2 normalization in `set`, and `valid` route through
  the Rust C ABI; `SpatialReference.cpp` no longer includes GDAL/OGR headers
  directly. Bbox corner transformations route through `pdal_srs_transform_*`.
  LAS 1.4 WKT/libLAS SRS VLR writing, no-axis-ordering LAS reprojection
  pipelines, and explicit axis-ordering transforms route through Rust.
  GeoTIFF VLR encoding remains C++ libgeotiff-backed.
  Strict bad-GeoTIFF-key detection for LAS reads now fails from the Rust LAS
  reader when a GeoTIFF SRS VLR is present but invalid.
- `pdal_point_view_test`: `getSet`, `getAsUint8`, `getAsInt32`, `getFloat`,
  `calculateBounds`, `pointRef`, `issue1264`, `bigfile`, `order`, and
  `getFloatNan`
  count. Point row add/mutate/swap behavior routes through the Rust C ABI.
  View identity ordering routes through Rust `PointView` IDs. C++ debug
  death-test behavior remains C++.
- `pdal_eigen_test`: `PointViewToEigen`, `RoundtripString`, `calcBounds`,
  `ComputeValues`, `Morphological`, `computeCentroid`, and `demeanTest` count.
  XYZ row export and raster math helpers route through the Rust C ABI. Matrix
  string round-tripping routes through the Rust transformation-matrix
  parser/formatter.
- `pdal_bounds_test`: 23 of 24 tests count. Bounds constructor, accessors,
  containment, scaling, intersection, growth, parsing, serialization, and output
  route through or are directly mirrored by the Rust C ABI. SRS-specific bounds
  behavior remains C++/GDAL.
- `pdal_utils_test`: all 25 pre-port tests count. Word wrapping, JSON/nonprinting
  escaping, base64 encoding/decoding, string splitting, case conversions,
  random/env helpers, numeric formatting, shell execution, extractor string
  reads, numeric cast helpers, and classic-locale numeric formatting/parsing
  route through the Rust C ABI.
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
- `pdal_point_table_test`: all 6 tests count. `resolveType`, `layoutLimit`,
  `userView`, `srs`, and `simple` route through the Rust C ABI as before, and
  `ColumnPointTable.typedStorage` now routes through `pdal_column_storage_*`,
  which owns the per-dimension blocked typed buffers in Rust. The C++
  `ColumnPointTable` keeps only an opaque handle plus per-dimension byte sizes
  and delegates `addPoint`, `finalize`, `getDimension`, and the
  set/getFieldInternal slot lookup to Rust; block expansion across the
  16384-point boundary is handled by `pdal_column_storage_add_point`, and
  returned slot pointers are stable for the storage's lifetime because the
  per-dim buffers are `Box<[u8]>` whose addresses do not move when the outer
  `Vec` grows.
- `pdal_kernel_test`: all 1 test counts; stage-option parsing routes through
  the Rust C ABI.
- `pdal_config_test`: all 1 test counts; version integer and full-version
  formatting route through the Rust C ABI while compile-time version constants
  remain C++.
- `pdal_log_test`: all 2 tests count. `t1` routes level-name formatting and the
  per-line `(LEADER LEVEL) ` prefix through the Rust C ABI via
  `pdal_log_format_prefix`. `t2` exercises the Rust `translate` kernel dispatch
  from the app while C++ still owns log sink selection.
- `pdal_app_test`: `option_file`, `load`, `log`, and `listCommands` count.
  `option_file` routes through the Rust translate option-file path. `load`
  exercises Rust kernel listing/dispatch (`pdal_kernel_list_json`,
  `pdal_kernel_run`) and the
  Rust-formatted unknown-command message
  (`pdal_app_unknown_command_message`). `listCommands` exercises the
  Rust-owned command metadata list for text and JSON output. `log` exercises
  the Rust-formatted `Log::get` line prefix (`pdal_log_format_prefix`) for
  `-v Debug` / `--verbose=3` / `--logtiming` / default-level behaviors.
- `pdal_stage_factory_test`: both pre-port cases count. `Load` reads the
  Rust-owned stage registry list, and `extensionTest` routes reader/writer
  driver inference through the Rust C ABI. The newer
  `stageExtensionsLoadPerInstance` and `stageExtensionsCustomMappingsOverrideDefaults`
  guard tests also route default lookup and custom extension overrides through
  Rust-owned C ABI helpers, but they are excluded from the pre-port denominator.
- `pdal_plugin_manager_test`: all 3 tests count. `validnames` checks plugin
  filename validation, `MissingPlugin` checks unknown-stage lookup, and
  `CreateObject` exercises runtime stage registration plus instantiation —
  `PluginManager<T>::l_registerPlugin` now writes the (namespace, plugin
  name, creator thunk, description, link) tuple into the Rust-owned runtime
  registry (`pdal_runtime_plugin_register`) keyed by `typeid(T).name()`, and
  `l_createObject` looks up the creator function pointer through
  `pdal_runtime_plugin_lookup_creator` before invoking it from C++. The
  legacy `m_plugins` map is still populated for any code that still walks
  it, but the test's lookup path is Rust-authoritative.
- `pdal_options_test`: `valid`, `programargs`, `nan`, `doublepreicison`,
  `issue_4751`, `conditional`, and `test_option_writing` count. Option-name
  validation, command-line formatting, JSON scalar formatting, and conditional
  option serialization route through Rust helpers. Option storage remains C++.
- `pdal_polygon_test`: `test_wkt_in`, `test_wkt_out`, `test_json_in`,
  `test_json_out`, `simplify`, `smooth`, `covers`, `valid`, `bounds`,
  `bounds2d`, and `bounds3d` count. Native geometry validity, WKT output,
  GeoJSON parse validity, WKT-to-GeoJSON serialization in GDAL's
  `OGR_G_ExportToJsonEx(COORDINATE_PRECISION)` byte-for-byte shape,
  area, simplification, point coverage, and bounds route through the Rust C
  ABI. Stream operators and polygon relational operators remain C++/GDAL.
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
- `pdal_filters_hag_test`: `delaunay` counts for `filters.hag_delaunay`, `dem`
  and `dem_clamps` count for `filters.hag_dem`, and `neighbors` and `closest`
  count for `filters.hag_nn`. DEM sampling, Delaunay interpolation, and
  nearest-neighbor HAG assignment route through the Rust C ABI.
- `pdal_filters_chipper_test`: `issue_2479`, `empty_buffer`, and
  `test_construction` count. View partitioning routes through the Rust C ABI.
  Factory smoke coverage remains C++ wrapper behavior.
- `pdal_filters_ferry_test`: `stream` and `test_ferry_copy_json` count.
  Dimension ferrying routes through the Rust C ABI in batch and streaming paths.
  Stage factory smoke coverage remains C++ wrapper behavior.
- `pdal_filters_h3_test`: only `stream_test_2` counts; H3 indexing routes
  through the Rust C ABI. Stage creation remains C++ factory behavior.
- `pdal_filters_hexbin_test`: `HexbinFilterTest_test_1`,
  `HexbinFilterTest_test_2`, `HexGrid_issue_2507`, `H3Grid_issue_2507`, and
  `issue_4899` count. Hexbin stage execution and the native/H3 hex-grid
  boundary generators route through the Rust C ABI. GeoJSON density file output
  routes through Rust for both standard and H3 grids; non-GeoJSON density and
  boundary output now route through the Rust native GDAL/OGR polygon writer.
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
- `pdal_metadata_test`: all 12 tests count. Metadata creation, cloning,
  scalar conversion, JSON scalar formatting, child updates, and colon-path child
  lookup route through the Rust C ABI. Pointer round-tripping is represented as
  an opaque Rust metadata scalar exposed by the C ABI.
- `pdal_pipeline_manager_test`: all 6 tests count. They execute
  reader-to-writer pipelines through the Rust C ABI, including root-array JSON,
  command-line stage-option overrides, LAS input globbing, validate-only
  object-valued options, and stage replacement dependency rewiring.
- `pdal_streaming_test`: all 7 tests count. Streaming pipeline execution (including diamond pipelines, callback-driven filters, bounds, counts, and spatial reference propagation) is fully supported and validated via the Rust C ABI and FFI process_one interfaces.
- `pdal_io_las_reader_test`: all currently built cases count by explicit audit
  list. Point materialization, callbacks, lazperf stream decoding, VLR/SRS
  handling, and the covered failure paths route through the Rust C ABI; keep
  future LAS reader cases explicit rather than relying on a broad `ALL` rule.
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
  Known environment-dependent gap: `pdal_wkt2_with_derivedprojcrs_vlr`
  expects exactly two SRS VLRs (WKT2 + PROJJSON) for a DerivedProjectedCRS,
  assuming GDAL's WKT1 export fails for that CRS so no WKT1 VLR is written.
  GDAL 3.13 instead exports a fallback WKT1 (logging a PROJ error but
  returning success), so two extra WKT1 VLRs are written and the count is 4.
  This matches the pre-port C++ path (`exportToWkt` used the buffer
  regardless of the return code), so it is a GDAL/PROJ-version behavior
  change, not a Rust-port regression. Revisit when the GeoTIFF/WKT1 SRS-VLR
  encoding holdout is addressed.
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
- `pdal_io_copc_reader_test`: `inspect`, `fullRead`, `boundedRead2d`,
  `boundedRead3d`, `stream`, `boundedCrop`, `boundedCropGeoJSON`,
  `polygonAndBoundsCrop`, `boundedCropReprojection`, `ogrCrop`,
  `multipleInputs`, `boundedpreview`, and `resolutionLimit` count. Local COPC
  point materialization, simple dataset-coordinate bounds, same-SRS WKT and
  GeoJSON polygon crops, EPSG:4326-to-source polygon reprojection, GeoJSON
  OGR polygon crops, streaming row-by-row materialization, multi-input
  diamond pipelines, bounds-backed preview, and resolution-limited
  execution (LAZ chunk-table-mapped point ranges streamed through the LAS
  reader) route through the Rust C ABI.
- `pdal_io_copc_remote_reader_test`: `vsi` counts. The autzen-classified
  COPC over both `https://` and `/vsicurl/` URLs is opened through
  `pdal-native::vsi::VsiFile`, the COPC info VLR and hierarchy pages are
  parsed by `pdal-io::copc_hierarchy`, and the C++ `CopcReader::inspect()`
  routes its bounds-and-resolution preview through `pdal_copc_preview`
  for the resulting point count and clipped bbox.
- `pdal_io_ept_reader_test`: `inspect`, `fullReadLaszip`, `fullReadBinary`,
  `fullReadZstandard`, `boundedRead2d`, `boundedRead3d`, `resolutionLimit`,
  `originReadVersion1_0_0`, `originRead`, `badOriginQuery`,
  `unreadableDataFailure`,
  `unreadableDataIgnored`, `unreadableDataIgnoredStreaming`,
  `unreadableTileFailure`, `unreadableTileFailureStreaming`,
  `badTilePointCountLaszip`, `badTilePointCountBinary`, `boundedCrop`,
  `polygonAndBoundsCrop`, `boundedCropReprojection`, `ogrCrop`, `bcbfToLonLat`,
  `bcbfToLonLat2dBoundsThrows`, and `duplicateInputs` count.
  Local EPT point
  materialization, simple dataset-coordinate bounds, depth pruning by
  `resolution`, origin selection, zstandard decompression, missing-tile error
  handling (both fail-fast and `ignore_unreadable`), corrupted-tile and
  hierarchy-vs-actual point-count failure detection, no-spatial-filter preview
  (bounds, point count, srs, dim names with laszip class-flag expansion),
  local streaming over Rust-materialized binary, LASzip, and zstandard views,
  same-SRS polygon cropping, EPSG:4326-to-source polygon reprojection,
  GeoJSON OGR polygon crops, reprojected 3D bounds filtering for BCBF data,
  bad-origin validation, multi-input diamond pipelines, and the three
  `*Stream` cases (`binaryStream`, `laszipStream`, `zstandardStream`) route
  through the Rust C ABI. Streaming works because the Rust reader stamps
  `EptNodeId`/`EptPointId` on each tile's full point set before bounds and
  polygon filters, then republishes an `ept::Artifact` (hierarchy step,
  per-tile `Overlap` entries, root bounds) into the C++ table's
  artifactManager so downstream stages — especially
  `writers.ept_addon` — keep working with the Rust read path. Spatial-filter
  preview remains C++.
- `pdal_io_ept_addon_writer_test`: all 4 tests count
  (`fullLoop`, `boundedWrite`, `boundedRead`, `mustDescendFromEptReader`).
  Each addon dimension's per-tile binary chunk writes, hierarchy JSON
  emission, and the top-level `ept-addon.json` metadata write are produced by
  `pdal_ept_addon_write` over a Rust-borrowed copy of the C++ view, with the
  C++ wrapper still managing the upstream `ept::Artifact` (hierarchy +
  hierarchy step + info bounds) and forwarding it across the C ABI as
  `pdal_ept_overlap_t` / `pdal_ept_root_bounds_t`.
- `pdal_io_stac_reader_test`: `local_catalog_test`, `item_collection_test`,
  `date_validate_test`, `date_prune_accept_test`,
  `date_start_end_time_accept_test`, `date_prune_reject_test`,
  `bounds_prune_accept_test`, `bounds_prune_reject_test`,
  `ogr_bounds_accept_test`, `ogr_bounds_reject_test`,
  `ogr_bounds_invalid_test`, `remote_item_test`, `catalog_test`,
  `nested_catalog_test`, `id_prune_test`, `local_data_test`,
  `collection_filter_test`, `collection_test`, `wrench_test`,
  `schema_validate_test`, and `multiple_readers_test` count. Local and remote
  STAC
  Feature/Collection traversal, direct asset reads, collection-id filtering,
  local and remote Catalog preview metadata (`item_ids`, `catalog_ids`,
  `collection_ids`, and total `pc:count`) including item-id pruning, local
  FeatureCollection item-id regex preview metadata, and single-item
  date/bounds/GeoJSON OGR-boundary preview pruning, schema-flag structural
  validation for local fixture shapes, property filtering, and per-reader args
  for the mixed EPT/COPC execution case route through the Rust C ABI.
  Remote preview JSON is fetched through GDAL VSI; nested local/remote catalog
  preview metadata is supported, and the covered remote EPT/COPC asset reads
  route through Rust. Full remote JSON-schema resolution and general OGR
  filters remain C++.
- `pdal_io_ogr_writer_test`: `json`, `creation_options`,
  `error_multicount_attrs`, `error_unknown_attr`, and `error_ogr` count.
  GeoJSON point and MultiPoint output routes through the Rust C ABI when the
  driver is GeoJSON without `measure_dim`, including the covered `WRITE_BBOX`
  and `COORDINATE_PRECISION` options. The multicount/attr_dims combination
  check, attr_dims missing-dimension error message, and RFC7946 unsupported-SRS
  error are formatted by the Rust C ABI before the C++ wrapper rethrows them via
  `Stage::throwError`. Shapefile, GeoPackage, measure dimensions, and native OGR
  driver behavior remain C++/GDAL.
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
- `assign` (simple `Dim[range]=value` assignments and the `condition` DimRange;
  the expression-based `value` option is rejected explicitly, not silently
  ignored — see the parity gap note below)
- `chipper`
- `cluster`
- `covariancefeatures`
- `dbscan`
- `decimation`
- `divider`
- `eigenvalues`
- `elm`
- `estimaterank`
- `faceraster`
- `gpstimeconvert`
- `groupby`
- `hag_delaunay`
- `hag_nn`
- `head`
- `hexbin`
- `iqr`
- `label_duplicates`
- `litree`
- `lloydkmeans`
- `locate`
- `lof`
- `m3c2`
- `mad`
- `merge`
- `miniball`
- `mortonorder`
- `nndistance`
- `normal`
- `optimalneighborhood`
- `outlier`
- `planefit`
- `pmf`
- `projpipeline`
- `radialdensity`
- `randomize`
- `reciprocity`
- `relaxationdartthrowing`
- `reprojection`
- `returns`
- `sample`
- `separatescanline`
- `smrf`
- `sparsesurface`
- `sort`
- `splitter`
- `stats`
- `straighten`
- `supervoxel`
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

- Private or specialized algorithms: trajectory-backed `Georeference` and
  Kazhdan-backed `Poisson`. Do not confuse `filters.georeference` with the
  already Rust-backed `pdal/util/Georeference` math or simple SRS transforms.
- Now Rust C ABI-backed: `Delaunay`.
- Now Rust C ABI-backed: `DEM`.
- Now Rust C ABI-backed: `HagDelaunay`.
- Now Rust C ABI-backed: `IterativeClosestPoint`.
- Now Rust C ABI-backed: `M3C2`.
- Now Rust C ABI-backed: `LiTree`.
- Now Rust C ABI-backed: `LloydKMeans`.
- Now Rust C ABI-backed: `PMF`.
- Now Rust C ABI-backed: `RelaxationDartThrowing`.
- Now Rust C ABI-backed: `GreedyProjection`.
- Now Rust C ABI-backed: `Straighten`.
- Now Rust C ABI-backed: `Supervoxel`.
- Now Rust C ABI-backed: `Normal`, including optional MST refinement.
- Now Rust C ABI-backed: `RadiusAssign`, including computed and conditional
  update expressions.
- Now Rust C ABI-backed: `NeighborClassifier`; the C++ wrapper only keeps
  option validation and candidate-reader loading.
- Now Rust C ABI-backed: `CovarianceFeatures`; the C++ wrapper only keeps
  option parsing, output dimension registration, and input layout validation.
- Now Rust C ABI-backed: `ProjPipeline`, including reverse coordinate
  operation mode through the native GDAL adapter.
- Now Rust C ABI-backed: `Shell` command execution and environment lookup; the
  C++ wrapper only keeps PDAL stage metadata/view plumbing.
- Now Rust C ABI-backed: `CS`; the C++ wrapper keeps return filtering,
  ignore-range segmentation, debug-directory validation, and class assignment
  plumbing around the Rust cloth classifier.
- Now Rust C ABI-backed: `HexBin`, including the standard `run` path,
  streaming execution (accumulated into a compatibility view before the Rust
  stage runs), density/boundary side-file output, and the exact-boundary path
  used by `kernels.tindex`. The private C++ `hexer` grid and old density OGR
  helper have been removed; the exported `HexBin` shell remains.
- `SMR` (`filters.smrf`): the Rust filter performs the `ignore` DimRange and
  `synthetic|keypoint|withheld` `classbits` pre-segmentation itself (matching the
  C++ `ignoreDimRanges`/`ignoreClassBits`). Both the Rust pipeline registry path
  AND the C++ `SMRFilter` stage now delegate the ignore/classbits case to Rust:
  `pdal_stage_create_smrf` was extended with `ignore` (a `pdal_dim_range_t[]`
  passed by component) and `classbits`, and `SMRFilter::run` delegates whenever
  `dir` is empty. `SMRFilterTest.ignoreDimRange` covers the C++->Rust ignore
  delegation. The Rust SMRF also implements the `dir` debug-raster output (12
  GeoTIFFs via `pdal-native`, format matching C++ `math::writeMatrix`), wired
  through the registry — so `pdal pipeline` smrf+`dir` now writes the rasters,
  closing a real gap (the Rust pipeline previously ran the Rust SMRF and
  *silently ignored* `dir`).

  **The C++ legacy `dir` path is intentionally RETAINED, not replaced.** An
  oracle comparison (Rust dir vs the C++ stage's legacy dir on interesting.las,
  cell=10) showed: the raw grids match (zimin pre-fill 0.55% cells differ,
  zilow 0%, ziobj 0.18%), but the knn-FILLED grids (zimin_fill/zinet/zipro/
  gsurfs) differ in ~96% of cells by up to ~20m. Cause: the Rust `knn_fill`
  diverges from C++ `knnfill` in the EMPTY cells (here ~99% of the 155k-cell
  grid — only 1065 sparse points). This does NOT affect classification (points
  sit in non-empty cells whose values match, hence ground agreement >=99.8%),
  but it means the Rust `dir` rasters are NOT a value-faithful reproduction of
  the C++ debug rasters. So `SMRFilter::run` still uses the legacy C++ path when
  `dir` is set (it dumps the C++ engine's grids), and the Rust `dir` output
  reflects the Rust engine's own grids.

  **Root cause of the void-cell divergence (investigated 2026-05-29 — NOT a
  fixable bug):** `knn_fill`/`knnfill` average the 8 nearest filled cells (an
  order-independent mean), so identical neighbor *sets* give identical fills.
  The divergence is purely in *which* 8 cells are selected: on SMRF's regular
  cell-center lattice inter-cell distances are highly degenerate
  (`cell*sqrt(dc^2+dr^2)`), so k-NN selection is pervasively tie-broken, and
  PDAL's KD-index (nanoflann) breaks those ties differently than Rust's `rstar`.
  This is the same cause `ground_command.rs` already documents ("tie-breaking
  inside the KD-tree inpainter"): ~96% of *void* cells differ, but only ~0.2% of
  *points* flip (points sit in non-void cells whose min-Z matches; only a point
  whose cell was stripped-then-filled is exposed). Matching it bit-for-bit would
  require replicating nanoflann's internal tie order in rstar — impractical, and
  it is the accepted >=99.8% SMRF parity floor. So the legacy C++ `dir` path is
  kept as an intentional debug holdout; it is not a "TODO to close".
- `Assign` (`filters.assign`): registered in the pipeline registry from the
  simple `Dim[range]=value` `assignment` list plus the `condition` DimRange,
  reusing the existing Rust `AssignFilter` and DimRange parser. **Parity gap:**
  the expression-based `value` option (the `expr::AssignStatement` form) is not
  yet parsed in the Rust registry path and is rejected with a clear error rather
  than silently ignored. The C++ `AssignFilter` (and its full `value` support)
  is unchanged and still reached for the non-registry C++ paths.
- Pipeline/process/framework behavior: `Info` streaming metadata accumulation
  and `StreamCallback`. `StreamCallback` is a C++ compatibility callback over
  `PointRef`; do not route C++ callable objects through the C ABI without a
  deliberate callback ABI design.

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
  `rust/scripts/measure_guardrails.sh --test-suites`
- Rust coverage reporting:
  `pixi run -e dev rust-coverage`
  `pixi run -e dev rust-coverage --html`
  `pixi run -e dev rust-coverage --lcov --output-path rust/target/llvm-cov/lcov.info`
- Rust mutation testing:
  `cargo install --locked cargo-mutants`
  `pixi run -e dev rust-mutants`
  `pixi run -e dev rust-mutants --package pdal-core`
