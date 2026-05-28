//! COPC (Cloud Optimized Point Cloud) writer.
//!
//! Faithful port of the C++ `io/private/copcwriter/` subsystem, which builds a
//! COPC octree (a `copc` info VLR, an EPT-style hierarchy EVLR, and per-node LAZ
//! chunks) on top of a LAS 1.4 / LAZ file. Built up incrementally:
//!
//! - `voxel_key`: octree node key (`VoxelKey`).
//!
//! Higher layers (grid binning, pyramid build, hierarchy/info-VLR output) land
//! in later increments; the C++ `writers.copc` remains the contract.

pub mod voxel_key;
