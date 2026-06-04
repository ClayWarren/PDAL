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


def write_pipeline_array(path: Path, stages: list[Any]) -> None:
    path.write_text(json.dumps(stages, indent=2))


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


def pcd_point_count(path: Path) -> int:
    header, rows = parse_ascii_pcd(path)
    points = int(header.get("POINTS", len(rows)))
    if points != len(rows):
        raise AssertionError(f"{path} POINTS header differs from row count")
    return points


def assert_unit_cube_pcd(path: Path) -> None:
    header, rows = parse_ascii_pcd(path)
    fields = header.get("FIELDS", "").split()
    required = ["x", "y", "z"]
    indexes = []
    for name in required:
        if name not in fields:
            raise AssertionError(f"{path} missing PCD field {name}")
        indexes.append(fields.index(name))
    for row_idx, row in enumerate(rows):
        for name, col_idx in zip(required, indexes):
            value = row[col_idx]
            if not 0.0 <= value <= 1.0:
                raise AssertionError(f"{path} row {row_idx} {name}={value} outside unit cube")


def pcd_field_values(path: Path, field: str) -> list[float]:
    header, rows = parse_ascii_pcd(path)
    fields = header.get("FIELDS", "").split()
    field_names = [name.lower() for name in fields]
    needle = field.lower()
    if needle not in field_names:
        raise AssertionError(f"{path} missing PCD field {field}")
    index = field_names.index(needle)
    return [row[index] for row in rows]


def pcd_counts_by_prefix(directory: Path, prefix: str) -> list[int]:
    return sorted(pcd_point_count(path) for path in directory.glob(f"{prefix}_*.pcd"))


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


