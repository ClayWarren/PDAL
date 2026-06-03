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

## Peak memory (RSS), CLI-vs-CLI

`rust/scripts/benchmark_memory_cpp_vs_rust.py` runs the same identical-work
commands through both full `pdal` CLIs under `/usr/bin/time -l` and reports the
median peak resident set size (out-of-process, so it captures the whole process,
not just what a Rust allocator hook would see).

Reproduce:

```sh
python3 rust/scripts/benchmark_memory_cpp_vs_rust.py --rust .build/bin/pdal --iters 7
# or via pixi: pixi run -e dev rust-bench-memory
```

### Results (macOS arm64, both `pdal 2.10.1`, 7 iterations, median)

| workload | C++ (MiB) | Rust-backed (MiB) | ratio |
|---|---:|---:|---:|
| baseline (`--version`) | 31.9 | 20.3 | 0.64 |
| faux 3M → `filters.sort` → null | 157.4 | 210.0 | 1.33 |
| LAS read → `filters.decimation` → LAS write | 51.3 | 39.4 | 0.77 |
| `info --stats` (autzen_trim.las) | 74.2 | 30.3 | 0.41 |

Reading the numbers: three of the four workloads use **less** peak RSS than C++,
and the streaming-eligible compute paths (`info --stats`, LAS decimation) are the
biggest wins. The one regression is `filters.sort` (1.33×): sort is not
streamable, so the Rust executor materializes all 3M points, and the gap vs C++
(210 vs 157) is the auxiliary sort-order/inverse-index arrays C++ reuses in
place. This matches the analysis recorded in the `rust/STATUS.md`
`Performance visibility` row; large fully-streamable file→file pipelines (e.g.
`large.las → filters.range → large.las`) are *lighter* than C++ because the
Rust CLI executor now streams them in fixed-size chunks.

The same build-config confound as the wall-clock table applies: the Homebrew
reference links a heavier optional-plugin closure, which inflates its baseline
RSS somewhat. Treat as directional, not a controlled same-flags A/B.

## Binary size and startup (existing Rust harnesses)

`cargo test -p pdal-cli --test binary_size -- --ignored --nocapture` compares
the Rust-native `pdal-rs` CLI against installed `pdal`. Current numbers are not
apples-to-apples: `pdal-rs` is measured as a **debug** build (~21 MiB, mostly
unstripped debug info) that statically links its dependencies, while the
installed `pdal` binary is a thin CLI (~0.28 MiB) that links `libpdalcpp` and
GDAL/PROJ/etc. as external shared libraries not counted in that number. A
meaningful size comparison needs a release+stripped `pdal-rs` measured against
the C++ CLI **plus** its shared-library closure; that is future work.

Startup (median of `--version`, same harness): `pdal-rs` **18.3 ms** vs
installed Homebrew `pdal` **89.9 ms** (0.20×). The statically-linked Rust CLI
loads far fewer shared libraries at launch than the thin C++ CLI plus its
GDAL/PROJ/plugin closure, so it starts faster here despite the larger on-disk
size. (Run the size sub-test with `DYLD_FALLBACK_LIBRARY_PATH=$CONDA_PREFIX/lib`
set so `pdal-rs` resolves its PROJ/GDAL dylibs; `pixi run -e dev` provides it.)
The CLI-vs-CLI table above instead measures this repo's Rust-backed C++ `pdal`,
which also starts faster than Homebrew's (leaner build).

## Build cost and guardrail harnesses

`rust/scripts/measure_guardrails.sh` records opt-in wall time and peak RSS for
installed-PDAL I/O pipelines, the Rust local-I/O perf harness, and the Rust
workspace build. Add `--test-suites` for the full C++ and Rust suite timing, or
`--cold-build` to `cargo clean` first and time a cold Rust workspace build. The
ctest run auto-detects the configured C++ build tree (`.build` or `build`, or
`PDAL_BUILD_DIR`).

Reproduce:

```sh
rust/scripts/measure_guardrails.sh                 # build cost + I/O harness
rust/scripts/measure_guardrails.sh --test-suites   # + full C++/Rust suite timing
```

### Build cost (macOS arm64, pixi dev env)

| metric | wall (s) | peak RSS (MiB) |
|---|---:|---:|
| installed I/O decimation (text/pcd/pts/ptx) | ~0.08 | ~32 |
| rust-local-io-perf-harness | 4.4 | 332.5 |
| rust-workspace-incremental-build | 1.5 | 381.1 |

The Rust incremental build (no source change) re-checks the workspace in ~1.5 s;
a cold `--cold-build` rebuild is much larger and is the meaningful compile-time
figure once recorded against a same-config C++ build.

## Full test-suite timing

`measure_guardrails.sh --test-suites` times the full C++ ctest sweep and the
full Rust workspace suite. Note the harness's auto-printed `cpp-full-test-suite`
line only appears when ctest exits clean; the numbers below are from direct runs
(macOS arm64, `.build`):

| suite | tests | wall (s) | result |
|---|---:|---:|---|
| C++ ctest (`ctest` in `.build`) | 187 executables | 304.8 | green |
| Rust workspace (`cargo test --workspace`) | 66 binaries | 22 | green |

This is **not** a fair head-to-head — it is a visibility data point. The C++
suite launches 187 separate per-test executables, each of which dynamically
loads `libpdalcpp` plus the GDAL/PROJ/plugin closure at startup; the Rust
workspace runs compiled test binaries grouped per crate. The ~14× wall-clock
difference is dominated by that process/startup structure, not by per-assertion
compute.

The C++ suite was later re-run after parity fixes for the seven failures exposed
by this measurement pass (`pdal_program_arg_test`, `pdal_info_test`,
`pdal_io_fbi_test`, `pdal_tindex_test`, `pc2pc_test`,
`pdal_io_draco_reader_test`, and the GDAL-version-sensitive LAS SRS VLR case).
It is now green locally on this branch; the timing value remains the original
direct-run measurement.

## Not yet measured

A controlled same-config compile-time A/B (cold Rust workspace vs an equivalent
pure-C++ build) is not yet recorded — the harness can time a cold Rust build
(`measure_guardrails.sh --cold-build`), but the matching pure-C++ cold build is
not isolatable because the Rust C ABI is a mandatory part of this build (see the
`pdal_features.hpp.in` row in `rust/STATUS.md`). All workloads here remain
small/local and macOS-only; Linux/Windows numbers and a release+stripped
`pdal-rs` size comparison against the C++ CLI's full shared-library closure are
still future work.
