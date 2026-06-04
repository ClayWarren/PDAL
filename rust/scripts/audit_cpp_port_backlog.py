#!/usr/bin/env python3
"""Audit the remaining first-party C++ implementation still to be ported to Rust.

The test-parity audit (`audit_cpp_test_parity.py`) reached 100%: every built C++
GoogleTest case is Rust C ABI-backed. That milestone proved the compatibility
layer satisfies the pre-port behavioral contract, but it does not measure how
much *implementation* still lives in C++. This script measures the next goal:
remaining first-party C++ source that is neither glue/wrapper nor a documented
intentional holdout, i.e. the real porting backlog.

Each first-party C++ file under the mainline areas (`pdal/`, `filters/`, `io/`,
`kernels/`, `apps/`, `tools/`) is classified as:

  - c-abi-backed: includes the Rust C ABI header (`pdal_capi.h`) or the
    `RustViewConverter.hpp` bridge (which itself pulls in `pdal_capi.h`), or is
    a known Rust bridge header. Already routes meaningful behavior through Rust
    (pure wrapper or a mixed file that still hides some implementation). A
    `.cpp`/`.hpp` pair is treated as one unit: a `.cpp` that delegates usually
    pulls the ABI in through its own paired header, so the signal propagates
    across the stem in both directions.
  - native-adapter: a C++ binding over an external native library or remote/
    vendor adapter path (GDAL/OGR, libgeotiff, GEOS, PROJ, Arbiter-backed STAC)
    whose Rust home is `pdal-native`/explicit FFI or a later remote I/O adapter,
    not a pure-Rust reimplementation. Tracked separately from the pure-Rust
    backlog.
  - holdout: a documented intentional C++ holdout (see HOLDOUTS).
  - port-candidate: pure C++ with no C ABI reference and not otherwise
    categorized. This is the actionable backlog.

LOC is non-blank/non-comment code, measured with `cloc` when available to match
the wrapper-LOC methodology in `rust/STATUS.md`; otherwise a conservative local
counter keeps the audit runnable on platforms where `cloc` is not packaged.

Usage:
  rust/scripts/audit_cpp_port_backlog.py
  rust/scripts/audit_cpp_port_backlog.py --area io --top 40
  rust/scripts/audit_cpp_port_backlog.py --show holdout
  rust/scripts/audit_cpp_port_backlog.py --include-plugins
"""

from __future__ import annotations

import argparse
from collections import defaultdict
import csv
import io
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

MAINLINE_AREAS = ["pdal", "filters", "io", "kernels", "apps", "tools"]
SOURCE_EXTS = (".cpp", ".hpp", ".h", ".cc", ".cxx")

# The reliable wrapper signal: you cannot call the Rust C ABI without including
# its header, and you cannot convert a PointView/raster across that ABI without
# the RustViewConverter bridge header (which itself includes pdal_capi.h). A
# file that includes either is, by construction, routing meaningful behavior
# through Rust -- a pure wrapper or a mixed file that still hides some C++.
C_ABI_HEADER_MARKERS = ("pdal_capi.h", "RustViewConverter.hpp")
KNOWN_BRIDGE_FILES = {
    "io/FbiHeader.hpp",
    "io/HeaderVal.hpp",
    "io/OptechCommon.hpp",
    "io/PcdHeader.hpp",
    "io/SbetCommon.hpp",
    "io/private/copcwriter/Common.hpp",
    "filters/private/RustMetadata.hpp",
    "filters/private/RustPipeline.hpp",
    "pdal/compression/Compression.hpp",
    "pdal/compression/GzipCompression.hpp",
    "pdal/KDIndex.cpp",
    "pdal/KDIndex.hpp",
    "pdal/private/FileSpecHelper.hpp",
}

