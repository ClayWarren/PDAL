# Rust Port Status

This is still a work in progress and is not yet at full feature parity with
C++ PDAL. Bugs may exist. Check this list before assuming a behavior is
intentional or before asking an agent to broaden the port.

Closed decisions live in `rust/DECISIONS.md`. When this status file says a
bucket is a native adapter, compatibility shell, or holdout, that ledger is the
source of truth for whether it should be ported, FFI-bound, left in C++, or
deferred.

For draft-PR review, `rust/HANDOFF.md` is the concise evidence packet. This file
remains the detailed status ledger.

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

Those numbers are guardrails, not the finish line. The implementation backlog
for the current first-party scope is closed; the work left is upstream
readiness and policy for the accepted boundaries below:

- OGR/vector-source breadth: covered local vector reads/writes, tindex
  create append/dedup with idempotent OGR field setup, and tindex merge reads
  are Rust-backed. Broader exotic OGR datasource/update behavior is a
  native-adapter boundary unless a concrete parity failure reopens it.
- EPT/COPC/STAC/remote breadth: covered EPT preview filters, local
  EPT/COPC/STAC workflows on non-Windows, selected remote/VSI/object-store
  paths, and structural STAC validation for covered item/catalog/collection
  link and asset shapes are Rust-backed. Windows full STAC execution currently
  stays on the C++ native path because the nested Rust reader path hit an MSVC
  stack overflow in CI; Rust STAC preview/probing remains active there.
  Arbiter-specific connector options, custom schema URL resolution, broad
  threaded crawling, and cloud credential workflows are deferred/native-adapter
  boundaries, not unplanned Rust implementation backlog.
- Metadata, pipeline, and command structural parity: covered workflows route
  through Rust. `PipelineWriter` serialization is semantically aligned with
  installed PDAL for covered JSON pipelines; byte-for-byte JSON object key order
  is intentionally not the contract. XML and future command artifact nits are
  regression bugs to file against the done surface, not broad open buckets.
- Packaging and platform readiness: install/export, release packaging,
  workflow/platform policy, license/vendor accounting, and downstream C ABI
  consumption must be stable enough for upstream use.
- Optional plugins: completed checkpoints are listed below; other plugins are
  native adapters or deferred until a versioned plugin boundary exists.
- Final quality gates: `rust/DECISIONS.md` records which gates are
  release-blocking and which remain visibility gates. Keep release-blocking
  gates green and refresh visibility evidence when behavior or build shape
  changes.

When continuing from here, do not restart old directory sweeps or add
placeholder Rust modules. Work only on a concrete regression, upstream-readiness
gate, packaging/platform failure, policy decision, or accepted native-adapter
milestone.

## Feature Status

