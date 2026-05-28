//! Per-octree-node working state for the COPC pyramid build.
//!
//! Port of `io/private/copcwriter/VoxelInfo.hpp`. A `VoxelInfo` holds a node's
//! spatial bounds (derived from its `VoxelKey` and the full cube), its eight
//! child octants, the parent octant being assembled, and the occupancy grid
//! used to subsample representative points for the level.

use std::collections::HashSet;

use pdal_core::bounds::Bounds3D;

use super::common::{CHILD_CELL_COUNT, ROOT_CELL_COUNT};
use super::grid_key::GridKey;
use super::octant_info::OctantInfo;
use super::voxel_key::VoxelKey;

pub struct VoxelInfo {
    bounds: Bounds3D,
    x_width: f64,
    y_width: f64,
    z_width: f64,
    grid_cell_width: f64,
    grid_x_count: i32,
    grid_y_count: i32,
    grid_z_count: i32,
    children: [OctantInfo; 8],
    octant: OctantInfo,
    grid: HashSet<GridKey>,
}

impl VoxelInfo {
    pub fn new(full_bounds: Bounds3D, key: VoxelKey) -> Self {
        let children = std::array::from_fn(|i| OctantInfo::new(key.child(i as i32)));

        let cells = 2_f64.powi(key.level());
        let x_width = (full_bounds.maxx - full_bounds.minx) / cells;
        let y_width = (full_bounds.maxy - full_bounds.miny) / cells;
        let z_width = (full_bounds.maxz - full_bounds.minz) / cells;

        let minx = full_bounds.minx + key.x() as f64 * x_width;
        let miny = full_bounds.miny + key.y() as f64 * y_width;
        let minz = full_bounds.minz + key.z() as f64 * z_width;
        let bounds = Bounds3D {
            minx,
            miny,
            minz,
            maxx: minx + x_width,
            maxy: miny + y_width,
            maxz: minz + z_width,
        };

        let max_width = x_width.max(y_width).max(z_width);
        // Child spacing is finer than the final spacing because the parent
        // selects points from the child grid.
        let grid_cell_width = if key == VoxelKey::ROOT {
            max_width / ROOT_CELL_COUNT as f64
        } else {
            max_width / CHILD_CELL_COUNT as f64
        };

        let grid_x_count = ((bounds.maxx - bounds.minx) / grid_cell_width).ceil() as i32;
        let grid_y_count = ((bounds.maxy - bounds.miny) / grid_cell_width).ceil() as i32;
        let grid_z_count = ((bounds.maxz - bounds.minz) / grid_cell_width).ceil() as i32;

        VoxelInfo {
            bounds,
            x_width,
            y_width,
            z_width,
            grid_cell_width,
            grid_x_count,
            grid_y_count,
            grid_z_count,
            children,
            octant: OctantInfo::new(key),
            grid: HashSet::new(),
        }
    }

    pub fn key(&self) -> VoxelKey {
        self.octant.key()
    }

    /// Prepare the parent octant's point view from a non-empty child, and give
    /// every empty child an empty view of the same layout. Matches the C++
    /// `VoxelInfo::initParentOctant`.
    pub fn init_parent_octant(&mut self) {
        let template = self
            .children
            .iter()
            .find_map(|c| c.source().map(|s| s.make_new()));
        let Some(template) = template else {
            // No non-empty children; nothing to initialize.
            return;
        };
        self.octant.set_source(template.make_new());
        for child in &mut self.children {
            if child.source().is_none() {
                child.set_source(template.make_new());
            }
        }
    }

    pub fn child(&self, dir: usize) -> &OctantInfo {
        &self.children[dir]
    }

    pub fn child_mut(&mut self, dir: usize) -> &mut OctantInfo {
        &mut self.children[dir]
    }

    pub fn octant(&self) -> &OctantInfo {
        &self.octant
    }

    pub fn octant_mut(&mut self) -> &mut OctantInfo {
        &mut self.octant
    }

