#!/usr/bin/env python3
"""Benchmark a pure-C++ `pdal` against the Rust-backed `pdal` build.

Both binaries are full `pdal` CLIs of the same version, so running identical
commands through each (process-spawned for both) isolates the impact of the
Rust-backed implementation behind the C ABI -- unlike the in-process
`perf_regression` harness, which compares a spawned C++ process against an
in-process Rust pipeline and is therefore dominated by process-startup cost.

Usage:
  benchmark_cpp_vs_rust.py [--ref <pdal>] [--rust <pdal>] [--iters N]

Defaults: --ref = Homebrew `pdal` when present, otherwise `pdal` on PATH;
--rust = build/bin/pdal.
Prints a CSV-style table of median wall-clock milliseconds and the
rust/reference ratio for each workload.
"""

import argparse
import json
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
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


def run_once(binary: str, args: list[str]) -> float:
    start = time.perf_counter()
    result = subprocess.run(
        [binary, *args],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        env=env_for(binary),
    )
    elapsed = (time.perf_counter() - start) * 1000.0
    if result.returncode != 0:
        raise RuntimeError(
            f"{binary} {' '.join(args)} failed:\n{result.stderr.decode(errors='replace')}"
        )
    return elapsed


def median_ms(binary: str, args: list[str], iters: int) -> float:
    run_once(binary, args)  # warm-up (caches, dynamic-link resolution)
    return statistics.median(run_once(binary, args) for _ in range(iters))


def write_pipeline(path: Path, stages: list[dict]) -> None:
    path.write_text(json.dumps({"pipeline": stages}))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ref", default=find_default_reference_pdal())
    parser.add_argument("--rust", default=find_default_rust_pdal())
    parser.add_argument("--iters", type=int, default=15)
    args = parser.parse_args()

    args.ref = resolve_binary(args.ref)
    args.rust = resolve_binary(args.rust)

    if not args.ref:
        print("error: reference pdal not found (pass --ref)", file=sys.stderr)
        return 2
    if not args.rust:
        print("error: rust-backed pdal not found (pass --rust)", file=sys.stderr)
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
    print(f"# iterations: {args.iters} (median wall-clock)\n")

    tmp = Path(tempfile.mkdtemp(prefix="pdal-bench-"))
    las = REPO / "test/data/las/autzen_trim.las"

    # CPU/memory-bound synthetic pipeline that dwarfs process startup.
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
        ("startup (--version)", ["--version"]),
        ("faux3M -> sort -> null", ["pipeline", str(faux_sort)]),
        ("las read -> decimate -> las write", ["pipeline", str(las_decimate)]),
        ("info --stats (autzen_trim.las)", ["info", "--stats", str(las)]),
    ]

    print("workload,ref_cpp_ms,rust_backed_ms,ratio_rust_to_cpp")
    for name, cmd in workloads:
        ref = median_ms(args.ref, cmd, args.iters)
        rust = median_ms(args.rust, cmd, args.iters)
        print(f"{name},{ref:.2f},{rust:.2f},{rust / ref:.2f}")

    shutil.rmtree(tmp, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
