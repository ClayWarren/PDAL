# PDAL Rust Port Notes

This directory is a Rust port spike behind a C ABI. The existing C++ API and
tests remain the behavioral contract while the Rust implementation grows.

For closed porting choices, read `rust/DECISIONS.md` first. It is the decision
ledger for vendor, plugin, native-adapter, C++ compatibility, and holdout
questions. `PORTING.md` describes how to move; `DECISIONS.md` says which
remaining forks are already settled.

## Goal: A Behavior-Preserving Port

The objective is a port to Rust that **keeps all existing functionality**. The
target is full functional parity with C++ PDAL: every user-visible feature —
stages, options, metadata, dimensions, spatial references, error/validation
behavior, streaming, and output artifacts — must remain available and behave the
same.

Functionality is never dropped to make progress. Each piece of first-party C++
must end up in exactly one of these states:

1. ported to Rust behind the C ABI, with parity coverage;
2. an explicit, documented C++ holdout still reached through the C ABI; or
3. a known, written-down parity gap (in `rust/STATUS.md`) — not a silent loss.

Hard rules that follow from this goal:

- Do not remove a feature, weaken validation, or delete/relax a behavioral test
  to make the port build or "pass". That is a regression, not progress.
- Passing tests are necessary but **not sufficient** evidence of parity. If an
  existing C++ test is weak, strengthen it before relying on a Rust port.
- Removing genuinely dead or duplicate C++ (code no translation unit compiles or
  references) is allowed and encouraged — that removes unused code, not
  functionality. Verify it is truly unreferenced and that the behavior already
  runs through Rust before deleting.
- A discovered pre-existing bug or test failure is part of the contract too:
  fix it or record it explicitly; do not leave it silently broken.

## Architecture

- Rust code owns Rust `PointLayout`, `PointView`, and `Stage` values.
- C++ code calls Rust only through `pdal-capi`.
- The C ABI is the contract between languages. Python, CLI, and C++ should be
  peers above that contract over time.
- Existing C++ tests were the first parity gate. The pre-port GoogleTest
  baseline now passes through Rust-backed C ABI paths, so the next work is
  implementation replacement breadth, install/export readiness, and final
  regression confidence. Rust unit tests are necessary, but not sufficient.

## Agent Guardrails

This port is intentionally incremental. If you are an AI agent continuing this
work, do not broaden the scope just because a Rust crate exists.

Off limits unless the user explicitly revises this plan:

- Do not rewrite PDAL by directory or claim a directory is complete because it
  builds.
- Do not replace the C ABI with Rust/C++ direct object sharing.
- Do not pass C++ object pointers across the C ABI as Rust handles.
- Do not design a Rust plugin loading SDK yet. Optional plugin ports are allowed
  only as narrow compatibility checkpoints after the equivalent first-party
  reader/writer/filter/kernel path exists and the work has fixture or parity
  coverage.
- Do not copy vendored C/C++ code into Rust crates. Follow `rust/VENDOR.md`.
- Do not start remote I/O, broad optional plugin work, or broad vendor-heavy
  work without a concrete parity milestone. LAS/LAZ, the first GDAL reader
  path, simple command work, and narrow plugin checkpoints have started; new
  work in those families must stay narrow and regression-backed.
- Do not reopen a decision in `rust/DECISIONS.md` unless a concrete parity
  failure proves the current decision cannot preserve PDAL behavior.
- Do not add new concrete CLI commands just because `pdal-cli` can dispatch
  them. User-visible commands must satisfy the command readiness checkpoint
  below and include installed-PDAL regression coverage.
- Do not add placeholder modules, placeholder crates, or broad skeletons that
  are not tied to a concrete parity milestone.
- Do not weaken existing C++ validation or tests to make a Rust port pass.
- Do not mark a stage/reader/writer "ported" without behavior coverage and the
  relevant C++ parity gate.
- Do not let `unsafe` spread casually. New unsafe code should be limited to
  `pdal-capi` ownership/lifetime boundaries or explicit native FFI adapters,
  and each new cluster should explain why safe Rust cannot own that boundary.