| Feature | Status | Notes |
|---|---|---|
| Rust core point model | done | `PointLayout`, `PointView`, dimension IDs/types, source indices, per-view SRS text, mesh faces, and 2D/3D bounds exist and support all current counted parity paths. Future expansion should be driven by a concrete stage/I/O/command parity case, not by mirroring C++ core files. |
| Rust stage model | done | Filter, reader, writer, and streamable traits exist for the current port surface. Multi-input `run()` behavior is present and `filters.merge` handles DAG inputs without redundant points. Future trait growth should come from a concrete parity case. |
| Rust pipeline graph | done | Reader/filter/writer DAG execution, tags, dependencies, roots/leaves, cycles, shared-input DAGs, `where`/`where_merge` splitting, metadata aggregation, summaries, error propagation, and conservative streaming fallback exist in Rust. Filename-string and FileSpec endpoint JSON stages infer reader/writer roles through the shared descriptor parser, and consecutive readers implicitly feed the next filter/writer with installed-PDAL serialization parity for covered multi-reader pipelines. The exported C++ `PipelineManager`/`PipelineExecutor` classes remain compatibility holdouts for SDK callers; remaining parser/serializer byte-shape caveats are tracked under Pipeline JSON parsing. |
| Options | done | String-keyed typed getters, stable command-line argument emission, duplicate-key preservation, conditional add, multi-option merge, conditional merge, remove, replace, JSON object option parsing with JSONC-style comments at the option-file boundary, null-to-empty pipeline option parity, pipeline-stage option parsing, command-line option text parsing, and C++ `Options` copy/assign/move/lifecycle plus `Options::fromFile` JSON/command-line dispatch route through Rust for the covered option-file shapes. The C ABI `ProgramArgs` parser covers the pre-port test surface plus focused bool/double extensions, including long bool defaults, default inversion, explicit `--flag=true`/`--flag=false` values, double precision, and JSON-safe `NaN` handling. The remaining public C++ `ProgramArgs` class is compatibility glue and is tracked as an intentional holdout, not port-candidate implementation. |
| Metadata | done | Typed scalar metadata trees, descriptions, instance/array node kind preservation, scalar JSON helpers, base64 scalar decoding, structural JSON serialization, and flat metadata JSON arrays for repeated children route through Rust, and the full pre-port `pdal_metadata_test` surface counts through the Rust C ABI. Pipeline serialization uses metadata/FileSpec/SRS helpers through the C ABI where covered; exact `PipelineWriter` byte-shape parity is tracked under Pipeline JSON parsing. |
| Spatial reference | done | `SpatialReference::set` (non-WKT user input and WKT1 to WKT2 normalization), `prettyWkt`, `getWKT1`, `getProj4`, `getPROJJSON`, `equals` (semantic IsSame fallback), `identifyHorizontalEPSG`, `identifyVerticalEPSG`, `getUTMZone`, `getHorizontal`, `getVertical`, `getHorizontalUnits`, `getVerticalUnits`, `isGeographic`, `isGeocentric`, `isProjected`, `getAxisOrdering`, and `valid` route through a Rust GDAL/OSR adapter (`pdal_srs_user_input_to_wkt`, `pdal_srs_wkt_to_wkt1`, `pdal_srs_wkt_to_wkt2`, `pdal_srs_pretty_wkt`, `pdal_srs_wkt_to_proj4`, `pdal_srs_wkt_to_projjson`, `pdal_srs_is_same`, `pdal_srs_identify_horizontal_epsg`, `pdal_srs_identify_vertical_epsg`, `pdal_srs_get_utm_zone`, `pdal_srs_get_horizontal_wkt`, `pdal_srs_get_vertical_wkt`, `pdal_srs_get_horizontal_units`, `pdal_srs_get_vertical_units`, `pdal_srs_is_geographic`, `pdal_srs_is_geocentric`, `pdal_srs_is_projected`, `pdal_srs_axis_ordering`, `pdal_srs_valid`). `SrsTransform` default-axis, explicit-axis, point, and array transforms now call the Rust C ABI transform handle. LAS/COPC WKT1/WKT2/PROJJSON SRS VLR payload generation uses the Rust GDAL/OSR adapter for the covered writer paths. LAS GeoTIFF SRS VLR decoding and pre-1.4 writer encoding route through the `pdal-native` libgeotiff adapter. Vertical extraction uses a Rust WKT bracket-matching parser because GDAL's C API has no `OGR_SRSNode` equivalent. `SrsTransform::get()` remains an intentional C++/OGR compatibility pointer for consumers like `OGRGeometry::transform`, not open port backlog. |
| Spatial index | done | `pdal-core::SpatialIndex2d` and `SpatialIndex3d` use `rstar` R-trees for XY/XYZ nearest-neighbor and radius queries; `SpatialIndex3d::radius_dims` also indexes the common XY and XYZ dimension sets used by spatial filters, while keeping a generic exact scan for arbitrary non-coordinate dimension sets. The C ABI spatial query path uses those indexes for `KD2Index`/`KD3Index` compatibility searches, and the full pre-port `pdal_kdindex_test` surface counts through Rust. The exported C++ `KD*Index` classes remain API/ABI shells, but their implementation now builds a Rust point-view copy and dispatches through the Rust C ABI instead of nanoflann. A flexible-dimension optimized index can still be added if profiling shows the generic KDFlex path matters; that is performance follow-up, not open parity work. Do not bake one-off neighbor searches into new filters. |
| Thread pool | done | `pdal::ThreadPool` delegates scheduling, stop/restart, await, queue clearing, and resize behavior through the Rust C ABI while keeping the existing C++ facade. Rust ABI tests and the C++ `pdal_thread_pool_test` pass through that path. |
| Expressions | done | Conditional, math, and assignment parser/evaluator support the documented PDAL expression syntax, the current Rust expression/assign work, and all counted pre-port `pdal_filters_expression_test` cases through the Rust C ABI. Future expression additions should extend the shared Rust parser/evaluator first rather than adding C++-only parser behavior. |
| C ABI bridge | done | Rust-owned handles are the contract. Metadata, summaries, views, `where` view splitting, and pipeline calls are exposed. The public header carries `PDAL_CAPI_ABI_VERSION_*` macros plus exported component/packed version functions so downstream consumers can assert the ABI generation they were built against. `rust-capi-header-audit` keeps installed header declarations, implemented C ABI symbols, and Rust/header ABI version constants in sync. The installed `PDAL::CAPI` smoke and C++ test-parity audit are green; never pass C++ object pointers as Rust handles. |
| C++ filter wrappers | done | Safe ports use explicit Rust view conversion, and the C++ filter test binaries count as Rust C ABI-backed in the pre-port parity audit. Keep future wrappers on the same explicit conversion path. |
| Filter ports | done | 84 first-party filter/static stage files exist in C++; the portable filter implementation backlog is now at 0 port-candidate LOC. Remaining C++ under `filters/` is wrapper/compatibility surface plus the exported `filters/private/Point` OGR geometry adapter used by C++ option parsing (`filters.crop` centers and `filters.normal` viewpoint). `filters.crop` accepts WKT and GeoJSON Polygon/MultiPolygon option strings through the Rust registry path and rejects non-polygon geometries like installed PDAL. `filters.delaunay` is registry-visible and attaches the `delaunay2d` mesh through the existing Rust triangulation helper. `filters.geomdistance` registry construction supports inline geometry plus OGR layer/SQL geometry sources for covered local vector datasources. Any remaining broad workflow gap belongs to pipeline/I/O/native-adapter parity, not to unported filter implementation. |
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
| LAS/LAZ I/O | done | `las`/`laz` crate path supports standard dimensions, V1.0-1.4 point formats, Extra Bytes (VLR and user `extra_dims`), `start`/`count`/`nosrs`/`srs_vlr_order`/`ignore_vlr` reader options, WKT/PROJJSON SRS extraction, GeoTIFF SRS extraction through the `pdal-native` libgeotiff adapter with EPSG-only fallback, compression/decompression, full-file GDAL VSI URL reads, and core writer header options with C++-style validation for malformed numeric/UUID values plus LAS major/minor version, global encoding, and creation day-of-year ranges. Direct Rust C ABI reader/writer constructors are covered, and the C++ `LasReader`/`LasWriter` wrappers route local read/write through Rust. LAS 1.4 WKT/libLAS SRS VLR writing preserves EPSG authority nodes through the Rust GDAL/OSR adapter, and LAS 1.0-1.3 SRS writing emits GeoTIFF GeoKey VLRs through the libgeotiff adapter. The writer rejects covered multi-view writes with mixed point spatial references, including empty-vs-nonempty SRS mixtures. Single-file LAS and LAZ writer output is streamable; `#` multi-file fan-out intentionally stays materialized because PDAL splits by input view. Rust LAS-focused tests pass (168 run, 1 ignored network smoke), and `pdal_io_las_reader_test`/`pdal_io_las_writer_test` pass against the current build. |
| COPC I/O | done | Local `.copc.laz` full-file and streaming reads plus no-filter `inspect()` metadata route through the LAS/LAZ path, with post-read 2D/3D bounds, same-SRS polygon filtering, polygon reprojection for the covered EPSG:4326 crop, and GeoJSON OGR polygon crop support for the existing fixture shape. GeoJSON OGR polygon datasource input can open local files and GDAL VSI/HTTP(S) text sources. A first-party COPC hierarchy walker (`pdal-io::copc_hierarchy`) parses the COPC info VLR and walks hierarchy/sub-hierarchy pages over either local files or the `pdal-native::vsi::VsiFile` byte-range adapter, applying 2D/3D bounds and `resolution` pruning that matches the C++ `depthEnd = max(1, ceil(log2(spacing/resolution)) + 1)` math. Resolution-limited execution materializes only the kept LAZ chunks by reading the LAZ chunk table to map each hierarchy entry's file offset to a `(start_point_idx, count)` range and streaming the LAS reader past unwanted records (the `laz` crate's variable-chunk seek is buggy, so we deliberately stay on a sequential read). The C++ `CopcReader::inspect()` routes bounds/resolution previews (no polygons/OGR) through the Rust `pdal_copc_preview` C ABI, which is what makes `pdal_io_copc_remote_reader_test.vsi` count. `writers.copc` is a first-party Rust COPC writer behind the C ABI and the C++ stage delegates the full write to it, including scale/offset, SRS, extra dimensions, selected header options, user/forwarded VLRs, and metadata/pipeline VLRs. Broad OGR datasource coverage belongs to the vector/native-adapter caveat, not to COPC implementation backlog. Rust COPC-focused tests pass (48 run, 2 ignored network smokes), and `pdal_io_copc_reader_test`/`pdal_io_copc_writer_test`/`pdal_io_copc_remote_reader_test` pass against the current build. |
| EPT reader | done | Local LASzip, uncompressed binary, and zstandard EPT full-file reads walk JSON hierarchy and merge local tiles. GDAL VSI-backed EPT JSON/hierarchy/tile reads work for LASzip, binary, and zstandard tile payloads; the LASzip path is covered by the STAC mixed-reader workflow, while binary/zstandard have targeted `/vsimem/` coverage. Resolution limits and query bounds prune hierarchy nodes before tile reads; origin, same-SRS polygon, SRS-bound polygon reprojection, transformed 2D/3D bounds filters, and GeoJSON OGR polygon crops are applied after tile reads. Authority-form EPT SRS metadata (`authority`/`horizontal`/`vertical`) feeds read output, metadata, previews, and bounds filtering through the same Rust helper as WKT-form SRS. Resolution-limited, same-SRS bounds, semantically same-SRS and true cross-SRS transformed bounds clipping/counts, origin, local polygon, and local GeoJSON OGR polygon `inspect()` preview route through the Rust C ABI and use the same hierarchy count as the Rust reader. The C++ `inspect()` wrapper tries that Rust preview path for GDAL VSI/HTTP(S)-readable EPT JSON before falling back to the legacy C++ preview. GeoJSON OGR polygon datasource input can open local files and GDAL VSI/HTTP(S) text sources. Tile point counts are validated and `ignore_unreadable` can skip unreadable tiles, with the C++ wrapper routing through the Rust path even when `ignore_unreadable` is set (an empty view is returned when every tile is skipped). Local binary EPT addon overlays are read through Rust for the existing addon round-trip checks, and the C++ `EptAddonWriter::writeOne` delegates its per-addon binary chunk writes, hierarchy JSON emission, and `ept-addon.json` metadata write to the Rust `pdal_ept_addon_write` C ABI. Rust EPT-focused tests pass (62 run), and `pdal_io_ept_reader_test`/`pdal_io_ept_addon_writer_test` pass against the current build. Broader remote catalog traversal belongs to the STAC/remote rows, not to EPT implementation backlog. |
| FBI I/O | done | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including reader header summary, dimension discovery, point reads, GDAL VSI/HTTP(S) byte-source opening, and basic writer round-trip coverage. Rust preserves XYZ through writer round-trips; installed PDAL 2.10.1's legacy writer shifts those coordinates, so byte-for-byte installed writer parity is intentionally not claimed. |
| TerraSolid reader | done | Existing C++ reader unit-test shapes pass through the Rust-backed path for deterministic TerraSolid format 1/2 fixtures. Reader input can open local files and GDAL VSI/HTTP(S) byte sources and is streamable in the Rust pipeline executor. `.bin` is intentionally not inferred because it conflicts with FBI. |
| Optech reader | done | Existing C++ reader unit-test shapes pass through the Rust-backed path, including deterministic Optech CSD fixture data and localized WGS84 georeference math. Reader input can open local files and GDAL VSI/HTTP(S) byte sources and is streamable in the Rust pipeline executor. |
| BPF I/O | done | Existing C++ reader/writer unit-test shapes pass through the Rust-backed path, including uncompressed and compressed point/dimension/byte interleaves, preview point count/SRS/header bounds/dimension labels, scaling, flex filenames, output dimensions, auto UTM, header data, bundled-file metadata, and ULEM/polar trailing-section skipping so later header metadata is read from the right offsets. Reader input can open local files and GDAL VSI/HTTP(S) byte sources. |
| GDAL reader/writer | done | Existing C++ GDAL reader unit-test shapes pass through the Rust-backed path for local raster-to-point-cloud behavior, and the Rust reader can stream raster rows through GDAL windowed reads. Standard-mode C++ GDAL writer cases with simple GDAL options route raster rendering through the Rust C ABI for core grid statistics, including Float32/Float64 and signed/unsigned integer raster output types, comma-separated GDAL dataset metadata, `pdal_metadata`/`pdal_pipeline` metadata on view-backed tables, GDAL creation options (`gdalopts`), `bounds` grid sizing, alternate-grid conflict validation, `override_srs`/`default_srs` conflict validation, and empty-view error behavior. Targeted Rust regression coverage and `pdal_io_gdal_reader_test`/`pdal_io_gdal_writer_test` pass against the current build. The remaining C++ `readyTable` metadata collection is compatibility glue around the C++ `PointTable`; serialization/base64 and final raster writing route through Rust-backed helpers. This is not broad GDAL/PROJ permission. |
| Raster writer | done | Raster attachments on Rust point views write through the Rust C ABI, and `pdal_filters_faceraster_test` exercises the Rust-backed `writers.raster` wrapper. Named/multi-raster output plus the C++ `data_type`, `gdalopts`, and no-data option shapes route through Rust for the covered GDAL raster attachment paths. |
| TIndex reader | done | GeoJSON tile-index reads route through the Rust C ABI from local files and GDAL VSI/HTTP(S) text sources, and the C++ wrapper uses that path for simple indexes with optional bounding-box filtering. FileSpec headers are inherited for GeoJSON index reads and referenced asset-reader dispatch. GeoJSON indexes with `where` or SQL route through the Rust OGR/GDAL path instead of falling back to C++, so filtering semantics come from the same adapter as native OGR indexes. Basic local OGR datasource reads use the Rust GDAL adapter for layer-0 or named-layer string location fields, optional `srs_column` SRS overrides, OGR `where` attribute filters, OGR SQL result layers, geometry-envelope bounds filtering, same-SRS WKT polygon filters, C++-parsed OGR polygon specs forwarded as WKT, `filter_srs` polygon reprojection when a target SRS is known, `t_srs` reprojection for indexed files with source SRS metadata, and object-or-array `reader_args` by reader type. The `tindex create` command path can append/dedup existing local OGR tile indexes, treats pre-existing OGR fields as idempotent, and accepts C++-style equals forms for the covered create/merge switches. Rust TIndex reader/kernel tests and `pdal_tindex_test` pass against the current build. Broad native vector datasource edge cases belong to the OGR/native-adapter caveat, not to TIndex implementation backlog. |
| STAC reader | done | Local STAC Item/Catalog/Collection/FeatureCollection traversal can read local and covered remote assets through already-ported readers on non-Windows. Preview supports item/catalog/date/bounds/collection/property pruning, local structural validation for the schema-flag fixture shapes, GeoJSON OGR-boundary bounds for covered Polygon/MultiPolygon feature collections, basic local OGR datasource boundary bounds, and native OGR SQL predicate bounds through the Rust GDAL adapter. Execution supports item/catalog/bounds/OGR/property filters, collection filtering including C++ `item_ids`/`catalog_ids`/`collection_ids` synonyms, and object-or-array reader-specific args for local supported STAC documents, including the existing mixed EPT/COPC catalog workflow. With `validate_schema=true`, the Rust path validates the covered structural fields for items/catalogs/collections/feature collections, including asset hrefs and required string `rel`/`href` link entries before traversal. The C++ wrapper's support probe uses the Rust/VSI-aware STAC type parser, so supported remote or `/vsi*` STAC documents can be identified without local-only C++ JSON probing; Windows full execution currently stays on the C++ native path after the nested Rust reader path hit an MSVC stack overflow in CI, while Rust preview/probing remains active there. Mixed-reader merges in Rust normalize to a union layout before appending views. Remote ranged COPC reads use GDAL VSI-backed byte sources for hierarchy, chunk-table, and ranged point materialization. Custom schema URL validation intentionally stays on the C++ schema-validator native-adapter path. Rust STAC-focused tests and `pdal_io_stac_reader_test` pass against the current build. Full remote JSON-schema resolution, broad remote traversal beyond covered Rust/VSI-readable documents, Windows full-execution Rust re-enablement, and threaded catalog crawling belong to the remote/native-adapter caveat, not to STAC implementation backlog. |
| Driver inference | done | Rust infers existing PDAL reader/writer names from filenames, including special path forms, and C ABI registry construction fails cleanly for inferred-but-unported drivers. |
| Pipeline JSON parsing | done | PDAL-style JSON arrays/root `pipeline` objects, JSONC-style comments, filename string and FileSpec endpoint stages, scalar/null/object option values, default linear dependencies, optional `tag`/`inputs`, generated tags, inferred serialized `inputs`, and framework `where`/`where_merge` options work for command readiness. C++ `PipelineReaderJSON`, command validation/serialization, registry pipeline construction, kernel serialization, and the C ABI JSON serializer share the same descriptor validator before building stages. Covered serialization normalizes explicit string `inputs`, numeric/bool stage option strings, and JSON-valued polygon/FileSpec/SRS option strings; covered execution also decodes FileSpec `filename` objects and JSON-valued FileSpec `filename` strings to their `path`, and `pdal pipeline --progress` reports FileSpec writer paths as file-level progress targets instead of falling back to pipeline-level progress. Invalid typed options and unexpected `filters.assign` options are rejected instead of silently defaulting. Shared-input DAG serialization follows C++ `Stage::serialize` recursion for covered JSON pipelines, including repeated shared ancestors, and terminal multi-writer branches are all emitted instead of only serializing the final stage. Branch-specific layout mutation into `filters.merge` executes without panicking. `pdal pipeline --validate` performs a conservative Rust prepare-layout pass: stages with known reader layouts (for example `readers.faux`) catch missing-dimension prepare errors such as invalid `filters.assign` targets, while unknown reader layouts are allowed through instead of inventing false validation failures. Installed-PDAL comparison for the representative serialization fixture is semantically equal; byte-for-byte JSON object key order is not treated as a contract. |
| `pdal-rs` command shell | done | Rust-native shell lists Rust-backed stages/commands and delegates first-party command execution to the same Rust C ABI kernel runner used by the C++ app/kernel wrappers. It is intentionally not the installed `pdal` executable; installed CLI behavior is tracked by the C++ app shell and implemented-command rows. |
| Command metadata | done | `--drivers`, `--list-commands`, and `--options <stage>` are backed by Rust-owned metadata for the implemented Rust surface, including JSON output and scoped option tables for covered readers, filters, and writers. |
| C++ `pdal` app shell | done | `apps/` is a single file (`apps/pdal.cpp`, ~345 LOC) and the port backlog audit reports it at **0 port-candidate LOC** -- it is a thin entry-point peer over the Rust C ABI with no portable implementation left. Every piece of behavior/data routes through Rust: version (`pdal_version_string`), driver listing (`pdal_stage_list_json` -> `pdal_rust_stage_list_json`), command listing (`pdal_kernel_list_json` -> `pdal_rust_kernel_list_json`), stage option metadata (`pdal_stage_options_text`/`_json`), kernel dispatch (`pdal_kernel_run`, with a Rust dispatch guard for every first-party command), the unknown-command message (`pdal_app_unknown_command_message`), and log line prefixes (`pdal_log_format_prefix`). What remains in C++ is the intentional entry-point glue PORTING.md designates as the thin executable peer: `ProgramArgs` CLI parsing, terminal-table layout for `--drivers`/`--list-commands` (formatted from Rust JSON), log-sink selection (`Log::makeLog`), and the debug `SIGSEGV` backtrace handler. All 4 `pdal_app_test` cases (`option_file`, `load`, `log`, `listCommands`) pass through Rust, and a built-binary smoke confirms `--version`/`--drivers` (124 stages)/`--list-commands` (16 command entries, including `kernels.fauxplugin`)/dispatch/unknown-command parity. Reopen only if a command's deeper output shape is found to diverge from installed PDAL. |
| Implemented commands | done | All 15 first-party C++ kernel commands (`chamfer`, `delta`, `density`, `eval`, `ground`, `hausdorff`, `info`, `merge`, `pipeline`, `random`, `sort`, `split`, `tile`, `tindex`, `translate`) are Rust-dispatchable through the C ABI and listed in Rust command metadata. They have installed-PDAL regression coverage for scoped workflows. `info` owns summary, metadata, stdin pipeline JSON, point lookup including lists/ranges, 2D/3D nearest query, stats with `--dimensions`/`--enumerate`/`--breakout`, schema, boundary via Rust `filters.hexbin`, all-mode schema/stat/metadata/boundary/STAC output, pipeline serialization, STAC `pc_type`, the existing STAC app guard, and remote LAS header/VLR/EVLR extraction through the Rust C ABI `pointless_las` helper. Local `info --stac` output emits the installed-PDAL STAC Feature top-level shape with geometry, bbox, assets, links, point-cloud schemas/statistics, and source projection bbox/geometry/WKT2/PROJJSON; broad remote/schema STAC parity belongs to the remote/native-adapter caveat. `tile` owns the existing app tests, including globbed input, text/LAS output, per-source reprojection to `out_srs`, writer text options, and the C++ single-`#` output-template validation. `tindex` owns the existing local GeoJSON create + bounds/polygon-filtered merge workflow with `filters.crop`, merge-time reprojection to `--t_srs`, stdin-fed create workflow, filelist create workflow, input-source conflict guard, invalid forwarded-filter diagnostic, GeoJSON stdout layer-description option, `--t_srs` layer SRS, `--a_srs` assignment/override behavior, C++ `--filespec`/`--smooth` create-option synonyms, bare `--skip_different_srs`, fast bbox boundaries, SRS mismatch warning/skip behavior, and exact hexer-driven boundary generation for explicit or auto-sized `--threshold`/`--resolution`/`--simplify` with optional `--where` point-expression filtering, plus create append/dedup and merge from local OGR tile-index datasources such as GeoPackage with named layers. GEOS topology-preserving simplification is applied through `pdal-native`. `sort` accepts the C++ writer switches (`--compress`/`-z`, `--metadata`/`-m`) plus the direct shell-forwarded `writers.las` option forms and maps them onto the Rust-built writer stage. `density` routes file inputs, JSON pipeline files, stdin JSON pipelines, and `.xml`-extension files containing JSON pipelines through the Rust kernel by appending a Rust-backed `filters.hexbin` stage; its Rust parser accepts the public C++ switch names (`sample_size`, `threshold`, `edge_length`, `hole_cull_area_tolerance`, `smooth`, `h3_grid`, `h3_resolution`) as well as explicit `filters.hexbin.*` options. `ground` compares per-point classification against installed PDAL (>=99.8% agreement on `interesting.las` with `cell=10`) after the Rust SMRF implementation gained the low-outlier mask, net cutting, KD-tree inpainting, and full validation. The Rust runner owns the full `GroundKernel` option surface (max_window_size/slope/cell_size/scalar/threshold/cut/returns/ignore mapped onto `filters.smrf`, `--reset` -> `filters.assign`, `--denoise` -> `filters.outlier`, `--extract` -> `filters.range`, accept-and-ignore max_distance/initial_distance and the basic switches) and never returns the -1 C++ fallback sentinel, so the behavior runs through Rust. The exported C++ `ChamferKernel`, `DeltaKernel`, `EvalKernel`, `GroundKernel`, `HausdorffKernel`, `InfoKernel`, `MergeKernel`, `PipelineKernel`, `RandomKernel`, `SortKernel`, `SplitKernel`, `TIndexKernel`, `TileKernel`, and `TranslateKernel` classes are **retained** as API/ABI shells; direct `execute()` paths for those shells dispatch through `pdal_rust_kernel_run` where their lower-layer behavior is Rust-backed, matching the app-path Rust behavior without deleting public symbols. `pdal delta` supports the direct C++ shell's `--detail` and `--alldims` options through `pdal_delta_ex`. The direct `PipelineKernel` shell routes normal execute, metadata, `--dims` output-dimension restriction, C++-style file progress markers, serialization, validate JSON output, hidden PointCloudSchema XML output, strict `--stream` enforcement, and `--nostream` materialized-execution selection through Rust. The direct `TranslateKernel` shell routes normal reader/filter/writer execution, JSON filter-extraction (`--json`), metadata output, `--dims` output-dimension restriction, pipeline serialization, strict `--stream` enforcement, `--nostream` materialized-execution selection, and the same-input `--overwrite` guard through Rust; serialized pipeline JSON is semantically aligned while byte-for-byte object key order is not a contract. The direct `TIndexKernel` shell routes parsed create/merge execution through Rust; broad OGR datasource/update parity beyond covered local tile-index reads belongs to the OGR/native-adapter caveat. `pdal_app_test`, `pdal_eval_test`, `pdal_info_test`, `pdal_tile_test`, `pdal_tindex_test`, `pdal_merge_test`, and Rust `pdal-capi` kernel ABI tests pass against the current build. `tools.lasdump` and `tools.nitfwrap` have Rust command paths for their scoped fixture-backed workflows. |
| Performance visibility | done | Ignored reporting harnesses exist for local I/O performance, binary size, startup time, memory, build cost, and opt-in full C++ vs Rust test-suite timing. Recorded results live in `rust/BENCHMARKS.md` for macOS arm64, version-matched 2.10.1. The earlier high-RSS pipeline regression was traced to redundant Rust executor copies, fixed with move/drop execution, in-place sort, and conservative streaming execution for linear streamable pipelines. The large LAS -> range -> LAS memory case now uses bounded memory through the Rust CLI path. The harnesses remain visibility tools, not hard release gates, until project policy says otherwise. |
| Rust coverage reporting | done | `pixi run -e dev rust-coverage` runs `cargo-llvm-cov` over the Rust workspace. The line-coverage threshold is enforced by `rust-coverage-check` inside `rust-guard`; keep the percentage in `pixi.toml` synced with the latest measured coverage. |
| Rust mutation testing | done | `pixi run -e dev rust-mutants` runs `cargo-mutants` when it is installed locally. This is an audit tool for mature buckets, not part of `rust-guard` until the project chooses a mutation-score release policy. |
| Unsafe Rust footprint | done | `pixi run -e dev rust-unsafe-audit` tracks source-only unsafe Rust accounting. Current first-party Rust count, excluding `rust/target`, is 377 `unsafe { ... }` blocks, 571 `unsafe extern "C" fn` exports, 88 total `unsafe fn` helpers, 2 unsafe extern callback type aliases, 0 unsafe extern blocks, and 1 `unsafe impl`. Unsafe remains concentrated in `pdal-capi`, `pdal-native`, and Rust callers of the C ABI; keep new unsafe at C/native boundaries or tests that exercise those boundaries. |
| Vendor/native strategy | done | `vendor/` has 11 top-level third-party dependency directories. `rust/VENDOR.md` describes the vendor boundary and `rust/DECISIONS.md` records the closed choices. Current decisions: `h3` -> `h3o`, `lazperf` -> `las`/`laz`, `eigen`/`gtest`/`nanoflann`/`nlohmann` are not direct Rust port targets, and `arbiter`/`kazhdan`/`lepcc`/`schema-validator`/`utfcpp` are adapter-bound or deferred to named milestones. Native GDAL/OGR/GEOS/PROJ/Nitro adapters belong in `pdal-native`; pure Rust replacements such as LAS/LAZ do not need to move through it. Reopen only for a concrete parity failure or a new native boundary decision. |
| Plugins | done | There are 18 top-level plugin directories. `rust/DECISIONS.md` records each plugin's current status. The optional plugin audit reports 0 port-candidate LOC: `faux`, `nitf`, and `spz` are completed C ABI-backed compatibility checkpoints, while all other optional plugins are native adapters/deferred by decision instead of unplanned pure-port backlog. A Rust plugin SDK and broad optional plugin sweep are intentionally later work, not remaining first-party port backlog. |
| Remote/object-store I/O | done | `pdal-native::vsi::VsiFile` opens local, URL, `/vsicurl/`, and documented object-store URL schemes through GDAL VSI (`s3://` -> `/vsis3/`, `gs://` -> `/vsigs/`, `az://` -> `/vsiaz/`) and implements `std::io::Read + Seek` so byte-range readers can stream over it. The Rust COPC hierarchy walker consumes the adapter end-to-end: `pdal_io_copc_remote_reader_test.vsi` (autzen-classified.copc.laz over both https and `/vsicurl/`) counts as Rust C ABI-backed. STAC remote JSON traversal, remote LASzip EPT reads, and `pdal info` remote pointless-LAS header/VLR/EVLR extraction consume the same adapter. Pipeline `FileSpec` filename objects propagate `query` maps into covered Rust HTTP/VSI/object-store paths by appending stable query parameters to the source URL. `FileSpec` `headers` are preserved by Rust options and applied as scoped GDAL VSI path-specific headers for LAS/COPC byte-source paths, EPT root/hierarchy/binary/zstd/LASzip tile reads, EPT addon/source-origin sidecars, STAC traversal plus asset-reader dispatch, and TIndex GeoJSON index/asset-reader dispatch. Broader Arbiter-style connector option parity and cloud credential workflows are deferred/native-adapter decisions, not unplanned Rust implementation backlog. |
| Broad kernels/apps/tools migration | done | `apps/`, `tools/`, and `kernels/` have 0 port-candidate LOC in the implementation-replacement audit. The C++ `pdal pipeline`, `pdal translate`, `pdal random`, `pdal density`, `pdal ground`, `pdal split`, `pdal sort`, `pdal merge`, `pdal delta`, `pdal tindex`, and simple `pdal tile` app paths execute through Rust for local reader/filter/writer workflows. `pdal-rs` routes every first-party command wrapper (`pipeline`, `info`, `translate`, `merge`, `sort`, `ground`, `density`, `random`, `split`, `tile`, `tindex`, and metric/eval commands) through that same Rust C ABI kernel runner after usage handling, so the CLI no longer carries parallel local command implementations. Direct exported C++ kernel classes remain public API shells and dispatch through Rust where their lower-layer behavior is Rust-backed. Standalone `lasdump` and `nitfwrap` dispatch through the Rust C ABI. Remaining command-output/workflow breadth belongs to the `Implemented commands` row, not to app/tool/kernel migration backlog. |

