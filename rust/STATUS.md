# Rust Port Status

This is still a work in progress and is not yet at full feature parity with
C++ PDAL. Bugs may exist. Check this list before assuming a behavior is
intentional or before asking an agent to broaden the port.

Status definitions:

- `done`: believed done for the stated scope. No known major deficits in that
  slice. OK to log bugs or add parity cases.
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
| Filter ports | in progress | 84 first-party filter/static stage files exist in C++; 51 are Rust-backed through the C ABI, 33 intentionally remain C++ for now, and 40 are visible through the Rust pipeline registry. Registry exposure is not the same as full pipeline parity. |
| Filter layout mutation | prototype | A narrow prepare/layout hook exists for registry-visible derived-dimension filters such as `NNDistance`, `RadialDensity`, `Eigenvalue*`, `ClusterID`, `HeightAboveGround`, `Coplanar`, `PlaneFit`, `Reciprocity`, and custom `filters.zsmooth` dimensions. More complex layout mutation remains open. |
| Pure/local I/O harness | in progress | `readers.faux` and `writers.null` support in-memory pipeline testing. |
| Text I/O | done | `readers.text` and `writers.text` cover the deterministic local text slice and installed-PDAL regression coverage. |
| PCD I/O | in progress | ASCII PCD read/write is covered. Binary and compressed PCD are deferred. |
| PTS/PTX readers | in progress | Deterministic Leica ASCII fixture behavior is covered, including installed-PDAL regressions. |
| ILVIS2 reader | in progress | Deterministic ASCII point path and fixture-shaped XML sidecar metadata are covered. |
| PLY I/O | in progress | ASCII PLY read/write is covered. Binary PLY is deferred. |
| OBJ reader | in progress | Deterministic Wavefront OBJ ASCII path, mesh faces, and VTN de-duplication are covered. |
| GLTF writer | in progress | Deterministic local GLB output from mesh-backed views is covered for existing C++ unit-test shapes. |
| QFIT reader | in progress | Deterministic NASA ATM QFIT binary path is covered. |
| SBET/SMRMSG I/O | in progress | SBET read/write and SMRMSG read coverage exist for deterministic local trajectory fixtures. |
| LAS/LAZ I/O | in progress | `las`/`laz` crate path supports standard dimensions, V1.0-1.4 point formats, Extra Bytes, SRS extraction, and compression/decompression. Keep parity tests honest before broad claims. |
| FBI I/O | in progress | TerraScan Fast Binary local path has byte-for-byte installed-PDAL read/write parity for the covered slice. |
| TerraSolid reader | in progress | Deterministic TerraSolid format 2 fixture is covered. `.bin` is not inferred because it conflicts with FBI. |
| Optech reader | in progress | Deterministic Optech CSD fixture and localized WGS84 georeference math are covered. |
| BPF I/O | in progress | Deterministic local uncompressed BPF is covered. Compression, remote files, bundled files, and ULEM/polar metadata are deferred. |
| GDAL reader | prototype | A narrow local raster-to-point-cloud slice has started. This is not broad GDAL/PROJ permission. |
| Driver inference | in progress | Rust can infer existing PDAL reader/writer names from filenames. Construction must still fail cleanly for unported drivers. |
| Pipeline JSON parsing | in progress | Narrow PDAL-style JSON arrays/root `pipeline` objects, filename string stages, scalar options, default linear dependencies, and optional `tag`/`inputs` work for command readiness. |
| `pdal-rs` command shell | in progress | Rust-native shell lists only Rust-backed stages/commands and no longer links the C++ helper dispatch shim. |
| Command metadata | in progress | `--drivers`, `--list-commands`, and `--options <stage>` are backed by Rust-owned metadata for the implemented Rust surface. |
| Implemented commands | in progress | `pipeline`, `info`, `translate`, `merge`, `sort`, `split`, `random`, `hausdorff`, `chamfer`, `delta`, `density`, `eval`, `tile`, and `tindex` have installed-PDAL regression coverage for their scoped workflows. `ground` currently compares point-count preservation only because the Rust SMRF implementation is still a simplified approximation. |
| Performance visibility | prototype | Ignored reporting harnesses exist for local I/O performance, binary size, startup time, memory, and build cost. They are visibility tools, not hard gates yet. |
| Vendor/native strategy | in progress | `rust/VENDOR.md` is the source of truth. Native GDAL/OGR/GEOS/PROJ adapters belong in `pdal-native`; pure Rust replacements such as LAS/LAZ do not need to move through it. |
| Plugins | not ready | `pdal-plugins` may hold discovery metadata. Optional plugin ports and a Rust plugin SDK wait until the first-party surface and C ABI are stable. |
| Remote/object-store I/O | deferred | Waits until local deterministic I/O and pipeline execution are stable. |
| Broad kernels/apps/tools migration | deferred | Simple `pdal-rs` commands may continue proving lower layers. Broad kernels, `apps/pdal.cpp`, `lasdump`, and `nitfwrap` wait on lower-layer parity. |

