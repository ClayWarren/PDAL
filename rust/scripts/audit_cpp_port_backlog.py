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
  - native-adapter: a C++ binding over an external native library (GDAL/OGR,
    libgeotiff, GEOS, PROJ) whose Rust home is `pdal-native`/explicit FFI, not a
    pure-Rust reimplementation. Tracked separately from the pure-Rust backlog.
  - holdout: a documented intentional C++ holdout (see HOLDOUTS).
  - port-candidate: pure C++ with no C ABI reference and not otherwise
    categorized. This is the actionable backlog.

LOC is non-blank/non-comment code, measured with `cloc` to match the wrapper-LOC
methodology in `rust/STATUS.md`.

Usage:
  rust/scripts/audit_cpp_port_backlog.py
  rust/scripts/audit_cpp_port_backlog.py --area io --top 40
  rust/scripts/audit_cpp_port_backlog.py --show holdout
  rust/scripts/audit_cpp_port_backlog.py --include-plugins
"""

from __future__ import annotations

import argparse
import csv
import io
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
    "filters/private/RustMetadata.hpp",
    "filters/private/RustPipeline.hpp",
    "pdal/KDIndex.cpp",
    "pdal/KDIndex.hpp",
}

# C++ bindings over external native libraries. Their Rust home is `pdal-native`
# or explicit FFI (see rust/VENDOR.md), not a from-scratch Rust reimplementation,
# so they are tracked apart from the pure-Rust backlog.
NATIVE_ADAPTER_PREFIXES = (
    "pdal/private/gdal/",
)

NATIVE_ADAPTER_FILES = {
    "io/private/GDALGrid.cpp",
    "io/private/GDALGrid.hpp",
    "kernels/private/density/OGR.cpp",
    "kernels/private/density/OGR.hpp",
    "pdal/DynamicLibrary.cpp",
    "pdal/private/DynamicLibrary.hpp",
    "pdal/compression/LazPerfVlrCompression.cpp",
    "pdal/compression/LazPerfVlrCompression.hpp",
    "pdal/util/VSIIO.cpp",
    "pdal/util/VSIIO.hpp",
    "pdal/util/private/BacktraceExecinfo.cpp",
    "pdal/util/private/BacktraceNone.cpp",
    "pdal/util/private/BacktraceUnwind.cpp",
}

# Documented intentional C++ holdouts. Keep this list small and cited; prefer
# moving a file out of here once its port lands rather than letting it grow.
HOLDOUTS = {
    # GeoTIFF GeoKey <-> CRS encoding via libgeotiff. rust/STATUS.md: "GeoTIFF
    # VLR encoding remains C++ libgeotiff-backed."
    "io/private/las/Geotiff.cpp": "libgeotiff GeoKey encoding (rust/VENDOR.md, no Rust replacement chosen)",
    "io/private/las/Geotiff.hpp": "libgeotiff GeoKey encoding (rust/VENDOR.md, no Rust replacement chosen)",
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


def area_of(path: str) -> str:
    return path.split("/", 1)[0]


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

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
