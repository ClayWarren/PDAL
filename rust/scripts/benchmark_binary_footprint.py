#!/usr/bin/env python3
"""Measure binary footprint for installed PDAL vs the Rust-backed build.

This is a visibility harness, not a release gate. It reports the executable
size and the size of the non-system shared-library closure reachable from each
binary. On macOS it resolves common `@rpath`, `@loader_path`, and
`@executable_path` references; on Linux it uses `ldd`.

Usage:
  benchmark_binary_footprint.py [--ref <pdal>] [--rust <pdal>]
"""

import argparse
import os
import platform
import re
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SYSTEM_PREFIXES = (
    "/System/",
    "/usr/lib/",
    "/lib/",
    "/lib64/",
    "/usr/lib64/",
)


def find_default_rust_pdal() -> str:
    for candidate in (REPO / ".build/bin/pdal", REPO / "build/bin/pdal"):
        if candidate.exists():
            return str(candidate)
    return ""


def find_default_reference_pdal() -> str:
    for candidate in ("/opt/homebrew/bin/pdal", "/usr/local/bin/pdal"):
        if Path(candidate).exists():
            return candidate
    return shutil.which("pdal") or ""


def resolve_binary(value: str) -> str:
    if not value:
        return ""
    path = Path(value)
    if path.exists():
        return str(path)
    return shutil.which(value) or ""


def is_system_path(path: Path) -> bool:
    text = str(path)
    return any(text.startswith(prefix) for prefix in SYSTEM_PREFIXES)


def run_text(argv: list[str]) -> str:
    result = subprocess.run(argv, check=True, capture_output=True, text=True)
    return result.stdout


def macos_rpaths(path: Path, executable: Path) -> list[Path]:
    text = run_text(["otool", "-l", str(path)])
    rpaths: list[Path] = []
    pending = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped == "cmd LC_RPATH":
            pending = True
            continue
        if pending and stripped.startswith("path "):
            raw = stripped.split()[1]
            raw = raw.replace("@loader_path", str(path.parent))
            raw = raw.replace("@executable_path", str(executable.parent))
            rpaths.append(Path(raw))
            pending = False
    return rpaths


def macos_deps(path: Path) -> list[str]:
    deps: list[str] = []
    for line in run_text(["otool", "-L", str(path)]).splitlines()[1:]:
        stripped = line.strip()
        if not stripped:
            continue
        deps.append(stripped.split(" (", 1)[0])
    return deps


def resolve_macos_dep(
    dep: str,
    loader: Path,
    executable: Path,
    rpaths: list[Path],
) -> Path | None:
    if dep.startswith("@loader_path/"):
        candidate = loader.parent / dep.removeprefix("@loader_path/")
        return candidate if candidate.exists() else None
    if dep.startswith("@executable_path/"):
        candidate = executable.parent / dep.removeprefix("@executable_path/")
        return candidate if candidate.exists() else None
    if dep.startswith("@rpath/"):
        suffix = dep.removeprefix("@rpath/")
        for rpath in rpaths:
            candidate = rpath / suffix
            if candidate.exists():
                return candidate
        return None
    path = Path(dep)
    if path.is_absolute():
        return path if path.exists() else None
    candidate = loader.parent / dep
    return candidate if candidate.exists() else None


def macos_closure(binary: Path) -> set[Path]:
    seen: set[Path] = set()
    queue = [binary.resolve()]
    extra_rpaths = [
        REPO / ".build/lib",
        REPO / "build/lib",
        Path("/opt/homebrew/lib"),
        Path("/usr/local/lib"),
    ]
    conda_prefix = os.environ.get("CONDA_PREFIX")
    if conda_prefix:
        extra_rpaths.append(Path(conda_prefix) / "lib")

    while queue:
        current = queue.pop()
        if current in seen or not current.exists() or is_system_path(current):
            continue
        seen.add(current)
        rpaths = macos_rpaths(current, binary) + extra_rpaths
        for dep in macos_deps(current):
            resolved = resolve_macos_dep(dep, current, binary, rpaths)
            if resolved is not None:
                queue.append(resolved.resolve())
    return seen


_LDD_RE = re.compile(r"=>\s+(\S+)\s+\(")


def linux_closure(binary: Path) -> set[Path]:
    seen: set[Path] = {binary.resolve()}
    for line in run_text(["ldd", str(binary)]).splitlines():
        match = _LDD_RE.search(line)
        if not match:
            continue
        dep = Path(match.group(1))
        if dep.exists() and not is_system_path(dep):
            seen.add(dep.resolve())
    return seen


def closure(binary: Path) -> set[Path]:
    if platform.system() == "Darwin":
        return macos_closure(binary)
    if platform.system() == "Linux":
        return linux_closure(binary)
    return {binary.resolve()}


def mib(num_bytes: int) -> float:
    return num_bytes / (1024.0 * 1024.0)


def size_of(paths: set[Path]) -> int:
    return sum(path.stat().st_size for path in paths if path.exists())


def version(binary: Path) -> str:
    result = subprocess.run([str(binary), "--version"], capture_output=True, text=True)
    for line in result.stdout.splitlines():
        if line.strip().startswith("pdal "):
            return line.strip()
    return "unknown"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ref", default=find_default_reference_pdal())
    parser.add_argument("--rust", default=find_default_rust_pdal())
    args = parser.parse_args()

    ref = resolve_binary(args.ref)
    rust = resolve_binary(args.rust)
    if not ref:
        print("error: reference pdal not found (pass --ref)", file=sys.stderr)
        return 2
    if not rust:
        print("error: rust-backed pdal not found (pass --rust)", file=sys.stderr)
        return 2

    ref_path = Path(ref).resolve()
    rust_path = Path(rust).resolve()
    ref_closure = closure(ref_path)
    rust_closure = closure(rust_path)

    print(f"# reference (C++): {ref_path}  [{version(ref_path)}]")
    print(f"# rust-backed:     {rust_path}  [{version(rust_path)}]")
    print(f"# platform: {platform.system()}")
    print("# system libraries are excluded from closure totals\n")
    print("binary,exe_mib,closure_mib,closure_files")
    ref_size = size_of(ref_closure)
    rust_size = size_of(rust_closure)
    print(f"reference_cpp,{mib(ref_path.stat().st_size):.2f},{mib(ref_size):.2f},{len(ref_closure)}")
    print(f"rust_backed,{mib(rust_path.stat().st_size):.2f},{mib(rust_size):.2f},{len(rust_closure)}")
    if ref_size:
        print(f"ratio_rust_to_cpp,,{rust_size / ref_size:.2f},")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
