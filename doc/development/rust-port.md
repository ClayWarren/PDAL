(development-rust-port)=

# Rust port

PDAL currently carries an experimental Rust port behind a stable C ABI. This is
developer-facing migration work, not a separate user-facing API surface.

The authoritative port notes live in the source tree:

- `rust/PORTING.md`: migration rules, architecture, guardrails, and finish-line
  milestones.
- `rust/STATUS.md`: current feature status, parity evidence, known gaps, and
  implementation-replacement measurements.
- `rust/VENDOR.md`: vendor/native dependency decisions.

The C ABI is the boundary between Rust and the existing C++ API. Existing C++
tests remain the behavioral contract while Rust implementations replace C++
behavior behind compatibility wrappers. New Rust work should keep that shape:
behavior first, C ABI boundary second, C++ wrapper compatibility third, and
tests/regression evidence throughout.

Do not treat a Rust module, crate, or directory as complete merely because it
builds. Porting work is complete only when the relevant behavior is covered,
the C++ parity path still passes, and any remaining C++ is either a thin public
API shell, a native dependency adapter, or an explicit documented holdout.
