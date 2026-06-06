# Rust Port Parity And Backlog Accounting

This ledger tracks the Rust port evidence that is too detailed for
`rust/STATUS.md`: C++ GoogleTest parity counting, implementation-replacement
backlog classification, mixed-test notes, and the commands used to recompute
those numbers.

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
behavioral contract; it is not the finish line. The implementation-replacement
audit below is also green: remaining first-party C++ is classified as
glue/wrappers, native adapters, or documented holdouts. The active goal is now
to keep those audits green while making install/export/CI/regression,
performance, and release-policy evidence strong enough for an upstreamable
port.

## C++ Implementation-Replacement Backlog

With test parity at 100%, the next-goal metric is how much first-party C++ is
*real implementation* still to be ported, versus glue/wrappers and documented
holdouts. Measure it with:

```sh
python3 rust/scripts/audit_cpp_port_backlog.py
python3 rust/scripts/audit_cpp_port_backlog.py --area io --top 40
python3 rust/scripts/audit_cpp_port_backlog.py --include-plugins --top 40
python3 rust/scripts/audit_cpp_port_backlog.py --show holdout
python3 rust/scripts/audit_cpp_port_backlog.py --include-plugins --json-report /tmp/pdal-cpp-backlog.json
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
- `holdout`: a documented intentional C++ holdout. These are exported C++ SDK
  compatibility surfaces, callback/debug boundaries, public stream/endian
  helper APIs, deprecated public compatibility classes, or narrow build-tool
  exceptions. They are not unplanned implementation backlog; reopen one only
  when the Rust port has an explicit replacement API plus parity coverage for
  the C++ surface it would retire.
  Keep this list small and cited in the script.
- `port-candidate`: pure C++ with no C ABI reference — the actionable backlog.

Current snapshot (mainline, excluding `test/`, `vendor/`, and optional
`plugins/`):

| category | LOC | files |
|---|---:|---:|
| port-candidate | 0 | 0 |
| c-abi-backed | 45,206 | 398 |
| native-adapter | 6,195 | 59 |
| holdout | 6,670 | 64 |
| total | 58,071 | 521 |

Port-candidate backlog by area: `pdal`, `io`, `filters`,
`kernels`, `apps` and `tools` are now at 0 (apps is a thin entry-point
peer; the only `tools` entry the audit had been counting was the in-tree
GoogleTest `tools/nitfwrap/NitfWrapTest.cpp`, which is behavioral contract, not
implementation — the audit now excludes files including `pdal_test_main.hpp`).
With `--include-plugins`, optional plugin port-candidate backlog is also 0
(`45,789` C ABI-backed LOC, `32,722` native-adapter LOC, `6,670` holdout LOC,
`85,181` total LOC): plugin tests are excluded, C ABI-backed checkpoints are
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
remote adapter work: local supported STAC paths route through Rust on
non-Windows, while those files preserve the Windows full-execution path plus
Arbiter/remote/schema fallback behavior for the later remote I/O milestone.
The COPC writer private `Common.hpp` option/header shell counts with
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
the port, is `972 / 1069` currently built C++ GoogleTest cases, or `90.93%`;
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
`45,206` c-abi-backed code LOC across `398` first-party files, plus `6,195`
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
