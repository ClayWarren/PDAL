#!/usr/bin/env python3
"""Benchmark peak memory (RSS) of a pure-C++ `pdal` vs the Rust-backed `pdal`.

Companion to `benchmark_cpp_vs_rust.py` (wall-clock) and the `binary_size.rs`
harness (size/startup). Both binaries are full `pdal` CLIs of the same version,
so running identical commands through each and recording peak resident set size
isolates the memory impact of the Rust-backed implementation behind the C ABI.

Peak RSS is sampled out-of-process so it captures the whole process, not just
allocations a Rust allocator hook would see:
  - macOS/BSD: `/usr/bin/time -l` ("maximum resident set size", bytes)
  - Linux:     `/usr/bin/time -v` ("Maximum resident set size", kibibytes)

Usage:
  benchmark_memory_cpp_vs_rust.py [--ref <pdal>] [--rust <pdal>] [--iters N]

Defaults: --ref = Homebrew `pdal` when present, otherwise `pdal` on PATH;
--rust = build/bin/pdal.
Prints a CSV-style table of median peak RSS (MiB) and the rust/reference ratio
for each workload.
"""

import argparse
import json
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


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


def env_for(binary: str) -> dict[str, str]:
    """Prefer the build-tree libpdalcpp when running a build-tree binary."""
    env = os.environ.copy()
    try:
        binary_path = Path(binary).resolve()
    except OSError:
        return env

    for build_dir in (REPO / ".build", REPO / "build"):
        bin_dir = (build_dir / "bin").resolve()
        lib_dir = (build_dir / "lib").resolve()
        try:
            if binary_path.is_relative_to(bin_dir) and lib_dir.exists():
                lib = str(lib_dir)
                for name in ("DYLD_LIBRARY_PATH", "LD_LIBRARY_PATH"):
                    current = env.get(name, "")
                    env[name] = lib if not current else f"{lib}:{current}"
                break
        except ValueError:
            continue
    return env


def _macos_time_argv(binary: str, args: list[str]) -> list[str]:
    return ["/usr/bin/time", "-l", binary, *args]


def _linux_time_argv(binary: str, args: list[str]) -> list[str]:
    return ["/usr/bin/time", "-v", binary, *args]


# Both BSD `time -l` and GNU `time -v` print a "... resident set size" line.
_RSS_RE = re.compile(r"(\d+)\s+maximum resident set size", re.IGNORECASE)
_RSS_GNU_RE = re.compile(r"Maximum resident set size[^\d]*(\d+)", re.IGNORECASE)


def peak_rss_bytes(binary: str, args: list[str]) -> int:
    """Run the command once under the platform timer and return peak RSS bytes."""
    is_linux = platform.system() == "Linux"
    argv = _linux_time_argv(binary, args) if is_linux else _macos_time_argv(binary, args)
    result = subprocess.run(
        argv,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        env=env_for(binary),
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"{binary} {' '.join(args)} failed:\n{result.stderr}"
        )
    text = result.stderr
    if is_linux:
        match = _RSS_GNU_RE.search(text)
        if not match:
            raise RuntimeError(f"could not parse GNU time output:\n{text}")
        return int(match.group(1)) * 1024  # GNU reports kibibytes
    match = _RSS_RE.search(text)
    if not match:
        raise RuntimeError(f"could not parse BSD time output:\n{text}")
    return int(match.group(1))  # BSD reports bytes


def median_rss_bytes(binary: str, args: list[str], iters: int) -> float:
    peak_rss_bytes(binary, args)  # warm-up (caches, dynamic-link resolution)
    return statistics.median(peak_rss_bytes(binary, args) for _ in range(iters))


def mib(num_bytes: float) -> float:
    return num_bytes / (1024.0 * 1024.0)


def write_pipeline(path: Path, stages: list[dict]) -> None:
    path.write_text(json.dumps({"pipeline": stages}))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ref", default=find_default_reference_pdal())
    parser.add_argument("--rust", default=find_default_rust_pdal())
    parser.add_argument("--iters", type=int, default=7)
    args = parser.parse_args()

    args.ref = resolve_binary(args.ref)
    args.rust = resolve_binary(args.rust)

    if not args.ref:
        print("error: reference pdal not found (pass --ref)", file=sys.stderr)
        return 2
    if not args.rust:
        print("error: rust-backed pdal not found (pass --rust)", file=sys.stderr)
        return 2
    if not Path("/usr/bin/time").exists():
        print("error: /usr/bin/time not found (needed for peak RSS)", file=sys.stderr)
        return 2

    def version(binary: str) -> str:
        out = subprocess.run(
            [binary, "--version"],
            capture_output=True,
            env=env_for(binary),
            text=True,
        )
        for line in out.stdout.splitlines():
            if line.strip().startswith("pdal "):
                return line.strip()
        return "unknown"

    print(f"# reference (C++): {args.ref}  [{version(args.ref)}]")
    print(f"# rust-backed:     {args.rust}  [{version(args.rust)}]")
    print(f"# iterations: {args.iters} (median peak RSS); platform: {platform.system()}\n")

    tmp = Path(tempfile.mkdtemp(prefix="pdal-membench-"))
    las = REPO / "test/data/las/autzen_trim.las"

    # Memory-bound synthetic pipeline that dwarfs baseline process footprint.
    faux_sort = tmp / "faux_sort.json"
    write_pipeline(
        faux_sort,
        [
            {"type": "readers.faux", "count": 3000000, "mode": "random",
             "bounds": "([0,1000],[0,1000],[0,1000])"},
            {"type": "filters.sort", "dimension": "X"},
            {"type": "writers.null"},
        ],
    )

    las_decimate = tmp / "las_decimate.json"
    write_pipeline(
        las_decimate,
        [
            {"type": "readers.las", "filename": str(las)},
            {"type": "filters.decimation", "step": 2},
            {"type": "writers.las", "filename": str(tmp / "out.las")},
        ],
    )

    workloads = [
        ("baseline (--version)", ["--version"]),
        ("faux3M -> sort -> null", ["pipeline", str(faux_sort)]),
        ("las read -> decimate -> las write", ["pipeline", str(las_decimate)]),
        ("info --stats (autzen_trim.las)", ["info", "--stats", str(las)]),
    ]

    print("workload,ref_cpp_mib,rust_backed_mib,ratio_rust_to_cpp")
    for name, cmd in workloads:
        ref = median_rss_bytes(args.ref, cmd, args.iters)
        rust = median_rss_bytes(args.rust, cmd, args.iters)
        print(f"{name},{mib(ref):.2f},{mib(rust):.2f},{rust / ref:.2f}")

    shutil.rmtree(tmp, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
