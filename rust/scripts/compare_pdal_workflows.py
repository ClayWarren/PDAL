#!/usr/bin/env python3
"""Compare installed PDAL with this Rust-backed build on workflow outputs.

This is an acceptance harness, not a unit-test replacement. It separates
byte-for-byte contracts from semantic contracts:

  * exact cases compare produced artifacts byte-for-byte.
  * semantic cases normalize intentionally non-contractual formatting or compare
    point payloads after both outputs are read by the same binary.

Defaults:
  --ref  = Homebrew /opt/homebrew/bin/pdal when available, otherwise PATH pdal
  --rust = .build/bin/pdal, otherwise build/bin/pdal
"""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parents[2]


def find_default_reference_pdal() -> str:
    for candidate in ("/opt/homebrew/bin/pdal", "/usr/local/bin/pdal"):
        if Path(candidate).exists():
            return candidate
    return shutil.which("pdal") or ""


def find_default_rust_pdal() -> str:
    for candidate in (REPO / ".build/bin/pdal", REPO / "build/bin/pdal"):
        if candidate.exists():
            return str(candidate)
    return ""


def resolve_binary(value: str) -> str:
    if not value:
        return ""
    path = Path(value)
    if path.exists():
        return str(path)
    return shutil.which(value) or ""


def env_for(binary: str) -> dict[str, str]:
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


def run(binary: str, args: list[str], *, stdout: int | None = subprocess.PIPE) -> subprocess.CompletedProcess[bytes]:
    result = subprocess.run(
        [binary, *args],
        stdout=stdout,
        stderr=subprocess.PIPE,
        env=env_for(binary),
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"{binary} {' '.join(args)} failed:\n"
            f"{result.stderr.decode(errors='replace')}"
        )
    return result


