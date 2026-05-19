# PDAL Rust Port Notes

This directory is a Rust port spike behind a C ABI. The existing C++ API and
tests remain the behavioral contract while the Rust implementation grows.

## Architecture

- Rust code owns Rust `PointLayout`, `PointView`, and `Stage` values.
- C++ code calls Rust only through `pdal-capi`.
- The C ABI is the contract between languages. Python, CLI, and C++ should be
  peers above that contract over time.
- Existing C++ tests are the first parity gate. Rust unit tests are necessary,
  but not sufficient.

## Agent Guardrails

This port is intentionally incremental. If you are an AI agent continuing this
work, do not broaden the scope just because a Rust crate exists.

Off limits unless the user explicitly revises this plan:

- Do not rewrite PDAL by directory or claim a directory is complete because it
  builds.
- Do not replace the C ABI with Rust/C++ direct object sharing.
- Do not pass C++ object pointers across the C ABI as Rust handles.
- Do not port optional plugins or design a Rust plugin loading SDK yet.
- Do not copy vendored C/C++ code into Rust crates. Follow `rust/VENDOR.md`.
- Do not start LAS/LAZ, GDAL, PROJ, compression, remote I/O, kernels, apps, or
  broad plugin work before the current deterministic local I/O slices are
  complete.
- Do not port concrete CLI commands yet. `pdal-kernels` may hold registry and
  command-contract infrastructure, but user-visible commands must wait for the
  command readiness checkpoint below.
- Do not add placeholder modules, placeholder crates, or broad skeletons that
  are not tied to a concrete parity milestone.
- Do not weaken existing C++ validation or tests to make a Rust port pass.
- Do not mark a stage/reader/writer "ported" without behavior coverage and the
  relevant C++ parity gate.

Current next milestone:

1. Extend deterministic local I/O one narrow format at a time, behind the C
   ABI, with fixture behavior covered before moving on.
2. Prove reader -> filter -> writer pipeline behavior through the Rust core
   boundary for each new format.
3. Stay on deterministic local formats for now. Narrow binary formats are
   acceptable when fixture-scoped and dependency-light, but compressed,
   GDAL-backed, LAS/LAZ, plugin-backed, and remote paths remain deferred until
   this local I/O loop is solid.

Every commit should say which checkpoint it advances. If the answer is "none",
it probably should not be part of this port.

## Whole Repo Migration Map

Approximate first-party code size, excluding comments and blanks:

- `pdal/`: 22.4k LOC. Core point model, pipeline model, options, metadata,
  dimensions, layouts, tables, views, and utility code.
- `filters/`: 21.1k LOC. Pure transforms and algorithmic stages. This is the
  current spike area because it can prove the Rust core -> C ABI -> C++ wrapper
  loop without starting with file-format or GDAL/PROJ complexity.
- `io/`: 24.6k LOC. Readers and writers. This is the largest first-party area
  and should wait until the core ABI and external-library FFI patterns are
  stable.
- `kernels/`: 3.3k LOC. CLI subcommands. These sit above the core and stage
  graph, so they are late migration work.
- `apps/` and `tools/`: 1.1k LOC. CLI entry points and small tools. These are
  also top-layer migration work.

Non-porting scope:

- `vendor/`: about 252k LOC. Do not port. Treat these dependencies as external
  libraries reached through Rust crates or explicit FFI. See `rust/VENDOR.md`
  for the current vendor mapping and rules.
- `test/`: about 26k LOC. Keep the existing C++ tests as the behavioral
  contract while Rust grows underneath the ABI.
- Plugins: about 34k LOC. Leave optional plugin drivers in C++ until the core,
  first-party stage surface, I/O, apps/tools, and command strategy are stable.
  Treat Rust plugin loading or a Rust plugin SDK as a final compatibility
  phase, not a way to make early progress.

The migration order is intentionally vertical-slice driven: build only the
core pieces needed by the next stage family, prove parity through the C ABI,
then move outward. A broad rewrite by directory is not the plan.

