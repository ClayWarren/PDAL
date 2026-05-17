use crate::math;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct NormalFilter {
    knn: usize,
    radius: Option<f64>,
    always_up: bool,
    viewpoint: Option<[f64; 3]>,
    refine: bool,
}

impl NormalFilter {
    pub fn new(
        knn: usize,
        radius: Option<f64>,
        always_up: bool,
        viewpoint: Option<[f64; 3]>,
        refine: bool,
    ) -> Self {
        Self {
            knn,
            radius,
            always_up,
            viewpoint,
            refine,
        }
    }
}

impl Filter for NormalFilter {
    fn name(&self) -> &str {
        "filters.normal"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let neighbors = match self.radius {
                Some(radius) => index.radius(idx, radius),
                None => index
                    .knn(idx, self.knn)
                    .into_iter()
                    .map(|(id, _)| id)
                    .collect(),
            };
            let Some((mut normal, curvature)) = find_normal(view, &neighbors) else {
                continue;
            };

            if let Some(viewpoint) = self.viewpoint {
                let direction = [
                    viewpoint[0] - view.get_f64(idx, &DimId::X),
                    viewpoint[1] - view.get_f64(idx, &DimId::Y),
                    viewpoint[2] - view.get_f64(idx, &DimId::Z),
                ];
                normal = orient_to_viewpoint(direction, normal);
            } else if self.always_up {
                normal = orient_up(normal);
            }

            set_normal(&mut output, idx, normal, curvature);
        }

        if self.refine {
            refine_normals(&mut output, &index, self.knn, self.radius);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for NormalFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

fn find_normal(view: &PointView, neighbors: &[PointId]) -> Option<([f64; 3], f64)> {
    if neighbors.len() < 3 {
        return None;
    }

    let covariance = math::covariance(view, neighbors);
    if math::is_zero_matrix(covariance) {
        return None;
    }

    let (values, vectors) = math::symmetric_eigen_decomposition(covariance);
    let sum = values[0] + values[1] + values[2];
    let curvature = if sum != 0.0 {
        (values[0] / sum).abs()
    } else {
        0.0
    };
    Some(([vectors[0][0], vectors[1][0], vectors[2][0]], curvature))
}

fn orient_up(normal: [f64; 3]) -> [f64; 3] {
    if normal[2] < 0.0 {
        [-normal[0], -normal[1], -normal[2]]
    } else {
        normal
    }
}

fn orient_to_viewpoint(direction: [f64; 3], normal: [f64; 3]) -> [f64; 3] {
    if dot(direction, normal) < 0.0 {
        [-normal[0], -normal[1], -normal[2]]
    } else {
        normal
    }
}

fn set_normal(view: &mut PointView, idx: PointId, normal: [f64; 3], curvature: f64) {
    view.set_f64(idx, &DimId::NormalX, normal[0]);
    view.set_f64(idx, &DimId::NormalY, normal[1]);
    view.set_f64(idx, &DimId::NormalZ, normal[2]);
    view.set_f64(idx, &DimId::Curvature, curvature);
}

fn get_normal(view: &PointView, idx: PointId) -> [f64; 3] {
    [
        view.get_f64(idx, &DimId::NormalX),
        view.get_f64(idx, &DimId::NormalY),
        view.get_f64(idx, &DimId::NormalZ),
    ]
}

fn refine_normals(
    output: &mut PointView,
    source_index: &SpatialIndex3d,
    knn: usize,
    radius: Option<f64>,
) {
    let mut edge_queue = Vec::<Edge>::new();
    let mut in_mst = vec![false; output.len() as usize];
    let mut count = 0usize;
    let mut next_idx = 0;

    while count < output.len() as usize {
        while next_idx < output.len() && in_mst[next_idx as usize] {
            next_idx += 1;
        }
        if next_idx >= output.len() {
            break;
        }

        update_mst(
            output,
            source_index,
            &mut in_mst,
            &mut edge_queue,
            next_idx,
            knn,
            radius,
            &mut count,
        );

        while !edge_queue.is_empty() && count < output.len() as usize {
            edge_queue.sort_by(|left, right| right.weight.total_cmp(&left.weight));
            let edge = edge_queue.pop().expect("edge queue is non-empty");
            let n0 = get_normal(output, edge.v0);
            let n1 = get_normal(output, edge.v1);
            let (new_idx, mut normal) = if !in_mst[edge.v0 as usize] {
                (edge.v0, n0)
            } else if !in_mst[edge.v1 as usize] {
                (edge.v1, n1)
            } else {
                continue;
            };

            if dot(n0, n1) < 0.0 {
                normal = [-normal[0], -normal[1], -normal[2]];
                output.set_f64(new_idx, &DimId::NormalX, normal[0]);
                output.set_f64(new_idx, &DimId::NormalY, normal[1]);
                output.set_f64(new_idx, &DimId::NormalZ, normal[2]);
            }

            update_mst(
                output,
                source_index,
                &mut in_mst,
                &mut edge_queue,
                new_idx,
                knn,
                radius,
                &mut count,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update_mst(
    output: &PointView,
    index: &SpatialIndex3d,
    in_mst: &mut [bool],
    edge_queue: &mut Vec<Edge>,
    update_idx: PointId,
    knn: usize,
    radius: Option<f64>,
    count: &mut usize,
) {
    in_mst[update_idx as usize] = true;
    *count += 1;

    let mut neighbors = match radius {
        Some(radius) => index.radius(update_idx, radius),
        None => index
            .knn(update_idx, knn)
            .into_iter()
            .map(|(id, _)| id)
            .collect(),
    };
    if !neighbors.is_empty() {
        neighbors.remove(0);
    }

    let n0 = get_normal(output, update_idx);
    for neighbor in neighbors {
        if !in_mst[neighbor as usize] {
            let n1 = get_normal(output, neighbor);
            edge_queue.push(Edge {
                v0: update_idx,
                v1: neighbor,
                weight: 1.0 - dot(n0, n1).abs(),
            });
        }
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

struct Edge {
    v0: PointId,
    v1: PointId,
    weight: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::NormalX, DimType::F64);
        layout.register(DimId::NormalY, DimType::F64);
        layout.register(DimId::NormalZ, DimType::F64);
        layout.register(DimId::Curvature, DimType::F64);
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
    fn xy_plane_normals_point_up() {
        let view = view(&[
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (1.0, 1.0, 0.0),
        ]);
        let mut filter = NormalFilter::new(4, None, true, None, false);
        let out = filter.run(&view).unwrap().remove(0);
        for idx in 0..out.len() {
            assert!(out.get_f64(idx, &DimId::NormalX).abs() < 1e-12);
            assert!(out.get_f64(idx, &DimId::NormalY).abs() < 1e-12);
            assert!((out.get_f64(idx, &DimId::NormalZ) - 1.0).abs() < 1e-12);
            assert!(out.get_f64(idx, &DimId::Curvature).abs() < 1e-12);
        }
    }
}
