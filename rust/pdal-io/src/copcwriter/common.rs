//! Shared COPC writer constants. Port of the relevant parts of
//! `io/private/copcwriter/Common.hpp`.

/// Target maximum number of points stored in a single octree node.
pub const MAX_POINTS_PER_NODE: i32 = 100_000;

/// `sqrt(3)`, used to size the per-node sampling grid.
pub const SQRT3: f64 = 1.732_050_807_57;

/// Sampling grid cell count for non-root octree nodes (`int(128 * sqrt(3))`).
pub const CHILD_CELL_COUNT: i32 = (128.0 * SQRT3) as i32;

/// Sampling grid cell count for the root octree node (`int(128 * sqrt(3) / 1.5)`).
pub const ROOT_CELL_COUNT: i32 = (128.0 * SQRT3 / 1.5) as i32;
