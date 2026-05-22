//! Minimal PDAL core, ported to Rust.
//!
//! This is the foundation layer of the PDAL Rust port spike: the point-buffer
//! model, stage traits, and options -- the smallest slice of `pdal/` needed to
//! run a single ported filter end to end. See `rust/README.md` for scope.

pub mod bounds;
pub mod config;
pub mod deflate;
pub mod driver;
pub mod expr;
pub mod file_spec;
pub mod gdal;
pub mod geometry;
pub mod georeference;
pub mod kernel;
pub mod log;
pub mod metadata;
pub mod metrics;
pub mod ogr_spec;
pub mod options;
pub mod pipeline;
pub mod plugin;
pub mod point;
pub mod raster;
pub mod scaling;
pub mod spatial;
pub mod srs;
pub mod stage;
pub mod utils;
pub mod uuid;
pub mod writer;
pub mod xml_schema;