## Finish-Line Milestones

Work toward these milestones in order. A later milestone may receive a small
boundary probe only when it directly unblocks an earlier one. Do not use this
roadmap as permission to sweep a directory.

0. Guard the contract continuously. **Always active.**
   Keep the existing C++ tests as the behavioral contract, keep Rust tests
   green, and keep every Rust-backed path behind the C ABI. No work counts if
   it weakens C++ validation, bypasses the C ABI, or lacks parity coverage.
1. Establish pre-port C++ test parity. **Complete for the current build.**
   The audit baseline is the pre-port C++ GoogleTest set, before local guard
   tests were added. It currently reports `819 / 819` Rust C ABI-backed cases.
   This proves the compatibility wrapper can satisfy the existing tests, but it
   is not the port finish line.
2. Convert the parity win into real implementation replacement.
   **Complete as an audit category for the current build.**
   For each area that already counts, identify whether Rust owns the actual
   behavior or merely owns a compatibility guard around C++ work. Replace the
   remaining substantial C++ implementation behind counted tests with Rust,
   or document an intentional C++ holdout when the boundary is external,
   vendor-heavy, or not worth porting yet. Track wrapper LOC so it shrinks
   instead of silently becoming the new permanent implementation. The current
   audit reports `0` unplanned port-candidate LOC, so do not reopen this as a
   generic C++ sweep unless a concrete parity failure identifies new
   implementation work.
3. Complete first-party filters by family.
   **Complete for the current scope.** Pure filters, spatial filters,
   linear/statistical filters, FFI-backed filters, and private-algorithm
   filters are classified in `STATUS.md`. Do not reopen filter-family work
   unless a concrete parity failure identifies new implementation work.
4. Complete deterministic first-party local I/O.
   **Complete for the current scope.** Local deterministic I/O, LAS/LAZ,
   covered GDAL/OGR paths, and selected remote/VSI workflows are classified in
   `STATUS.md`. New I/O work should be parity-gap-driven or an accepted
   native-adapter milestone, not a broad reader/writer sweep.
5. Close the core behaviors exposed by real pipelines.
   Expand `pdal-core` only as needed by stages, I/O, and commands: pipeline
   JSON, stage registry, layout mutation, dimensions, metadata, SRS, options,
   bounds, scaling, and error/reporting behavior. Avoid a standalone rewrite of
   `pdal/` by source directory.
6. Close apps, tools, and kernels as user-visible compatibility surfaces.
   `pdal-rs` can keep proving command parity, but top-layer migration closes
   only when commands, app dispatch, option/driver introspection, logging,
   error text, stdout/stderr shape, and produced artifacts match installed
   PDAL for agreed workflows. `lasdump` and `nitfwrap` remain narrow tool
   checkpoints unless their underlying format strategies are complete.
7. Stabilize build, install, packaging, CI, and downstream ABI exports.
   The Rust C ABI must have an install/export story, versioned symbols or an
   equivalent stability policy, CI coverage on supported platforms, release
   packaging rules, license/vendor accounting, and downstream `find_package`
   behavior before this can be treated as an upstreamable replacement.
8. Define plugin compatibility after the first-party surface.
   Keep broad optional plugins in C++ until the first-party library, command
   surface, C ABI versioning, ownership/lifetime rules, dynamic loading, and
   metadata behavior are stable. Tiny compatibility checkpoints such as
   `kernels.fauxplugin`, or a single plugin reader/writer that reuses an
   already-proven first-party path, may land earlier when they are
   regression-backed. `pdal-plugins` may hold discovery metadata earlier, but
   not a new plugin SDK.
9. Prove final replacement readiness.
   The port is not "done" until the Rust-backed stack can run the agreed
   first-party PDAL workflows with C++ tests green, Rust tests green, installed
   PDAL regression deltas explained, stable C ABI behavior, acceptable unsafe
   and native boundaries, and recorded performance, memory, binary-size,
   startup-time, compile-time, and full-suite timing comparisons.