# C++ bindings over external native libraries. Their Rust home is `pdal-native`
# or explicit FFI (see rust/VENDOR.md), not a from-scratch Rust reimplementation,
# so they are tracked apart from the pure-Rust backlog.
NATIVE_ADAPTER_PREFIXES = (
    "io/private/connector/",
    "io/private/esri/",
    "io/private/stac/",
    "pdal/private/gdal/",
    "plugins/arrow/",
    "plugins/draco/",
    # Bundled third-party library used by the optional E57 plugin. Treat it
    # like vendor/native dependency code, not PDAL implementation to port
    # line-by-line.
    "plugins/e57/io/",
    "plugins/e57/libE57Format/",
    "plugins/hdf/",
    "plugins/icebridge/",
    "plugins/matlab/",
    "plugins/mbio/",
    "plugins/pgpointcloud/",
    "plugins/rdb/",
    "plugins/rxp/",
    "plugins/tiledb/",
    "plugins/trajectory/",
)

NATIVE_ADAPTER_FILES = {
    "filters/private/Point.cpp",
    "filters/private/Point.hpp",
    "io/EsriReader.cpp",
    "io/EsriReader.hpp",
    "io/I3SReader.cpp",
    "io/I3SReader.hpp",
    "io/SlpkReader.cpp",
    "io/SlpkReader.hpp",
    "io/private/GDALGrid.cpp",
    "io/private/GDALGrid.hpp",
    "io/private/copc/Tile.cpp",
    "io/private/copc/Tile.hpp",
    # Arbiter/Connector-backed EPT per-tile fetch. Local supported EPT reads
    # route through the Rust reader in EptReader::ready(); TileContents only
    # runs on the remote (or 2D-bounds-with-SRS) Connector fallback, the same
    # shape as the io/private/stac native-adapter files.
    "io/private/ept/TileContents.cpp",
    "io/private/ept/TileContents.hpp",
    "io/private/las/ChunkInfo.hpp",
    "io/private/las/Geotiff.cpp",
    "io/private/las/Geotiff.hpp",
    "kernels/private/density/OGR.cpp",
    "kernels/private/density/OGR.hpp",
    "plugins/cpd/filters/CpdFilter.cpp",
    "plugins/cpd/filters/CpdFilter.hpp",
    "plugins/openscenegraph/io/OSGReader.cpp",
    "plugins/openscenegraph/io/OSGReader.hpp",
    "plugins/nitf/io/NitfFileWriter.cpp",
    "plugins/nitf/io/NitfFileWriter.hpp",
    "plugins/nitf/io/tre_plugins.cpp",
    "plugins/nitf/io/tre_plugins.hpp",
    "plugins/teaser/filters/TeaserFilter.cpp",
    "plugins/teaser/filters/TeaserFilter.hpp",
    "pdal/DynamicLibrary.cpp",
    "pdal/private/DynamicLibrary.hpp",
    "pdal/compression/LazPerfVlrCompression.cpp",
    "pdal/compression/LazPerfVlrCompression.hpp",
    "pdal/util/Backtrace.cpp",
    "pdal/util/Backtrace.hpp",
    "pdal/util/portable_endian.hpp",
    "pdal/util/private/BacktraceImpl.hpp",
    "pdal/util/VSIIO.cpp",
    "pdal/util/VSIIO.hpp",
    "pdal/util/private/BacktraceExecinfo.cpp",
    "pdal/util/private/BacktraceNone.cpp",
    "pdal/util/private/BacktraceUnwind.cpp",
}

