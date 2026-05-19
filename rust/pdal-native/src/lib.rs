//! Native dependency adapters for the PDAL Rust port.
//!
//! Keep direct GDAL/OGR, GEOS, PROJ, LASzip/laz-perf, and similar native
//! bindings behind this layer or another explicit adapter crate. Higher-level
//! crates should expose PDAL behavior, not vendor-specific types.

pub mod gdal;
pub mod geometry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeCapability {
    Gdal,
    Geos,
}

pub fn built_capabilities() -> &'static [NativeCapability] {
    &[NativeCapability::Gdal, NativeCapability::Geos]
}
