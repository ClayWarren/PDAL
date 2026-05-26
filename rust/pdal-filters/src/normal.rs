use crate::math;
use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

/// `filters.normal`: estimate per-point surface normals and curvature from the
/// covariance of each point's neighborhood.
///
/// `knn` is the effective neighbor count (the query point counts as one of its
/// own neighbors).
pub struct NormalFilter {
    knn: usize,
    radius: Option<f64>,
    viewpoint: Option<[f64; 3]>,
    always_up: bool,
    refine: bool,
}

impl NormalFilter {
    pub fn new(
        knn: usize,
        radius: Option<f64>,
        viewpoint: Option<[f64; 3]>,
        always_up: bool,
        refine: bool,
    ) -> Self {
        Self {
            knn,
            radius,
            viewpoint,
            always_up,
            refine,
        }
    }
}

#[derive(Clone, Copy)]
struct RefinementEdge {
    a: PointId,
    b: PointId,
    weight: f64,
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

fn normal_at(view: &PointView, idx: PointId) -> [f64; 3] {
    [
        view.get_f64(idx, &DimId::from_name("NormalX")),
        view.get_f64(idx, &DimId::from_name("NormalY")),
        view.get_f64(idx, &DimId::from_name("NormalZ")),
    ]
}

fn set_normal(view: &mut PointView, idx: PointId, normal: [f64; 3]) {
    view.set_f64(idx, &DimId::from_name("NormalX"), normal[0]);
    view.set_f64(idx, &DimId::from_name("NormalY"), normal[1]);
    view.set_f64(idx, &DimId::from_name("NormalZ"), normal[2]);
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn update_refinement_edges(
    view: &PointView,
    coords: &[[f64; 3]],
    idx: PointId,
    in_mst: &mut [bool],
    edges: &mut Vec<RefinementEdge>,
    knn: usize,
    radius: Option<f64>,
) {
    in_mst[idx as usize] = true;

    let neighbors = refinement_neighbors(coords, idx, knn, radius);
    let n0 = normal_at(view, idx);
    for neighbor in neighbors {
        if neighbor == idx || in_mst[neighbor as usize] {
            continue;
        }
        let n1 = normal_at(view, neighbor);
        edges.push(RefinementEdge {
            a: idx,
            b: neighbor,
            weight: 1.0 - dot(n0, n1).abs(),
        });
    }
}

fn refinement_neighbors(
    coords: &[[f64; 3]],
    idx: PointId,
    knn: usize,
    radius: Option<f64>,
) -> Vec<PointId> {
    let origin = coords[idx as usize];
    let mut distances: Vec<(PointId, f64)> = coords
        .iter()
        .enumerate()
        .map(|(candidate, coord)| {
            let dx = coord[0] - origin[0];
            let dy = coord[1] - origin[1];
            let dz = coord[2] - origin[2];
            (candidate as PointId, dx * dx + dy * dy + dz * dz)
        })
        .collect();

    if let Some(radius) = radius {
        let radius_sqr = radius * radius;
        distances
            .into_iter()
            .filter_map(|(id, distance)| (distance <= radius_sqr).then_some(id))
            .collect()
    } else {
        distances.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        distances.truncate(knn.min(distances.len()));
        distances.into_iter().map(|(id, _)| id).collect()
    }
}

fn pop_lightest_edge(edges: &mut Vec<RefinementEdge>) -> Option<RefinementEdge> {
    let (idx, _) = edges
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.weight.total_cmp(&b.weight))?;
    Some(edges.swap_remove(idx))
}

fn refine_normals(view: &mut PointView, knn: usize, radius: Option<f64>) {
    let len = view.len() as usize;
    let coords: Vec<[f64; 3]> = (0..view.len())
        .map(|idx| {
            [
                view.get_f64(idx, &DimId::X),
                view.get_f64(idx, &DimId::Y),
                view.get_f64(idx, &DimId::Z),
            ]
        })
        .collect();
    let mut in_mst = vec![false; len];
    let mut edges = Vec::new();
    let mut count = 0usize;
    let mut next = 0usize;

    while count < len {
        while next < len && in_mst[next] {
            next += 1;
        }
        if next >= len {
            break;
        }

        update_refinement_edges(
            view,
            &coords,
            next as PointId,
            &mut in_mst,
            &mut edges,
            knn,
            radius,
        );
        count += 1;

        while let Some(edge) = pop_lightest_edge(&mut edges) {
            if count >= len {
                break;
            }

            let n0 = normal_at(view, edge.a);
            let n1 = normal_at(view, edge.b);
            let (new_idx, mut normal) = if !in_mst[edge.a as usize] {
                (edge.a, n0)
            } else if !in_mst[edge.b as usize] {
                (edge.b, n1)
            } else {
                continue;
            };

            if dot(n0, n1) < 0.0 {
                normal = negate(normal);
                set_normal(view, new_idx, normal);
            }

            update_refinement_edges(view, &coords, new_idx, &mut in_mst, &mut edges, knn, radius);
            count += 1;
        }
    }
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

        if self.refine {
            refine_normals(&mut output, self.knn, self.radius);
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
        let mut filter = NormalFilter::new(4, None, None, true, false);
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
        let mut filter = NormalFilter::new(4, None, None, true, false);
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
        let mut filter = NormalFilter::new(4, None, Some([1.0, 1.0, -100.0]), true, false);
        let out = filter.run_one(&view).unwrap().pop().unwrap();
        let nz = DimId::from_name("NormalZ");
        for idx in 0..out.len() {
            assert!((out.get_f64(idx, &nz) + 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn refine_flips_neighbor_normals_into_consistent_orientation() {
        let mut view = grid_view(&[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0)]);
        let nx = DimId::from_name("NormalX");
        let ny = DimId::from_name("NormalY");
        let nz = DimId::from_name("NormalZ");
        for idx in 0..view.len() {
            view.set_f64(idx, &nx, 0.0);
            view.set_f64(idx, &ny, 0.0);
        }
        view.set_f64(0, &nz, 1.0);
        view.set_f64(1, &nz, -1.0);
        view.set_f64(2, &nz, -1.0);

        refine_normals(&mut view, 3, None);

        for idx in 0..view.len() {
            assert!((view.get_f64(idx, &nz) - 1.0).abs() < 1e-9);
        }
    }
}
