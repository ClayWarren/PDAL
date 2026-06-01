# Rust-backed PDAL vs C++ PDAL Benchmarks

This records current performance comparisons for the port. It is evidence for
the whole-port completion criterion "performance ... has a comparison harness
and no unexplained major regression," not a marketing claim.

## CLI-vs-CLI (fairest comparison)

`rust/scripts/benchmark_cpp_vs_rust.py` runs identical `pdal` commands through
two full `pdal` CLIs and reports median wall-clock time:

- **reference (C++)**: stock Homebrew `pdal` (pure C++).
- **rust-backed**: this repo's `pdal` (`.build/bin/pdal`), whose C++ wrappers
  route through the Rust C ABI.

Both are process-spawned, so this isolates the Rust-backed implementation rather
than measuring an in-process Rust call against a spawned C++ process (the flaw
in the `pdal-io` `perf_regression` harness, whose ~0.00 ratios are pure
process-startup artifacts and should not be read as real speedups).

Reproduce:

```sh
python3 rust/scripts/benchmark_cpp_vs_rust.py --iters 21
# or point at specific binaries:
python3 rust/scripts/benchmark_cpp_vs_rust.py --ref /opt/homebrew/bin/pdal --rust .build/bin/pdal
```

### Results (macOS arm64, both `pdal 2.10.1`, 21 iterations, median)

| workload | C++ (ms) | Rust-backed (ms) | ratio | ratio, startup-subtracted |
|---|---:|---:|---:|---:|
| startup (`--version`) | 85.4 | 19.7 | 0.23 | — |
| faux 3M → `filters.sort` → null | 1383.7 | 731.6 | 0.53 | 0.55 |
| LAS read → `filters.decimation` → LAS write | 179.8 | 61.3 | 0.34 | 0.45 |
| `info --stats` (autzen_trim.las) | 346.6 | 56.4 | 0.16 | 0.15 |

"startup-subtracted" removes each binary's own `--version` startup median to
approximate the compute-only ratio.

Equivalent-work verified before timing: `info --stats` yields identical output
on both (110,000 points, 20 dimensions, identical X mean), and the decimation
pipeline yields 55,000 points on both.

### Reading the numbers

The Rust-backed build is at least as fast as, and on these workloads faster
than, the stock C++ build. The compute-bound `info --stats` and the LAS I/O
pipeline show the largest gains; `filters.sort` over 3M synthetic points is
~2x. There is **no major regression** on any measured path.

Honest caveats (do not over-read the ratios):

- **Build-config confound.** The Homebrew reference is a full plugin build with
  heavy optional dependencies (HDF5, database drivers, etc.); this repo's build
  is leaner. That inflates the **startup** advantage in particular (fewer shared
  libraries to dynamically link), so the 0.23 startup ratio overstates the Rust
  effect. The startup-subtracted column is the more meaningful compute signal.
- Compiler, optimization flags, and dependency versions may differ between the
  Homebrew build and this build; this is not a controlled same-flags A/B.
- A true same-config A/B (this repo built pure-C++ vs Rust-backed) is not
  available because the Rust C ABI is a mandatory, non-optional part of this
  build (see `rust/STATUS.md`, `pdal_features.hpp.in` row).
- Workloads are small/local and macOS-only; treat as directional, not a full
  performance suite.

## Binary size and startup (existing Rust harnesses)

`cargo test -p pdal-cli --test binary_size -- --ignored --nocapture` compares
the Rust-native `pdal-rs` CLI against installed `pdal`. Current numbers are not
apples-to-apples: `pdal-rs` is measured as a **debug** build (~21 MiB, mostly
unstripped debug info) that statically links its dependencies, while the
installed `pdal` binary is a thin CLI (~0.28 MiB) that links `libpdalcpp` and
GDAL/PROJ/etc. as external shared libraries not counted in that number. A
meaningful size comparison needs a release+stripped `pdal-rs` measured against
the C++ CLI **plus** its shared-library closure; that is future work.

Startup of `pdal-rs --version` vs installed `pdal --version` is ~equal (~86 ms)
in that harness; note the CLI-vs-CLI table above instead measures this repo's
Rust-backed C++ `pdal`, which starts faster than Homebrew's (leaner build).

## Not yet measured

Memory high-water mark, compile time, and full C++-vs-Rust test-suite timing
have prototype harnesses (see the `Performance visibility` row in
`rust/STATUS.md`) but are not recorded here yet.
