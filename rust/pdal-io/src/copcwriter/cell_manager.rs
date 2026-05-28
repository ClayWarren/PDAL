//! Map from octree `VoxelKey` to the points collected for that hierarchy entry.
//!
//! Port of `io/private/copcwriter/CellManager.hpp`. Cells are created lazily as
//! empty point views sharing the source view's layout.

use std::collections::HashMap;

use pdal_core::point::PointView;

use super::voxel_key::VoxelKey;

pub struct CellManager {
    cells: HashMap<VoxelKey, PointView>,
    // Template view used to mint empty cells with the right layout (the C++
    // `m_sourceView`).
    source_view: PointView,
}

impl CellManager {
    pub fn new(source_view: PointView) -> Self {
        CellManager {
            cells: HashMap::new(),
            source_view,
        }
    }

    /// Mutable access to the cell for `key`, creating an empty view if absent
    /// (matches C++ `CellManager::get`).
    pub fn get(&mut self, key: VoxelKey) -> &mut PointView {
        self.cells
            .entry(key)
            .or_insert_with(|| self.source_view.make_new())
    }

    pub fn contains(&self, key: VoxelKey) -> bool {
        self.cells.contains_key(&key)
    }

    pub fn remove(&mut self, key: VoxelKey) -> Option<PointView> {
        self.cells.remove(&key)
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&VoxelKey, &PointView)> {
        self.cells.iter()
    }

    /// Move every cell from `src` into this manager (matches C++
    /// `CellManager::merge`, which uses `std::unordered_map::merge`). Keys
    /// already present here are kept; the C++ `merge` likewise does not
    /// overwrite existing keys.
    pub fn merge(&mut self, src: &mut CellManager) {
        for (key, view) in src.cells.drain() {
            self.cells.entry(key).or_insert(view);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout};
    use std::rc::Rc;

    fn empty_source() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        PointView::new(Rc::new(layout))
    }

    #[test]
    fn get_creates_empty_cell_lazily() {
        let mut mgr = CellManager::new(empty_source());
        assert!(!mgr.contains(VoxelKey::ROOT));
        let cell = mgr.get(VoxelKey::ROOT);
        assert_eq!(cell.len(), 0);
        let id = cell.add_point();
        cell.set_f64(id, &DimId::X, 1.0);
        // Same key returns the populated cell.
        assert_eq!(mgr.get(VoxelKey::ROOT).len(), 1);
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn merge_moves_cells_without_overwriting() {
        let mut dst = CellManager::new(empty_source());
        dst.get(VoxelKey::ROOT).add_point();

        let mut src = CellManager::new(empty_source());
        src.get(VoxelKey::ROOT).add_point(); // duplicate key, must not overwrite
        src.get(VoxelKey::new(1, 0, 0, 1)).add_point();

        dst.merge(&mut src);
        assert_eq!(dst.len(), 2);
        // Existing ROOT cell kept (1 point), new key adopted.
        assert_eq!(dst.get(VoxelKey::ROOT).len(), 1);
        assert_eq!(dst.get(VoxelKey::new(1, 0, 0, 1)).len(), 1);
        assert!(src.is_empty());
    }
}
