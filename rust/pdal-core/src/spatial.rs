use crate::point::{DimId, PointId, PointView};
use rstar::{PointDistance, RTree, RTreeObject, AABB};

/// 2D neighbor-query API used by C++ `KD2Index` compatibility shims.
pub struct SpatialIndex2d<'a> {
    view: &'a PointView,
    tree: RTree<IndexedPoint2d>,
}

impl<'a> SpatialIndex2d<'a> {
    pub fn new(view: &'a PointView) -> Self {
        let points = (0..view.len())
            .map(|id| IndexedPoint2d {
                id,
                point: [view.get_f64(id, &DimId::X), view.get_f64(id, &DimId::Y)],
            })
            .collect();
        Self {
            view,
            tree: RTree::bulk_load(points),
        }
    }

    pub fn radius_xy(&self, x: f64, y: f64, radius: f64) -> Vec<PointId> {
        let query = [x, y];
        let mut ids: Vec<PointId> = self
            .tree
            .locate_within_distance(query, radius * radius)
            .map(|point| point.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    pub fn knn_xy(&self, x: f64, y: f64, k: usize) -> Vec<(PointId, f64)> {
        if k == 0 {
            return Vec::new();
        }
        let query = [x, y];
        let mut neighbors: Vec<(PointId, f64)> = self
            .tree
            .nearest_neighbor_iter(&query)
            .take(k)
            .map(|point| (point.id, point.distance_2(&query)))
            .collect();
        neighbors.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        neighbors
    }

    pub fn squared_distance(&self, idx: PointId, x: f64, y: f64) -> f64 {
        let dx = self.view.get_f64(idx, &DimId::X) - x;
        let dy = self.view.get_f64(idx, &DimId::Y) - y;
        dx * dx + dy * dy
    }
}

/// 3D neighbor-query API used by spatial filters.
pub struct SpatialIndex3d<'a> {
    view: &'a PointView,
    tree: RTree<IndexedPoint3d>,
    tree2d: RTree<IndexedPoint2d>,
}

impl<'a> SpatialIndex3d<'a> {
    pub fn new(view: &'a PointView) -> Self {
        let points3d = (0..view.len())
            .map(|id| IndexedPoint3d {
                id,
                point: [
                    view.get_f64(id, &DimId::X),
                    view.get_f64(id, &DimId::Y),
                    view.get_f64(id, &DimId::Z),
                ],
            })
            .collect();
        let points2d = (0..view.len())
            .map(|id| IndexedPoint2d {
                id,
                point: [view.get_f64(id, &DimId::X), view.get_f64(id, &DimId::Y)],
            })
            .collect();
        Self {
            view,
            tree: RTree::bulk_load(points3d),
            tree2d: RTree::bulk_load(points2d),
        }
    }

    pub fn radius(&self, idx: PointId, radius: f64) -> Vec<PointId> {
        let x = self.view.get_f64(idx, &DimId::X);
        let y = self.view.get_f64(idx, &DimId::Y);
        let z = self.view.get_f64(idx, &DimId::Z);
        self.radius_xyz(x, y, z, radius)
    }

    pub fn radius_2d_excluding(&self, idx: PointId, radius: f64) -> Vec<PointId> {
        let x = self.view.get_f64(idx, &DimId::X);
        let y = self.view.get_f64(idx, &DimId::Y);
        let query = [x, y];
        let mut ids: Vec<PointId> = self
            .tree2d
            .locate_within_distance(query, radius * radius)
            .filter_map(|point| (point.id != idx).then_some(point.id))
            .collect();
        ids.sort_unstable();
        ids
    }

    pub fn radius_xyz(&self, x: f64, y: f64, z: f64, radius: f64) -> Vec<PointId> {
        let query = [x, y, z];
        let mut ids: Vec<PointId> = self
            .tree
            .locate_within_distance(query, radius * radius)
            .map(|point| point.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    pub fn radius_dims(&self, idx: PointId, radius: f64, dims: &[DimId]) -> Vec<PointId> {
        if dims == [DimId::X, DimId::Y] {
            let x = self.view.get_f64(idx, &DimId::X);
            let y = self.view.get_f64(idx, &DimId::Y);
            let query = [x, y];
            let mut ids: Vec<PointId> = self
                .tree2d
                .locate_within_distance(query, radius * radius)
                .map(|point| point.id)
                .collect();
            ids.sort_unstable();
            return ids;
        }
        if dims == [DimId::X, DimId::Y, DimId::Z] {
            return self.radius(idx, radius);
        }

        let radius_sqr = radius * radius;
        let mut ids = Vec::new();
        for candidate in 0..self.view.len() {
            let mut distance = 0.0;
            for dim in dims {
                let delta = self.view.get_f64(candidate, dim) - self.view.get_f64(idx, dim);
                distance += delta * delta;
            }
            if distance <= radius_sqr {
                ids.push(candidate);
            }
        }
        ids
    }

    pub fn knn(&self, idx: PointId, k: usize) -> Vec<(PointId, f64)> {
        let x = self.view.get_f64(idx, &DimId::X);
        let y = self.view.get_f64(idx, &DimId::Y);
        let z = self.view.get_f64(idx, &DimId::Z);

        self.knn_xyz(x, y, z, k)
    }

    pub fn knn_xyz(&self, x: f64, y: f64, z: f64, k: usize) -> Vec<(PointId, f64)> {
        if k == 0 {
            return Vec::new();
        }
        let query = [x, y, z];
        let mut neighbors: Vec<(PointId, f64)> = self
            .tree
            .nearest_neighbor_iter(&query)
            .take(k)
            .map(|point| (point.id, point.distance_2(&query)))
            .collect();
        neighbors.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        neighbors
    }

    pub fn squared_distance(&self, idx: PointId, x: f64, y: f64, z: f64) -> f64 {
        let dx = self.view.get_f64(idx, &DimId::X) - x;
        let dy = self.view.get_f64(idx, &DimId::Y) - y;
        let dz = self.view.get_f64(idx, &DimId::Z) - z;
        dx * dx + dy * dy + dz * dz
    }
}

#[derive(Clone, Copy, Debug)]
struct IndexedPoint2d {
    id: PointId,
    point: [f64; 2],
}

impl RTreeObject for IndexedPoint2d {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.point)
    }
}

impl PointDistance for IndexedPoint2d {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let dx = self.point[0] - point[0];
        let dy = self.point[1] - point[1];
        dx * dx + dy * dy
    }
}

