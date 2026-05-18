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
  libraries reached through Rust crates or explicit FFI.
- `test/`: about 26k LOC. Keep the existing C++ tests as the behavioral
  contract while Rust grows underneath the ABI.
- Plugins: about 34k LOC. Leave optional plugin drivers in C++ until the core
  and first-party stage surface are stable.

The migration order is intentionally vertical-slice driven: build only the
core pieces needed by the next stage family, prove parity through the C ABI,
then move outward. A broad rewrite by directory is not the plan.

## Migration Order

Follow this order unless the plan is deliberately revised:

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
   pipelines through the C ABI.
5. `kernels/` last. Kernels are CLI subcommands above the pipeline/stage
   system; porting them before the core, filters, and I/O layers are stable
   creates top-down churn instead of proving behavior.

Do not jump to `kernels/` or broad `io/` work just because those areas are
smaller or visible. The first milestone is filter parity through the C ABI.

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
helpers. The accepted set includes the dependency-free filters, spatial-index
filters, and the first pure linear/statistical filters that pass the existing
C++ filter suite.

The rejected broad sweep in commit `a1e67b5dc` is useful only as source
material. Its C++ wiring passed C++ object pointers across the C ABI and broke
existing filter tests. Do not reapply it wholesale.

## Deferred Filter Families

These should not be marked complete without dedicated ABI design and parity
tests:

- GDAL/PROJ/SRS filters: colorization, overlay, DEM, HagDem, reprojection,
  ProjPipeline, H3, GeomDistance, Crop, Georeference.
- Metadata-heavy filters: ExpressionStats and similar filters that publish
  structured metadata.
- Private-algorithm filters: CSF, Delaunay, FaceRaster, HexBin, Poisson, SMR,
  PMF, Supervoxel, and similar specialized implementations.
- Framework or shell filters that depend on process execution or PDAL pipeline
  internals.

## Completion Criteria For Each Port

1. Rust unit/parity tests pass.
2. The matching C++ test binary passes.
3. The full `pdal_filters_*` CTest slice passes before leaving `filters/`.
4. No unsafe reinterpret-cast crosses the C ABI.
5. The port preserves user-visible behavior, not just compile/link success.
