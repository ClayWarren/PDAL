//! COPC (Cloud Optimized Point Cloud) writer.
//!
//! Faithful port of the C++ `io/private/copcwriter/` subsystem, which builds a
//! COPC octree (a `copc` info VLR, an EPT-style hierarchy EVLR, and per-node LAZ
//! chunks) on top of a LAS 1.4 / LAZ file. Built up incrementally:
//!
//! - `voxel_key`: octree node key (`VoxelKey`).
//! - `common`: shared constants (`MAX_POINTS_PER_NODE`, cell counts).
//! - `grid`: octree depth sizing and point-to-voxel mapping (`Grid`).
//! - `octant_info`: per-octant point storage (`OctantInfo`).
//! - `cell_manager`: `VoxelKey` -> point view map (`CellManager`).
//! - `grid_key`: packed sampling-grid cell key (`GridKey`).
//! - `voxel_info`: per-node bounds, children, and occupancy grid (`VoxelInfo`).
//! - `processor`: per-node redistribution + occupancy-grid subsampling.
//! - `pyramid`: bottom-up octree build driver (`Pyramid`).
//! - `output_format`: byte-exact `copc` info VLR + hierarchy entries.
//!
//! The remaining layer (LAS header + LAZ chunk encoding + file assembly) builds
//! on `output_format`; the C++ `writers.copc` remains the contract until then.

pub mod cell_manager;
pub mod chunk_writer;
pub mod common;
pub mod grid;
pub mod grid_key;
pub mod hierarchy;
pub mod octant_info;
pub mod output;
pub mod output_format;
pub mod processor;
pub mod pyramid;
pub mod voxel_info;
pub mod voxel_key;
pub mod writer;
