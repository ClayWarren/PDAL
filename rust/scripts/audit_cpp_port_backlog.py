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

  - c-abi-backed: includes the Rust C ABI header, or is a known Rust bridge
    header. Already routes meaningful behavior through Rust (pure wrapper or a
    mixed file that still hides some implementation).
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
# its header. A handful of bridge headers are glue by definition.
C_ABI_HEADER_MARKERS = ("pdal_capi.h",)
KNOWN_BRIDGE_FILES = {
    "filters/private/RustMetadata.hpp",
    "filters/private/RustPipeline.hpp",
}

# C++ bindings over external native libraries. Their Rust home is `pdal-native`
# or explicit FFI (see rust/VENDOR.md), not a from-scratch Rust reimplementation,
# so they are tracked apart from the pure-Rust backlog.
NATIVE_ADAPTER_PREFIXES = (
    "pdal/private/gdal/",
)

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


def list_source_files(areas: list[str]) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", *areas],
        check=True,
        capture_output=True,
        text=True,
    )
    files = []
    for path in out.stdout.splitlines():
        if path.endswith(SOURCE_EXTS):
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
    if any(path.startswith(prefix) for prefix in NATIVE_ADAPTER_PREFIXES):
        return "native-adapter"
    if is_c_abi_backed(path):
        return "c-abi-backed"
    return "port-candidate"


def stem(path: str) -> str:
    return path.rsplit(".", 1)[0]


def classify_all(files: list[str]) -> dict[str, str]:
    """Classify every file. A header inherits its sibling .cpp's category when
    one exists, so interface headers paired with a Rust-backed implementation
    are not double-counted as backlog. Header-only files are classified on their
    own content (they are the real implementation)."""
    impl_category: dict[str, str] = {}
    for path in files:
        if path.endswith(IMPL_EXTS):
            impl_category[path] = classify_own(path)
    impl_by_stem = {stem(p): cat for p, cat in impl_category.items()}

    result: dict[str, str] = dict(impl_category)
    for path in files:
        if path in result:
            continue
        inherited = impl_by_stem.get(stem(path))
        result[path] = inherited if inherited is not None else classify_own(path)
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