## Target Rust Layout

The Rust tree is not intended to be a 1:1 mirror of the C++ source tree. C++
directories are migration source areas and behavioral references; Rust crates
are organized around contracts that should remain stable as the implementation
changes.

Current target crates:

- `pdal-core`: point model, dimensions, metadata, options, pipeline, SRS,
  spatial/geometry helpers, and shared stage traits.
- `pdal-capi`: stable C ABI. This is the real cross-language contract.
- `pdal-filters`: first-party filters.
- `pdal-io`: first-party readers and writers. The deterministic text I/O
  vertical slice has started here.
- `pdal-kernels`: CLI subcommands. Kernel registry foundation only; concrete
  kernel ports remain intentionally last.
- `pdal-cli`: thin executable surface.
- `pdal-plugins`: plugin metadata and discovery helpers. Do not port optional
  plugins or add a loading SDK until a versioned plugin boundary is designed.

What should stay 1:1 with C++:

- Stage names such as `filters.decimation`.
- User-visible options, metadata, dimensions, and error behavior.
- Test inputs and expected outputs.
- C ABI symbols where C++ wrappers already call Rust.

What should not be 1:1 with C++:

- Header/source file split.
- Class inheritance shape.
- Historical private helper boundaries.
- Optional plugin layout.
- Directory names when Rust module families are clearer.

Do not fill placeholder crates with broad skeleton code. Add modules only when
starting a real milestone, and include the parity tests or C++ behavior link
that justifies the module.

## Migration Order

Follow this order unless the plan is deliberately revised. Earlier checkpoint
work may continue while later crates get small boundary foundations, but those
foundations are not permission to skip ahead to broad top-layer ports.

1. `filters/` first, backed by the minimum `pdal/` core needed for each filter
   family. Start with pure transforms, then spatial-index filters, then
   linear/statistical filters, then only the harder FFI or private-algorithm
   families once their ABI shape is clear.
2. Expand `pdal/` core as filters require it. Do not attempt a standalone
   directory-wide core rewrite; each new core capability should be justified by
   a stage parity need.
3. `io/` after the filter/core loop is stable. Readers and writers introduce
   file formats, byte layout, compression, GDAL, PROJ, LASzip, and similar FFI
   concerns, so they should not be the next frontier until the ABI and point
   model are proven.
4. `apps/` and `tools/` after the library surface is stable enough to run real
   pipelines through the C ABI. This is small by LOC but high in dependency
   density: `apps/pdal.cpp` owns CLI dispatch and driver/option introspection,
   while `tools/lasdump` and `tools/nitfwrap` are tied to LAS/LAZ and NITF
   strategy decisions.
5. `kernels/` last. Kernels are CLI subcommands above the pipeline/stage
   system; porting them before the core, filters, and I/O layers are stable
   creates top-down churn instead of proving behavior.
6. Optional plugins after the first-party library and command surface are
   stable. Until then, plugins stay in C++ or are handled only as metadata and
   discovery compatibility helpers.

Do not jump to `kernels/`, apps/tools, plugins, vendor-heavy work, or broad
`io/` work just because those areas are smaller or visible. The active
post-filter milestone is the narrow deterministic text I/O slice.

Command work is allowed only after the library can run realistic pipelines
without special-case test plumbing:

- At least one local reader -> filter -> writer path runs through the Rust
  pipeline and C ABI with installed-PDAL regression coverage.
- Stage creation from user-visible names and options is available for the
  stages used by the command.
- Reader/writer inference, option parsing, metadata/errors, and output paths
  needed by the command have Rust parity coverage.
- The command can be regression-tested against the installed C++ `pdal` binary
  for exit status, stdout/stderr shape, and output artifacts.

When those gates are true, start with `pipeline`, then `info`, then simple
pipeline-shaped commands such as `translate`, `merge`, `sort`, and `split`.
Keep `tile`, `tindex`, `ground`, and other GDAL/remote/spatially complex
commands deferred until their lower-layer dependencies are Rust-backed.

