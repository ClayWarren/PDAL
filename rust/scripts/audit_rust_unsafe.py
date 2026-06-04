#!/usr/bin/env python3
"""Count unsafe Rust in the first-party port workspace.

This is an accounting aid for `rust/STATUS.md`, not a safety proof. It counts
lexical occurrences in first-party Rust sources while excluding build output.
Use it after C ABI/native-adapter work so the documented unsafe footprint stays
honest.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUST_ROOT = ROOT / "rust"

PATTERNS = {
    "unsafe_blocks": re.compile(r"\bunsafe\s*\{"),
    "unsafe_extern_c_exports": re.compile(r'\bunsafe\s+extern\s+"C"\s+fn\b'),
    "unsafe_fn_helpers": re.compile(r"\bunsafe\s+fn\b"),
    "unsafe_extern_callback_aliases": re.compile(r"\btype\b.*\bunsafe\s+extern\b"),
    "unsafe_extern_blocks": re.compile(r"\bunsafe\s+extern\s*\{"),
    "unsafe_impls": re.compile(r"\bunsafe\s+impl\b"),
}


def rust_files() -> list[Path]:
    return sorted(
        p
        for p in RUST_ROOT.rglob("*.rs")
        if "target" not in p.relative_to(RUST_ROOT).parts
    )


def count_patterns(files: list[Path]) -> dict[str, int]:
    counts = {name: 0 for name in PATTERNS}
    for path in files:
        text = path.read_text(errors="ignore")
        for name, pattern in PATTERNS.items():
            counts[name] += len(pattern.findall(text))
    return counts


def format_status_sentence(counts: dict[str, int]) -> str:
    extern_blocks = counts["unsafe_extern_blocks"]
    extern_block_word = "block" if extern_blocks == 1 else "blocks"
    unsafe_impls = counts["unsafe_impls"]
    unsafe_impl_word = "`unsafe impl`" if unsafe_impls == 1 else "`unsafe impl`s"
    return (
        "Current first-party Rust count, excluding `rust/target`, is "
        f"{counts['unsafe_blocks']} `unsafe {{ ... }}` blocks, "
        f"{counts['unsafe_extern_c_exports']} `unsafe extern \"C\" fn` exports, "
        f"{counts['unsafe_fn_helpers']} total `unsafe fn` helpers, "
        f"{counts['unsafe_extern_callback_aliases']} unsafe extern callback type aliases, "
        f"{extern_blocks} unsafe extern {extern_block_word}, and "
        f"{unsafe_impls} {unsafe_impl_word}."
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--status-sentence",
        action="store_true",
        help="print the sentence fragment used by rust/STATUS.md",
    )
    args = parser.parse_args()

    files = rust_files()
    counts = count_patterns(files)
    if args.status_sentence:
        print(format_status_sentence(counts))
    else:
        print("Unsafe Rust footprint (first-party, excluding rust/target)")
        for name, count in counts.items():
            print(f"{name:32} {count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