# Documented intentional C++ holdouts. Keep this list small and cited; prefer
# moving a file out of here once its port lands rather than letting it grow.
HOLDOUTS = {
    # Permanent C++ public compatibility shims. These are not portable PDAL
    # implementation and should not be ported behind the Rust C ABI.
    "pdal/JsonFwd.hpp": "deprecated public nlohmann forward-declaration shim",
    "pdal/PluginHelper.hpp": "C++ plugin registration macro compatibility surface",
    "pdal/PluginInfo.hpp": "C++ plugin metadata compatibility surface",
    "pdal/PointContainer.hpp": "intentional compile-time error compatibility header",
    "pdal/pdal.hpp": "C++ umbrella include compatibility surface",
    "pdal/pdal_export.hpp": "C++ symbol export macro compatibility surface",
    "pdal/pdal_internal.hpp": "C++ internal compatibility include",
    "pdal/pdal_types.hpp": "C++ public typedef/exception/log-level compatibility surface",
    "pdal/util/pdal_util_export.hpp": "C++ utility symbol export macro compatibility surface",
    "pdal/util/pdal_util_internal.hpp": "C++ utility platform macro compatibility surface",
    "pdal/util/Extractor.hpp": "C++ public endian buffer extractor compatibility surface",
    "pdal/util/Inserter.hpp": "C++ public endian buffer inserter compatibility surface",
    "pdal/util/IStream.hpp": "C++ public std::istream/endian stream compatibility surface",
    "pdal/util/NullOStream.hpp": "C++ public logging ostream compatibility surface",
    "pdal/util/OStream.hpp": "C++ public std::ostream/endian stream compatibility surface",
    "pdal/Artifact.hpp": "C++ artifact compatibility base class",
    "pdal/ArtifactManager.hpp": "C++ artifact manager compatibility surface",
    "pdal/DbReader.cpp": "C++ database reader base compatibility shell",
    "pdal/DbReader.hpp": "C++ database reader base compatibility shell",
    "pdal/DbWriter.cpp": "C++ database writer base compatibility shell",
    "pdal/DbWriter.hpp": "C++ database writer base compatibility shell",
    "pdal/DimDetail.hpp": "C++ dimension detail compatibility surface",
    "pdal/DimType.hpp": "C++ dimension/type pair compatibility surface",
    "pdal/FlexWriter.hpp": "C++ flexible writer base compatibility shell",
    "pdal/Mesh.hpp": "C++ mesh compatibility surface",
    "pdal/PointRef.cpp": "C++ point reference compatibility surface",
    "pdal/PointRef.hpp": "C++ point reference compatibility surface",
    "pdal/PointTable.cpp": "C++ point table compatibility surface",
    "pdal/PointTable.hpp": "C++ point table compatibility surface",
    "pdal/PipelineExecutor.cpp": "deprecated C++ PipelineManager facade compatibility surface",
    "pdal/PipelineExecutor.hpp": "deprecated C++ PipelineManager facade compatibility surface",
    "pdal/PipelineManager.cpp": "C++ mutable Stage*/PointViewSet pipeline SDK compatibility surface",
    "pdal/PipelineManager.hpp": "C++ mutable Stage*/PointViewSet pipeline SDK compatibility surface",
    "pdal/QuickInfo.hpp": "C++ quick-info compatibility surface",
    "pdal/Reader.cpp": "C++ reader base compatibility shell",
    "pdal/Reader.hpp": "C++ reader base compatibility shell",
    "pdal/StageWrapper.hpp": "C++ test/compatibility access wrapper",
    "pdal/SubcommandKernel.cpp": "C++ subcommand kernel compatibility shell",
    "pdal/SubcommandKernel.hpp": "C++ subcommand kernel compatibility shell",
    "pdal/private/StageRunner.cpp": "C++ stage-runner compatibility helper",
    "pdal/private/StageRunner.hpp": "C++ stage-runner compatibility helper",
    "pdal/util/Algorithm.hpp": "C++ public algorithm helper compatibility surface",
    "pdal/util/Random.cpp": "C++ std::mt19937 compatibility surface",
    "pdal/util/Random.hpp": "C++ std::mt19937 compatibility surface",
    "io/OptechRotationMatrix.hpp": "empty public compatibility include",
    "io/point_types.hpp": "PCL point type compatibility header",
    "io/BpfHeader.cpp": "exported BPF header compatibility API surface",
    "io/BpfHeader.hpp": "exported BPF header compatibility API surface",
    "io/BufferReader.hpp": "exported C++ in-memory reader compatibility shell",
    "io/LasHeader.cpp": "exported deprecated LAS header compatibility API surface",
    "io/LasHeader.hpp": "exported deprecated LAS header compatibility API surface",
    "io/LasVLR.cpp": "exported deprecated LAS VLR compatibility API surface",
    "io/LasVLR.hpp": "exported deprecated LAS VLR compatibility API surface",
    "io/private/ept/Artifact.hpp": "C++ EPT artifact compatibility shell",
    "io/private/ept/Addon.cpp": "C++ EPT addon metadata/layout compatibility shell",
    "io/private/ept/Addon.hpp": "C++ EPT addon metadata/layout compatibility shell",
    "io/private/ept/FixedPointLayout.cpp": "C++ PointLayout ordering compatibility adapter",
    "io/private/ept/FixedPointLayout.hpp": "C++ PointLayout ordering compatibility adapter",
    "io/private/ept/Overlap.hpp": "C++ EPT addon hierarchy compatibility key",
    "io/private/ept/VectorPointTable.hpp": "C++ PointTable view compatibility adapter",
    "pdal/util/private/JsonSupport.hpp": "C++ nlohmann/Options JSON compatibility glue",
    # C++ compatibility argument-binding API used by exported C++ shells. The
    # Rust CLI/kernel path has its own parser; this remains glue for the legacy
    # C++ API surface (rust/STATUS.md C++ app shell row).
    "pdal/util/ProgramArgs.hpp": "C++ compatibility CLI argument-binding glue (rust/STATUS.md)",
    # C++ compatibility callback over PointRef. PORTING.md / STATUS.md: routing
    # C++ callables across the C ABI needs a deliberate callback ABI design.
    "filters/StreamCallbackFilter.cpp": "C++ callback ABI holdout (rust/STATUS.md)",
    "filters/StreamCallbackFilter.hpp": "C++ callback ABI holdout (rust/STATUS.md)",
}


