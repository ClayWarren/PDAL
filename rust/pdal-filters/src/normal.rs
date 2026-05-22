use crate::math;
use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

/// `filters.normal`: estimate per-point surface normals and curvature from the
/// covariance of each point's neighborhood.
///
/// This covers the `compute` path of `pdal::NormalFilter`. Minimum-spanning-tree
/// `refine` propagation deliberately stays in C++. `knn` is the effective
/// neighbor count (the query point counts as one of its own neighbors).
pub struct NormalFilter {
    knn: usize,
    radius: Option<f64>,
    viewpoint: Option<[f64; 3]>,
    always_up: bool,
}

impl NormalFilter {
    pub fn new(
        knn: usize,
        radius: Option<f64>,
        viewpoint: Option<[f64; 3]>,
        always_up: bool,
    ) -> Self {
        Self {
            knn,
            radius,
            viewpoint,
            always_up,
        }
    }
}

/// Estimate the normal and curvature for a neighborhood, mirroring
/// `pdal::math::findNormal`. Returns `None` when the point should be skipped.
fn find_normal(view: &PointView, ids: &[PointId]) -> Option<([f64; 3], f64)> {
    if ids.len() < 3 {
        return None;
    }
    let covariance = math::covariance(view, ids);
    if math::is_zero_matrix(covariance) {
        return None;
    }

    // Eigenvalues ascending; the normal is the smallest-eigenvalue eigenvector.
    let (values, vectors) = math::symmetric_eigen_decomposition(covariance);
    let sum = values[0] + values[1] + values[2];
    let curvature = if sum != 0.0 {
        (values[0] / sum).abs()
    } else {
        0.0
    };
    let normal = [vectors[0][0], vectors[1][0], vectors[2][0]];
    Some((normal, curvature))
}

fn negate(v: [f64; 3]) -> [f64; 3] {
    [-v[0], -v[1], -v[2]]
}

impl Filter for NormalFilter {
    fn name(&self) -> &str {
        "filters.normal"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![
            (DimId::from_name("NormalX"), DimType::F64),
            (DimId::from_name("NormalY"), DimType::F64),
            (DimId::from_name("NormalZ"), DimType::F64),
            (DimId::from_name("Curvature"), DimType::F64),
        ]
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let normal_x = DimId::from_name("NormalX");
        let normal_y = DimId::from_name("NormalY");
        let normal_z = DimId::from_name("NormalZ");
        let curvature_dim = DimId::from_name("Curvature");

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let ids: Vec<PointId> = match self.radius {
                Some(radius) => index.radius(idx, radius),
                None => index
                    .knn(idx, self.knn)
                    .into_iter()
                    .map(|(id, _)| id)
                    .collect(),
            };

            let Some((mut normal, curvature)) = find_normal(view, &ids) else {
                continue;
            };

            if let Some(viewpoint) = self.viewpoint {
                // Flip the normal to face the viewpoint.
                let dx = viewpoint[0] - view.get_f64(idx, &DimId::X);
                let dy = viewpoint[1] - view.get_f64(idx, &DimId::Y);
                let dz = viewpoint[2] - view.get_f64(idx, &DimId::Z);
                if dx * normal[0] + dy * normal[1] + dz * normal[2] < 0.0 {
                    normal = negate(normal);
                }
            } else if self.always_up && normal[2] < 0.0 {
                normal = negate(normal);
            }

            output.set_f64(idx, &normal_x, normal[0]);
            output.set_f64(idx, &normal_y, normal[1]);
            output.set_f64(idx, &normal_z, normal[2]);
            output.set_f64(idx, &curvature_dim, curvature);
        }

        Ok(vec![output])
    }
}

impl Streamable for NormalFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{PointLayout, PointView};
    use std::rc::Rc;

    fn grid_view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        for name in ["NormalX", "NormalY", "NormalZ", "Curvature"] {
            layout.register(DimId::from_name(name), DimType::F64);
        }
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Y, *y);
            view.set_f64(idx, &DimId::Z, *z);
        }
        view
    }

    #[test]
    fn xy_plane_normal_points_up() {
        let view = grid_view(&[
            (0.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (0.0, 2.0, 0.0),
            (2.0, 2.0, 0.0),
        ]);
        let mut filter = NormalFilter::new(4, None, None, true);
        let out = filter.run_one(&view).unwrap().pop().unwrap();
        let (nx, ny) = (DimId::from_name("NormalX"), DimId::from_name("NormalY"));
        let (nz, cv) = (DimId::from_name("NormalZ"), DimId::from_name("Curvature"));
        for idx in 0..out.len() {
            assert!(out.get_f64(idx, &nx).abs() < 1e-9);
            assert!(out.get_f64(idx, &ny).abs() < 1e-9);
            assert!((out.get_f64(idx, &nz) - 1.0).abs() < 1e-9);
            assert!(out.get_f64(idx, &cv).abs() < 1e-9);
        }
    }

    #[test]
    fn xz_plane_normal_points_along_y() {
        let view = grid_view(&[
            (0.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (0.0, 0.0, 2.0),
            (2.0, 0.0, 2.0),
        ]);
        let mut filter = NormalFilter::new(4, None, None, true);
        let out = filter.run_one(&view).unwrap().pop().unwrap();
        let ny = DimId::from_name("NormalY");
        for idx in 0..out.len() {
            assert!((out.get_f64(idx, &ny).abs() - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn viewpoint_orients_normal_toward_viewpoint() {
        let view = grid_view(&[
            (0.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (0.0, 2.0, 0.0),
            (2.0, 2.0, 0.0),
        ]);
        // Viewpoint well below the plane: normals should face -Z.
        let mut filter = NormalFilter::new(4, None, Some([1.0, 1.0, -100.0]), true);
        let out = filter.run_one(&view).unwrap().pop().unwrap();
        let nz = DimId::from_name("NormalZ");
        for idx in 0..out.len() {
            assert!((out.get_f64(idx, &nz) + 1.0).abs() < 1e-9);
        }
    }
}
