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
//!
//! The remaining layer (LAZ chunk encoding + hierarchy/info-VLR file output)
//! lands in a later increment; the C++ `writers.copc` remains the contract.

pub mod cell_manager;
pub mod common;
pub mod grid;
pub mod grid_key;
pub mod octant_info;
pub mod processor;
pub mod pyramid;
pub mod voxel_info;
pub mod voxel_key;
