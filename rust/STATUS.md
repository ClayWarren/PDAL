# Rust Port Status

This is still a work in progress and is not yet at full feature parity with
C++ PDAL. Bugs may exist. Check this list before assuming a behavior is
intentional or before asking an agent to broaden the port.

Closed decisions live in `rust/DECISIONS.md`. When this status file says a
bucket is a native adapter, compatibility shell, or holdout, that ledger is the
source of truth for whether it should be ported, FFI-bound, left in C++, or
deferred.

Status definitions:

- `done`: believed done for the stated scope. No known major deficits in that
  area. OK to log bugs or add parity cases.
- `in progress`: actively being built. Some behavior works and some does not.
  OK to log crashes, panics, missing parity, and concrete gaps.
- `prototype`: proof-of-concept only. Do not treat as supported behavior.
- `not ready`: not started, or far enough from parity that it should not be
  used as a migration base yet.
- `deferred`: intentionally left for a later milestone or left in C++ for now.

Promotion to `done` is intentionally strict. A row is done only when there is
no known remaining work for that row's stated scope: C++ behavior is preserved
through the Rust C ABI and compatibility wrapper where applicable, pre-existing
C++ tests for that area pass through the Rust-backed path, Rust tests and
installed-PDAL parity/regression coverage exist for externally visible
behavior, and any remaining C++ is only documented glue, public API
compatibility, or an intentional native/vendor adapter. If the notes still name
an unsupported option, parity gap, packaging uncertainty, missing test path, or
"broader X remains open" caveat, the row is not done.

`native-adapter` is a terminal status when a plugin or dependency is
intentionally kept behind a C/C++/vendor boundary. It is not hidden pure-port
backlog unless a later milestone explicitly reopens that adapter.

## Current North Star

The old first target is complete: the pre-port C++ GoogleTest denominator is
`819 / 819` Rust C ABI-backed, and newly added local guard tests are tracked
separately. The implementation-replacement audit is also at `0`
port-candidate LOC for the main first-party surface and remains `0` when
optional plugins are included.

Those numbers are guardrails, not the finish line. The work left is to close
or explicitly accept the named caveats below:

- OGR/vector-source breadth: broad OGR datasource/update workflows and similar
  native vector edge cases.
- EPT/COPC/STAC/remote breadth: transformed EPT preview point counts and true
  cross-SRS transformed preview bounds clipping, plus local origin/polygon/OGR
  preview point counts, are Rust-pruned, but broad remote traversal, schema
  validation, object-store option parity, and accepted addon limitations remain.
- Metadata, pipeline, and command byte-shape parity: covered workflows route
  through Rust, but full C++ `PipelineWriter`, XML, broad STAC remote/schema
  behavior, and some stdout/stderr/artifact shapes remain scoped rather than
  globally done.
- Packaging and platform readiness: install/export, release packaging,
  workflow/platform policy, license/vendor accounting, and downstream C ABI
  consumption must be stable enough for upstream use.
- Optional plugins: completed checkpoints are listed below; other plugins are
  native adapters or deferred until a versioned plugin boundary exists.
- Final quality gates: Rust/C++ test parity, coverage, mutation testing,
  unsafe accounting, performance, memory, binary size, startup time, compile
  time, and full-suite timing need recorded pass/fail criteria before final
  replacement readiness.

When continuing the port, choose one of those caveats and close it with focused
parity coverage, or document why it is an intentional holdout/native adapter.
Do not restart old directory sweeps or add placeholder Rust modules.

## Feature Status