## Root-Level Migration Status

The Rust port is not complete just because Rust-backed tests pass. The root
build, install, packaging, CI, examples, and docs must also describe and verify
the Rust-backed shape of PDAL.

| Area | Status | Notes |
|---|---|---|
| Root CMake | done | `libpdal_capi.a` is built, linked into `pdalcpp`, and sourced from `cmake/rust.cmake` so the dependency list tracks every current Rust crate that can affect the C ABI or linked implementation. The C++ app/kernel C ABI bridge source is also compiled into `pdalcpp`, so installed `PDAL::CAPI` consumers resolve the full public C ABI from the exported PDAL library instead of an app-local static archive. |
| `cmake/` modules | done | Rust build options live in `cmake/rust.cmake`, which now owns both Rust integration macros: `pdal_build_rust_capi()` (the cargo `add_custom_command`/target and the `pdalcpp` dependency edge) and `pdal_link_rust_capi()` (the Rust C ABI archive plus the native libraries it embeds -- GEOS via the `geos` crate, the Nitro NITF bridge, and CoreFoundation on Apple). The previously-scattered link lines are folded into that one macro and called from the three targets that link `libpdal_capi.a` directly: the root `CMakeLists.txt` `pdalcpp` build and the standalone `tools/lasdump` and `tools/nitfwrap` launchers (verified: pdalcpp/`pdal`, `lasdump`, and `nitfwrap`+`nitfwrap_test` all build, link, and pass through the macro). Source packaging excludes generated Rust build output. The only Rust value still set in the root `CMakeLists.txt` is `RUST_CAPI_HEADER_DIR`, set early on purpose so `add_subdirectory(plugins)` sees it before `rust.cmake` runs its compile-time discovery -- documented ordering glue, not scattered link wiring. |
| `pixi.toml` | done | The developer environment now includes the Rust toolchain and explicit `rust-fmt`, `rust-check`, `rust-clippy`, `rust-test`, `rust-coverage`, `rust-license-audit`, `rust-unsafe-audit`, `rust-capi-header-audit`, `rust-cpp-port-audit`, `rust-cpp-test-parity`, `rust-workflow-parity`, `rust-capi-install-smoke`, `rust-source-package-smoke`, `rust-bench`, `rust-bench-memory`, `rust-bench-size`, `rust-guard`, and aggregate `rust-release-gate` tasks for the port workspace. `rust-guard` includes unsafe-footprint accounting and C ABI header sync auditing so the C ABI/native boundary stays visible in the default Rust gate; `rust-release-gate` is the single upstream-readiness smoke for the full release-blocking set, including installed-PDAL workflow parity and the C++ CTest suite. |
| GitHub workflows | done | The Pixi workflow runs the aggregate `rust-release-gate`, so CI and local upstream-readiness checks share the same Rust workspace guard, C++ implementation-replacement audit, C++ build, installed-PDAL workflow parity, installed `PDAL::CAPI` consumer smoke, Rust source-package smoke, C++ test-parity audit, and C++ CTest suite. The legacy Linux, macOS, Windows, and Alpine compile scripts also run the installed `PDAL::CAPI` consumer smoke after `ninja install`; the conda package example job runs the same smoke against the installed package prefix; release source confirmation reaches the same smoke through the Linux compile script. Shared CI conda environments and Docker/Alpine build images install Rust/Cargo explicitly for CMake builds that require the Rust C ABI. Reopen this row only for a concrete runner/platform failure or an upstream policy decision that changes the required gate shape. |
| `PDALConfig.cmake.in` | done | Downstream `find_package(PDAL)` keeps the C++ target as the primary link surface, exposes `PDAL_CAPI_INCLUDE_DIRS`, and provides an installed `PDAL::CAPI` interface target for the stable C ABI header plus the backing `PDAL::PDAL` link surface. Verified with a temporary install prefix and an out-of-tree CMake consumer that includes `<pdal_capi.h>`, links `PDAL::CAPI`, calls `pdal_version_string()`, checks the compile-time `PDAL_CAPI_ABI_VERSION_*` macros against the exported runtime ABI-version functions, calls the STAC type probe, and executes a tiny `readers.faux` pipeline through the installed header/library pair while loading the installed library. A separate C ABI library export is not part of the current shape because the C ABI is exported by `pdalcpp`. |
| `pdal_features.hpp.in` | done | Decision: no Rust-backed-build feature macro is added. The Rust C ABI is an unconditional, mandatory part of the build -- `cargo` is required (fatal if missing), `pdal_rust_capi`/`libpdal_capi.a` is always a dependency of `pdalcpp`, and there is no `option()` to disable it -- so a `PDAL_HAVE_RUST`-style macro would always be defined and provide no supported conditional to branch on (and the guidance is to avoid preprocessor branching). The one Rust-related compile define, `PDAL_UTILS_NO_RUST_CAPI`, is a private switch for the standalone `dimbuilder` generator only (see `dimbuilder/`), not a feature-availability macro for the installed library. Revisit only if a future supported build mode actually makes the Rust C ABI optional. |
| `dimbuilder/` | done | Intentional generator-tool exception. `dimbuilder` is a standalone code generator (`Dimension.json` -> `Dimension.hpp`) that, by existing PDAL design, compiles `Utils.cpp` directly into the executable rather than linking `pdalcpp` (so Linux packagers who disable rpath can still run it during the build). It therefore builds with `PDAL_UTILS_NO_RUST_CAPI`, which selects the pure-C++ fallback for each Rust-C-ABI-backed `Utils` function. Verified: a clean standalone build compiles `Utils.cpp` + `DimBuilder.cpp` with no Rust capi linked, and the generated `Dimension.hpp` is byte-identical to the build-tree header (modulo the input-path comment). The tool only uses `Utils::split`/`toupper`/`trim`/`wordWrap`, all of which have correct C++ fallbacks under the guard. This is the accepted permanent shape, not a half-port; the guarded C++ fallbacks must stay behavior-equal to the Rust path. |
| `package.sh` and release packaging | done | Source packaging keeps Rust sources while excluding `rust/target/`, and `package.sh` installs Rust/Cargo alongside the C++ build tools. `rust-source-package-smoke` runs `package_source` and verifies the source archives contain every Rust workspace crate manifest, the C ABI header, the port status/decision docs, and the Rust guard scripts while excluding build/cache artifacts. The CPack source-ignore regex for the root `package.sh` is anchored so it no longer drops `rust/scripts/check_source_package.sh`. The release workflow runs the same archive check after `ninja dist`. `rust-license-audit` reports third-party crate license metadata from `cargo metadata` and currently finds no missing license metadata. Verified locally with `pixi run -e dev rust-source-package-smoke`. |
| `examples/` | done | Installed-prefix executable and plugin-authoring examples link through the exported `PDAL::PDAL` target instead of the legacy raw library variable/link-directory pattern. Verified `batch-streamer`, `filter-streamer`, `reading-streamer`, `writing-streamer`, and `writing` configure/build against `/tmp/pdal-rust-prefix`, and the `writing` tutorial runs and emits `myfile.las` through the Rust-backed writer path. Verified `writing-filter`, `writing-kernel`, `writing-reader`, and `writing-writer` configure/build against the current installed Pixi prefix with AppleClang after `Utils::fromString(int/double)` became exported overloads instead of compiler-sensitive constrained template specializations. |
| `doc/` | done | Developer docs expose the Rust port as an experimental migration effort and point to `rust/PORTING.md`, `rust/STATUS.md`, and `rust/VENDOR.md` as the authoritative sources. Public user-facing docs are deferred until upstream/release policy decides how the Rust-backed build should be presented to end users. |

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
| `tools/lasdump` | done | Standalone `lasdump` is a thin C++ launcher over the Rust C ABI. Rust covers LAS/LAZ header, VLR/EVLR, and point checksum output; command tests, a LAZ smoke, and an ignored installed-`lasdump` stdout parity regression are in place. |
| `tools/nitfwrap` | done | Standalone `nitfwrap` is a thin C++ launcher over the Rust C ABI. Rust wraps and unwraps LAS/BPF through Nitro, preserves embedded bytes, unwraps the existing NITF fixture, and passes the existing `nitfwrap_test`. Full NITF reader/writer stage parity is tracked under `plugins/nitf` and I/O. |

## Parity And Implementation Accounting

Detailed C++ test-parity accounting, implementation-replacement backlog
classification, mixed-binary notes, and audit commands live in
`rust/PARITY.md`. Keep that document and the audit scripts in sync whenever
classification rules or parity counts change.

Current headline checkpoints:

- Pre-port C++ GoogleTest parity: `819 / 819` Rust C ABI-backed.
- Main first-party implementation backlog: `0` port-candidate LOC.
- Optional-plugin-inclusive implementation backlog: `0` port-candidate LOC.

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