Current active milestone:

1. Keep the two audits green, but do not mistake either for completion:
   `audit_cpp_test_parity.py` is currently `819 / 819`, and
   `audit_cpp_port_backlog.py --include-plugins` is currently `0`
   port-candidate LOC. Those are guardrails, not the finish line.
2. Do not reopen closed caveats in `rust/STATUS.md` as generic porting work.
   OGR/vector-source breadth, remote/STAC/schema breadth, optional plugins,
   and vendor-heavy behavior are accepted native-adapter/deferred boundaries
   unless a concrete parity failure proves otherwise.
3. Current useful work is upstream-readiness work: package/install/export
   verification, C ABI stability maintenance, CI/platform hardening, final
   regression/performance evidence, and keeping the release-blocking gates in
   `rust/DECISIONS.md` green.
4. New I/O, command, vendor, and plugin work should be narrow and
   fixture-backed. Broad sweeps are appropriate only after a closed boundary is
   deliberately reopened as a concrete, regression-testable milestone.
5. If work reaches a native dependency, plugin, or public C++ compatibility
   question, check `rust/DECISIONS.md`. A listed decision is not a TODO; it is
   the current shape unless a named parity gate fails.

Every commit should say which checkpoint it advances. If the answer is "none",
it probably should not be part of this port.

## Whole Repo Migration Map

Approximate first-party code size, excluding comments and blanks:

- `pdal/`: 22.4k LOC. Core point model, pipeline model, options, metadata,
  dimensions, layouts, tables, views, and utility code.
- `filters/`: 21.1k LOC. Pure transforms and algorithmic stages. This is the
  current spike area because it can prove the Rust core -> C ABI -> C++ wrapper
  loop without starting with file-format or GDAL/PROJ complexity.
- `io/`: 24.6k LOC. Readers and writers. The Rust port has an active local
  I/O path here now; keep additions narrow, fixture-scoped, and
  parity-backed.
- `kernels/`: 3.3k LOC. CLI subcommands. The first simple, pipeline-shaped
  commands are now present in `pdal-cli`; broader kernels remain late
  migration work.
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

The migration order is intentionally end-to-end driven: build only the
core pieces needed by the next stage family, prove parity through the C ABI,
then move outward. A broad rewrite by directory is not the plan.

## Target Rust Layout

The Rust tree is not intended to be a 1:1 mirror of the C++ source tree. C++
directories are migration source areas and behavioral references; Rust crates
are organized around contracts that should remain stable as the implementation
changes.

Current target crates:

- `pdal-core`: point model, dimensions, metadata, options, pipeline, SRS,
  spatial helpers, and shared stage traits.
- `pdal-native`: explicit native-library adapter boundary. GDAL/OGR, GEOS, and
  PROJ are already routed through this crate. Future native integrations should
  land here or behind an equally explicit adapter before higher-level crates
  depend on them. Pure Rust replacements, such as the current LAS/LAZ path, do
  not need to move through `pdal-native`.
- `pdal-capi`: stable C ABI. This is the real cross-language contract.
- `pdal-filters`: first-party filters.
- `pdal-io`: first-party readers and writers. The deterministic local I/O
  path is active here.
- `pdal-kernels`: CLI subcommands and command-contract helpers. Keep heavy
  kernel behavior out until the underlying core/I/O/filter capabilities have
  parity coverage.
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
3. `io/` after the filter/core loop is stable. This checkpoint is complete for
   the current scope: local text/ASCII/binary formats, LAS/LAZ, GDAL/OGR
   families, and covered remote/VSI paths are classified in `STATUS.md`. Future
   I/O work should start from a concrete parity failure or native-adapter
   milestone.