| Feature | Status | Notes |
|---|---|---|
| Rust core point model | in progress | `PointLayout`, `PointView`, dimension IDs/types, source indices, per-view SRS text, mesh faces, and 2D/3D bounds exist. Continue expanding only for real stage/I/O/command needs. |
| Rust stage model | in progress | Filter and streamable traits exist. Multi-input `run()` behavior is present and `filters.merge` handles DAG inputs without redundant points. |
| Rust pipeline graph | in progress | Reader/filter/writer DAG execution, tags, dependencies, `where`/`where_merge` splitting, metadata aggregation, summaries, and error propagation exist. Filename-string JSON stages infer reader/writer roles through the shared descriptor parser, and consecutive readers implicitly feed the next filter/writer with installed-PDAL serialization parity for covered multi-reader pipelines. Not a full C++ `PipelineManager` replacement. |
| Options | in progress | String-keyed typed getters, stable command-line argument emission, duplicate-key preservation, conditional add, multi-option merge, conditional merge, remove, replace, JSON object option parsing with JSONC-style comments at the option-file boundary, null-to-empty pipeline option parity, pipeline-stage option parsing, command-line option text parsing, and C++ `Options` copy/assign/move/lifecycle plus `Options::fromFile` JSON/command-line dispatch route through Rust for the covered option-file shapes. Full `ProgramArgs` surface parity is not claimed. |
| Metadata | in progress | Typed scalar metadata trees, descriptions, instance/array node kind preservation, scalar JSON helpers, base64 scalar decoding, structural JSON serialization, and flat metadata JSON arrays for repeated children exist. Pipeline serialization now emits a normalized object pipeline with inferred stage types/tags and decodes JSON-valued FileSpec/SRS option strings for the covered command paths. Full C++ `PipelineWriter` byte-shape parity is not claimed. |
| Spatial reference | in progress | `SpatialReference::set` (non-WKT user input and WKT1 to WKT2 normalization), `prettyWkt`, `getWKT1`, `getProj4`, `getPROJJSON`, `equals` (semantic IsSame fallback), `identifyHorizontalEPSG`, `identifyVerticalEPSG`, `getUTMZone`, `getHorizontal`, `getVertical`, `getHorizontalUnits`, `getVerticalUnits`, `isGeographic`, `isGeocentric`, `isProjected`, `getAxisOrdering`, and `valid` route through a Rust GDAL/OSR adapter (`pdal_srs_user_input_to_wkt`, `pdal_srs_wkt_to_wkt1`, `pdal_srs_wkt_to_wkt2`, `pdal_srs_pretty_wkt`, `pdal_srs_wkt_to_proj4`, `pdal_srs_wkt_to_projjson`, `pdal_srs_is_same`, `pdal_srs_identify_horizontal_epsg`, `pdal_srs_identify_vertical_epsg`, `pdal_srs_get_utm_zone`, `pdal_srs_get_horizontal_wkt`, `pdal_srs_get_vertical_wkt`, `pdal_srs_get_horizontal_units`, `pdal_srs_get_vertical_units`, `pdal_srs_is_geographic`, `pdal_srs_is_geocentric`, `pdal_srs_is_projected`, `pdal_srs_axis_ordering`, `pdal_srs_valid`). `SrsTransform` default-axis, explicit-axis, point, and array transforms now call the Rust C ABI transform handle. LAS/COPC WKT1/WKT2/PROJJSON SRS VLR payload generation uses the Rust GDAL/OSR adapter for the covered writer paths. LAS GeoTIFF SRS VLR decoding and pre-1.4 writer encoding route through the `pdal-native` libgeotiff adapter. Vertical extraction uses a Rust WKT bracket-matching parser because GDAL's C API has no `OGR_SRSNode` equivalent. `SrsTransform::get()` remains an intentional C++/OGR compatibility pointer for consumers like `OGRGeometry::transform`. |
| Spatial index | in progress | `pdal-core::SpatialIndex2d` and `SpatialIndex3d` use `rstar` R-trees for XY/XYZ nearest-neighbor and radius queries; `SpatialIndex3d::radius_dims` also indexes the common XY and XYZ dimension sets used by spatial filters, while keeping a generic exact scan for arbitrary non-coordinate dimension sets. The C ABI spatial query path uses those indexes for `KD2Index`/`KD3Index` compatibility searches. The exported C++ `KD*Index` classes remain API/ABI shells, but their implementation now builds a Rust point-view copy and dispatches through the Rust C ABI instead of nanoflann. A flexible-dimension optimized index can still be added if profiling shows the generic KDFlex path matters. Do not bake one-off neighbor searches into new filters. |
| Thread pool | done | `pdal::ThreadPool` delegates scheduling, stop/restart, await, queue clearing, and resize behavior through the Rust C ABI while keeping the existing C++ facade. Rust ABI tests and the C++ `pdal_thread_pool_test` pass through that path. |
| Expressions | in progress | Conditional, math, and assignment parser/evaluator support current Rust expression/assign work. Full C++ expression surface is not claimed. |
| C ABI bridge | done | Rust-owned handles are the contract. Metadata, summaries, views, `where` view splitting, and pipeline calls are exposed. The public header carries `PDAL_CAPI_ABI_VERSION_*` macros plus exported component/packed version functions so downstream consumers can assert the ABI generation they were built against. The installed `PDAL::CAPI` smoke and C++ test-parity audit are green; never pass C++ object pointers as Rust handles. |
| C++ filter wrappers | done | Safe ports use explicit Rust view conversion, and the C++ filter test binaries count as Rust C ABI-backed in the pre-port parity audit. Keep future wrappers on the same explicit conversion path. |
| Filter ports | in progress | 84 first-party filter/static stage files exist in C++; the portable filter implementation backlog is now at 0 port-candidate LOC. Remaining C++ under `filters/` is wrapper/compatibility surface plus the exported `filters/private/Point` OGR geometry adapter used by C++ option parsing (`filters.crop` centers and `filters.normal` viewpoint). `filters.crop` accepts WKT and GeoJSON Polygon/MultiPolygon option strings through the Rust registry path and rejects non-polygon geometries like installed PDAL. `filters.delaunay` is now registry-visible and attaches the `delaunay2d` mesh through the existing Rust triangulation helper. `filters.geomdistance` registry construction supports inline geometry plus OGR layer/SQL geometry sources for covered local vector datasources. Registry exposure is not the same as full pipeline parity. |
| Filter layout mutation | done | The Rust filter trait exposes declared output dimensions and the pipeline prepares those dimensions before execution. Registry coverage checks the derived-dimension filters that write classification, clustering, HAG, normal/curvature, covariance, color, H3, LOF, M3C2, and custom output dimensions. More complex layout mutation can still grow from future concrete parity cases, but there is no known unhandled layout-mutation class in the current first-party filter parity scope. |
| Pure/local I/O harness | done | `readers.faux` and `writers.null` support in-memory pipeline testing, streaming pipeline execution, Rust C ABI kernel dispatch, and Rust CLI pipeline command coverage. |
| Text I/O | done | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, and installed-PDAL regression coverage exists for scoped workflows. Reader input can open local files and GDAL VSI/HTTP(S) text sources. Reader input plus CSV and GeoJSON writer output are streamable in the Rust pipeline executor. |
| PCD I/O | done | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including ASCII, binary, binary-compressed, precision, streaming, and double-field coverage. ASCII, binary, and binary-compressed reader input plus ASCII, binary, and binary-compressed writer output are streamable in the Rust pipeline executor; compressed reads still decompress the required field-transposed payload before chunking, and writers use a temporary row stream so the final `POINTS` header and compressed payload remain correct. Reader input can open local files and GDAL VSI/HTTP(S) byte sources. Installed-PDAL parity coverage includes the decimation pipeline and compressed writer round-trip. |
| PTS/PTX readers | done | Existing C++ reader unit-test shapes pass through the Rust-backed path, including Leica ASCII fixture behavior and installed-PDAL regressions. PTS reader input can stream in the Rust pipeline executor over local or GDAL VSI/HTTP(S) seekable sources; PTX reader input can stream scan blocks with per-cloud transforms. Reader input can open local files and GDAL VSI/HTTP(S) text sources. |
| ILVIS2 reader | done | Existing C++ reader and metadata-sidecar unit-test shapes pass through the Rust-backed path for deterministic ASCII point and fixture-shaped XML metadata behavior. Point input and metadata sidecars can open local files and GDAL VSI/HTTP(S) text sources. |
| PLY I/O | done | C++ reader/writer unit-test shapes pass through the Rust-backed path, including ASCII/binary reads, ASCII/binary writes, mesh faces, precision/dim typing, and `#` flex filenames. ASCII/binary vertex-only reader input plus ASCII/binary single-file vertex writer output are streamable in the Rust pipeline executor, with the writer using a temporary row stream so the final vertex count remains correct. Mesh/list reads, mesh-face writes, and `#` fan-out writes intentionally use materialized execution because they need whole-view mesh or multi-output state. Reader input can open local files and GDAL VSI/HTTP(S) byte sources. Installed-PDAL parity coverage includes ASCII reader/writer pipelines and binary little-endian writer readback. |
| OBJ reader | done | Existing C++ reader unit-test shapes pass through the Rust-backed path, including deterministic Wavefront OBJ ASCII data, mesh faces, and VTN de-duplication. Reader input can open local files and GDAL VSI/HTTP(S) text sources. |
| GLTF writer | done | Existing C++ writer unit-test shapes pass through the Rust-backed path for deterministic local GLB output from mesh-backed views. Installed-PDAL parity coverage exercises `readers.text -> filters.delaunay -> writers.gltf` and compares the GLB binary-buffer and mesh structure. |
| OGR writer | done | GeoJSON point and MultiPoint FeatureCollection output is covered, including `attr_dims`, `multicount`, and the GeoJSON `WRITE_BBOX`/`COORDINATE_PRECISION` creation options used by the C++ tests. Plain Shapefile and GeoPackage point output, native OGR layer creation options, attribute fields, Shapefile MultiPoint grouping, and Shapefile measured point output go through a Rust native GDAL/OGR adapter. The native vector writer starts and commits OGR datasource transactions when the driver supports them, matching the C++ writer's transaction intent for drivers such as GeoPackage. The C++ `OGRWriter` delegates those GeoJSON, Shapefile, and GeoPackage cases to the Rust C ABI and routes multicount/attr_dims option validation, missing-attr_dims-dimension errors, and the RFC7946 unsupported-SRS error through Rust. All pre-port `pdal_io_ogr_writer_test` cases route through Rust. The `pdal-native` `VectorPointWriter` also supports polygon layers (`create_polygon`, `write_polygon`, `write_geometry_wkt`), and `filters.hexbin`/`kernels.density` use it for non-GeoJSON density/boundary output. |
| QFIT reader | done | Existing C++ reader unit-test shapes pass through the Rust-backed path for deterministic NASA ATM QFIT binary fixtures, with reader input able to open local files and GDAL VSI/HTTP(S) byte sources. Reader input is streamable in the Rust pipeline executor. |
| SBET/SMRMSG I/O | done | Existing C++ SBET reader/writer and SMRMSG reader unit-test shapes pass through the Rust-backed path for deterministic trajectory fixtures. Reader input can open local files and GDAL VSI/HTTP(S) byte sources. SBET and SMRMSG reader input are streamable in the Rust pipeline executor, and installed-PDAL parity tests cover the SBET read/write pipeline and SMRMSG reader output. |
| LAS/LAZ I/O | in progress | `las`/`laz` crate path supports standard dimensions, V1.0-1.4 point formats, Extra Bytes (VLR and user `extra_dims`), `start`/`count`/`nosrs`/`srs_vlr_order`/`ignore_vlr` reader options, WKT/PROJJSON SRS extraction, GeoTIFF SRS extraction through the `pdal-native` libgeotiff adapter with EPSG-only fallback, compression/decompression, full-file GDAL VSI URL reads, and core writer header options with C++-style validation for malformed numeric/UUID values plus LAS major/minor version, global encoding, and creation day-of-year ranges. Direct Rust C ABI reader/writer constructors are covered, and the C++ `LasReader`/`LasWriter` wrappers now route local read/write through Rust. LAS 1.4 WKT/libLAS SRS VLR writing preserves EPSG authority nodes through the Rust GDAL/OSR adapter, and LAS 1.0-1.3 SRS writing emits GeoTIFF GeoKey VLRs through the libgeotiff adapter. The writer rejects covered multi-view writes with mixed point spatial references, including empty-vs-nonempty SRS mixtures. Single-file LAS and LAZ writer output is streamable; `#` multi-file fan-out intentionally stays materialized because PDAL splits by input view. Keep parity tests honest before broad claims. |
| COPC I/O | in progress | Local `.copc.laz` full-file and streaming reads plus no-filter `inspect()` metadata route through the LAS/LAZ path, with post-read 2D/3D bounds, same-SRS polygon filtering, polygon reprojection for the covered EPSG:4326 crop, and GeoJSON OGR polygon crop support for the existing fixture shape. GeoJSON OGR polygon datasource input can open local files and GDAL VSI/HTTP(S) text sources. A first-party COPC hierarchy walker (`pdal-io::copc_hierarchy`) parses the COPC info VLR and walks hierarchy/sub-hierarchy pages over either local files or the `pdal-native::vsi::VsiFile` byte-range adapter, applying 2D/3D bounds and `resolution` pruning that matches the C++ `depthEnd = max(1, ceil(log2(spacing/resolution)) + 1)` math. Resolution-limited execution materializes only the kept LAZ chunks by reading the LAZ chunk table to map each hierarchy entry's file offset to a `(start_point_idx, count)` range and streaming the LAS reader past unwanted records (the `laz` crate's variable-chunk seek is buggy, so we deliberately stay on a sequential read). The C++ `CopcReader::inspect()` now routes bounds/resolution previews (no polygons/OGR) through the Rust `pdal_copc_preview` C ABI, which is what makes `pdal_io_copc_remote_reader_test.vsi` count. `writers.copc` is a first-party Rust COPC writer behind the C ABI and the C++ stage delegates the full write to it, including scale/offset, SRS, extra dimensions, selected header options, user/forwarded VLRs, and metadata/pipeline VLRs. Addons and broad OGR datasource coverage remain deferred. |
| EPT reader | in progress | Local LASzip, uncompressed binary, and zstandard EPT full-file reads walk JSON hierarchy and merge local tiles. GDAL VSI-backed EPT JSON/hierarchy/tile reads work for LASzip, binary, and zstandard tile payloads; the LASzip path is covered by the STAC mixed-reader workflow, while binary/zstandard have targeted `/vsimem/` coverage. Resolution limits and query bounds prune hierarchy nodes before tile reads; origin, same-SRS polygon, SRS-bound polygon reprojection, transformed 2D/3D bounds filters, and GeoJSON OGR polygon crops are applied after tile reads. Authority-form EPT SRS metadata (`authority`/`horizontal`/`vertical`) feeds read output, metadata, previews, and bounds filtering through the same Rust helper as WKT-form SRS. Resolution-limited, same-SRS bounds, semantically same-SRS and true cross-SRS transformed bounds clipping/counts, origin, local polygon, and local GeoJSON OGR polygon `inspect()` preview route through the Rust C ABI and use the same hierarchy count as the Rust reader. GeoJSON OGR polygon datasource input can open local files and GDAL VSI/HTTP(S) text sources. Tile point counts are validated and `ignore_unreadable` can skip unreadable tiles, with the C++ wrapper routing through the Rust path even when `ignore_unreadable` is set (an empty view is returned when every tile is skipped). Local binary EPT addon overlays are read through Rust for the existing addon round-trip checks, and the C++ `EptAddonWriter::writeOne` delegates its per-addon binary chunk writes, hierarchy JSON emission, and `ept-addon.json` metadata write to the Rust `pdal_ept_addon_write` C ABI. |
| FBI I/O | done | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including reader header summary, dimension discovery, point reads, GDAL VSI/HTTP(S) byte-source opening, and basic writer round-trip coverage. Rust preserves XYZ through writer round-trips; installed PDAL 2.10.1's legacy writer shifts those coordinates, so byte-for-byte installed writer parity is intentionally not claimed. |
| TerraSolid reader | done | Existing C++ reader unit-test shapes pass through the Rust-backed path for deterministic TerraSolid format 1/2 fixtures. Reader input can open local files and GDAL VSI/HTTP(S) byte sources and is streamable in the Rust pipeline executor. `.bin` is intentionally not inferred because it conflicts with FBI. |
| Optech reader | done | Existing C++ reader unit-test shapes pass through the Rust-backed path, including deterministic Optech CSD fixture data and localized WGS84 georeference math. Reader input can open local files and GDAL VSI/HTTP(S) byte sources and is streamable in the Rust pipeline executor. |
| BPF I/O | done | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including uncompressed and compressed point/dimension/byte interleaves, preview point count/SRS/header bounds/dimension labels, scaling, flex filenames, output dimensions, auto UTM, header data, bundled-file metadata, and ULEM/polar trailing-section skipping so later header metadata is read from the right offsets. Reader input can open local files and GDAL VSI/HTTP(S) byte sources. |
| GDAL reader/writer | in progress | Existing C++ GDAL reader unit-test shapes pass through the Rust-backed path for local raster-to-point-cloud behavior, and the Rust reader can stream raster rows through GDAL windowed reads. Standard-mode C++ GDAL writer cases with simple GDAL options now route raster rendering through the Rust C ABI for core grid statistics, including Float32/Float64 and signed/unsigned integer raster output types, comma-separated GDAL dataset metadata, `pdal_metadata`/`pdal_pipeline` metadata on view-backed tables, GDAL creation options (`gdalopts`), `bounds` grid sizing, alternate-grid conflict validation, `override_srs`/`default_srs` conflict validation, and empty-view error behavior. Targeted Rust regression coverage and `pdal_io_gdal_reader_test`/`pdal_io_gdal_writer_test` pass against the current build. Metadata on streaming tables remains C++. This is not broad GDAL/PROJ permission. |
| Raster writer | done | Raster attachments on Rust point views write through the Rust C ABI, and `pdal_filters_faceraster_test` exercises the Rust-backed `writers.raster` wrapper. Named/multi-raster output plus the C++ `data_type`, `gdalopts`, and no-data option shapes route through Rust for the covered GDAL raster attachment paths. |
| TIndex reader | in progress | GeoJSON tile-index reads can route through the Rust C ABI from local files and GDAL VSI/HTTP(S) text sources, and the C++ wrapper uses that path for simple indexes with optional bounding-box filtering. Basic local OGR datasource reads now use the Rust GDAL adapter for layer-0 or named-layer string location fields, optional `srs_column` SRS overrides, OGR `where` attribute filters, OGR SQL result layers, geometry-envelope bounds filtering, same-SRS WKT polygon filters, C++-parsed OGR polygon specs forwarded as WKT, `filter_srs` polygon reprojection when a target SRS is known, `t_srs` reprojection for indexed files with source SRS metadata, and JSON-array `reader_args` by reader type. Broad OGR index mutation/update workflows remain on the C++ path. |
| STAC reader | in progress | Local STAC Item/Catalog/Collection/FeatureCollection traversal can read local and covered remote assets through already-ported readers. Preview supports item/catalog/date/bounds/collection/property pruning, local structural validation for the schema-flag fixture shapes, GeoJSON OGR-boundary bounds for covered Polygon/MultiPolygon feature collections, basic local OGR datasource boundary bounds, and native OGR SQL predicate bounds through the Rust GDAL adapter. Execution supports item/catalog/bounds/OGR/property filters, collection filtering, and reader-specific args for local supported STAC documents, including the existing mixed EPT/COPC catalog workflow. Mixed-reader merges in Rust normalize to a union layout before appending views. Remote ranged COPC reads now use GDAL VSI-backed byte sources for hierarchy, chunk-table, and ranged point materialization. Custom schema URL validation intentionally stays on the C++ schema-validator path. Full remote JSON-schema resolution, broad remote traversal, and threaded catalog crawling are deferred. |
| Driver inference | done | Rust infers existing PDAL reader/writer names from filenames, including special path forms, and C ABI registry construction fails cleanly for inferred-but-unported drivers. |
| Pipeline JSON parsing | in progress | Narrow PDAL-style JSON arrays/root `pipeline` objects, JSONC-style comments, filename string stages, scalar/null/object option values, default linear dependencies, optional `tag`/`inputs`, generated tags, inferred serialized `inputs`, and framework `where`/`where_merge` options work for command readiness. C++ `PipelineReaderJSON`, command validation/serialization, and registry pipeline construction share the same descriptor validator before building stages. Covered serialization now normalizes explicit string `inputs`, numeric/bool stage option strings, and JSON-valued polygon/FileSpec/SRS option strings; covered execution also decodes JSON FileSpec `filename` strings to their `path`, and rejects invalid typed options and unexpected `filters.assign` options instead of silently defaulting. Shared-input DAG serialization now follows C++ `Stage::serialize` recursion for covered JSON pipelines, including repeated shared ancestors, and branch-specific layout mutation into `filters.merge` executes without panicking. `pdal pipeline --validate` now performs a conservative Rust prepare-layout pass: stages with known reader layouts (for example `readers.faux`) catch missing-dimension prepare errors such as invalid `filters.assign` targets, while unknown reader layouts are allowed through instead of inventing false validation failures. Full `PipelineWriter` byte-shape parity is not claimed. |
| `pdal-rs` command shell | in progress | Rust-native shell lists Rust-backed stages/commands and delegates first-party command execution to the same Rust C ABI kernel runner used by the C++ app/kernel wrappers. It is not yet the installed `pdal` executable. |
| Command metadata | done | `--drivers`, `--list-commands`, and `--options <stage>` are backed by Rust-owned metadata for the implemented Rust surface, including JSON output and scoped option tables for covered readers, filters, and writers. |
| C++ `pdal` app shell | done | `apps/` is a single file (`apps/pdal.cpp`, ~345 LOC) and the port backlog audit reports it at **0 port-candidate LOC** -- it is a thin entry-point peer over the Rust C ABI with no portable implementation left. Every piece of behavior/data routes through Rust: version (`pdal_version_string`), driver listing (`pdal_stage_list_json` -> `pdal_rust_stage_list_json`), command listing (`pdal_kernel_list_json` -> `pdal_rust_kernel_list_json`), stage option metadata (`pdal_stage_options_text`/`_json`), kernel dispatch (`pdal_kernel_run`, with a Rust dispatch guard for every first-party command), the unknown-command message (`pdal_app_unknown_command_message`), and log line prefixes (`pdal_log_format_prefix`). What remains in C++ is the intentional entry-point glue PORTING.md designates as the thin executable peer: `ProgramArgs` CLI parsing, terminal-table layout for `--drivers`/`--list-commands` (formatted from Rust JSON), log-sink selection (`Log::makeLog`), and the debug `SIGSEGV` backtrace handler. All 4 `pdal_app_test` cases (`option_file`, `load`, `log`, `listCommands`) pass through Rust, and a built-binary smoke confirms `--version`/`--drivers` (124 stages)/`--list-commands` (16 command entries, including `kernels.fauxplugin`)/dispatch/unknown-command parity. Reopen only if a command's deeper output shape is found to diverge from installed PDAL. |
| Implemented commands | in progress | All 15 first-party C++ kernel commands (`chamfer`, `delta`, `density`, `eval`, `ground`, `hausdorff`, `info`, `merge`, `pipeline`, `random`, `sort`, `split`, `tile`, `tindex`, `translate`) are Rust-dispatchable through the C ABI and listed in Rust command metadata. They have installed-PDAL regression coverage for scoped workflows. `info` owns summary, metadata, stdin pipeline JSON, point lookup including lists/ranges, 2D/3D nearest query, stats with `--dimensions`/`--enumerate`/`--breakout`, schema, boundary via Rust `filters.hexbin`, all-mode schema/stat/metadata/boundary/STAC output, pipeline serialization, STAC `pc_type`, the existing STAC app guard, and remote LAS header/VLR/EVLR extraction through the Rust C ABI `pointless_las` helper. Local `info --stac` output now emits the installed-PDAL STAC Feature top-level shape with geometry, bbox, assets, links, point-cloud schemas/statistics, and source projection bbox/geometry/WKT2/PROJJSON; broad remote/schema STAC parity remains limited. `tile` owns the existing app tests, including globbed input, text/LAS output, per-source reprojection to `out_srs`, writer text options, and the C++ single-`#` output-template validation. `tindex` owns the existing local GeoJSON create + bounds/polygon-filtered merge workflow with `filters.crop`, merge-time reprojection to `--t_srs`, stdin-fed create workflow, filelist create workflow, input-source conflict guard, invalid forwarded-filter diagnostic, GeoJSON stdout layer-description option, `--t_srs` layer SRS, `--a_srs` assignment/override behavior, C++ `--filespec`/`--smooth` create-option synonyms, bare `--skip_different_srs`, fast bbox boundaries, SRS mismatch warning/skip behavior, and exact hexer-driven boundary generation for `--threshold`/`--resolution`/`--simplify` with optional `--where` point-expression filtering. GEOS topology-preserving simplification is applied through `pdal-native`. `sort` accepts the C++ writer switches (`--compress`/`-z`, `--metadata`/`-m`) plus the direct shell-forwarded `writers.las` option forms and maps them onto the Rust-built writer stage. `density` now routes file inputs, JSON pipeline files, stdin JSON pipelines, and `.xml`-extension files containing JSON pipelines through the Rust kernel by appending a Rust-backed `filters.hexbin` stage; its Rust parser accepts the public C++ switch names (`sample_size`, `threshold`, `edge_length`, `hole_cull_area_tolerance`, `smooth`, `h3_grid`, `h3_resolution`) as well as explicit `filters.hexbin.*` options. `ground` compares per-point classification against installed PDAL (>=99.8% agreement on `interesting.las` with `cell=10`) after the Rust SMRF implementation gained the low-outlier mask, net cutting, KD-tree inpainting, and full validation. The Rust runner now owns the full `GroundKernel` option surface (max_window_size/slope/cell_size/scalar/threshold/cut/returns/ignore mapped onto `filters.smrf`, `--reset` -> `filters.assign`, `--denoise` -> `filters.outlier`, `--extract` -> `filters.range`, accept-and-ignore max_distance/initial_distance and the basic switches) and never returns the -1 C++ fallback sentinel, so the behavior runs through Rust. The exported C++ `ChamferKernel`, `DeltaKernel`, `EvalKernel`, `GroundKernel`, `HausdorffKernel`, `InfoKernel`, `MergeKernel`, `PipelineKernel`, `RandomKernel`, `SortKernel`, `SplitKernel`, `TIndexKernel`, `TileKernel`, and `TranslateKernel` classes are **retained** as API/ABI shells; direct `execute()` paths for those shells now dispatch through `pdal_rust_kernel_run` where their lower-layer behavior is Rust-backed, matching the app-path Rust behavior without deleting public symbols. `pdal delta` now supports the direct C++ shell's `--detail` and `--alldims` options through `pdal_delta_ex`. The direct `PipelineKernel` shell now routes normal execute, metadata, `--dims` output-dimension restriction, C++-style file progress markers, serialization, validate JSON output, hidden PointCloudSchema XML output, strict `--stream` enforcement, and `--nostream` materialized-execution selection through Rust. The direct `TranslateKernel` shell now routes normal reader/filter/writer execution, JSON filter-extraction (`--json`), metadata output, pipeline serialization, strict `--stream` enforcement, `--nostream` materialized-execution selection, and the same-input `--overwrite` guard through Rust; serialized pipeline byte shape is Rust JSON rather than exact C++ `PipelineWriter` output. The direct `TIndexKernel` shell now routes parsed create/merge execution through Rust; broad OGR datasource/update parity and non-GeoJSON tile-index workflows remain deferred to the Rust tindex runner's documented scope. `pdal ground` smoke-tested across basic/extract/reset/denoise/combined/ignore/smrf-passthrough and the direct exported class path is now Rust-dispatched too. `tools.lasdump` and `tools.nitfwrap` have Rust command paths for their scoped fixture-backed workflows. |
| Performance visibility | prototype | Ignored reporting harnesses exist for local I/O performance, binary size, startup time, memory, build cost, and opt-in full C++ vs Rust test-suite timing. They are visibility tools, not hard gates yet. **Recorded results now live in `rust/BENCHMARKS.md`:** CLI-vs-CLI wall-clock, the peak-RSS (memory high-water mark) table, binary-size/startup, build cost, and the full C++/Rust suite-timing data point are all captured there (macOS arm64, version-matched 2.10.1) -- closing the previous "not yet recorded" gap for memory, build cost, and suite timing. The `measure_guardrails.sh --test-suites` ctest path was hardcoded to a nonexistent `build/` tree; it now auto-detects `.build`/`build`/`PDAL_BUILD_DIR`. Wall-clock and peak-RSS comparisons against a reference C++ `pdal` are discoverable as `pixi run -e dev rust-bench` and `rust-bench-memory` (both depend on `build`; reference `pdal` resolved from PATH). Peak-RSS harness `rust/scripts/benchmark_memory_cpp_vs_rust.py` (version-matched 2.10.1, `/usr/bin/time -l`) found 3 of 4 workloads equal-or-better, but Rust-backed `pdal pipeline` used far more peak RSS on memory-bound pipelines. **Root cause is the pure-Rust CLI executor, NOT the C ABI bridge** (confirmed by isolating stages; Rust per-point storage width matches C++ at ~44 bytes/pt, so the gap was redundant *copies*, not wider storage). The contributing copies were: (1) `Pipeline::execute` (`rust/pdal-core/src/pipeline.rs`) keeping every producer's output alive in its `outputs` map for the whole run while cloning it into each consumer; (2) `filters.sort` (`rust/pdal-filters/src/sort.rs`) rebuilding a full second view instead of sorting the index table in place; (3) no streaming-mode selection, so a fully-streamable `faux -> null` that C++ never materializes is read entirely into RAM. **Landed (two fixes):** (a) `execute()` now moves a producer's views into their final consumer and drops them once fully consumed (`take_node_inputs`, cloning only for earlier consumers of a multi-consumer/diamond producer); (b) `filters.sort` gained an in-place owned path: a new `Filter::run_owned(Vec<PointView>)`/`StageWrapper::run_owned` (default delegates to the borrowing `run`; only the in-process executor's no-`where` path uses it, so the C ABI filter bridge is unchanged) lets sort reorder rows via `PointView::reorder` (in-place gather permutation by swap cycles) instead of building a second view. For 3M points: `faux -> null` 279->116 MiB (-58%, 2 copies -> 1) and `faux -> sort -> null` 416->210 MiB (-49%; 3 copies -> ~1 plus the sort-order and inverse index arrays). **Remaining gap vs C++:** sort 210 vs 157 is the auxiliary index arrays (C++ reuses one), minor; the larger remaining item is a streaming `Pipeline::execute` path when every stage is `Streamable`. This matters for real workflows, not just synthetic faux: on a **large LAS file (3M points) -> `filters.range` -> LAS**, C++ streams at constant ~61 MiB while Rust materializes all points at ~338 MiB (**5.4x**). Small files hide it (autzen_trim ~110k pts: Rust is actually *lighter* than C++ at 0.59-0.72x), large files expose it. **Streaming landed and wired to the CLI (in progress, C++ has always had this -- it is parity catch-up):** `Pipeline::execute_streaming()` runs a fully-streamable linear reader->filters...->writer chain in fixed 10k-point chunks (bounded peak memory; matches C++'s default `StreamPointTable` capacity) and returns `Ok(None)` to fall back to the unchanged `execute()` when any stage is non-streamable, fans out, or has a `where`. New trait hooks (`streamable()`, reader `stream_next`, filter `stream_chunk`, writer `stream_write`/`stream_finish`) default to unsupported so the gate is conservative; `PointView::reorder`/`copy_point_within` support in-place compaction. Streamable stages: `readers.faux` (RNG-continuous chunks), `readers.las` (chunked `las::Reader::read_points`, gated to plain local-file reads -- glob/VSI/NITF-offset/lenient fall back), `readers.text`, `readers.pts`, `readers.ptx`, `readers.qfit`, `readers.optech`, `readers.sbet`, `readers.smrmsg`, `readers.terrasolid`, and `readers.pcd` ASCII, binary, and binary-compressed input (chunked line/fixed-record/field-transposed-payload reads over local or GDAL VSI/HTTP(S) seekable sources), `filters.assign` for range and value-expression assignments, `filters.range`, `filters.crop`, `filters.head`, `filters.ferry`, `filters.expression` for single-expression pipelines, `filters.h3`, `writers.null`, `writers.las` (incremental `write_points` + bounds accumulated across chunks and patched at close; single-file LAS/LAZ only), `writers.text` CSV and GeoJSON output (header/features written incrementally), `writers.pcd` ASCII, binary, and binary-compressed output (rows spill to a temp stream so the final point-count header and compressed field-transposed payload are correct), and `readers.ply` ASCII/binary vertex-only input, `writers.ply` ASCII/binary single-file vertex output (rows spill to a temp stream so the final vertex-count header is correct). The `pdal pipeline` kernel path (`kernel_abi/pipeline/command.rs`) and the `pdal-rs` CLI try `execute_streaming()` first when no `--metadata` summary is requested, falling back otherwise (new C ABI `pdal_pipeline_execute_streaming`, sentinel `-2` = not eligible). **Measured (3M points):** `faux -> range -> null` 196->**22.8 MiB**, `faux -> assign -> null` 247->**22.1 MiB**, and the **big-file headline case `large.las -> filters.range -> large.las` (file->file) 338->24.9 MiB vs C++ 62.6** -- the original 5.4x gap is closed and Rust is now ~2.5x lighter than C++; `faux -> sort -> null` correctly falls back (210). Parity-tested: streaming output == standard `execute()` (incl. compaction), faux/LAS/text/PTS/PCD chunked reads == `read()` (all dims + SRS where relevant), assign/range `stream_chunk` == `run_one`, **streamed LAS write is byte-identical to the materialized `write()` and streamed LAZ write round-trips through the LAS reader**, output point-count/bounds match C++, non-streamable pipelines fall back. **Remaining (minor):** `#`-multi-file LAS writes stay on the materializing path; mesh/list PLY input and mesh/fan-out PLY output are still materialized. Note: `pdal_streaming_test` exercises a separate C ABI `process_one` interface, not the CLI executor. |
| Rust coverage reporting | done | `pixi run -e dev rust-coverage` runs `cargo-llvm-cov` over the Rust workspace. The line-coverage threshold is enforced by `rust-coverage-check` inside `rust-guard`; keep the percentage in `pixi.toml` synced with the latest measured coverage. |
| Rust mutation testing | prototype | `pixi run -e dev rust-mutants` runs `cargo-mutants` when it is installed locally. This is an audit tool for mature buckets, not part of `rust-guard`. |
| Unsafe Rust footprint | done | `pixi run -e dev rust-unsafe-audit` tracks source-only unsafe Rust accounting. Current first-party Rust count, excluding `rust/target`, is 358 `unsafe { ... }` blocks, 567 `unsafe extern "C" fn` exports, 87 total `unsafe fn` helpers, 2 unsafe extern callback type aliases, 0 unsafe extern blocks, and 1 `unsafe impl`. Unsafe remains concentrated in `pdal-capi`, `pdal-native`, and Rust callers of the C ABI; keep new unsafe at C/native boundaries or tests that exercise those boundaries. |
| Vendor/native strategy | in progress | `vendor/` has 11 top-level third-party dependency directories. `rust/VENDOR.md` describes the vendor boundary and `rust/DECISIONS.md` records the closed choices. Current decisions: `h3` -> `h3o`, `lazperf` -> `las`/`laz`, `eigen`/`gtest`/`nanoflann`/`nlohmann` are not direct Rust port targets, and `arbiter`/`kazhdan`/`lepcc`/`schema-validator`/`utfcpp` are adapter-bound or deferred to named milestones. Native GDAL/OGR/GEOS/PROJ/Nitro adapters belong in `pdal-native`; pure Rust replacements such as LAS/LAZ do not need to move through it. |
| Plugins | in progress | There are 18 top-level plugin directories. Track each plugin below and in `rust/DECISIONS.md`. The optional plugin audit reports 0 port-candidate LOC: `faux`, `nitf`, and `spz` are completed C ABI-backed compatibility checkpoints, while all other optional plugins are native adapters/deferred by decision instead of unplanned pure-port backlog. A Rust plugin SDK and broad optional plugin sweep are still not ready. |
| Remote/object-store I/O | in progress | `pdal-native::vsi::VsiFile` opens local, URL, and `/vsicurl/` paths through GDAL VSI and now implements `std::io::Read + Seek` so byte-range readers can stream over it. The Rust COPC hierarchy walker consumes the adapter end-to-end: `pdal_io_copc_remote_reader_test.vsi` (autzen-classified.copc.laz over both https and `/vsicurl/`) now counts as Rust C ABI-backed. STAC remote JSON traversal, remote LASzip EPT reads, and `pdal info` remote pointless-LAS header/VLR/EVLR extraction consume the same adapter. Broader object-store option parity remains open. |
| Broad kernels/apps/tools migration | in progress | Simple `pdal-rs` commands may continue proving lower layers. **`apps/` is complete** (0 port-candidate LOC -- `apps/pdal.cpp` is a thin entry-point peer; see the `C++ \`pdal\` app shell` row). The standalone tools have C ABI-backed dispatch shells, and broad `kernels/` command parity still depends on lower-layer kernel coverage. The C++ `pdal pipeline`, `pdal translate`, `pdal random`, `pdal density`, `pdal ground`, `pdal split`, `pdal sort`, `pdal merge`, `pdal delta`, `pdal tindex`, and simple `pdal tile` app paths now execute through Rust for local reader/filter/writer workflows. `pdal-rs` now routes every first-party command wrapper (`pipeline`, `info`, `translate`, `merge`, `sort`, `ground`, `density`, `random`, `split`, `tile`, `tindex`, and metric/eval commands) through that same Rust C ABI kernel runner after usage handling, so the CLI no longer carries parallel local command implementations. `pdal density` also owns JSON pipeline, stdin JSON pipeline, and `.xml`-extension JSON pipeline inputs through Rust. Direct exported C++ `DeltaKernel`, `EvalKernel`, `GroundKernel`, `MergeKernel`, `PipelineKernel`, `RandomKernel`, `SortKernel`, `SplitKernel`, `TIndexKernel`, and `TileKernel` execution now reaches the same Rust C ABI runner, reducing the remaining kernel implementation backlog while preserving the public C++ classes. `pdal info` supports stdin pipeline JSON through Rust. `pdal translate` supports `filters.range` option files for the existing app guard. Standalone `lasdump` and `nitfwrap` dispatch through the Rust C ABI; `lasdump` covers LAS/LAZ header, VLR/EVLR, and point checksum output, and `nitfwrap` uses the Nitro native adapter for LIDARA DES wrap/unwrap with LAS/BPF fixture parity. |