def run_raw(binary: str, args: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [binary, *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env_for(binary),
    )


def write_pipeline(path: Path, stages: list[dict[str, Any]]) -> None:
    path.write_text(json.dumps({"pipeline": stages}, indent=2))


def compare_bytes(left: Path, right: Path) -> None:
    left_bytes = left.read_bytes()
    right_bytes = right.read_bytes()
    if left_bytes != right_bytes:
        raise AssertionError(
            f"bytes differ: {left} ({len(left_bytes)} bytes) vs "
            f"{right} ({len(right_bytes)} bytes)"
        )


def parse_ascii_pcd(path: Path) -> tuple[dict[str, str], list[list[float]]]:
    header: dict[str, str] = {}
    rows: list[list[float]] = []
    in_data = False
    for raw in path.read_text().splitlines():
        line = raw.strip()
        if not line:
            continue
        if not in_data:
            parts = line.split(maxsplit=1)
            key = parts[0].upper()
            value = parts[1] if len(parts) == 2 else ""
            header[key] = value
            if key == "DATA":
                if value != "ascii":
                    raise AssertionError(f"{path} is not ASCII PCD")
                in_data = True
            continue
        rows.append([float(value) for value in line.split()])
    return header, rows


def compare_pcd_ascii(left: Path, right: Path) -> None:
    left_header, left_rows = parse_ascii_pcd(left)
    right_header, right_rows = parse_ascii_pcd(right)
    for key in ("FIELDS", "SIZE", "TYPE", "COUNT", "WIDTH", "HEIGHT", "POINTS", "DATA"):
        if left_header.get(key) != right_header.get(key):
            raise AssertionError(f"PCD header {key} differs: {left_header.get(key)!r} != {right_header.get(key)!r}")
    if len(left_rows) != len(right_rows):
        raise AssertionError(f"PCD row count differs: {len(left_rows)} != {len(right_rows)}")
    for row_idx, (left_row, right_row) in enumerate(zip(left_rows, right_rows)):
        if len(left_row) != len(right_row):
            raise AssertionError(f"PCD row {row_idx} width differs")
        for col_idx, (a, b) in enumerate(zip(left_row, right_row)):
            if not math.isclose(a, b, rel_tol=0.0, abs_tol=1e-9):
                raise AssertionError(f"PCD row {row_idx} col {col_idx} differs: {a} != {b}")


def parse_csv_numbers(path: Path) -> tuple[list[str], list[dict[str, float]]]:
    lines = path.read_text().splitlines()
    if not lines:
        raise AssertionError(f"{path} is empty")
    header = lines[0].split(",")
    rows: list[dict[str, float]] = []
    for raw in lines[1:]:
        if not raw.strip():
            continue
        values = raw.split(",")
        if len(values) != len(header):
            raise AssertionError(f"{path} row width differs from header")
        rows.append({name: float(value) for name, value in zip(header, values)})
    return header, rows


def compare_csv_common_numeric(left: Path, right: Path, tolerances: dict[str, float]) -> None:
    left_header, left_rows = parse_csv_numbers(left)
    right_header, right_rows = parse_csv_numbers(right)
    common = [name for name in left_header if name in right_header]
    if len(left_rows) != len(right_rows):
        raise AssertionError(f"CSV row count differs: {len(left_rows)} != {len(right_rows)}")
    for row_idx, (left_row, right_row) in enumerate(zip(left_rows, right_rows)):
        for name in common:
            tol = tolerances.get(name, 1e-9)
            a = left_row[name]
            b = right_row[name]
            if not math.isclose(a, b, rel_tol=0.0, abs_tol=tol):
                raise AssertionError(f"CSV row {row_idx} {name} differs: {a} != {b} (tol {tol})")


def compare_completed_exact(left: subprocess.CompletedProcess[bytes], right: subprocess.CompletedProcess[bytes]) -> None:
    if left.returncode != right.returncode:
        raise AssertionError(f"exit status differs: {left.returncode} != {right.returncode}")
    if left.stdout != right.stdout:
        raise AssertionError("stdout differs")
    if left.stderr != right.stderr:
        raise AssertionError("stderr differs")


def parse_pdal_version(output: bytes) -> str:
    match = re.search(r"\bpdal\s+([0-9]+(?:\.[0-9]+)+)\b", output.decode(errors="replace"))
    if not match:
        raise AssertionError(f"unable to parse pdal version from {output!r}")
    return match.group(1)


def command_names(output: bytes) -> set[str]:
    names: set[str] = set()
    for raw in output.decode(errors="replace").splitlines():
        line = raw.strip()
        if not line:
            continue
        if line.startswith("- "):
            line = line[2:].strip()
        if line and " " not in line and line.replace("_", "").replace("-", "").isalnum():
            names.add(line)
    return names


def canonical_las_points(pdal: str, source: Path, output: Path) -> None:
    pipeline = output.with_suffix(".json")
    write_pipeline(
        pipeline,
        [
            {"type": "readers.las", "filename": str(source)},
            {
                "type": "writers.text",
                "filename": str(output),
                "order": "X,Y,Z,Intensity,ReturnNumber,NumberOfReturns",
                "quote_header": False,
                "precision": 8,
            },
        ],
    )
    run(pdal, ["pipeline", str(pipeline)], stdout=subprocess.DEVNULL)


def compare_info_stats(ref: str, rust: str, las: Path) -> None:
    ref_json = json.loads(run(ref, ["info", "--stats", str(las)]).stdout)
    rust_json = json.loads(run(rust, ["info", "--stats", str(las)]).stdout)
    ref_stats = {item["name"]: item for item in ref_json["stats"]["statistic"]}
    rust_stats = {item["name"]: item for item in rust_json["stats"]["statistic"]}
    if ref_stats.keys() != rust_stats.keys():
        raise AssertionError("info --stats dimension set differs")
    numeric_keys = ("count", "minimum", "maximum", "average", "stddev", "variance")
    for dim, ref_item in ref_stats.items():
        rust_item = rust_stats[dim]
        for key in numeric_keys:
            if key not in ref_item and key not in rust_item:
                continue
            if key not in ref_item or key not in rust_item:
                raise AssertionError(f"info --stats {dim}.{key} presence differs")
            a = float(ref_item[key])
            b = float(rust_item[key])
            if not math.isclose(a, b, rel_tol=0.0, abs_tol=1e-7):
                raise AssertionError(f"info --stats {dim}.{key} differs: {a} != {b}")


def case_version_exact(ref: str, rust: str, _tmp: Path) -> None:
    ref_result = run_raw(ref, ["--version"])
    rust_result = run_raw(rust, ["--version"])
    if ref_result.returncode != rust_result.returncode:
        raise AssertionError(f"exit status differs: {ref_result.returncode} != {rust_result.returncode}")
    if ref_result.stderr != rust_result.stderr:
        raise AssertionError("stderr differs")
    ref_version = parse_pdal_version(ref_result.stdout)
    rust_version = parse_pdal_version(rust_result.stdout)
    if ref_version != rust_version:
        raise AssertionError(f"pdal version differs: {ref_version} != {rust_version}")


def case_unknown_command_exact(ref: str, rust: str, _tmp: Path) -> None:
    compare_completed_exact(
        run_raw(ref, ["__definitely_missing__"]),
        run_raw(rust, ["__definitely_missing__"]),
    )


def case_list_commands_semantic(ref: str, rust: str, _tmp: Path) -> None:
    ref_result = run(ref, ["--list-commands"])
    rust_result = run(rust, ["--list-commands"])
    ref_commands = command_names(ref_result.stdout)
    rust_commands = command_names(rust_result.stdout)
    if ref_commands != rust_commands:
        raise AssertionError(
            f"command set differs: ref-only={sorted(ref_commands - rust_commands)} "
            f"rust-only={sorted(rust_commands - ref_commands)}"
        )


def case_text_decimation_exact(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/text/utm17_1.txt"
    ref_out = tmp / "text-ref.txt"
    rust_out = tmp / "text-rust.txt"
    for binary, output in ((ref, ref_out), (rust, rust_out)):
        pipeline = output.with_suffix(".json")
        write_pipeline(
            pipeline,
            [
                {"type": "readers.text", "filename": str(input_path)},
                {"type": "filters.decimation", "step": 2},
                {
                    "type": "writers.text",
                    "filename": str(output),
                    "order": "X,Y,Z",
                    "quote_header": False,
                    "precision": 2,
                },
            ],
        )
        run(binary, ["pipeline", str(pipeline)], stdout=subprocess.DEVNULL)
    compare_bytes(ref_out, rust_out)


def case_pcd_decimation_semantic(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/pcd/utm17_space.pcd"
    ref_out = tmp / "pcd-ref.pcd"
    rust_out = tmp / "pcd-rust.pcd"
    for binary, output in ((ref, ref_out), (rust, rust_out)):
        pipeline = output.with_suffix(".json")
        write_pipeline(
            pipeline,
            [
                {"type": "readers.pcd", "filename": str(input_path)},
                {"type": "filters.decimation", "step": 2},
                {
                    "type": "writers.pcd",
                    "filename": str(output),
                    "order": "X,Y,Z",
                    "precision": 2,
                },
            ],
        )
        run(binary, ["pipeline", str(pipeline)], stdout=subprocess.DEVNULL)
    compare_pcd_ascii(ref_out, rust_out)


def case_las_head_points_semantic(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/autzen/autzen-utm.las"
    ref_las = tmp / "head-ref.las"
    rust_las = tmp / "head-rust.las"
    for binary, output in ((ref, ref_las), (rust, rust_las)):
        pipeline = output.with_suffix(".json")
        write_pipeline(
            pipeline,
            [
                {"type": "readers.las", "filename": str(input_path)},
                {"type": "filters.head", "count": 100},
                {"type": "writers.las", "filename": str(output)},
            ],
        )
        run(binary, ["pipeline", str(pipeline)], stdout=subprocess.DEVNULL)

    ref_txt = tmp / "head-ref.txt"
    rust_txt = tmp / "head-rust.txt"
    canonical_las_points(rust, ref_las, ref_txt)
    canonical_las_points(rust, rust_las, rust_txt)
    compare_csv_common_numeric(ref_txt, rust_txt, {"ScanAngleRank": 0.0021})


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ref", default=find_default_reference_pdal())
    parser.add_argument("--rust", default=find_default_rust_pdal())
    parser.add_argument("--keep-temp", action="store_true")
    args = parser.parse_args()

    ref = resolve_binary(args.ref)
    rust = resolve_binary(args.rust)
    if not ref:
        print("error: reference pdal not found (pass --ref)", file=sys.stderr)
        return 2
    if not rust:
        print("error: rust-backed pdal not found (pass --rust)", file=sys.stderr)
        return 2

    tmp = Path(tempfile.mkdtemp(prefix="pdal-workflow-parity-"))
    cases = [
        ("pdal --version semantic version", "semantic", case_version_exact),
        ("unknown command exit/stderr", "exact", case_unknown_command_exact),
        ("pdal --list-commands command set", "semantic", case_list_commands_semantic),
        ("text decimation artifact bytes", "exact", case_text_decimation_exact),
        ("PCD decimation point payload", "semantic", case_pcd_decimation_semantic),
        ("LAS head point payload", "semantic", case_las_head_points_semantic),
        (
            "info --stats numeric payload",
            "semantic",
            lambda ref_bin, rust_bin, _tmp: compare_info_stats(
                ref_bin, rust_bin, REPO / "test/data/las/autzen_trim.las"
            ),
        ),
    ]

    print(f"# reference:   {ref}")
    print(f"# rust-backed: {rust}")
    print(f"# temp:        {tmp}\n")
    try:
        for name, contract, func in cases:
            func(ref, rust, tmp)
            print(f"ok,{contract},{name}")
    finally:
        if args.keep_temp:
            print(f"\n# kept temp directory: {tmp}")
        else:
            shutil.rmtree(tmp, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
