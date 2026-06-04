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

When run from the Pixi environment, the benchmark scripts still prefer the
external Homebrew `/opt/homebrew/bin/pdal` reference if it exists; this avoids
accidentally comparing against Pixi's environment `pdal` instead of the installed
reference binary. For the build-tree Rust-backed binary, the scripts prepend
`.build/lib` (or `build/lib`) to the dynamic library path so they benchmark the
matching build-tree `libpdalcpp`.

Before reading timing numbers, run the workflow comparison harness:

```sh
pixi run -e dev rust-workflow-parity
# handoff artifact:
python3 rust/scripts/compare_pdal_workflows.py --rust .build/bin/pdal --json-report /tmp/pdal-workflow-parity.json
```

It compares the installed/reference PDAL binary with the Rust-backed build on
representative CLI surfaces and workflows. Current checks cover semantic
`--version`, exact unknown-command exit/stderr, semantic `--list-commands`
command names, byte-for-byte deterministic text output, pipeline JSON arrays and
root objects, filename-string pipeline stages, semantic PCD/PLY/LAS point
payloads, and installed-PDAL command workflows for `translate`, `merge`, `sort`,
`split`, `random`, `tile`, `info --summary`, `info --stats`, `delta`, `chamfer`,
`hausdorff`, `eval`, `density`, `ground`, and `tindex`. PCD/PLY/LAS/info and
command workflows use semantic comparison because binary headers, metadata
timestamps, point-format defaults, randomized samples, classification
tie-breaks, and floating-point formatting are not always byte-stable contracts.

### Results (macOS arm64, both `pdal 2.10.1`, 15 iterations, median)

| workload | C++ (ms) | Rust-backed (ms) | ratio | ratio, startup-subtracted |
|---|---:|---:|---:|---:|
| startup (`--version`) | 95.5 | 23.5 | 0.25 | — |
| faux 3M → `filters.sort` → null | 1643.0 | 1217.0 | 0.74 | 0.77 |
| LAS read → `filters.decimation` → LAS write | 188.9 | 84.9 | 0.45 | 0.66 |
| `info --stats` (autzen_trim.las) | 373.8 | 88.3 | 0.24 | 0.23 |

"startup-subtracted" removes each binary's own `--version` startup median to
approximate the compute-only ratio.

Equivalent-work verified before timing: `info --stats` yields identical output
on both (110,000 points, 20 dimensions, identical X mean), and the decimation
pipeline yields 55,000 points on both.

### Reading the numbers

The Rust-backed build is faster than the stock C++ build on these workloads,
with the clearest gains in startup and `info --stats`. The `filters.sort` path
still wins, but by a smaller margin than previous local runs. There is **no
major regression** on any measured path.

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

The same reference-binary and build-tree dynamic-library rules as the wall-clock
benchmark apply here.

### Results (macOS arm64, both `pdal 2.10.1`, 7 iterations, median)

| workload | C++ (MiB) | Rust-backed (MiB) | ratio |
|---|---:|---:|---:|
| baseline (`--version`) | 31.8 | 20.3 | 0.64 |
| faux 3M → `filters.sort` → null | 157.4 | 210.1 | 1.34 |
| LAS read → `filters.decimation` → LAS write | 51.3 | 39.4 | 0.77 |
| `info --stats` (autzen_trim.las) | 74.1 | 30.3 | 0.41 |

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

## Binary footprint

`rust/scripts/benchmark_binary_footprint.py` measures each `pdal` executable
plus the non-system shared-library closure reachable from it. This is more
useful than comparing only the tiny C++ CLI executable against a Rust binary,
because most of PDAL's footprint lives in `libpdalcpp`, plugins, GDAL/PROJ, and
other shared dependencies.

Reproduce:

```sh
python3 rust/scripts/benchmark_binary_footprint.py --rust .build/bin/pdal
# or via pixi: pixi run -e dev rust-bench-size
```

### Results (macOS arm64, both `pdal 2.10.1`)

| binary | executable (MiB) | non-system closure (MiB) | closure files |
|---|---:|---:|---:|
| C++ Homebrew reference | 0.28 | 176.85 | 195 |
| Rust-backed build-tree `pdal` | 0.31 | 131.65 | 64 |

The Rust-backed build-tree CLI is roughly the same executable size as the
Homebrew CLI, and its measured non-system shared-library closure is **0.74×**
the Homebrew reference. This mostly reflects the leaner build/plugin closure,
not just Rust implementation choices. It is still the right operational number
for "what does this executable load on this machine?"

The Rust-native `pdal-rs` debug binary-size test still exists as a local
visibility check, but it is not the apples-to-apples footprint comparison: a
debug Rust binary with static debug info is not comparable to a thin C++ CLI
unless the shared-library closure is counted.

## Build cost and guardrail harnesses

`rust/scripts/measure_guardrails.sh` records opt-in wall time and peak RSS for
installed-PDAL I/O pipelines, the Rust local-I/O perf harness, and the Rust
workspace build. Add `--test-suites` for the full C++ and Rust suite timing, or
`--cold-build` to `cargo clean` first and time a cold Rust workspace build. The
ctest run auto-detects the configured C++ build tree (`.build` or `build`, or
`PDAL_BUILD_DIR`).
The script prefers the external Homebrew PDAL reference on macOS, accepts
`PDAL_REFERENCE_PDAL=<path>` for other installed references, and prepends the
detected build-tree `lib/` directory when timing the C++ CTest suite so it does
not accidentally load an environment `libpdalcpp`.

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
small/local and macOS-only; Linux/Windows numbers and a controlled same-config
pure-C++ baseline are still future work.