Apps/tools work is allowed when it directly supports a command-readiness gate:

- `apps/pdal.cpp` can be touched when Rust has enough pipeline JSON, stage
  registry, driver listing, option introspection, logging/error, and output
  behavior to compare a `pdal` command against the installed C++ binary.
- `tools/lasdump` waits for the LAS/LAZ reader/writer and compression strategy.
- `tools/nitfwrap` waits for the NITF/plugin or specialized I/O strategy.

Do not mark apps/tools complete because the LOC is small. They close only when
their underlying command or format strategy has parity coverage.

Vendor compatibility work is allowed only when a concrete Rust port reaches the
stage, reader, writer, or core behavior that depends on that vendor boundary:

- Linear algebra choices happen with broader linear/statistical filters, behind
  shared Rust math APIs, not by porting `vendor/eigen`.
- GDAL/PROJ/GEOS choices happen with SRS, raster/vector, geometry, crop,
  overlay, reprojection, DEM, and related filters or readers.
- LASzip/lazperf and other compression choices happen when the I/O milestone
  reaches LAS/LAZ or another compressed binary format.
- Remote/object-store compatibility happens only after local deterministic I/O
  and pipeline execution are stable.
- JSON-schema compatibility happens when Rust owns pipeline JSON validation.
- Private algorithm vendors such as Kazhdan are decided per stage: port to
  Rust, bind through explicit FFI, or leave the C++ stage in place.

Do not start a broad `vendor/` compatibility pass. Each vendor decision should
name the user-visible stage or core behavior it unlocks, cite the parity test
that will hold it honest, and follow `rust/VENDOR.md`.

Plugin implementation work is allowed only after the first-party Rust surface
can run real command and pipeline workflows:

- `pdal-plugins` may keep metadata and filename-discovery helpers that mirror
  stable parts of the existing C++ plugin convention.
- Do not port optional plugin readers, writers, filters, or kernels until the
  equivalent first-party reader/writer/filter/kernel family is already proven.
- Do not design a Rust plugin loading SDK until the C ABI, stage registry,
  ownership/lifetime rules, metadata, errors, versioning, and dynamic library
  compatibility story are stable.
- Plugin-by-plugin ports should be driven by demand and parity tests, not by
  sweeping the `plugins/` directory.

In practice, plugins are after first-party filters, core, I/O, apps/tools, and
commands. The likely first plugin work is compatibility/discovery validation,
then a single low-risk plugin port if it proves the SDK boundary. Heavy
database, HDF/E57/NITF/TileDB, registration, trajectory, and format-specific
plugins remain later or stay C++ behind the ABI.

## Checkpoint Roadmap

Treat these as ordered checkpoints on the way to a complete Rust-backed PDAL.
Each checkpoint should end in a commit with the listed gates passing. Do not
advance by claiming a directory is "done" only because it builds.

### 1. Filter ABI And Pure Filter Parity

Goal: prove the Rust core -> C ABI -> C++ wrapper loop with filters that do not
need external libraries.

Required shape:

- Rust-owned `PointLayout`, `PointView`, options, stages, source indices,
  per-view spatial reference, and explicit C++/Rust view conversion.
- Dependency-free filters ported in small families, with existing C++ tests
  passing.
- C++ tests strengthened first where they do not assert output behavior.

Done when:

- Rust unit/parity tests pass.
- The matching C++ filter test binaries pass.
- The full `pdal_filters_*` CTest slice passes.
- No C++ object pointer crosses the C ABI.

### 2. Filter Bridge Contract Complete

Goal: make the shared C ABI bridge strict enough that later ports cannot pass
by accidentally dropping behavior.

Required shape:

- Dimension names and storage types round-trip through the C ABI.
- Rust output dimensions are verified against PDAL's prepared C++ layout.
- Spatial reference text and coordinate epoch round-trip for Rust-backed views.
- Metadata produced by Rust can be copied into C++ `MetadataNode` trees.
- Streaming, in-place, single-output, and multi-output wrapper paths each have
  representative coverage.