4. `apps/` and `tools/` after the library surface is stable enough to run real
   pipelines through the C ABI. This is small by LOC but high in dependency
   density: `apps/pdal.cpp` owns CLI dispatch and driver/option introspection,
   while `tools/lasdump` and `tools/nitfwrap` are tied to LAS/LAZ and NITF
   strategy decisions. The Rust `lasdump` path can cover uncompressed LAS
   behavior before the broader LAZ compression strategy is complete.
5. `kernels/` last. The first simple command surface lives in `pdal-cli`
   because the pipeline/reader/filter/writer loop can exercise it. Do not use
   that as permission to sweep kernels broadly; only add commands whose lower
   layers are already Rust-backed and regression-tested.
6. Optional plugins after the first-party library and command surface are
   stable. Until then, plugins stay in C++ or are handled only as metadata and
   discovery compatibility helpers.

Do not jump to broad `kernels/`, apps/tools, plugins, vendor-heavy work, or
broad `io/` work just because those areas are smaller or visible. The active
post-filter milestone is the local I/O plus simple command surface, kept
end-to-end driven.

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
- `tools/lasdump` can progress for uncompressed LAS once LAS header/VLR/point
  checksum behavior is parity-tested; LAZ checksum parity waits for the
  compression strategy.
- `tools/nitfwrap` can progress through a Nitro-backed adapter for the narrow
  wrap/unwrap tool workflow. Do not treat that as full `readers.nitf` or
  `writers.nitf` parity.

Do not mark apps/tools complete because the LOC is small. They close only when
their underlying command or format strategy has parity coverage.

Vendor compatibility work is allowed only when a concrete Rust port reaches the
stage, reader, writer, or core behavior that depends on that vendor boundary:

- Linear algebra choices happen with broader linear/statistical filters, behind
  shared Rust math APIs, not by porting `vendor/eigen`.
- GDAL/PROJ/GEOS choices happen with SRS, raster/vector, geometry, crop,
  overlay, reprojection, DEM, and related filters or readers. Native bindings
  should be centralized through `pdal-native` instead of imported directly by
  feature crates.
- LAS/LAZ compression is already covered by the `las` crate with its `laz`
  feature. Do not port or bind `vendor/lazperf` unless parity testing later
  proves the Rust replacement cannot match a required PDAL behavior.
- Remote/object-store compatibility happens only after local deterministic I/O
  and pipeline execution are stable.
- JSON-schema compatibility happens when Rust owns pipeline JSON validation.
- Private algorithm vendors such as Kazhdan are decided per stage: port to
  Rust, bind through explicit FFI, or leave the C++ stage in place.

Do not start a broad `vendor/` compatibility pass. Each vendor decision should
name the user-visible stage or core behavior it unlocks, cite the parity test
that will hold it honest, and follow `rust/VENDOR.md`.

Plugin implementation work is mostly late, but narrow compatibility checkpoints
are now allowed when they exercise an already-proven Rust family:

- `pdal-plugins` may keep metadata and filename-discovery helpers that mirror
  stable parts of the existing C++ plugin convention.
- Do not port optional plugin readers, writers, filters, or kernels unless the
  equivalent first-party reader/writer/filter/kernel family is already proven
  and the plugin is covered by focused fixtures or installed-PDAL parity.
- Do not design a Rust plugin loading SDK until the C ABI, stage registry,
  ownership/lifetime rules, metadata, errors, versioning, and dynamic library
  compatibility story are stable.
- Plugin-by-plugin ports should be driven by demand and parity tests, not by
  sweeping the `plugins/` directory.

In practice, most plugins are after first-party filters, core, I/O, apps/tools,
and commands. Early plugin work should stay to compatibility/discovery
validation, then a single low-risk plugin port that proves the boundary. Heavy
database, HDF/E57/NITF/TileDB, registration, trajectory, and format-specific
plugins remain later or stay C++ behind the ABI.

## Checkpoint Roadmap

Treat these as ordered checkpoints on the way to a complete Rust-backed PDAL.
They are the detailed work breakdown for the finish-line milestones above.
Each checkpoint should end in a commit with the listed gates passing. Do not
advance by claiming a directory is "done" only because it builds, and do not
skip a checkpoint because a placeholder crate or command stub exists.

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
- The full `pdal_filters_*` CTest subset passes.
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