# GoogleTest harness header included by every in-tree C++ test. Files that
# include it are behavioral-contract tests, not portable implementation, and
# must be excluded from the backlog even when they live under a mainline area
# (e.g. `tools/nitfwrap/NitfWrapTest.cpp`) rather than under `test/`.
TEST_MARKER = "pdal_test_main.hpp"


def is_in_tree_test(path: str) -> bool:
    if path == "pdal/pdal_test_main.hpp":
        return True
    if path.startswith("plugins/") and "/test/" in path:
        return True
    try:
        return TEST_MARKER in Path(path).read_text(errors="replace")
    except OSError:
        return False


def list_source_files(areas: list[str]) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", *areas],
        check=True,
        capture_output=True,
        text=True,
    )
    files = []
    for path in out.stdout.splitlines():
        if path.endswith(SOURCE_EXTS) and not is_in_tree_test(path):
            files.append(path)
    return sorted(files)


def is_c_abi_backed(path: str) -> bool:
    if path in KNOWN_BRIDGE_FILES:
        return True
    try:
        text = Path(path).read_text(errors="replace")
    except OSError:
        return False
    return any(marker in text for marker in C_ABI_HEADER_MARKERS)


IMPL_EXTS = (".cpp", ".cc", ".cxx")
HEADER_EXTS = (".hpp", ".h")


def classify_own(path: str) -> str:
    """Classify a file from its own path and content."""
    if path in HOLDOUTS:
        return "holdout"
    if path in NATIVE_ADAPTER_FILES:
        return "native-adapter"
    if any(path.startswith(prefix) for prefix in NATIVE_ADAPTER_PREFIXES):
        return "native-adapter"
    if is_c_abi_backed(path):
        return "c-abi-backed"
    return "port-candidate"


def stem(path: str) -> str:
    return path.rsplit(".", 1)[0]


def classify_all(files: list[str]) -> dict[str, str]:
    """Classify every file, treating a `.cpp`/`.hpp` pair sharing a stem as one
    translation unit. A `.cpp` routinely pulls the C ABI in through its own
    paired header (e.g. `BpfWriter.cpp` includes `BpfWriter.hpp`, which includes
    `pdal_capi.h`), and conversely an interface header is backed by its `.cpp`'s
    delegation. So the wrapper signal propagates in both directions: if either
    file of a stem is c-abi-backed, the whole stem is. Header-only files are
    classified on their own content (they are the real implementation)."""
    own = {path: classify_own(path) for path in files}

    # A stem is c-abi-backed if any of its files is.
    abi_stems = {
        stem(path) for path, cat in own.items() if cat == "c-abi-backed"
    }

    result: dict[str, str] = {}
    for path in files:
        cat = own[path]
        if cat == "port-candidate" and stem(path) in abi_stems:
            cat = "c-abi-backed"
        result[path] = cat
    return result


