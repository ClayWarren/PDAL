//! First-party readers and writers for the PDAL Rust port.
//!
//! Keep real reader/writer ports behind parity tests against existing PDAL
//! fixtures and C++ behavior.

pub mod faux;
pub mod nullwriter;