## C++ Test Parity Accounting

The first target is the pre-existing C++ test suite running against Rust
implementations through the C ABI and C++ wrappers. Rust linkage alone does not
count.

Current checkpoint: `768 / 927` individual C++ GoogleTest cases, or `82.85%`,
are validated against Rust-backed behavior.

Current test-suite size: `28,793` C++ code LOC under `test/`. These tests remain
the behavioral contract and should not be counted as unported implementation.

Current C++ compatibility wrapper/adapter surface: `15,069` code LOC across
`111` first-party C++ files that include or directly declare Rust C ABI entry
points, split approximately as `pdal/` 7,126 LOC, `filters/` 5,659 LOC, and
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

Known mixed binaries:

- `pdal_kdindex_test`: all 5 tests currently count; KNN/radius queries route
  through Rust spatial query ABI.
- `pdal_spatial_reference_test`: only `calcZone` and `wgs84FromZone` count.
  Most SRS normalization, authority lookup, WKT/PROJJSON, and LAS SRS behavior
  remains C++ GDAL/OGR-backed.
- `pdal_point_view_test`: only `calculateBounds` counts. The broader point
  view/table data model is still C++.
- `pdal_bounds_test`: only `test_ctor`, `test_clip`, `test_intersect`,
  `test_grow`, `test_bounds_grow_2_3_args`, and `test_invalid` count. Bounds
  clear/empty/contains/overlap/clip/grow arithmetic routes through the Rust C
  ABI. Equality, accessors, parsing, formatting, WKT/GeoJSON, SRS bounds, and
  `ProgramArgs` integration remain C++.
- `pdal_utils_test`: only `test_base64`, `blanks`, `replaceAll`,
  `escapeNonprinting`, and `escapeJSON` count. Other utility cases still test
  C++ templates, stream helpers, process helpers, or local formatting behavior.
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
- `pdal_kernel_test`: all 1 test counts; stage-option parsing routes through
  the Rust C ABI.
- `pdal_stage_factory_test`: only `extensionTest` counts; reader/writer driver
  inference routes through the Rust C ABI. Plugin loading and per-instance
  extension tables remain C++.
- `pdal_plugin_manager_test`: only `validnames` counts; plugin filename
  validation routes through the Rust C ABI. Plugin registration and object
  creation remain C++.
- `pdal_options_test`: only `valid` counts; option-name validation routes
  through the Rust C ABI. Option storage, conditional merging, metadata, JSON,
  and `ProgramArgs` behavior remain mixed C++.
- `pdal_polygon_test`: only `valid` counts; geometry validity routes through
  the Rust native geometry ABI. Polygon construction, serialization, bounds,
  simplification, and point coverage remain C++/GDAL.
- `pdal_metadata_test`: do not count as a binary yet. Scalar conversion and
  JSON formatting use Rust helpers, but the metadata tree implementation is
  still C++.
- `pdal_io_las_reader_test` and `pdal_io_las_writer_test`: do not count as
  binaries yet. Rust LAS/LAZ exists, but the C++ reader/writer wrappers still
  have substantial legacy header, VLR, SRS, streaming, and option behavior.

## Command-Ready Filters

Pipeline JSON can currently construct this command-ready filter subset:

- `approximatecoplanar`
- `chipper`
- `cluster`
- `dbscan`
- `decimation`
- `eigenvalues`
- `elm`
- `estimaterank`
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

- GDAL/PROJ/SRS/OGR-backed: `Colorinterp`, `Colorization`, `Crop`, `DEM`,
  `GeomDistance`, `H3`, `HagDem`, `Overlay`, `ProjPipeline`.
- Private or specialized algorithms: `CS`, `Delaunay`, `FaceRaster`,
  `Georeference`, `HagDelaunay`, `LiTree`, `LloydKMeans`, `M3C2`,
  `Miniball`, `Normal`, `PMF`, `Poisson`, `Straighten`, `Supervoxel`,
  `GreedyProjection`, `IterativeClosestPoint`, `RelaxationDartThrowing`.
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