## Root-Level Migration Status

The Rust port is not complete just because Rust-backed tests pass. The root
build, install, packaging, CI, examples, and docs must also describe and verify
the Rust-backed shape of PDAL.

| Area | Status | Notes |
|---|---|---|
| Root CMake | done | `libpdal_capi.a` is built, linked into `pdalcpp`, and sourced from `cmake/rust.cmake` so the dependency list tracks every current Rust crate that can affect the C ABI or linked implementation. The C++ app/kernel C ABI bridge source is also compiled into `pdalcpp`, so installed `PDAL::CAPI` consumers resolve the full public C ABI from the exported PDAL library instead of an app-local static archive. |
| `cmake/` modules | done | Rust build options live in `cmake/rust.cmake`, which now owns both Rust integration macros: `pdal_build_rust_capi()` (the cargo `add_custom_command`/target and the `pdalcpp` dependency edge) and `pdal_link_rust_capi()` (the Rust C ABI archive plus the native libraries it embeds -- GEOS via the `geos` crate, the Nitro NITF bridge, and CoreFoundation on Apple). The previously-scattered link lines are folded into that one macro and called from the three targets that link `libpdal_capi.a` directly: the root `CMakeLists.txt` `pdalcpp` build and the standalone `tools/lasdump` and `tools/nitfwrap` launchers (verified: pdalcpp/`pdal`, `lasdump`, and `nitfwrap`+`nitfwrap_test` all build, link, and pass through the macro). Source packaging excludes generated Rust build output. The only Rust value still set in the root `CMakeLists.txt` is `RUST_CAPI_HEADER_DIR`, set early on purpose so `add_subdirectory(plugins)` sees it before `rust.cmake` runs its compile-time discovery -- documented ordering glue, not scattered link wiring. |
| `pixi.toml` | done | The developer environment now includes the Rust toolchain and explicit `rust-fmt`, `rust-check`, `rust-clippy`, `rust-test`, `rust-coverage`, `rust-license-audit`, `rust-cpp-port-audit`, `rust-cpp-test-parity`, `rust-capi-install-smoke`, `rust-source-package-smoke`, and `rust-guard` tasks for the port workspace. |
| GitHub workflows | in progress | The Pixi workflow runs the Rust workspace guard, the C++ implementation-replacement audit, the C++ build, an installed `PDAL::CAPI` consumer smoke, the C++ test-parity audit, and the C++ tests. The legacy Linux, macOS, Windows, and Alpine compile scripts also run the installed `PDAL::CAPI` consumer smoke after `ninja install`; the conda package example job runs the same smoke against the installed package prefix; release source confirmation reaches the same smoke through the Linux compile script. Shared CI conda environments and Docker/Alpine build images install Rust/Cargo explicitly for CMake builds that require the Rust C ABI. Remaining upstreamability work is broader platform parity and final policy review, not missing Rust toolchain wiring. |
| `PDALConfig.cmake.in` | done | Downstream `find_package(PDAL)` keeps the C++ target as the primary link surface, exposes `PDAL_CAPI_INCLUDE_DIRS`, and provides an installed `PDAL::CAPI` interface target for the stable C ABI header plus the backing `PDAL::PDAL` link surface. Verified with a temporary install prefix and an out-of-tree CMake consumer that includes `<pdal_capi.h>`, links `PDAL::CAPI`, calls `pdal_version_string()`, and checks the compile-time `PDAL_CAPI_ABI_VERSION_*` macros against the exported runtime ABI-version functions while loading the installed library. A separate C ABI library export is not part of the current shape because the C ABI is exported by `pdalcpp`. |
| `pdal_features.hpp.in` | done | Decision: no Rust-backed-build feature macro is added. The Rust C ABI is an unconditional, mandatory part of the build -- `cargo` is required (fatal if missing), `pdal_rust_capi`/`libpdal_capi.a` is always a dependency of `pdalcpp`, and there is no `option()` to disable it -- so a `PDAL_HAVE_RUST`-style macro would always be defined and provide no supported conditional to branch on (and the guidance is to avoid preprocessor branching). The one Rust-related compile define, `PDAL_UTILS_NO_RUST_CAPI`, is a private switch for the standalone `dimbuilder` generator only (see `dimbuilder/`), not a feature-availability macro for the installed library. Revisit only if a future supported build mode actually makes the Rust C ABI optional. |
| `dimbuilder/` | done | Intentional generator-tool exception. `dimbuilder` is a standalone code generator (`Dimension.json` -> `Dimension.hpp`) that, by existing PDAL design, compiles `Utils.cpp` directly into the executable rather than linking `pdalcpp` (so Linux packagers who disable rpath can still run it during the build). It therefore builds with `PDAL_UTILS_NO_RUST_CAPI`, which selects the pure-C++ fallback for each Rust-C-ABI-backed `Utils` function. Verified: a clean standalone build compiles `Utils.cpp` + `DimBuilder.cpp` with no Rust capi linked, and the generated `Dimension.hpp` is byte-identical to the build-tree header (modulo the input-path comment). The tool only uses `Utils::split`/`toupper`/`trim`/`wordWrap`, all of which have correct C++ fallbacks under the guard. This is the accepted permanent shape, not a half-port; the guarded C++ fallbacks must stay behavior-equal to the Rust path. |
| `package.sh` and release packaging | done | Source packaging keeps Rust sources while excluding `rust/target/`, and `package.sh` installs Rust/Cargo alongside the C++ build tools. `rust-source-package-smoke` runs `package_source` and verifies the source archives contain the Rust workspace files and C ABI header while excluding build/cache artifacts. The release workflow runs the same archive check after `ninja dist`. `rust-license-audit` reports third-party crate license metadata from `cargo metadata` and currently finds no missing license metadata. Verified locally with `pixi run -e dev rust-source-package-smoke`. |
| `examples/` | done | Installed-prefix executable and plugin-authoring examples link through the exported `PDAL::PDAL` target instead of the legacy raw library variable/link-directory pattern. Verified `batch-streamer`, `filter-streamer`, `reading-streamer`, `writing-streamer`, and `writing` configure/build against `/tmp/pdal-rust-prefix`, and the `writing` tutorial runs and emits `myfile.las` through the Rust-backed writer path. Verified `writing-filter`, `writing-kernel`, `writing-reader`, and `writing-writer` configure/build against the current installed Pixi prefix with AppleClang after `Utils::fromString(int/double)` became exported overloads instead of compiler-sensitive constrained template specializations. |
| `doc/` | in progress | Developer docs now expose the Rust port as an experimental migration effort and point to `rust/PORTING.md`, `rust/STATUS.md`, and `rust/VENDOR.md` as the authoritative sources. Public user-facing docs still wait until build, install, plugin, and ABI boundaries are stable enough to describe accurately. |

