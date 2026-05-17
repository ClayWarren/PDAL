//! Minimal PDAL core, ported to Rust.
//!
//! This is the foundation layer of the PDAL Rust port spike: the point-buffer
//! model, stage traits, and options -- the smallest slice of `pdal/` needed to
//! run a single ported filter end to end. See `rust/README.md` for scope.

pub mod options;
pub mod point;
pub mod spatial;
pub mod stage;
