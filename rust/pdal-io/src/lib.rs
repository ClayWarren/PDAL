//! First-party readers and writers for the PDAL Rust port.
//!
//! Keep real reader/writer ports behind parity tests against existing PDAL
//! fixtures and C++ behavior.

pub mod faux;
pub mod ilvis2;
pub mod nullwriter;
pub mod obj;
pub mod pcd;
pub mod ply;
pub mod pts;
pub mod ptx;
pub mod qfit;
pub mod text;
pub mod text_writer;
