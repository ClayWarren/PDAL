//! Per-octant point storage for the COPC pyramid build.
//!
//! Port of `io/private/copcwriter/OctantInfo.hpp`. An `OctantInfo` pairs an
//! octree `VoxelKey` with the point view collected for that node, plus a
//! `must_write` flag the pyramid sets for nodes that must be emitted even when
//! empty.

use pdal_core::point::PointView;

use super::voxel_key::VoxelKey;

pub struct OctantInfo {
    key: VoxelKey,
    source: Option<PointView>,
    must_write: bool,
}

impl OctantInfo {
    pub fn new(key: VoxelKey) -> Self {
        OctantInfo {
            key,
            source: None,
            must_write: false,
        }
    }

    /// Move `other`'s points into this octant, leaving `other` with an empty
    /// view (matches C++ `OctantInfo::movePoints`).
    pub fn move_points(&mut self, other: &mut OctantInfo) {
        let Some(other_source) = other.source.as_mut() else {
            return;
        };
        match self.source.as_mut() {
            Some(source) => source.append(other_source),
            None => self.source = Some(other_source.clone()),
        }
        // Reset the source to an empty view, like C++ `source->makeNew()`.
        *other_source = other_source.make_new();
    }

    pub fn source(&self) -> Option<&PointView> {
        self.source.as_ref()
    }

    pub fn source_mut(&mut self) -> Option<&mut PointView> {
        self.source.as_mut()
    }

    pub fn set_source(&mut self, view: PointView) {
        self.source = Some(view);
    }

    pub fn take_source(&mut self) -> Option<PointView> {
        self.source.take()
    }

    pub fn num_points(&self) -> usize {
        self.source.as_ref().map(|v| v.len() as usize).unwrap_or(0)
    }

    pub fn key(&self) -> VoxelKey {
        self.key
    }

    pub fn set_key(&mut self, key: VoxelKey) {
        self.key = key;
    }

    pub fn must_write(&self) -> bool {
        self.must_write
    }

    pub fn set_must_write(&mut self, must_write: bool) {
        self.must_write = must_write;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout};
    use std::rc::Rc;

    fn view_with_xs(xs: &[f64]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for &x in xs {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, x);
        }
        view
    }

    #[test]
    fn move_points_appends_and_empties_source() {
        let mut dst = OctantInfo::new(VoxelKey::ROOT);
        dst.set_source(view_with_xs(&[1.0, 2.0]));

        let mut src = OctantInfo::new(VoxelKey::new(0, 0, 0, 1));
        src.set_source(view_with_xs(&[3.0, 4.0, 5.0]));

        dst.move_points(&mut src);
        assert_eq!(dst.num_points(), 5);
        // Source emptied but still present.
        assert_eq!(src.num_points(), 0);
    }

    #[test]
    fn move_points_into_empty_destination_adopts_source() {
        let mut dst = OctantInfo::new(VoxelKey::ROOT);
        let mut src = OctantInfo::new(VoxelKey::ROOT);
        src.set_source(view_with_xs(&[7.0]));
        dst.move_points(&mut src);
        assert_eq!(dst.num_points(), 1);
        assert_eq!(src.num_points(), 0);
    }

    #[test]
    fn move_points_with_no_source_is_noop() {
        let mut dst = OctantInfo::new(VoxelKey::ROOT);
        dst.set_source(view_with_xs(&[1.0]));
        let mut src = OctantInfo::new(VoxelKey::ROOT);
        dst.move_points(&mut src);
        assert_eq!(dst.num_points(), 1);
    }
}