- The full `pdal_filters_*` CTest subset passes.
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
- The full `pdal_filters_*` CTest subset passes.
- The shared Rust APIs are documented enough that new filters use them instead
  of duplicating local algorithms.

### 4. Former Deferred Filter Families

Goal: preserve the discipline used for formerly deferred filter families. This
is no longer the active filter worklist; see `Remaining Finish-Line Work`
and `STATUS.md` for current gaps.

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
- The full `pdal_filters_*` CTest subset passes.

### 5. Core Pipeline Checkpoint

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

### 6. I/O End-To-End Checkpoint

Goal: prove readers/writers after the core and filter ABI are stable.

This checkpoint is complete for the current scope. Keep the same discipline if
it is reopened: each reader/writer family should be narrow, fixture-scoped,
option-aware, and regression-tested against installed PDAL where possible.
Local deterministic I/O is broad, streaming coverage exists for many local
paths, and the implementation-replacement audit has no unplanned I/O
port-candidate files. Remaining I/O-related work is accepted-boundary work from
`STATUS.md`: broader GDAL/OGR/vector-source behavior, remote/schema/cloud
behavior, vendor-heavy adapters, and packaging/regression evidence.

Required shape:

- One reader and one writer path behind the C ABI.
- Byte-level and metadata behavior checked against existing C++ tests or
  fixtures.
- External dependencies stay external through Rust crates or explicit FFI.

Done when:

- The matching C++ I/O test binary passes.
- The pipeline checkpoint can use the ported I/O path end to end.
- Existing filter/core tests still pass.

Current reader/writer status and exact regression commands live in
`rust/STATUS.md`. Keep this roadmap focused on closing named caveats before
moving to broader remote, command, vendor, or plugin work.

### 7. Apps, Tools, Then Kernels

Goal: finish top-layer command behavior as a compatibility surface, not as a
new parallel CLI.

Required shape:

- `apps/` and `tools/` are now compatibility shells over Rust C ABI paths.
  Do not reopen them unless a command or format parity test shows a concrete
  divergence.
- All first-party kernel names are Rust-dispatchable through the C ABI for
  scoped workflows. Remaining command work is exact parity breadth: option
  edge cases, legacy file/content boundaries, OGR/update workflows, STAC
  geometry output, stdout/stderr byte shape, and installed-PDAL regression
  deltas.
- `pdal-cli` should not grow separate command implementations. It should route
  through the same Rust C ABI kernel runner used by the C++ app/kernel shells.
- CLI output, exit behavior, and artifacts must be regression-tested against
  installed PDAL before marking a command scope done.

Current status:

- `pdal-rs` is the Rust-native command shell for the port spike.
- Simple pipeline-shaped commands exist for Rust-backed local workflows.
- Command-ready filters, implemented commands, and exact regression commands
  live in `rust/STATUS.md`.
- Do not add additional kernels unless their lower-layer behavior is already
  Rust-backed and they can be tested against installed PDAL for exit status,
  stdout/stderr shape, and output artifacts.

Done when:

- The relevant app/tool/kernel tests pass.
- Regression comparisons against the C++ implementation are clean or explained.
- Lower-layer core/filter/I/O tests remain green.

## Current Status

The live implementation inventory, status definitions, command-ready filters,
remaining C++ filter families, and useful regression commands live in
`rust/STATUS.md`. Keep this file as the roadmap and rules document.

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

## Remaining Finish-Line Work

Do not use old filter-family buckets as the worklist. The filter and
implementation-replacement audits now have no unplanned port-candidate files.
The remaining work is upstream-readiness and release policy for the explicit
boundaries in `rust/STATUS.md`.

Accepted boundary classes:

- OGR/vector-source breadth: covered local vector reads/writes, tindex
  create append/dedup, and tindex merge reads are Rust-backed. Broad exotic
  OGR datasource/update workflows are a native-adapter boundary unless a
  concrete parity failure reopens them.