def cloc_loc(files: list[str]) -> dict[str, int]:
    """Return a {path: code_loc} map using cloc's by-file CSV output."""
    if not files:
        return {}
    if shutil.which("cloc") is None:
        return fallback_loc(files)
    loc: dict[str, int] = {}
    with tempfile.NamedTemporaryFile("w", suffix=".lst", delete=False) as fh:
        fh.write("\n".join(files))
        listfile = fh.name
    try:
        out = subprocess.run(
            ["cloc", "--quiet", "--csv", "--by-file", f"--list-file={listfile}"],
            check=True,
            capture_output=True,
            text=True,
        )
    finally:
        Path(listfile).unlink(missing_ok=True)
    reader = csv.DictReader(io.StringIO(out.stdout))
    for row in reader:
        filename = row.get("filename")
        code = row.get("code")
        if not filename or code is None:
            continue
        # cloc prefixes with "./" sometimes; normalize to the path we passed.
        norm = filename[2:] if filename.startswith("./") else filename
        loc[norm] = int(code)
    return loc


def fallback_loc(files: list[str]) -> dict[str, int]:
    """Count non-blank, non-comment source lines when cloc is unavailable."""
    return {path: count_code_lines(path) for path in files}


def count_code_lines(path: str) -> int:
    try:
        text = Path(path).read_text(errors="replace")
    except OSError:
        return 0

    loc = 0
    in_block = False
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped:
            continue

        code_seen = False
        i = 0
        while i < len(stripped):
            if in_block:
                end = stripped.find("*/", i)
                if end == -1:
                    i = len(stripped)
                else:
                    in_block = False
                    i = end + 2
                continue

            if stripped.startswith("//", i):
                break
            if stripped.startswith("/*", i):
                in_block = True
                i += 2
                continue

            code_seen = True
            break

        if code_seen:
            loc += 1
    return loc


def area_of(path: str) -> str:
    return path.split("/", 1)[0]


