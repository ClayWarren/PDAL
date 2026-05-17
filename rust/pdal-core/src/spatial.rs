use crate::point::{DimId, PointId, PointView};

/// 3D neighbor-query API used by spatial filters.
///
/// The first backend is intentionally simple: an exact brute-force scan. That
/// keeps the behavioral contract obvious while the Rust/C ABI slice settles;
/// a real KD-tree can replace the internals without changing filter code.
pub struct SpatialIndex3d<'a> {
    view: &'a PointView,
}

impl<'a> SpatialIndex3d<'a> {
    pub fn new(view: &'a PointView) -> Self {
        Self { view }
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
        let radius_sqr = radius * radius;
        let mut ids = Vec::new();
        for candidate in 0..self.view.len() {
            if candidate == idx {
                continue;
            }
            let dx = self.view.get_f64(candidate, &DimId::X) - x;
            let dy = self.view.get_f64(candidate, &DimId::Y) - y;
            if dx * dx + dy * dy <= radius_sqr {
                ids.push(candidate);
            }
        }
        ids
    }

    pub fn radius_xyz(&self, x: f64, y: f64, z: f64, radius: f64) -> Vec<PointId> {
        let radius_sqr = radius * radius;
        let mut ids = Vec::new();
        for idx in 0..self.view.len() {
            if self.squared_distance(idx, x, y, z) <= radius_sqr {
                ids.push(idx);
            }
        }
        ids
    }

    pub fn knn(&self, idx: PointId, k: usize) -> Vec<(PointId, f64)> {
        let x = self.view.get_f64(idx, &DimId::X);
        let y = self.view.get_f64(idx, &DimId::Y);
        let z = self.view.get_f64(idx, &DimId::Z);

        let mut neighbors = Vec::with_capacity(self.view.len() as usize);
        for candidate in 0..self.view.len() {
            neighbors.push((candidate, self.squared_distance(candidate, x, y, z)));
        }
        neighbors.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        neighbors.truncate(k.min(neighbors.len()));
        neighbors
    }

    fn squared_distance(&self, idx: PointId, x: f64, y: f64, z: f64) -> f64 {
        let dx = self.view.get_f64(idx, &DimId::X) - x;
        let dy = self.view.get_f64(idx, &DimId::Y) - y;
        let dz = self.view.get_f64(idx, &DimId::Z) - z;
        dx * dx + dy * dy + dz * dz
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

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
}