- EPT/COPC/STAC preview and remote breadth: covered EPT preview filters, local
  EPT/COPC/STAC workflows, and selected remote/VSI/object-store path forms are
  Rust-backed, including `FileSpec` query propagation and scoped GDAL VSI
  header forwarding through LAS/COPC, EPT root/hierarchy/tile reads, EPT
  addon/source-origin sidecars, STAC traversal/asset dispatch, and TIndex
  GeoJSON/asset dispatch. Broad remote traversal, object-store credential
  workflows, custom schema URL resolution, and native-connector parity are
  deferred/native-adapter boundaries unless a concrete parity failure reopens
  them.
- Metadata/pipeline structural parity: Rust owns covered structural behavior,
  and covered pipeline JSON serialization is semantically aligned with
  installed PDAL. Byte-for-byte JSON object key order is not the contract.
  Remaining artifact-shape work belongs to command stdout/stderr/XML/report
  outputs, not to `PipelineWriter` reimplementation.
- Packaging, CI, install/export, and release policy: these must prove the
  Rust C ABI as an installed, upstreamable surface across supported platforms.
- Optional plugins: the broad triage is closed for the current first-party port.
  `faux`, `nitf`, and `spz` are compatibility checkpoints; other plugins are
  native adapters or deferred by decision until a plugin SDK/versioning policy
  exists. Do not treat them as active port-candidate backlog.
- Performance and quality gates: coverage, unsafe tracking, C++ parity,
  C++ implementation-backlog classification, install/export, and source
  packaging are release-blocking gates per `rust/DECISIONS.md`; mutation
  testing and performance/memory/binary/startup/compile-time comparisons remain
  visibility gates until a controlled threshold policy is chosen.

If an accepted boundary is deliberately reopened, update `STATUS.md` and
`DECISIONS.md` with the concrete parity failure or policy change that justifies
the new milestone. Do not create placeholder Rust code for accepted native
adapters.

## Completion Criteria For Each Port

1. Rust unit/parity tests pass.
2. The matching C++ test binary passes.
3. The full `pdal_filters_*` CTest subset passes before leaving `filters/`.
4. The unsafe footprint is reviewed. Run:
   `rg -n "unsafe\\s*\\{|unsafe extern|unsafe fn" rust --glob '*.rs'`
   and confirm any new unsafe use is confined to C ABI wrappers, native FFI
   adapters, or a documented exception.
5. No unsafe reinterpret-cast crosses the C ABI.
6. The port preserves user-visible behavior, not just compile/link success.

For non-filter ports, replace item 3 with the matching focused CTest subset and
any lower-layer regression subset the change can affect. For example, I/O work
should run the matching `pdal_io_*` tests when C++ wrappers are involved, plus
the Rust workspace gates.

## Whole-Port Completion Criteria

The Rust port is ready to replace first-party PDAL behavior only when all of
these are true:

1. The agreed first-party reader, writer, filter, pipeline, and command surface
   runs through Rust-owned implementations behind the C ABI.
2. Existing C++ tests, Rust workspace tests, and command-level installed-PDAL
   regressions pass, or every remaining delta is explicit and accepted.
3. The C ABI is documented, versioned, and stable enough for C++, Python, and
   CLI layers to be peers above it.
4. CLI output, exit status, error shape, metadata, dimensions, bounds, SRS, and
   output artifact behavior match installed PDAL for covered workflows.
5. Native dependencies and vendor replacements are documented in
   `rust/VENDOR.md`, with no vendored C/C++ code copied into Rust crates.
6. Unsafe Rust is small, audited by boundary, and concentrated in `pdal-capi`
   and native FFI crates rather than algorithmic code.
7. Performance, memory usage, binary size, startup time, and compile time have
   comparison harnesses and no unexplained major regression.
8. Optional plugins either remain supported through the C++ compatibility path
   or have a deliberately versioned Rust plugin boundary with parity coverage.
