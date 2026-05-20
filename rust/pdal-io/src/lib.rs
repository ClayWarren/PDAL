//! First-party readers and writers for the PDAL Rust port.
//!
//! Keep real reader/writer ports behind parity tests against existing PDAL
//! fixtures and C++ behavior.

pub mod bpf;
pub mod faux;
pub mod fbi;
pub mod fbi_writer;
pub mod gdal_reader;
pub mod gltf;
pub mod ilvis2;
pub mod ilvis2_metadata;
pub mod las;
pub mod las_writer;
pub mod nullwriter;
pub mod obj;
pub mod optech;
pub mod pcd;
pub mod ply;
pub mod pts;
pub mod ptx;
pub mod qfit;
pub mod sbet;
pub mod sbet_writer;
pub mod smrmsg;
pub mod stac;
pub mod terrasolid;
pub mod text;
pub mod text_writer;
pub mod tindex;
