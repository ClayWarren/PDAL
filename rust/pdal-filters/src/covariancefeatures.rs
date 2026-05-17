use crate::math;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};
use std::f64::consts::PI;

#[derive(Clone, Copy)]
pub enum CovarianceFeaturesMode {
    Raw,
    Sqrt,
    Normalized,
}

pub struct CovarianceFeaturesFilter {
    knn: usize,
    radius: Option<f64>,
    min_k: usize,
    stride: usize,
    mode: CovarianceFeaturesMode,
    optimal: bool,
    dims: Vec<DimId>,
}

impl CovarianceFeaturesFilter {
    pub fn new(
        knn: usize,
        radius: Option<f64>,
        min_k: usize,
        stride: usize,
        mode: CovarianceFeaturesMode,
        optimal: bool,
        dims: Vec<DimId>,
    ) -> Self {
        Self {
            knn,
            radius,
            min_k,
            stride: stride.max(1),
            mode,
            optimal,
            dims,
        }
    }
}

impl Filter for CovarianceFeaturesFilter {
    fn name(&self) -> &str {
        "filters.covariancefeatures"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let ids = neighbor_ids(view, &index, idx, self);
            if ids.len() < self.min_k && self.radius.is_some() && !self.optimal {
                continue;
            }

            let covariance = math::covariance(view, &ids);
            if math::is_zero_matrix(covariance) {
                continue;
            }

            let (eigenvalues, eigenvectors) = math::symmetric_eigen_decomposition(covariance);
            let mut lambda = [
                eigenvalues[2].max(0.0),
                eigenvalues[1].max(0.0),
                eigenvalues[0].max(0.0),
            ];
            let sum = lambda.iter().sum::<f64>();
            if lambda[0] == 0.0 {
                return Err(StageError(
                    "Eigenvalues are all 0. Can't compute local features.".to_string(),
                ));
            }

            match self.mode {
                CovarianceFeaturesMode::Raw => {}
                CovarianceFeaturesMode::Sqrt => {
                    for value in &mut lambda {
                        *value = value.sqrt();
                    }
                }
                CovarianceFeaturesMode::Normalized => {
                    for value in &mut lambda {
                        *value /= sum;
                    }
                }
            }

            let v1 = [eigenvectors[0][2], eigenvectors[1][2], eigenvectors[2][2]];
            let v2 = [eigenvectors[0][1], eigenvectors[1][1], eigenvectors[2][1]];
            let v3 = [eigenvectors[0][0], eigenvectors[1][0], eigenvectors[2][0]];

            for dim in &self.dims {
                match dim {
                    DimId::Linearity => {
                        output.set_f64(idx, dim, (lambda[0] - lambda[1]) / lambda[0]);
                    }
                    DimId::Planarity => {
                        output.set_f64(idx, dim, (lambda[1] - lambda[2]) / lambda[0]);
                    }
                    DimId::Scattering => {
                        output.set_f64(idx, dim, lambda[2] / lambda[0]);
                    }
                    DimId::Verticality => {
                        let mut unary = [0.0; 3];
                        let mut norm = 0.0;
                        for i in 0..3 {
                            unary[i] = lambda[0] * v1[i].abs()
                                + lambda[1] * v2[i].abs()
                                + lambda[2] * v3[i].abs();
                            norm += unary[i] * unary[i];
                        }
                        output.set_f64(idx, dim, unary[2] / norm.sqrt());
                    }
                    DimId::Omnivariance => {
                        output.set_f64(idx, dim, (lambda[2] * lambda[1] * lambda[0]).cbrt());
                    }
                    DimId::EigenvalueSum => output.set_f64(idx, dim, sum),
                    DimId::Eigenentropy => {
                        let entropy = -(lambda[2] * lambda[2].ln()
                            + lambda[1] * lambda[1].ln()
                            + lambda[0] * lambda[0].ln());
                        output.set_f64(idx, dim, entropy);
                    }
                    DimId::Anisotropy => {
                        output.set_f64(idx, dim, (lambda[0] - lambda[2]) / lambda[0]);
                    }
                    DimId::SurfaceVariation => output.set_f64(idx, dim, lambda[2] / sum),
                    DimId::DemantkeVerticality => output.set_f64(idx, dim, 1.0 - v3[2].abs()),
                    DimId::Density => {
                        let kopt = view.get_f64(idx, &DimId::OptimalKNN);
                        let ropt = view.get_f64(idx, &DimId::OptimalRadius);
                        output.set_f64(idx, dim, (kopt + 1.0) / ((4.0 / 3.0) * PI * ropt.powi(3)));
                    }
                    _ => {}
                }
            }
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for CovarianceFeaturesFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

fn neighbor_ids(
    view: &PointView,
    index: &SpatialIndex3d,
    idx: PointId,
    filter: &CovarianceFeaturesFilter,
) -> Vec<PointId> {
    if filter.optimal {
        let k = view.get_f64(idx, &DimId::OptimalKNN) as usize;
        return index
            .knn(idx, k)
            .into_iter()
            .map(|(id, _dist)| id)
            .collect();
    }

    if let Some(radius) = filter.radius {
        return index.radius(idx, radius);
    }

    strided_knn(index, idx, filter.knn + 1, filter.stride)
}

fn strided_knn(index: &SpatialIndex3d, idx: PointId, count: usize, stride: usize) -> Vec<PointId> {
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
        for dim in [
            DimId::X,
            DimId::Y,
            DimId::Z,
            DimId::Linearity,
            DimId::Planarity,
            DimId::Scattering,
            DimId::Verticality,
        ] {
            layout.register(dim, DimType::F64);
        }
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
    fn planar_neighborhood_sets_dimensionality_features() {
        let view = plane();
        let mut filter = CovarianceFeaturesFilter::new(
            8,
            None,
            3,
            1,
            CovarianceFeaturesMode::Sqrt,
            false,
            vec![
                DimId::Linearity,
                DimId::Planarity,
                DimId::Scattering,
                DimId::Verticality,
            ],
        );
        let out = filter.run(&view).unwrap().remove(0);
        for idx in 0..out.len() {
            assert!(out.get_f64(idx, &DimId::Planarity) > 0.0);
            assert_eq!(out.get_f64(idx, &DimId::Scattering), 0.0);
            assert_eq!(out.get_f64(idx, &DimId::Verticality), 0.0);
        }
    }
}