Done when:

- The full `pdal_filters_*` CTest slice passes.
- Rust `cargo fmt`, `cargo clippy --workspace -- -D warnings`, and
  `cargo test --workspace` pass.
- New bridge behavior has direct Rust or C++ regression coverage.

### 3. Spatial And Linear Filter Families

Goal: finish the filter families that need shared algorithmic core support.

Required shape:

- Spatial filters share one Rust spatial-neighbor API. The initial
  implementation may be brute force, but filters must not bake in a private
  one-off neighbor search that prevents a later KD-tree swap.
- Linear/statistical filters share one Rust linear algebra/statistics layer
  where practical.
- Existing C++ tests remain the parity gate, with added assertions before ports
  when tests only exercise code without checking outputs.

Done when:

- Ported spatial and linear/statistical filters pass their C++ test binaries.
- The full `pdal_filters_*` CTest slice passes.
- The shared Rust APIs are documented enough that new filters use them instead
  of duplicating local algorithms.

### 4. Deferred Filter Families

Goal: handle the remaining filters only after their ABI or algorithm choice is
explicit.

Required shape:

- GDAL/PROJ/SRS filters choose a Rust crate or explicit FFI strategy before
  porting implementation code.
- Private-algorithm filters decide per algorithm whether to port to Rust,
  bind through FFI, or intentionally leave C++ in place.
- Process/pipeline/framework filters are not ported until the relevant core
  pipeline model exists behind the C ABI.

Done when:

- Each family has a short design note or commit message explaining port versus
  FFI versus defer.
- Existing C++ tests pass, with coverage added first where needed.
- The full `pdal_filters_*` CTest slice passes.

### 5. Core Pipeline Slice

Goal: expand from filter execution into the minimum `pdal/` core needed to run
simple pipelines through the ABI.

Required shape:

- Stable C ABI handles for pipeline construction, options, stage creation,
  point tables/views, metadata, and errors.
- C++ wrappers remain compatibility peers, not the hidden contract.
- Pipeline behavior is validated against existing C++ tests or equivalent
  parity tests using the same inputs and asserted outputs.

Done when:

- A simple reader/filter/writer or in-memory pipeline can run through the Rust
  core boundary.
- Relevant `pdal/` unit tests or parity tests pass.
- Existing filter tests still pass.

### 6. I/O Vertical Slice

Goal: prove readers/writers after the core and filter ABI are stable.

Start with the smallest deterministic format path that avoids unnecessary
external FFI. Do not begin with LAS, GDAL, PROJ, compression, or remote I/O
unless the spike explicitly requires that complexity.

Required shape:

- One reader and one writer path behind the C ABI.
- Byte-level and metadata behavior checked against existing C++ tests or
  fixtures.
- External dependencies stay external through Rust crates or explicit FFI.

Done when:

- The matching C++ I/O test binary passes.
- The pipeline slice can use the ported I/O path end to end.
- Existing filter/core tests still pass.

- `readers.faux` and `writers.null` live in `pdal-io` as the in-memory pipeline
  harness.
- `readers.text` has a Rust implementation covering existing C++ fixture
  behavior for simple delimited numeric text, CRLF headers, override/inserted
  headers, quoted headers, duplicate dimensions, and skipped malformed rows.
- `writers.text` has a Rust implementation covering existing C++ fixture
  behavior for CSV output, dimension order, per-dimension precision, custom
  delimiters, quoted/unquoted headers, and simple GeoJSON output.
- The Rust pipeline has a reader -> decimation filter -> writer regression test
  for the text slice.