## Plugin Status

These are optional PDAL plugin directories, not the core static stage surface.
The broad plugin triage is complete enough to separate C ABI-backed
checkpoints from dependency-bound native adapters without forcing a plugin SDK
decision.

Plugin backlog is now measurable with:

```sh
python3 rust/scripts/audit_cpp_port_backlog.py --include-plugins --top 40
```

Current plugin snapshot, excluding plugin tests and treating bundled
`plugins/e57/libE57Format` as native/vendor adapter code rather than PDAL
implementation to port line-by-line: `12,001` port-candidate LOC across `93`
files. After closing `plugins/faux`/`plugins/spz`, removing dead NITF
reader-side C++ helpers, and classifying dependency-bound optional plugins as
native adapters, this is `0` port-candidate LOC across `0` files. Optional
plugin work is not finished in the product sense; it is now explicitly split
between C ABI-backed checkpoints (`faux`, `spz`, NITF wrappers) and native
dependency adapters that wait on a future plugin SDK/packaging decision.

| Plugin | Status | Notes |
|---|---|---|
| `plugins/arrow` | native-adapter | Decision: keep Arrow/Parquet integration as an optional columnar native dependency adapter. A Rust Arrow path would be a separate plugin milestone, not remaining pure-port backlog. |
| `plugins/cpd` | native-adapter | Decision: keep the registration filter wrapper as an optional native solver adapter over the external CPD library. |
| `plugins/draco` | native-adapter | Decision: keep Draco mesh/point-cloud codec integration as an optional native codec adapter. |
| `plugins/e57` | native-adapter | E57 reader/writer is a major external-format adapter with bundled `libE57Format` native/vendor code. Do not port the bundled library line-by-line. |
| `plugins/faux` | done | `kernels.fauxplugin` is a thin C++ plugin shell over the Rust C ABI kernel runner; plugin command discovery and the existing app plugin load test pass through Rust. |
| `plugins/hdf` | native-adapter | Decision: keep HDF integration as an optional HDF5 native dependency adapter. |
| `plugins/icebridge` | native-adapter | Decision: keep IceBridge as an optional domain/HDF native adapter. |
| `plugins/matlab` | native-adapter | Decision: keep MATLAB reader/filter integration as an optional external-runtime adapter. |
| `plugins/mbio` | native-adapter | Decision: keep MB-System bathymetry integration as an optional native/domain adapter. |
| `plugins/nitf` | done | `tools.nitfwrap` has a Nitro-backed native adapter for byte-preserving LAS/BPF wrap and unwrap workflows. `readers.nitf` and `writers.nitf` Rust stages run behind the C ABI: the reader uses `pdal_nitf_lidar_segment` plus a shifted `LasReader` (via `start_offset`) for the embedded LAS payload and exposes NITF header/TRE metadata through `pdal_nitf_read_metadata`; the writer plumbs `ftitle`/`fsclas`/`oname`/`ophone`/`idatim`/`iid2`/`aimidb`/`acftb`/security through `pdal_nitf_write`, defers LAS payload generation to `LasWriter` (writing to a temp file that gets wrapped), and supports `#` multi-view filename templating. The C++ plugin wrappers in `plugins/nitf/io/NitfReader.cpp` and `NitfWriter.cpp` are thin shims over those C ABI entries; `pdal_io_nitf_reader_test` and `pdal_io_nitf_writer_test` pass through Rust. Dead reader-side Nitro metadata helpers were removed from the plugin build; the remaining `NitfFileWriter`/TRE helper is tracked as a Nitro native adapter for writer option storage and TRE registration. |
| `plugins/openscenegraph` | native-adapter | Decision: keep OpenSceneGraph scene reading as an optional native scene adapter. |
| `plugins/pgpointcloud` | native-adapter | Decision: keep PostgreSQL PointCloud I/O as an optional database-backed native/service adapter. |
| `plugins/rdb` | native-adapter | Decision: keep RIEGL RDB integration as an optional proprietary/native adapter. |
| `plugins/rxp` | native-adapter | Decision: keep RIEGL RXP integration as an optional proprietary/native adapter. |
| `plugins/spz` | done | `readers.spz` and `writers.spz` have Rust fixture-backed implementations through `pdal-io`, and the C++ plugin classes are thin C ABI-backed reader/writer shells. Current local CMake has SPZ disabled, so C++ plugin tests are not built in this configuration; Rust SPZ tests and C++ wrapper syntax checks pass. |
| `plugins/teaser` | native-adapter | Decision: keep TEASER++/Eigen behavior as an optional native solver adapter. |
| `plugins/tiledb` | native-adapter | Decision: keep TileDB I/O as an optional array-storage native dependency adapter. |
| `plugins/trajectory` | native-adapter | Decision: keep trajectory fitting as an optional Ceres/Eigen/SuiteSparse native solver adapter. |

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

