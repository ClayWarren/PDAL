use crate::math;
use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct OptimalNeighborhoodFilter {
    min_k: usize,
    max_k: usize,
}

impl OptimalNeighborhoodFilter {
    pub fn new(min_k: usize, max_k: usize) -> Self {
        Self { min_k, max_k }
    }
}

impl Filter for OptimalNeighborhoodFilter {
    fn name(&self) -> &str {
        "filters.optimalneighborhood"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let neighbors = index.knn(idx, self.max_k);
            let mut min_entropy = f64::MAX;
            let mut optimal_k = 0usize;
            let mut optimal_radius_sqr = 0.0;

            for k in self.min_k..=self.max_k {
                if k > neighbors.len() {
                    break;
                }

                let ids = neighbors
                    .iter()
                    .take(k)
                    .map(|(id, _dist)| *id)
                    .collect::<Vec<_>>();
                let eigenvalues = math::symmetric_eigenvalues(math::covariance(view, &ids));
                let mut lambda = [
                    eigenvalues[2].max(0.0),
                    eigenvalues[1].max(0.0),
                    eigenvalues[0].max(0.0),
                ];
                let sum = lambda.iter().sum::<f64>();
                for value in &mut lambda {
                    *value /= sum;
                }

                let entropy = -(lambda[2] * lambda[2].ln()
                    + lambda[1] * lambda[1].ln()
                    + lambda[0] * lambda[0].ln());
                if entropy < min_entropy {
                    min_entropy = entropy;
                    optimal_k = k;
                    optimal_radius_sqr = neighbors[k - 1].1;
                }
            }

            output.set_f64(idx, &DimId::OptimalKNN, optimal_k as f64);
            output.set_f64(idx, &DimId::OptimalRadius, optimal_radius_sqr.sqrt());
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for OptimalNeighborhoodFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn optimal_k_stays_within_requested_window() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::OptimalKNN, DimType::F64);
        layout.register(DimId::OptimalRadius, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for idx in 0..60 {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, (idx % 7) as f64);
            view.set_f64(point, &DimId::Y, (idx / 7) as f64);
            view.set_f64(point, &DimId::Z, (idx % 5) as f64);
        }

        let mut filter = OptimalNeighborhoodFilter::new(5, 8);
        let out = filter.run(&view).unwrap().remove(0);
        for idx in 0..out.len() {
            let k = out.get_f64(idx, &DimId::OptimalKNN);
            assert!((5.0..=8.0).contains(&k));
            assert!(out.get_f64(idx, &DimId::OptimalRadius) > 0.0);
        }
    }
}