def canonical_xyz_text(pdal: str, source: Path, output: Path) -> None:
    pipeline = output.with_suffix(".json")
    write_pipeline_array(
        pipeline,
        [
            str(source),
            {
                "type": "writers.text",
                "filename": str(output),
                "order": "X,Y,Z",
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


def compare_info_summary(ref: str, rust: str, path: Path) -> None:
    ref_json = json.loads(run(ref, ["info", "--summary", str(path)]).stdout)
    rust_json = json.loads(run(rust, ["info", "--summary", str(path)]).stdout)
    if summary_count(ref_json) != summary_count(rust_json):
        raise AssertionError("info --summary point count differs")
    ref_bounds = summary_bounds(ref_json)
    rust_bounds = summary_bounds(rust_json)
    for key in ("minx", "maxx", "miny", "maxy", "minz", "maxz"):
        a = float(ref_bounds[key])
        b = float(rust_bounds[key])
        if not math.isclose(a, b, rel_tol=0.0, abs_tol=1e-9):
            raise AssertionError(f"info --summary {key} differs: {a} != {b}")


def summary_count(value: dict[str, Any]) -> int:
    for node in (value, value.get("summary", {})):
        if isinstance(node, dict):
            for key in ("point_count", "num_points"):
                count = node.get(key)
                if count is not None:
                    return int(count)
    raise AssertionError("unable to find summary point count")


def summary_bounds(value: dict[str, Any]) -> dict[str, Any]:
    bounds = value.get("bounds_3d")
    if isinstance(bounds, dict):
        return bounds
    summary = value.get("summary", {})
    bounds = summary.get("bounds") if isinstance(summary, dict) else None
    if isinstance(bounds, dict):
        return bounds
    raise AssertionError("unable to find summary bounds")


def assert_json_number_close(left: Any, right: Any, path: str, tolerance: float = 1e-6) -> None:
    a = float(left)
    b = float(right)
    if not math.isclose(a, b, rel_tol=0.0, abs_tol=tolerance):
        raise AssertionError(f"{path} differs: {a} != {b}")


def compare_delta_command(ref: str, rust: str, source: Path, candidate: Path) -> None:
    ref_json = json.loads(run(ref, ["delta", str(source), str(candidate)]).stdout)
    rust_json = json.loads(run(rust, ["delta", str(source), str(candidate)]).stdout)
    for dim in ("X", "Y", "Z"):
        for stat in ("min", "mean", "max"):
            assert_json_number_close(ref_json[dim][stat], rust_json[dim][stat], f"delta {dim}.{stat}")


def compare_metric_command(ref: str, rust: str, command: str, source: Path, candidate: Path, keys: tuple[str, ...]) -> None:
    ref_json = json.loads(run(ref, [command, str(source), str(candidate)]).stdout)
    rust_json = json.loads(run(rust, [command, str(source), str(candidate)]).stdout)
    for key in keys:
        assert_json_number_close(ref_json[key], rust_json[key], f"{command} {key}")


def parse_confusion_matrix(value: Any) -> Any:
    if isinstance(value, str):
        return json.loads(value)
    return value


def compare_eval_command(ref: str, rust: str, path: Path) -> None:
    ref_json = json.loads(
        run(ref, ["eval", f"--predicted={path}", f"--truth={path}", "--labels=1,2"]).stdout
    )
    rust_json = json.loads(run(rust, ["eval", str(path), str(path), "--labels=1,2"]).stdout)
    if parse_confusion_matrix(ref_json["confusion_matrix"]) != parse_confusion_matrix(rust_json["confusion_matrix"]):
        raise AssertionError("eval confusion matrix differs")
    for key in ("overall_accuracy", "mean_intersection_over_union", "f1_score"):
        assert_json_number_close(ref_json[key], rust_json[key], f"eval {key}")
    ref_labels = ref_json["labels"]
    rust_labels = rust_json["labels"]
    if len(ref_labels) != len(rust_labels):
        raise AssertionError("eval label count differs")
    for idx, (ref_label, rust_label) in enumerate(zip(ref_labels, rust_labels)):
        if int(ref_label["support"]) != int(rust_label["support"]):
            raise AssertionError(f"eval label {idx} support differs")


def geojson_feature_counts(path: Path) -> list[int]:
    value = json.loads(path.read_text())
    features = value.get("features")
    if not isinstance(features, list) or not features:
        raise AssertionError(f"{path} has no GeoJSON features")
    counts = [int(feature["properties"]["COUNT"]) for feature in features]
    return sorted(counts)


def compare_density_command(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/las/interesting.las"
    ref_out = tmp / "density-ref.geojson"
    rust_out = tmp / "density-rust.geojson"
    run(ref, ["density", str(input_path), str(ref_out), "-f", "GeoJSON", "--edge_length=25", "--threshold=2"])
    run(
        rust,
        [
            "density",
            str(input_path),
            str(rust_out),
            "--filters.hexbin.edge_length=25",
            "--filters.hexbin.threshold=2",
        ],
    )
    ref_counts = geojson_feature_counts(ref_out)
    rust_counts = geojson_feature_counts(rust_out)
    if ref_counts != rust_counts:
        raise AssertionError(f"density feature counts differ: {ref_counts} != {rust_counts}")


def compare_ground_command(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/las/interesting.las"
    ref_out = tmp / "ground-ref.pcd"
    rust_out = tmp / "ground-rust.pcd"
    run(ref, ["ground", str(input_path), str(ref_out), "--filters.smrf.cell=10"])
    run(rust, ["ground", str(input_path), str(rust_out), "--filters.smrf.cell=10"])
    ref_classification = pcd_field_values(ref_out, "classification")
    rust_classification = pcd_field_values(rust_out, "classification")
    if len(ref_classification) != len(rust_classification):
        raise AssertionError("ground point count differs")
    matches = sum(1 for a, b in zip(ref_classification, rust_classification) if a == b)
    agreement = matches / len(ref_classification)
    if agreement < 0.998:
        raise AssertionError(
            f"ground classification agreement too low: {matches}/{len(ref_classification)} ({agreement:.4f})"
        )


def compare_tindex_command(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/las/interesting.las"
    ref_out = tmp / "tindex-ref.geojson"
    rust_out = tmp / "tindex-rust.geojson"
    for binary, output in ((ref, ref_out), (rust, rust_out)):
        run(
            binary,
            [
                "tindex",
                "create",
                "--tindex",
                str(output),
                "--ogrdriver",
                "GeoJSON",
                "--fast_boundary",
                str(input_path),
            ],
        )
    ref_json = json.loads(ref_out.read_text())
    rust_json = json.loads(rust_out.read_text())
    if ref_json.get("type") != "FeatureCollection" or rust_json.get("type") != "FeatureCollection":
        raise AssertionError("tindex output is not a FeatureCollection")
    ref_features = ref_json.get("features", [])
    rust_features = rust_json.get("features", [])
    if len(ref_features) != 1 or len(rust_features) != 1:
        raise AssertionError("tindex feature count differs")
    if ref_features[0]["properties"]["location"] != rust_features[0]["properties"]["location"]:
        raise AssertionError("tindex location property differs")


def summary_point_count(pdal: str, path: Path) -> int:
    value = json.loads(run(pdal, ["info", "--summary", str(path)]).stdout)
    for node in (value, value.get("summary", {})):
        if isinstance(node, dict):
            for key in ("point_count", "num_points"):
                count = node.get(key)
                if count is not None:
                    return int(count)
    raise AssertionError(f"unable to find point count in summary for {path}")


def las_counts_by_name(pdal: str, directory: Path) -> dict[str, int]:
    return {
        path.name: summary_point_count(pdal, path)
        for path in sorted(directory.glob("*.las"))
    }


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


def case_pipeline_array_text_exact(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/text/utm17_1.txt"
    ref_out = tmp / "pipeline-array-ref.txt"
    rust_out = tmp / "pipeline-array-rust.txt"
    for binary, output in ((ref, ref_out), (rust, rust_out)):
        pipeline = output.with_suffix(".json")
        write_pipeline_array(
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


def case_pipeline_filename_strings_pcd_semantic(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/ply/simple_text.ply"
    ref_out = tmp / "pipeline-string-ref.pcd"
    rust_out = tmp / "pipeline-string-rust.pcd"
    for binary, output in ((ref, ref_out), (rust, rust_out)):
        pipeline = output.with_suffix(".json")
        write_pipeline_array(
            pipeline,
            [str(input_path), {"type": "filters.decimation", "step": 2}, str(output)],
        )
        run(binary, ["pipeline", str(pipeline)], stdout=subprocess.DEVNULL)
    compare_pcd_ascii(ref_out, rust_out)


def case_pipeline_pcd_semantic(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/pcd/utm17_space.pcd"
    ref_out = tmp / "pipeline-pcd-ref.pcd"
    rust_out = tmp / "pipeline-pcd-rust.pcd"
    for binary, output in ((ref, ref_out), (rust, rust_out)):
        pipeline = output.with_suffix(".json")
        write_pipeline_array(
            pipeline,
            [
                {"type": "readers.pcd", "filename": str(input_path)},
                {"type": "filters.decimation", "step": 2},
                {"type": "writers.pcd", "filename": str(output), "order": "X,Y,Z", "precision": 2},
            ],
        )
        run(binary, ["pipeline", str(pipeline)], stdout=subprocess.DEVNULL)
    compare_pcd_ascii(ref_out, rust_out)


def case_pipeline_ply_semantic(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/ply/simple_text.ply"
    ref_out = tmp / "pipeline-ply-ref.ply"
    rust_out = tmp / "pipeline-ply-rust.ply"
    for binary, output in ((ref, ref_out), (rust, rust_out)):
        pipeline = output.with_suffix(".json")
        write_pipeline_array(
            pipeline,
            [
                {"type": "readers.ply", "filename": str(input_path)},
                {"type": "filters.decimation", "step": 2},
                {
                    "type": "writers.ply",
                    "filename": str(output),
                    "storage_mode": "ascii",
                    "precision": 6,
                },
            ],
        )
        run(binary, ["pipeline", str(pipeline)], stdout=subprocess.DEVNULL)
    ref_txt = tmp / "pipeline-ply-ref.txt"
    rust_txt = tmp / "pipeline-ply-rust.txt"
    canonical_xyz_text(rust, ref_out, ref_txt)
    canonical_xyz_text(rust, rust_out, rust_txt)
    compare_csv_common_numeric(ref_txt, rust_txt, {})


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


def case_translate_command_pcd_semantic(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/ply/simple_text.ply"
    ref_out = tmp / "translate-ref.pcd"
    rust_out = tmp / "translate-rust.pcd"
    for binary, output in ((ref, ref_out), (rust, rust_out)):
        run(binary, ["translate", str(input_path), str(output)])
    compare_pcd_ascii(ref_out, rust_out)


def case_merge_command_pcd_semantic(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/ply/simple_text.ply"
    ref_out = tmp / "merge-ref.pcd"
    rust_out = tmp / "merge-rust.pcd"
    for binary, output in ((ref, ref_out), (rust, rust_out)):
        run(binary, ["merge", str(input_path), str(input_path), str(output)])
    compare_pcd_ascii(ref_out, rust_out)


def case_sort_command_pcd_semantic(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/ply/simple_text.ply"
    ref_out = tmp / "sort-ref.pcd"
    rust_out = tmp / "sort-rust.pcd"
    for binary, output in ((ref, ref_out), (rust, rust_out)):
        run(binary, ["sort", str(input_path), str(output)])
    compare_pcd_ascii(ref_out, rust_out)


def case_split_command_counts_semantic(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/ply/simple_text.ply"
    ref_out = tmp / "split-ref.pcd"
    rust_out = tmp / "split-rust.pcd"
    run(ref, ["split", str(input_path), str(ref_out), "--capacity=2"])
    run(rust, ["split", str(input_path), str(rust_out), "--capacity=2"])
    ref_counts = pcd_counts_by_prefix(tmp, "split-ref")
    rust_counts = pcd_counts_by_prefix(tmp, "split-rust")
    if not ref_counts or not rust_counts:
        raise AssertionError(f"split produced no PCD outputs: {ref_counts} / {rust_counts}")
    if ref_counts != rust_counts:
        raise AssertionError(f"split PCD counts differ: {ref_counts} != {rust_counts}")
    if sum(ref_counts) != 3:
        raise AssertionError(f"split total point count differs from fixture: {sum(ref_counts)}")


def case_random_command_count_semantic(ref: str, rust: str, tmp: Path) -> None:
    ref_out = tmp / "random-ref.pcd"
    rust_out = tmp / "random-rust.pcd"
    run(ref, ["random", str(ref_out), "--count=50"])
    run(rust, ["random", str(rust_out), "--count=50"])
    if pcd_point_count(ref_out) != pcd_point_count(rust_out):
        raise AssertionError("random point count differs")
    assert_unit_cube_pcd(ref_out)
    assert_unit_cube_pcd(rust_out)


def case_tile_command_counts_semantic(ref: str, rust: str, tmp: Path) -> None:
    input_path = REPO / "test/data/las/interesting.las"
    ref_dir = tmp / "tile-ref"
    rust_dir = tmp / "tile-rust"
    ref_dir.mkdir()
    rust_dir.mkdir()
    run(ref, ["tile", str(input_path), str(ref_dir / "t#.las"), "--length=1000"])
    run(rust, ["tile", str(input_path), str(rust_dir / "t#.las"), "--length=1000"])
    ref_counts = las_counts_by_name(rust, ref_dir)
    rust_counts = las_counts_by_name(rust, rust_dir)
    if not ref_counts or not rust_counts:
        raise AssertionError(f"tile produced no LAS outputs: {ref_counts} / {rust_counts}")
    if ref_counts != rust_counts:
        raise AssertionError(f"tile LAS counts differ: {ref_counts} != {rust_counts}")
    if sum(ref_counts.values()) != 1065:
        raise AssertionError(f"tile total point count differs from fixture: {sum(ref_counts.values())}")


def case_info_summary_semantic(ref: str, rust: str, _tmp: Path) -> None:
    compare_info_summary(ref, rust, REPO / "test/data/ply/simple_text.ply")


def case_delta_command_semantic(ref: str, rust: str, _tmp: Path) -> None:
    compare_delta_command(
        ref,
        rust,
        REPO / "test/data/ply/simple_text.ply",
        REPO / "test/data/ply/text_extradim.ply",
    )


def case_chamfer_command_semantic(ref: str, rust: str, _tmp: Path) -> None:
    compare_metric_command(
        ref,
        rust,
        "chamfer",
        REPO / "test/data/ply/simple_text.ply",
        REPO / "test/data/ply/text_extradim.ply",
        ("chamfer",),
    )


def case_hausdorff_command_semantic(ref: str, rust: str, _tmp: Path) -> None:
    compare_metric_command(
        ref,
        rust,
        "hausdorff",
        REPO / "test/data/ply/simple_text.ply",
        REPO / "test/data/ply/text_extradim.ply",
        ("hausdorff", "modified_hausdorff"),
    )


def case_eval_command_semantic(ref: str, rust: str, _tmp: Path) -> None:
    compare_eval_command(ref, rust, REPO / "test/data/las/interesting.las")


def case_density_command_semantic(ref: str, rust: str, tmp: Path) -> None:
    compare_density_command(ref, rust, tmp)


def case_ground_command_semantic(ref: str, rust: str, tmp: Path) -> None:
    compare_ground_command(ref, rust, tmp)


def case_tindex_command_semantic(ref: str, rust: str, tmp: Path) -> None:
    compare_tindex_command(ref, rust, tmp)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ref", default=find_default_reference_pdal())
    parser.add_argument("--rust", default=find_default_rust_pdal())
    parser.add_argument(
        "--json-report",
        type=Path,
        help="Optional path to write a machine-readable summary after all checks pass.",
    )
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
        ("pipeline array text artifact bytes", "exact", case_pipeline_array_text_exact),
        ("PCD decimation point payload", "semantic", case_pcd_decimation_semantic),
        ("pipeline filename string PCD payload", "semantic", case_pipeline_filename_strings_pcd_semantic),
        ("pipeline PCD payload", "semantic", case_pipeline_pcd_semantic),
        ("pipeline PLY payload", "semantic", case_pipeline_ply_semantic),
        ("LAS head point payload", "semantic", case_las_head_points_semantic),
        ("translate command PLY to PCD", "semantic", case_translate_command_pcd_semantic),
        ("merge command PLY to PCD", "semantic", case_merge_command_pcd_semantic),
        ("sort command PLY to PCD", "semantic", case_sort_command_pcd_semantic),
        ("split command capacity counts", "semantic", case_split_command_counts_semantic),
        ("random command count and bounds", "semantic", case_random_command_count_semantic),
        ("tile command LAS counts", "semantic", case_tile_command_counts_semantic),
        ("info --summary bounds/count", "semantic", case_info_summary_semantic),
        ("delta command numeric payload", "semantic", case_delta_command_semantic),
        ("chamfer command numeric payload", "semantic", case_chamfer_command_semantic),
        ("hausdorff command numeric payload", "semantic", case_hausdorff_command_semantic),
        ("eval command metrics", "semantic", case_eval_command_semantic),
        ("density command GeoJSON counts", "semantic", case_density_command_semantic),
        ("ground command classification", "semantic", case_ground_command_semantic),
        ("tindex command location index", "semantic", case_tindex_command_semantic),
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
    results: list[dict[str, str]] = []
    try:
        for name, contract, func in cases:
            func(ref, rust, tmp)
            results.append({"name": name, "contract": contract, "status": "ok"})
            print(f"ok,{contract},{name}")
        if args.json_report:
            exact = sum(1 for item in results if item["contract"] == "exact")
            semantic = sum(1 for item in results if item["contract"] == "semantic")
            report = {
                "reference": {
                    "path": str(Path(ref).resolve()),
                    "version": parse_pdal_version(run_raw(ref, ["--version"]).stdout),
                },
                "rust_backed": {
                    "path": str(Path(rust).resolve()),
                    "version": parse_pdal_version(run_raw(rust, ["--version"]).stdout),
                },
                "summary": {
                    "total": len(results),
                    "exact": exact,
                    "semantic": semantic,
                },
                "cases": results,
            }
            args.json_report.parent.mkdir(parents=True, exist_ok=True)
            args.json_report.write_text(json.dumps(report, indent=2) + "\n")
            print(f"\n# wrote JSON report: {args.json_report}")
    finally:
        if args.keep_temp:
            print(f"\n# kept temp directory: {tmp}")
        else:
            shutil.rmtree(tmp, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