    pub fn has_points(&self) -> bool {
        self.octant.num_points() != 0 || self.children.iter().any(|c| c.num_points() != 0)
    }

    pub fn min_width(&self) -> f64 {
        self.x_width.min(self.y_width).min(self.z_width)
    }

    pub fn max_width(&self) -> f64 {
        self.x_width.max(self.y_width).max(self.z_width)
    }

    pub fn x_width(&self) -> f64 {
        self.x_width
    }
    pub fn y_width(&self) -> f64 {
        self.y_width
    }
    pub fn z_width(&self) -> f64 {
        self.z_width
    }

    /// Map a point to its occupancy-grid cell within this node. Matches
    /// `VoxelInfo::gridKey`.
    pub fn grid_key(&self, x: f64, y: f64, z: f64) -> GridKey {
        let gx = ((x - self.bounds.minx) / self.grid_cell_width) as i32;
        let gy = ((y - self.bounds.miny) / self.grid_cell_width) as i32;
        let gz = ((z - self.bounds.minz) / self.grid_cell_width) as i32;
        GridKey::new(gx, gy, gz)
    }

    pub fn grid(&mut self) -> &mut HashSet<GridKey> {
        &mut self.grid
    }

    pub fn grid_x_count(&self) -> i32 {
        self.grid_x_count
    }
    pub fn grid_y_count(&self) -> i32 {
        self.grid_y_count
    }
    pub fn grid_z_count(&self) -> i32 {
        self.grid_z_count
    }

    pub fn bounds(&self) -> Bounds3D {
        self.bounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn cube() -> Bounds3D {
        Bounds3D {
            minx: 0.0,
            miny: 0.0,
            minz: 0.0,
            maxx: 16.0,
            maxy: 16.0,
            maxz: 16.0,
        }
    }

    fn empty_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        PointView::new(Rc::new(layout))
    }

    #[test]
    fn root_bounds_and_children_keys() {
        let vi = VoxelInfo::new(cube(), VoxelKey::ROOT);
        assert_eq!(vi.bounds(), cube());
        assert_eq!(vi.child(0).key(), VoxelKey::new(0, 0, 0, 1));
        assert_eq!(vi.child(7).key(), VoxelKey::new(1, 1, 1, 1));
    }

    #[test]
    fn level_one_node_bounds_offset_by_key() {
        // Level-1 node (1,1,1): each axis half the cube, offset to the far cell.
        let vi = VoxelInfo::new(cube(), VoxelKey::new(1, 1, 1, 1));
        let b = vi.bounds();
        assert_eq!((b.minx, b.maxx), (8.0, 16.0));
        assert_eq!((b.miny, b.maxy), (8.0, 16.0));
        assert_eq!((b.minz, b.maxz), (8.0, 16.0));
        assert_eq!(vi.x_width(), 8.0);
    }

    #[test]
    fn grid_key_is_relative_to_node_bounds() {
        let vi = VoxelInfo::new(cube(), VoxelKey::ROOT);
        // A point at the node origin lands in cell (0,0,0).
        assert_eq!(vi.grid_key(0.0, 0.0, 0.0), GridKey::new(0, 0, 0));
    }

    #[test]
    fn has_points_reflects_octant_and_children() {
        let mut vi = VoxelInfo::new(cube(), VoxelKey::ROOT);
        assert!(!vi.has_points());
        let mut v = empty_view();
        v.add_point();
        vi.child_mut(3).set_source(v);
        assert!(vi.has_points());
    }

    #[test]
    fn init_parent_octant_fills_octant_and_empty_children() {
        let mut vi = VoxelInfo::new(cube(), VoxelKey::ROOT);
        let mut v = empty_view();
        v.add_point();
        vi.child_mut(2).set_source(v);
        vi.init_parent_octant();
        // Parent octant now has an (empty) source.
        assert!(vi.octant().source().is_some());
        assert_eq!(vi.octant().num_points(), 0);
        // Every child now has a source.
        for dir in 0..8 {
            assert!(vi.child(dir).source().is_some());
        }
    }
}