- Installed-PDAL regression for the text slice is available with:
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test text_regression -- --ignored`.
- `readers.pcd` and `writers.pcd` have a Rust ASCII-only implementation for
  local PCD fixtures. The slice preserves existing whitespace parsing,
  comma-row skipping, missing-header rejection, float32 XYZ storage behavior,
  dimension order, per-dimension type/precision, and reader -> decimation ->
  writer flow. Binary and compressed PCD are intentionally deferred.
- Installed-PDAL regression for the ASCII PCD slice is available with:
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test pcd_regression -- --ignored`.
- `readers.pts` has a Rust implementation for Leica PTS ASCII fixtures,
  including declared point counts, 3/4/7-field layouts, intensity offset
  mapping, skipped malformed rows, and reader -> decimation -> PCD writer
  installed-PDAL regression coverage.
- `readers.ptx` has a Rust implementation for Leica PTX ASCII fixtures,
  including single/multiple clouds, optional RGB, missing-point discard,
  transform application, intensity scaling, and reader -> decimation -> PCD
  writer installed-PDAL regression coverage.
- Installed-PDAL regressions for these readers are available with:
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test pts_regression -- --ignored`
  and
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test ptx_regression -- --ignored`.
- `readers.ilvis2` has a Rust implementation for the deterministic ILVIS2
  ASCII point path, including `low`, `high`, and `all` mapping behavior,
  longitude normalization, skipped two-line headers, malformed-row rejection,
  XML metadata sidecar parsing for the existing fixture shape, and reader ->
  decimation -> PCD writer installed-PDAL regression coverage.
- `readers.ply` and `writers.ply` have Rust ASCII-only implementations for
  local PLY fixtures, including vertex properties, extra dimensions, list
  properties on non-vertex elements, mesh faces, `dims`, `sized_types`, and
  precision. Binary PLY remains intentionally deferred.
- `readers.obj` has a Rust implementation for the deterministic Wavefront OBJ
  ASCII path, including vertex properties, normals, texture coordinates,
  triangulation for VTN de-duplication, mesh face storage, and de-duplication
  logic matching PDAL's C++ behavior.
- `writers.gltf` has a Rust implementation for deterministic local GLB output
  from mesh-backed views, including vertex indices, XYZ vertices, optional
  normals/colors, and file-size parity for the existing C++ unit-test shapes.
  This slice adds the minimal `pdal-core` triangular-mesh model needed by the
  writer; broader mesh face parity for other readers remains deferred.
- `readers.qfit` has a Rust implementation for the deterministic NASA ATM QFIT
  binary path, including 10, 12, and 14-word formats, endian probing,
  dimension mapping matching PDAL's C++ reader, and regression coverage
  against installed PDAL.
- `readers.sbet` and `writers.sbet` have a Rust implementation for the
  deterministic Applanix SBET trajectory format, including all 17 dimensions,
  little-endian double parsing/writing, angular conversion logic (radians
  to degrees and back), and bit-parity coverage (when conversion is disabled).
- `readers.smrmsg` has a Rust implementation for the SBET RMS message format,
  covering 10 RMS error dimensions with bit-parity matching PDAL's behavior.
- `readers.fbi` and `writers.fbi` have a Rust implementation for the TerraScan
  Fast Binary local path, including separate dimension streams, header offsets,
  color stream ordering, and byte-for-byte installed-PDAL read/write parity.
- `readers.terrasolid` has a Rust implementation for the deterministic
  TerraSolid format 2 fixture, including time/color fields and C++ dimension
  mapping. The `.bin` extension is not inferred because it conflicts with FBI.
- `readers.optech` has a Rust implementation for the deterministic Optech CSD
  fixture, including the localized WGS84 georeference math and `EPSG:4326`
  spatial reference behavior.
- Installed-PDAL regressions for these readers/writers are available with:
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test ilvis2_regression -- --ignored`,
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test ply_regression -- --ignored`,
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test ply_writer_regression -- --ignored`,
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test obj_regression -- --ignored`,
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test qfit_regression -- --ignored`,
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test sbet_regression -- --ignored`,
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test fbi_regression -- --ignored`,
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test terrasolid_regression -- --ignored`,
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test optech_regression -- --ignored`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test smrmsg_regression -- --ignored`.
- `pdal_core::driver` can infer PDAL reader/writer driver names from filenames
  for existing PDAL extensions. `pdal-capi` has a narrow registry that can
  construct only currently implemented Rust local readers/writers by driver
  name. Inference may return unported PDAL drivers; construction must still
  fail cleanly until the stage is actually ported.