#[derive(Clone, Copy, Debug)]
struct IndexedPoint3d {
    id: PointId,
    point: [f64; 3],
}

impl RTreeObject for IndexedPoint3d {
    type Envelope = AABB<[f64; 3]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.point)
    }
}

impl PointDistance for IndexedPoint3d {
    fn distance_2(&self, point: &[f64; 3]) -> f64 {
        let dx = self.point[0] - point[0];
        let dy = self.point[1] - point[1];
        let dz = self.point[2] - point[2];
        dx * dx + dy * dy + dz * dz
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn two_dimensional_knn_sorts_by_squared_distance() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);

        for (x, y, z) in [(0.0, 0.0, 50.0), (2.0, 0.0, 0.0), (1.0, 0.0, -50.0)] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }

        let index = SpatialIndex2d::new(&view);
        assert_eq!(
            index.knn_xy(0.0, 0.0, 3),
            vec![(0, 0.0), (2, 1.0), (1, 4.0)]
        );
    }

    #[test]
    fn two_dimensional_radius_ignores_z() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);

        for (x, y, z) in [
            (0.0, 0.0, 100.0),
            (0.5, 0.0, -100.0),
            (0.0, 0.5, 200.0),
            (1.1, 0.0, 0.0),
        ] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }

        let index = SpatialIndex2d::new(&view);
        assert_eq!(index.radius_xy(0.0, 0.0, 1.0), vec![0, 1, 2]);
    }

    #[test]
    fn radius_includes_query_point_and_boundary() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);

        for (x, y, z) in [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.1, 0.0, 0.0)] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }

        let index = SpatialIndex3d::new(&view);
        assert_eq!(index.radius(0, 1.0), vec![0, 1]);
    }

    #[test]
    fn knn_sorts_by_squared_distance() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);

        for (x, y, z) in [(0.0, 0.0, 0.0), (2.0, 0.0, 0.0), (1.0, 0.0, 0.0)] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }

        let index = SpatialIndex3d::new(&view);
        assert_eq!(index.knn(0, 3), vec![(0, 0.0), (2, 1.0), (1, 4.0)]);
    }

    #[test]
    fn knn_truncates_to_available_points_and_allows_zero_neighbors() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);

        for (x, y, z) in [(0.0, 0.0, 0.0), (3.0, 0.0, 0.0)] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }

        let index = SpatialIndex3d::new(&view);
        assert!(index.knn(0, 0).is_empty());
        assert_eq!(index.knn(0, 10), vec![(0, 0.0), (1, 9.0)]);
    }

    #[test]
    fn radius_xyz_queries_an_arbitrary_coordinate() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);

        for (x, y, z) in [(10.0, 0.0, 0.0), (11.0, 0.0, 0.0), (12.1, 0.0, 0.0)] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }

        let index = SpatialIndex3d::new(&view);
        assert_eq!(index.radius_xyz(10.5, 0.0, 0.0, 0.5), vec![0, 1]);
    }

    #[test]
    fn radius_2d_excluding_ignores_z_and_query_point() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);

        for (x, y, z) in [
            (0.0, 0.0, 0.0),
            (0.5, 0.0, 100.0),
            (1.1, 0.0, 0.0),
            (0.0, 0.5, -100.0),
        ] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }

        let index = SpatialIndex3d::new(&view);
        assert_eq!(index.radius_2d_excluding(0, 1.0), vec![1, 3]);
    }

    #[test]
    fn radius_dims_uses_only_requested_dimensions() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);

        for (x, y, z) in [(0.0, 0.0, 0.0), (0.5, 0.0, 100.0), (2.0, 0.0, 0.0)] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }

        let index = SpatialIndex3d::new(&view);
        assert_eq!(index.radius_dims(0, 1.0, &[DimId::X, DimId::Y]), vec![0, 1]);
        assert_eq!(
            index.radius_dims(0, 1.0, &[DimId::X, DimId::Y, DimId::Z]),
            vec![0]
        );
    }
}
