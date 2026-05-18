use crate::math;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct EigenvaluesFilter {
    knn: usize,
    normalize: bool,
    stride: usize,
    radius: Option<f64>,
    min_k: usize,
}

impl EigenvaluesFilter {
    pub fn new(
        knn: usize,
        normalize: bool,
        stride: usize,
        radius: Option<f64>,
        min_k: usize,
    ) -> Self {
        Self {
            knn,
            normalize,
            stride: stride.max(1),
            radius,
            min_k,
        }
    }
}

impl Filter for EigenvaluesFilter {
    fn name(&self) -> &str {
        "filters.eigenvalues"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let ids = match self.radius {
                Some(radius) => {
                    let ids = index.radius(idx, radius);
                    if ids.len() < self.min_k {
                        continue;
                    }
                    ids
                }
                None => strided_knn(&index, idx, self.knn + 1, self.stride),
            };

            let covariance = math::covariance(view, &ids);
            if math::is_zero_matrix(covariance) {
                continue;
            }

            let mut eigenvalues = math::symmetric_eigenvalues(covariance);
            if self.normalize {
                let sum = eigenvalues.iter().sum::<f64>();
                for value in &mut eigenvalues {
                    *value /= sum;
                }
            }

            output.set_f64(idx, &DimId::Eigenvalue0, eigenvalues[0]);
            output.set_f64(idx, &DimId::Eigenvalue1, eigenvalues[1]);
            output.set_f64(idx, &DimId::Eigenvalue2, eigenvalues[2]);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for EigenvaluesFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

fn strided_knn(
    index: &SpatialIndex3d,
    idx: pdal_core::point::PointId,
    count: usize,
    stride: usize,
) -> Vec<PointId> {
    let neighbors = index.knn(idx, count.saturating_mul(stride));
    if stride == 1 {
        return neighbors.into_iter().map(|(id, _dist)| id).collect();
    }

    neighbors
        .into_iter()
        .step_by(stride)
        .take(count)
        .map(|(id, _dist)| id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn plane() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Eigenvalue0, DimType::F64);
        layout.register(DimId::Eigenvalue1, DimType::F64);
        layout.register(DimId::Eigenvalue2, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for x in 0..3 {
            for y in 0..3 {
                let idx = view.add_point();
                view.set_f64(idx, &DimId::X, x as f64);
                view.set_f64(idx, &DimId::Y, y as f64);
                view.set_f64(idx, &DimId::Z, 0.0);
            }
        }
        view
    }

    #[test]
    fn planar_neighborhood_has_zero_smallest_eigenvalue() {
        let view = plane();
        let mut filter = EigenvaluesFilter::new(8, false, 1, None, 3);
        let out = filter.run(std::slice::from_ref(&view)).unwrap().remove(0);
        for idx in 0..out.len() {
            assert!(out.get_f64(idx, &DimId::Eigenvalue0).abs() < 1e-12);
            assert!(out.get_f64(idx, &DimId::Eigenvalue1) > 0.0);
            assert!(out.get_f64(idx, &DimId::Eigenvalue2) > 0.0);
        }
    }

    #[test]
    fn normalized_eigenvalues_sum_to_one() {
        let view = plane();
        let mut filter = EigenvaluesFilter::new(8, true, 1, None, 3);
        let out = filter.run(std::slice::from_ref(&view)).unwrap().remove(0);
        for idx in 0..out.len() {
            let sum = out.get_f64(idx, &DimId::Eigenvalue0)
                + out.get_f64(idx, &DimId::Eigenvalue1)
                + out.get_f64(idx, &DimId::Eigenvalue2);
            assert!((sum - 1.0).abs() < 1e-12);
        }
    }
}
