#!/usr/bin/env python3
"""Check that the installed C ABI header matches implemented C ABI symbols."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
HEADER = ROOT / "rust/pdal-capi/include/pdal_capi.h"
RUST_SRC = ROOT / "rust/pdal-capi/src"
CONFIG_ABI = RUST_SRC / "config_abi.rs"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def strip_c_comments(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)
    return re.sub(r"//.*", "", text)


def header_functions() -> set[str]:
    text = strip_c_comments(read(HEADER))
    functions: set[str] = set()
    for match in re.finditer(
        r"([^;{}#]*(?:\n[^;{}#]*)*?\b(pdal_[A-Za-z0-9_]+)\s*\([^;{}]*\)\s*;)",
        text,
    ):
        declaration = match.group(1)
        name = match.group(2)
        if re.search(r"^\s*typedef\b", declaration):
            continue
        functions.add(name)
    return functions


def rust_functions() -> set[str]:
    functions: set[str] = set()
    for path in RUST_SRC.rglob("*.rs"):
        rel = path.relative_to(RUST_SRC)
        if rel.parts[0] == "tests" or rel.name == "tests.rs":
            continue
        text = read(path)
        functions.update(
            re.findall(r'extern\s+"C"\s+fn\s+(pdal_[A-Za-z0-9_]+)\s*\(', text)
        )
    return functions


def unguarded_rust_functions() -> set[str]:
    """Return exported Rust ABI functions missing the shared panic boundary."""
    functions: set[str] = set()
    declaration = re.compile(
        r"(?P<attrs>(?:(?:\s*#\[[^\]]+\]\s*)|(?:\s*///[^\n]*\n))*)"
        r"pub\s+(?:unsafe\s+)?extern\s+\"C\"\s+fn\s+"
        r"(?P<name>pdal_[A-Za-z0-9_]+)\s*\(",
        flags=re.MULTILINE,
    )
    for path in RUST_SRC.rglob("*.rs"):
        rel = path.relative_to(RUST_SRC)
        if rel.parts[0] == "tests" or rel.name == "tests.rs":
            continue
        for match in declaration.finditer(read(path)):
            if "pdal_capi_macros::ffi_export" not in match.group("attrs"):
                functions.add(match.group("name"))
    return functions


def cpp_functions() -> set[str]:
    functions: set[str] = set()
    for path in RUST_SRC.rglob("*.cpp"):
        text = read(path)
        for match in re.finditer(
            r"PDAL_CAPI_EXPORT[\s\S]{0,240}?\b(pdal_[A-Za-z0-9_]+)\s*\(",
            text,
        ):
            functions.add(match.group(1))
    return functions


def header_versions() -> dict[str, int]:
    text = read(HEADER)
    versions: dict[str, int] = {}
    for name in ("MAJOR", "MINOR", "PATCH"):
        match = re.search(rf"#define\s+PDAL_CAPI_ABI_VERSION_{name}\s+(\d+)u\b", text)
        if not match:
            raise ValueError(f"missing PDAL_CAPI_ABI_VERSION_{name} in {HEADER}")
        versions[name] = int(match.group(1))
    return versions


def rust_versions() -> dict[str, int]:
    text = read(CONFIG_ABI)
    versions: dict[str, int] = {}
    for name in ("MAJOR", "MINOR", "PATCH"):
        match = re.search(
            rf"PDAL_CAPI_ABI_VERSION_{name}:\s*u32\s*=\s*(\d+);", text
        )
        if not match:
            raise ValueError(f"missing PDAL_CAPI_ABI_VERSION_{name} in {CONFIG_ABI}")
        versions[name] = int(match.group(1))
    return versions


def version_number(parts: dict[str, int]) -> int:
    return parts["MAJOR"] * 1_000_000 + parts["MINOR"] * 1_000 + parts["PATCH"]


def main() -> int:
    header = header_functions()
    implemented = rust_functions() | cpp_functions()

    missing = sorted(header - implemented)
    undocumented = sorted(implemented - header)

    errors: list[str] = []
    if missing:
        errors.append("Header declarations without implementation:\n  " + "\n  ".join(missing))
    if undocumented:
        errors.append("Implemented C ABI symbols missing from header:\n  " + "\n  ".join(undocumented))

    unguarded = sorted(unguarded_rust_functions())
    if unguarded:
        errors.append(
            "Rust C ABI symbols missing #[pdal_capi_macros::ffi_export]:\n  "
            + "\n  ".join(unguarded)
        )

    h_versions = header_versions()
    r_versions = rust_versions()
    if h_versions != r_versions:
        errors.append(f"ABI version mismatch: header={h_versions}, rust={r_versions}")

    header_version = version_number(h_versions)
    runtime_version = version_number(r_versions)
    if header_version != runtime_version:
        errors.append(
            f"Packed ABI version mismatch: header={header_version}, rust={runtime_version}"
        )

    if errors:
        print("\n\n".join(errors), file=sys.stderr)
        return 1

    print(
        f"C ABI header sync ok: {len(header)} declarations, "
        f"ABI {h_versions['MAJOR']}.{h_versions['MINOR']}.{h_versions['PATCH']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