**This milestone is complete.** `819 / 819` baseline C++ GoogleTest cases
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
python3 rust/scripts/audit_cpp_port_backlog.py --include-plugins --top 40
python3 rust/scripts/audit_cpp_port_backlog.py --show holdout
```

The script classifies each mainline C++ file (`pdal/`, `filters/`, `io/`,
`kernels/`, `apps/`, `tools/`) and, with `--include-plugins`, optional plugin
files by `cloc` code LOC into:

- `c-abi-backed`: includes the Rust C ABI header (or is a known Rust bridge
  header). A header inherits its sibling `.cpp`'s category, so interface headers
  over a Rust-backed implementation are not counted as backlog.
- `native-adapter`: C++ bindings over an external native library, platform API,
  or remote/vendor adapter path whose Rust home is `pdal-native`/FFI or a later
  remote I/O adapter (e.g. `pdal/private/gdal/`, GDAL VSI, GDAL grid/density OGR
  adapters, lazperf compatibility, Arbiter-backed STAC traversal, dynamic
  library loading, and backtrace adapters), not a from-scratch port.
- `holdout`: a documented intentional C++ holdout (the `StreamCallback`
  callback ABI and the `ProgramArgs` compatibility glue).
  Keep this list small and cited in the script.
- `port-candidate`: pure C++ with no C ABI reference — the actionable backlog.

Current snapshot (mainline, excluding `test/`, `vendor/`, and optional
`plugins/`):

| category | LOC | files |
|---|---:|---:|
| port-candidate | 0 | 0 |
| c-abi-backed | 45,195 | 398 |
| native-adapter | 6,195 | 59 |
| holdout | 6,670 | 64 |
| total | 58,060 | 521 |

Port-candidate backlog by area: `pdal`, `io`, `filters`,
`kernels`, `apps` and `tools` are now at 0 (apps is a thin entry-point
peer; the only `tools` entry the audit had been counting was the in-tree
GoogleTest `tools/nitfwrap/NitfWrapTest.cpp`, which is behavioral contract, not
implementation — the audit now excludes files including `pdal_test_main.hpp`).
With `--include-plugins`, optional plugin port-candidate backlog is also 0
(`45,778` C ABI-backed LOC, `32,721` native-adapter LOC, `6,670` holdout LOC,
`85,169` total LOC): plugin tests are excluded, C ABI-backed checkpoints are
tracked as backed, and dependency-bound plugin integrations are tracked as
native adapters rather than line-by-line Rust rewrite work.
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
`GzipDecompressor` methods and routes both zlib-format deflate and gzip/zlib
auto-detect decompression through Rust. Public C++ typedef, exception,
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
backlog unless they route through Rust or are proved dead. The private LAS
summary counter/bounds helper now routes through Rust behind its C++ compatibility
class. The private COPC info payload decoder also routes through Rust while
preserving the C++ `copc::Info` struct for callers. EPT key parsing,
stringification, and bisection math now route through Rust while preserving the
exported C++ value type fields and operators. The private LAS tile byte buffer
and cursor now route through Rust behind the C++ `las::Tile` shell. Obvious
exported C++ `BufferReader`, private C++ EPT artifact/table/layout adapters
(`Artifact`, `FixedPointLayout`, `VectorPointTable`), and exported C++
SDK shells under `pdal/` (point/table handles, reader/writer bases, DB bases,
dimension/mesh/quick-info/artifact helpers, subcommand shells, and
`Utils::Random` with its `std::mt19937&` API) are likewise compatibility
holdouts. `PipelineManager` and the deprecated `PipelineExecutor` are now also
tracked as intentional C++ SDK compatibility surfaces rather than portable
implementation backlog: Rust owns an independent pipeline graph/executor through
`pdal_pipeline_*`, while these exported C++ classes expose mutable `Stage*`,
`PointTable`, `PointViewSet`, logging, and manager-reference APIs for existing
SDK callers and execute C++ `Stage*` objects (which cannot route through Rust
without unifying the stage model). `PipelineReaderJSON`'s JSON
parsing/validation now routes through the Rust `pdal_pipeline_reader_parse_json` C ABI
(`pdal-capi::pipeline_reader_abi`): Rust owns JSONC comment stripping,
root-structure validation, per-stage `type`/`tag`/`inputs` validation, and
reader/writer/filter classification, returning a pre-validated descriptor array;
the C++ wrapper keeps only the C++-object work (glob, plugin loading,
`FileSpec`/`Options` construction, `makeReader/Writer/Filter`, input wiring).
Malformed-JSON error text differs from C++ nlohmann (no test pins it).
The private EPT `EptInfo` SRS user-input building (wkt, or authority +
horizontal [+ vertical]) now routes through the Rust `pdal_ept_srs_wkt_from_info`
C ABI (`pdal-io::ept::ept_srs_wkt`), with the C++ rules and error messages
preserved in one place. With that, the `io` port-candidate backlog is 0:
`io/private/ept/TileContents.{cpp,hpp}` is reclassified as a native adapter,
matching the `io/private/stac` precedent. Its entire read path goes through the
Arbiter-backed `connector::Connector` (`m_connector`), and the C++ `EptReader`
only reaches it on the remote fallback. `EptReader::ready()` routes every local
read through the Rust `pdal_reader_create_ept` path, including 2D query bounds
with an SRS. Rust EPT now accepts and reprojects 2D query bounds with an SRS,
matching the existing transformed 3D bounds path.
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
the port, is `964 / 1069` currently built C++ GoogleTest cases, or `90.18%`;
compute that with `--include-added-tests`. That number is informational because
many added guard tests intentionally still exercise C++ compatibility seams,
native-adapter optional plugins, or non-port holdouts; do not use it as the
headline parity denominator.

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

Current test-suite size: `29,965` C++/header code LOC under `test/`. These tests remain
the behavioral contract and should not be counted as unported implementation.

Current C++ compatibility wrapper/adapter surface, using the same
`audit_cpp_port_backlog.py` classification as the implementation backlog, is
`45,195` c-abi-backed code LOC across `398` first-party files, plus `6,195`
native-adapter LOC and `6,670` documented holdout LOC. This is a coarse ceiling
because several files still mix compatibility shells, native-adapter work, and
Rust C ABI calls; the number should shrink only when wrappers are split from
implementation or a holdout/native-adapter decision changes.

Next implementation-replacement checkpoint: all portable first-party C++
implementation has moved behind the Rust C ABI, leaving only compatibility
glue/wrappers and documented intentional C++ holdouts. This is separate from
final port completion: the C++ tests can pass before every portable
implementation is replaced, and final completion still requires packaging,
install/export, CI, performance, platform, and plugin decisions.

Current remaining C++ port-candidate ceiling for that checkpoint, excluding
C++ tests and vendor, is `0` code LOC across `0` files in the main first-party
surface (`pdal/`, `filters/`, `io/`, `kernels/`, `apps/`, `tools/`) and remains
`0` when optional `plugins/` are included. This does not mean the port is
complete: mixed wrapper files may still contain legacy compatibility behavior,
native adapters remain intentionally native, and documented holdouts remain C++
until the C++ SDK compatibility surface is no longer required or a specific
replacement design is approved.

Do not use the older coarse manual LOC estimate as a progress metric. The
authoritative measurement is now
`rust/scripts/audit_cpp_port_backlog.py --include-plugins`, which folds
interface headers into their backed `.cpp`, separates native adapters and
documented holdouts, excludes C++ tests, and reports `0` port-candidate LOC.
If future work needs a different classification, update that script and this
status note together.

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
  `SrsTransform::get()` remains a C++/OGR compatibility pointer for consumers
  like `OGRGeometry::transform`; GeoTIFF VLR decoding and pre-1.4 writer
  encoding route through `pdal-native` libgeotiff.
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
- `pdal_filters_colorinterp_test`: `minmax`, `missingz`, `badramp`,
  `cantstream`, `autorange`, `k`, and
  `mad` count. Color interpolation execution routes through the Rust C ABI.
  Missing-dimension validation and streamability checks remain C++ wrapper
  behavior. `filters.colorinterp` is now also registered in the pipeline
  registry, so `pdal pipeline` runs it: the Rust filter is self-sufficient for
  the registry path -- it resolves the named built-in ramps (`pestel_shades`
  etc.) by decoding their embedded PNG bytes (the data moved to
  `pdal-filters::colorinterp_ramps`, with `pdal_colorinterp_default_ramp`
  delegating to it and the `png` crate decoding to RGB bands), computes auto
  `minimum`/`maximum` (plus the `k`/`mad`/`mad_multiplier` modes) when bounds
  are NaN, and declares Red/Green/Blue (uint16) via `output_dimensions()`. With
  this, every first-party filter with a Rust implementation is registry-visible.
  The C++ `ColorinterpFilter` now **delegates** ramp resolution and
  minimum/maximum/k/MAD computation to that Rust filter (the C ABI
  `pdal_stage_create_colorinterp` gained `mad`/`mad_multiplier`/`k` params), so
  the `openRamp`/`/vsimem` dance, the GDAL band reads in `ready()`, and the
  C++ `filter()` stats pass were removed (the file dropped from ~205 to ~108
  code LOC). The `autorange`/`k`/`mad`/`minmax` cases of
  `pdal_filters_colorinterp_test` now gate the Rust bounds math directly.
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
  through the Rust C ABI. Stage creation remains C++ factory behavior. The
  uint64 H3 index is now carried losslessly end-to-end: Rust stores it via the
  exact `PointView::set_u64` path (not `u64 as f64`), and the
  `RustViewConverter` readback uses the typed `pdal_point_view_get_u64` C ABI
  for `Unsigned64` dimensions so the low bits survive Rust -> C ABI -> C++.
  `stream_test_2` was strengthened from a no-assertion smoke into a real parity
  check: it requires resolution-12 indexes whose low bits an `f64` mantissa
  cannot hold, which fails on the old `u64 -> f64` path and passes on the typed
  path. With the typed setter in place, `filters.h3` is now registered in the
  pipeline registry (`FILTER_DRIVERS` + `create_filter`), with
  `output_dimensions()` declaring the uint64 `H3` dim; `pdal pipeline` runs
  `filters.h3` end-to-end (verified against `las/test_utm17.las` producing a
  well-formed resolution-12 cell). The C ABI registry tests cover the
  lossless-index path and the required-`resolution` error.
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
  The direct C++ wrapper still prepares the expression mask before constructing
  the Rust stage, while the Rust pipeline registry now parses and evaluates
  expression mode inside `DividerFilter` for JSON/registry pipelines.
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
  enhanced SRS VLRs, EPSG authority-node preservation in LAS 1.4 WKT/libLAS
  SRS VLRs, configured extra bytes, discard-high-return handling, auto scale/offset, legacy header count zeroing, and
  supported header options route through the Rust C ABI for the gated subset. C++ header inspection
  tests remain legacy. The GDAL-version-sensitive
  `pdal_wkt2_with_derivedprojcrs_vlr` case now passes by honoring explicit SRS
  VLR bytes from the C++ compatibility wrapper instead of regenerating forms
  that the CRS cannot express in that toolchain.
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
  simple GDAL options, typed output, dataset metadata, `gdalopts` creation
  options, fixed-grid validation, SRS override/default conflict validation, and
  no-point error behavior. Metadata on streaming tables remains C++ wrapper
  behavior.
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
- `pdal_io_ept_reader_test`: `inspect`, `inspectBounds`, `fullReadLaszip`,
  `fullReadBinary`, `fullReadZstandard`, `boundedRead2d`, `boundedRead3d`, `resolutionLimit`,
  `originReadVersion1_0_0`, `originRead`, `badOriginQuery`,
  `unreadableDataFailure`,
  `unreadableDataIgnored`, `unreadableDataIgnoredStreaming`,
  `unreadableTileFailure`, `unreadableTileFailureStreaming`,
  `badTilePointCountLaszip`, `badTilePointCountBinary`, `boundedCrop`,
  `polygonAndBoundsCrop`, `boundedCropReprojection`, `ogrCrop`, `bcbfToLonLat`,
  `bcbfToLonLat2dBounds`, and `duplicateInputs` count.
  Local EPT point
  materialization, simple dataset-coordinate bounds, depth pruning by
  `resolution`, origin selection, zstandard decompression, missing-tile error
  handling (both fail-fast and `ignore_unreadable`), corrupted-tile and
  hierarchy-vs-actual point-count failure detection, no-spatial-filter preview
  (bounds, point count, srs, dim names with laszip class-flag expansion),
  same-SRS and true cross-SRS bounds-backed preview upper-bound counts and
  clipped preview bounds, local streaming over Rust-materialized binary,
  LASzip, and zstandard views, same-SRS polygon cropping,
  EPSG:4326-to-source polygon reprojection, GeoJSON OGR polygon crops,
  reprojected 3D bounds filtering for BCBF data, bad-origin validation,
  multi-input diamond pipelines, and the three `*Stream` cases (`binaryStream`,
  `laszipStream`, `zstandardStream`) route through the Rust C ABI. Streaming
  works because the Rust reader stamps
  `EptNodeId`/`EptPointId` on each tile's full point set before bounds and
  polygon filters, then republishes an `ept::Artifact` (hierarchy step,
  per-tile `Overlap` entries, root bounds) into the C++ table's
  artifactManager so downstream stages — especially
  `writers.ept_addon` — keep working with the Rust read path.
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
  GeoJSON point and MultiPoint output routes through the Rust C ABI, including
  the covered `WRITE_BBOX` and `COORDINATE_PRECISION` options. Plain Shapefile
  and GeoPackage point output, attribute fields, Shapefile MultiPoint grouping,
  and Shapefile measured point output also route through the Rust native
  GDAL/OGR adapter. The multicount/attr_dims combination check, attr_dims
  missing-dimension error message, and RFC7946 unsupported-SRS error are
  formatted by the Rust C ABI before the C++ wrapper rethrows them via
  `Stage::throwError`. Native OGR layer creation options route through the
  Rust GDAL/OGR adapter for covered Shapefile and GeoPackage paths, including
  transaction start/commit when the driver supports it.
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
- `assign` (simple `Dim[range]=value` assignments, the `condition` DimRange,
  and the expression-based `value` option)
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
  and from the expression-based `value` option (`expr::AssignStatement`).
  Registry-built assign filters advertise new assignment-target dimensions,
  prepare statements against the prepared layout, and evaluate statements in
  order so later `value` expressions can see earlier assignments.
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
