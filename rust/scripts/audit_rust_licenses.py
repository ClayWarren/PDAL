#!/usr/bin/env python3
"""Report Rust third-party crate licenses from cargo metadata."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "Cargo.toml"


def cargo_metadata() -> dict:
    result = subprocess.run(
        [
            "cargo",
            "metadata",
            "--manifest-path",
            str(MANIFEST),
            "--format-version",
            "1",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--format",
        choices=("table", "csv"),
        default="table",
        help="Output format.",
    )
    args = parser.parse_args()

    metadata = cargo_metadata()
    workspace_members = set(metadata["workspace_members"])
    packages = [
        package
        for package in metadata["packages"]
        if package["id"] not in workspace_members
    ]
    packages.sort(key=lambda package: (package["name"], package["version"]))

    missing = [
        package
        for package in packages
        if not package.get("license") and not package.get("license_file")
    ]

    if args.format == "csv":
        print("name,version,license,license_file")
        for package in packages:
            print(
                ",".join(
                    [
                        package["name"],
                        package["version"],
                        package.get("license") or "",
                        package.get("license_file") or "",
                    ]
                )
            )
    else:
        print(f"third_party_crates: {len(packages)}")
        print(f"missing_license_metadata: {len(missing)}")
        for package in packages:
            license_value = package.get("license") or package.get("license_file")
            print(f"{package['name']} {package['version']} {license_value}")

    if missing:
        print("\nCrates missing license metadata:", file=sys.stderr)
        for package in missing:
            print(f"  {package['name']} {package['version']}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