def plugin_of(path: str) -> str:
    parts = path.split("/")
    return parts[1] if len(parts) > 1 and parts[0] == "plugins" else ""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--area", help="Restrict the ranked list to one area (e.g. io)")
    parser.add_argument("--top", type=int, default=30, help="How many backlog files to list (default 30)")
    parser.add_argument(
        "--show",
        choices=["port-candidate", "holdout", "native-adapter", "c-abi-backed"],
        default="port-candidate",
        help="Which category to rank in the file list (default port-candidate)",
    )
    parser.add_argument("--include-plugins", action="store_true", help="Also scan plugins/ (deferred area)")
    parser.add_argument(
        "--json-report",
        type=Path,
        help="Optional path to write a machine-readable summary.",
    )
    args = parser.parse_args()

    areas = list(MAINLINE_AREAS)
    if args.include_plugins:
        areas.append("plugins")

    files = list_source_files(areas)
    loc = cloc_loc(files)
    file_category = classify_all(files)

    # category -> area -> [loc, file_count]
    summary: dict[str, dict[str, list[int]]] = {}
    for path in files:
        cat = file_category[path]
        code = loc.get(path, 0)
        summary.setdefault(cat, {}).setdefault(area_of(path), [0, 0])
        summary[cat][area_of(path)][0] += code
        summary[cat][area_of(path)][1] += 1

    categories = ["port-candidate", "c-abi-backed", "native-adapter", "holdout"]

    print("Remaining C++ implementation backlog (first-party mainline source)")
    print(f"Areas scanned: {', '.join(areas)}")
    print()
    header = f"{'category':<16}{'LOC':>10}{'files':>8}"
    print(header)
    print("-" * len(header))
    grand_loc = grand_files = 0
    for cat in categories:
        cat_loc = sum(v[0] for v in summary.get(cat, {}).values())
        cat_files = sum(v[1] for v in summary.get(cat, {}).values())
        grand_loc += cat_loc
        grand_files += cat_files
        print(f"{cat:<16}{cat_loc:>10}{cat_files:>8}")
    print("-" * len(header))
    print(f"{'total':<16}{grand_loc:>10}{grand_files:>8}")

    print()
    print("Port-candidate backlog by area (the work that remains):")
    pc = summary.get("port-candidate", {})
    for area in sorted(pc, key=lambda a: pc[a][0], reverse=True):
        print(f"  {area:<10}{pc[area][0]:>8} LOC  ({pc[area][1]} files)")

    if args.include_plugins:
        plugin_rows: dict[str, dict[str, list[int]]] = defaultdict(
            lambda: defaultdict(lambda: [0, 0])
        )
        for path in files:
            plugin = plugin_of(path)
            if not plugin:
                continue
            cat = file_category[path]
            plugin_rows[plugin][cat][0] += loc.get(path, 0)
            plugin_rows[plugin][cat][1] += 1

        print()
        print("Plugin backlog by plugin:")
        print(
            f"{'plugin':<18}{'port':>11}{'abi':>11}{'native':>11}{'holdout':>11}"
        )
        for plugin in sorted(
            plugin_rows,
            key=lambda name: plugin_rows[name]["port-candidate"][0],
            reverse=True,
        ):
            row = plugin_rows[plugin]

            def cell(cat: str) -> str:
                return f"{row[cat][0]}/{row[cat][1]}"

            print(
                f"{plugin:<18}{cell('port-candidate'):>11}"
                f"{cell('c-abi-backed'):>11}{cell('native-adapter'):>11}"
                f"{cell('holdout'):>11}"
            )

    print()
    label = args.show
    ranked = [
        (loc.get(p, 0), p)
        for p, cat in file_category.items()
        if cat == label and (not args.area or area_of(p) == args.area)
    ]
    ranked.sort(reverse=True)
    scope = f" in {args.area}/" if args.area else ""
    print(f"Top {min(args.top, len(ranked))} {label} files{scope} by code LOC:")
    for code, path in ranked[: args.top]:
        note = f"   # {HOLDOUTS[path]}" if path in HOLDOUTS else ""
        print(f"  {code:>6}  {path}{note}")

    if args.json_report:
        category_totals = {}
        for cat in categories:
            category_totals[cat] = {
                "loc": sum(v[0] for v in summary.get(cat, {}).values()),
                "files": sum(v[1] for v in summary.get(cat, {}).values()),
            }
        area_totals = {}
        for cat, areas_for_cat in summary.items():
            for area, values in areas_for_cat.items():
                area_totals.setdefault(area, {})[cat] = {
                    "loc": values[0],
                    "files": values[1],
                }
        plugin_totals = {}
        if args.include_plugins:
            for path in files:
                plugin = plugin_of(path)
                if not plugin:
                    continue
                cat = file_category[path]
                entry = plugin_totals.setdefault(plugin, {}).setdefault(
                    cat, {"loc": 0, "files": 0}
                )
                entry["loc"] += loc.get(path, 0)
                entry["files"] += 1

        report = {
            "areas": areas,
            "categories": category_totals,
            "total": {"loc": grand_loc, "files": grand_files},
            "port_candidate_by_area": {
                area: {"loc": values[0], "files": values[1]}
                for area, values in sorted(pc.items())
            },
            "area_totals": area_totals,
            "plugin_totals": plugin_totals,
            "ranked": [
                {
                    "path": path,
                    "loc": code,
                    "category": label,
                    "note": HOLDOUTS.get(path, ""),
                }
                for code, path in ranked[: args.top]
            ],
        }
        args.json_report.parent.mkdir(parents=True, exist_ok=True)
        args.json_report.write_text(json.dumps(report, indent=2) + "\n")
        print(f"\nWrote JSON report: {args.json_report}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