- `pdal-capi` can now build a Rust `Pipeline` from a narrow PDAL-style JSON
  array: stage objects, scalar options, first/last filename inference, linear
  dependencies by default, and optional `tag` / `inputs` wiring. This is a
  command-readiness bridge for local reader -> filter -> writer regressions,
  not a full replacement for C++ `PipelineManager` yet.
- Local I/O performance comparison is available as an ignored, reporting-only
  harness with:
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test perf_regression -- --ignored --nocapture`.
  Set `PDAL_RUST_PERF_ITERS=<n>` to change the per-case iteration count. This
  is for regression visibility, not a hard performance gate. Current results
  compare installed `pdal pipeline` process execution against in-process Rust
  pipeline execution, so use them to catch large regressions and guide followup
  investigation rather than to claim end-user CLI speedups.
- Runtime memory and build cost visibility is available with:
  `rust/scripts/measure_guardrails.sh`. On macOS it uses `/usr/bin/time -l` to
  report wall time and peak RSS for installed local I/O pipelines, the Rust
  local I/O perf harness, and an incremental Rust workspace build. Use
  `--cold-build` only when intentionally measuring a clean build because it
  runs `cargo clean`.
- Binary-size visibility is available as an ignored, reporting-only CLI test:
  `cargo test --manifest-path rust/Cargo.toml -p pdal-cli --test binary_size -- --ignored --nocapture`.
  It compares the installed `pdal` executable on `PATH` with Cargo's built
  `pdal-rs` test binary. The same harness also reports median CLI startup time
  for `pdal --version` versus `pdal-rs --version`.
- `readers.bpf` and `writers.bpf` have a Rust implementation for
  deterministic local, uncompressed BPF, including v3 and v1/v2 read support,
  point-major, dimension-major, and byte-major point layouts, XYZ transforms,
  dimension labels, and reader -> decimation -> PCD writer installed-PDAL
  regression coverage. Compression, remote files, bundled files, and
  ULEM/polar metadata remain intentionally deferred.
- Installed-PDAL regression for the BPF slice is available with:
  `cargo test --manifest-path rust/Cargo.toml -p pdal-io --test bpf_regression -- --ignored`.
- The deterministic local I/O slice is now broad enough to support the first
  command-readiness work. Additional I/O targets should still be narrow, local,
  deterministic, and dependency-light. Avoid compression-backed, GDAL-backed,
  LAS/LAZ, plugin-backed, or remote reader/writer work until their strategies
  are explicit.

### 7. Apps, Tools, Then Kernels

Goal: move top-layer command behavior only after the library surface is stable.

Required shape:

- `apps/` and `tools/` move before `kernels/`.
- `kernels/` remain last because they sit above pipeline, stage, options, I/O,
  logging, and error behavior.
- `apps/pdal.cpp` is treated as the CLI dispatch surface for command parity,
  not as an independent early port.
- `tools/lasdump` and `tools/nitfwrap` remain deferred until their LAS/LAZ and
  NITF lower-layer strategies are explicit.
- Concrete kernel work does not start until the command readiness gates in
  `Migration Order` are satisfied.
- `pdal pipeline` is the first command candidate because it proves the library
  surface directly. `pdal info` follows only after metadata, stats, bounds, and
  reader behavior are ready.
- CLI output and exit behavior are regression-tested against existing commands.

Current status:

- `pdal-rs` is a Rust-native command shell for the port spike. It lists only
  Rust-backed stages/commands and no longer links the C++ helper dispatch shim.
- `--drivers`, `--list-commands`, and `--options <stage>` are backed by
  Rust-owned stage/command metadata for the currently implemented Rust surface.
- `pipeline` is the only concrete Rust command currently enabled. It accepts a
  pipeline JSON filename and executes implemented Rust stages through the same
  Rust C ABI pipeline parser used by lower-layer tests. With `--showjson`, it
  reports the Rust pipeline execution summary and serialized pipeline metadata.
- Do not add `info`, `translate`, or other kernels yet; they need richer
  metadata, bounds, option, and I/O parity first.

Done when:

- The relevant app/tool/kernel tests pass.
- Regression comparisons against the C++ implementation are clean or explained.
- Lower-layer core/filter/I/O tests remain green.

## Core Status

Current Rust core primitives:

- Point model: `PointLayout`, `PointView`, dimension IDs/types, source index
  tracking, per-view spatial reference storage, mesh face storage, and 2D/3D
  bounds calculation.
- Point summaries: basic per-dimension count/min/max/mean over a view, for
  later `info`-style reporting without running a filter as a side effect. The
  C ABI exposes these summaries as Rust-owned JSON.
- Stage model: filter and streamable traits for Rust-backed stages.
- Pipeline model: minimal reader/filter/writer DAG execution, tags,
  dependencies, metadata aggregation, execution summaries with bounds and
  dimension summaries, and error propagation.
- Options: string-keyed typed getters matching PDAL's option flow.
- Expressions: conditional/math/assignment parser and evaluator used by the
  Rust-backed expression/assign work.
- Spatial index: exact brute-force neighbor queries behind an API that can be
  replaced by a real KD-tree without changing filters.
- Metadata: named tree nodes with typed scalar values.
- Metadata bridge: C++ copies Rust metadata trees into PDAL `MetadataNode`
  through `filters/private/RustMetadata.hpp`; ownership stays explicit on both
  sides of the C ABI. The C ABI also exposes a Rust-owned JSON serialization
  shape for command summaries.
- Spatial reference: text plus coordinate epoch, with metadata export.
- Bounds ABI: C-repr 2D/3D bounds structs plus point-view and pipeline-summary
  calculation calls for later command summaries.

Recent stabilization checkpoint:

- `pdal-capi` was split into ABI-focused modules. `lib.rs` is now only the
  module root and C ABI smoke tests.
- `pdal-core::pipeline` was split into graph execution, traits/adapters, and
  tests.
- Rust core guard tests now cover pipeline tag/dependency/error behavior,
  point source-index and typed-storage behavior, metadata/options/SRS scalar
  behavior, and spatial/geometry wrapper behavior.
- Refactored the Stage Model to support multiple input views in `run()`,
  matching PDAL's `PointViewSet` behavior. Added a default implementation for
  looping over inputs and calling `run_one()`, and fixed `filters.merge` to
  correctly handle DAG pipelines without producing redundant points.
- This was a stabilization pass, not a new broad porting phase. Do not keep
  adding incidental tests here unless they protect a behavior needed by the
  next concrete porting milestone.
- Latest validation for this checkpoint: Rust workspace tests (including DAG
  pipeline tests) passed and the full `pdal_filters_*` CTest slice passed.

Current deliberate gaps:

- No GDAL/PROJ-backed SRS normalization, reprojection, authority lookup,
  WKT1/WKT2 conversion, axis ordering, or unit handling yet.
- The metadata bridge covers typed scalar tree copying, but not every C++
  `MetadataNode` feature such as descriptions, array/list kind preservation,
  JSON/base64 typed nodes, or full pipeline serialization parity.
- No geometry ABI yet for polygons, OGR geometries, point-in-polygon, or bounds
  reprojection.

## Boundary Rules

- Never cast C++ `pdal::PointView*`, `pdal::PointRef*`, or other C++ objects to
  Rust opaque C ABI types.
- Use `filters/private/RustViewConverter.hpp` or another explicit copy/bridge
  layer when C++ wrappers call Rust stages.
- A filter is not considered ported until its existing C++ test binary passes.
- If the existing C++ test is weak, add coverage before relying on a Rust port.
- Preserve C++ validation, metadata, spatial reference, layout mutation,
  streaming, and multi-output behavior unless the migration explicitly replaces
  that contract with passing parity tests.

## Filters Status

Safe filter ports currently use Rust-owned views and existing conversion
helpers. The accepted Rust-backed set includes the dependency-free filters,
spatial-index filters, the first pure linear/statistical filters, and the
remaining pure partition/time filters that pass their existing C++ test
binaries.

Current C++ stage inventory:

- 84 first-party filter/static stage files in `filters/`.
- 49 are Rust-backed through the C ABI.
- 35 intentionally remain C++ for now.

The latest pure filters moved behind the C ABI are:

- `filters.chipper`
- `filters.gpstimeconvert`
- `filters.splitter`

The remaining C++ filters are not "missed easy ports"; they are holdouts whose
Rust port should start with an ABI/algorithm decision, not a direct rewrite:

- GDAL/PROJ/SRS/OGR-backed: `Colorinterp`, `Colorization`, `Crop`, `DEM`,
  `GeomDistance`, `H3`, `HagDem`, `Overlay`, `ProjPipeline`,
  `Reprojection`.
- Private or specialized algorithms: `CS`, `Delaunay`, `FaceRaster`,
  `Georeference`, `HagDelaunay`, `HexBin`, `LiTree`, `LloydKMeans`, `M3C2`,
  `Miniball`, `Normal`, `PMF`, `Poisson`, `SMR`, `Straighten`,
  `Supervoxel`, `GreedyProjection`, `IterativeClosestPoint`,
  `RelaxationDartThrowing`.
- Pipeline/process/framework behavior: `Info`, `Shell`, `StreamCallback`.
- Expression/KD-tree hybrid behavior still needing a design pass:
  `RadiusAssign`, `NeighborClassifier`, `CovarianceFeatures`.

The rejected broad sweep in commit `a1e67b5dc` is useful only as source
material. Its C++ wiring passed C++ object pointers across the C ABI and broke
existing filter tests. Do not reapply it wholesale.

## Deferred Filter Families

These should not be marked complete without dedicated ABI design and parity
tests:

- GDAL/PROJ/SRS filters: `Colorinterp`, `Colorization`, `Crop`, `DEM`,
  `GeomDistance`, `H3`, `HagDem`, `Overlay`, `ProjPipeline`,
  `Reprojection`.
- Metadata-heavy filters beyond `ExpressionStats` that require richer
  `MetadataNode` features or pipeline serialization parity.
- Private-algorithm filters: `CS`, `Delaunay`, `FaceRaster`, `Georeference`,
  `HagDelaunay`, `HexBin`, `LiTree`, `LloydKMeans`, `M3C2`, `Miniball`,
  `Normal`, `PMF`, `Poisson`, `SMR`, `Straighten`, `Supervoxel`,
  `GreedyProjection`, `IterativeClosestPoint`, `RelaxationDartThrowing`, and
  similar specialized implementations.
- Framework or shell filters that depend on process execution or PDAL pipeline
  internals: `Info`, `Shell`, `StreamCallback`.
- Expression/KD-tree hybrid filters that need a small dedicated design before
  porting: `RadiusAssign`, `NeighborClassifier`, `CovarianceFeatures`.

## Completion Criteria For Each Port

1. Rust unit/parity tests pass.
2. The matching C++ test binary passes.
3. The full `pdal_filters_*` CTest slice passes before leaving `filters/`.
4. No unsafe reinterpret-cast crosses the C ABI.
5. The port preserves user-visible behavior, not just compile/link success.

For non-filter ports, replace item 3 with the matching focused CTest slice and
any lower-layer regression slice the change can affect. For example, I/O work
should run the matching `pdal_io_*` tests when C++ wrappers are involved, plus
the Rust workspace gates.
