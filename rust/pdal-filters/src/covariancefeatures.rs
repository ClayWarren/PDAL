use crate::math;
use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

/// Eigenvalue scaling mode, mirroring `CovarianceFeaturesFilter::Mode`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Raw,
    Sqrt,
    Normalized,
}

impl Mode {
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => Mode::Raw,
            2 => Mode::Normalized,
            _ => Mode::Sqrt,
        }
    }
}

/// Local geometric features derived from neighborhood covariance eigenvalues.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Feature {
    Linearity,
    Planarity,
    Scattering,
    Verticality,
    Omnivariance,
    Anisotropy,
    Eigenentropy,
    EigenvalueSum,
    SurfaceVariation,
    DemantkeVerticality,
    Density,
}

impl Feature {
    fn dim_name(self) -> &'static str {
        match self {
            Feature::Linearity => "Linearity",
            Feature::Planarity => "Planarity",
            Feature::Scattering => "Scattering",
            Feature::Verticality => "Verticality",
            Feature::Omnivariance => "Omnivariance",
            Feature::Anisotropy => "Anisotropy",
            Feature::Eigenentropy => "Eigenentropy",
            Feature::EigenvalueSum => "EigenvalueSum",
            Feature::SurfaceVariation => "SurfaceVariation",
            Feature::DemantkeVerticality => "DemantkeVerticality",
            Feature::Density => "Density",
        }
    }
}

const DIMENSIONALITY: [Feature; 4] = [
    Feature::Linearity,
    Feature::Planarity,
    Feature::Scattering,
    Feature::Verticality,
];

const ALL_FEATURES: [Feature; 11] = [
    Feature::Linearity,
    Feature::Planarity,
    Feature::Scattering,
    Feature::Verticality,
    Feature::Omnivariance,
    Feature::Anisotropy,
    Feature::Eigenentropy,
    Feature::EigenvalueSum,
    Feature::SurfaceVariation,
    Feature::DemantkeVerticality,
    Feature::Density,
];

/// Expand a comma-separated `feature_set` string into concrete features,
/// mirroring `CovarianceFeaturesFilter::addDimensions`.
fn expand_features(feature_set: &str) -> Vec<Feature> {
    let mut features: Vec<Feature> = Vec::new();
    for raw in feature_set.split(',') {
        match raw.trim().to_lowercase().as_str() {
            "dimensionality" => features.extend_from_slice(&DIMENSIONALITY),
            "all" => features.extend_from_slice(&ALL_FEATURES),
            "linearity" => features.push(Feature::Linearity),
            "planarity" => features.push(Feature::Planarity),
            "scattering" => features.push(Feature::Scattering),
            "verticality" => features.push(Feature::Verticality),
            "omnivariance" => features.push(Feature::Omnivariance),
            "anisotropy" => features.push(Feature::Anisotropy),
            "eigenentropy" => features.push(Feature::Eigenentropy),
            "eigenvaluesum" => features.push(Feature::EigenvalueSum),
            "surfacevariation" => features.push(Feature::SurfaceVariation),
            "demantkeverticality" => features.push(Feature::DemantkeVerticality),
            "density" => features.push(Feature::Density),
            _ => {}
        }
    }
    features
}

/// `filters.covariancefeatures`: local features from neighborhood covariance.
pub struct CovarianceFeaturesFilter {
    knn: usize,
    stride: usize,
    radius: Option<f64>,
    min_k: usize,
    mode: Mode,
    optimal: bool,
    features: Vec<Feature>,
}

impl CovarianceFeaturesFilter {
    pub fn new(
        knn: usize,
        stride: usize,
        radius: Option<f64>,
        min_k: usize,
        mode: Mode,
        optimal: bool,
        feature_set: &str,
    ) -> Self {
        Self {
            knn,
            stride: stride.max(1),
            radius,
            min_k,
            mode,
            optimal,
            features: expand_features(feature_set),
        }
    }

    /// Select the neighbor ids for `idx`, mirroring `setDimensionality`.
    fn neighbors(
        &self,
        index: &SpatialIndex3d,
        view: &PointView,
        idx: PointId,
    ) -> Option<Vec<PointId>> {
        if self.optimal {
            let k = view.get_f64(idx, &DimId::OptimalKNN) as usize;
            return Some(index.knn(idx, k).into_iter().map(|(id, _)| id).collect());
        }
        if let Some(radius) = self.radius {
            let ids = index.radius(idx, radius);
            if ids.len() < self.min_k {
                return None;
            }
            return Some(ids);
        }
        let count = (self.knn + 1).saturating_mul(self.stride);
        let neighbors = index.knn(idx, count);
        let ids = if self.stride == 1 {
            neighbors.into_iter().map(|(id, _)| id).collect()
        } else {
            neighbors
                .into_iter()
                .step_by(self.stride)
                .take(self.knn + 1)
                .map(|(id, _)| id)
                .collect()
        };
        Some(ids)
    }
}

impl Filter for CovarianceFeaturesFilter {
    fn name(&self) -> &str {
        "filters.covariancefeatures"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        self.features
            .iter()
            .map(|f| (DimId::from_name(f.dim_name()), DimType::F64))
            .collect()
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let Some(ids) = self.neighbors(&index, view, idx) else {
                continue;
            };

            let covariance = math::covariance(view, &ids);
            if math::is_zero_matrix(covariance) {
                continue;
            }

            // Eigenvalues ascending; build descending, non-negative lambda.
            let (values, vectors) = math::symmetric_eigen_decomposition(covariance);
            let mut lambda = [values[2].max(0.0), values[1].max(0.0), values[0].max(0.0)];
            let sum = lambda[0] + lambda[1] + lambda[2];
            if lambda[0] == 0.0 {
                return Err(StageError(
                    "Eigenvalues are all 0. Can't compute local features.".to_string(),
                ));
            }

            // Eigenvectors paired with the descending eigenvalues.
            let v1 = [vectors[0][2], vectors[1][2], vectors[2][2]];
            let v2 = [vectors[0][1], vectors[1][1], vectors[2][1]];
            let v3 = [vectors[0][0], vectors[1][0], vectors[2][0]];
            // Smallest-eigenvalue eigenvector, used raw by DemantkeVerticality.
            let e3_z = vectors[2][0];

            match self.mode {
                Mode::Sqrt => {
                    for value in &mut lambda {
                        *value = value.sqrt();
                    }
                }
                Mode::Normalized => {
                    for value in &mut lambda {
                        *value /= sum;
                    }
                }
                Mode::Raw => {}
            }

            for feature in &self.features {
                let value = match feature {
                    Feature::Linearity => (lambda[0] - lambda[1]) / lambda[0],
                    Feature::Planarity => (lambda[1] - lambda[2]) / lambda[0],
                    Feature::Scattering => lambda[2] / lambda[0],
                    Feature::Verticality => {
                        let mut unary = [0.0f64; 3];
                        let mut norm = 0.0;
                        for i in 0..3 {
                            unary[i] = lambda[0] * v1[i].abs()
                                + lambda[1] * v2[i].abs()
                                + lambda[2] * v3[i].abs();
                            norm += unary[i] * unary[i];
                        }
                        unary[2] / norm.sqrt()
                    }
                    Feature::Omnivariance => (lambda[2] * lambda[1] * lambda[0]).cbrt(),
                    Feature::Anisotropy => (lambda[0] - lambda[2]) / lambda[0],
                    Feature::Eigenentropy => {
                        -(lambda[2] * lambda[2].ln()
                            + lambda[1] * lambda[1].ln()
                            + lambda[0] * lambda[0].ln())
                    }
                    Feature::EigenvalueSum => sum,
                    Feature::SurfaceVariation => lambda[2] / sum,
                    Feature::DemantkeVerticality => 1.0 - e3_z.abs(),
                    Feature::Density => {
                        let kopt = view.get_f64(idx, &DimId::OptimalKNN);
                        let ropt = view.get_f64(idx, &DimId::OptimalRadius);
                        // Mirror the truncated pi literal used by the C++ filter.
                        #[allow(clippy::approx_constant)]
                        let pi = 3.141_592_65;
                        (kopt + 1.0) / ((4.0 / 3.0) * pi * ropt.powi(3))
                    }
                };
                output.set_f64(idx, &DimId::from_name(feature.dim_name()), value);
            }
        }

        Ok(vec![output])
    }
}

impl Streamable for CovarianceFeaturesFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{PointLayout, PointView};
    use std::rc::Rc;

    fn view_with(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        for feat in ALL_FEATURES {
            layout.register(DimId::from_name(feat.dim_name()), DimType::F64);
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

    fn run(points: &[(f64, f64, f64)], knn: usize) -> PointView {
        let mut filter = CovarianceFeaturesFilter::new(
            knn,
            1,
            None,
            3,
            Mode::Sqrt,
            false,
            "dimensionality,Omnivariance,Anisotropy,EigenvalueSum,SurfaceVariation",
        );
        filter.run_one(&view_with(points)).unwrap().pop().unwrap()
    }

    #[test]
    fn linear_neighborhood_is_linear() {
        let out = run(&[(0.0, 0.0, 0.0), (0.0, 0.0, 1.0), (0.0, 0.0, 2.0)], 3);
        let lin = DimId::from_name("Linearity");
        let plan = DimId::from_name("Planarity");
        let scat = DimId::from_name("Scattering");
        let anis = DimId::from_name("Anisotropy");
        let esum = DimId::from_name("EigenvalueSum");
        for idx in 0..out.len() {
            assert!((out.get_f64(idx, &lin) - 1.0).abs() < 1e-6);
            assert!(out.get_f64(idx, &plan).abs() < 1e-6);
            assert!(out.get_f64(idx, &scat).abs() < 1e-6);
            assert!((out.get_f64(idx, &anis) - 1.0).abs() < 1e-6);
            assert!((out.get_f64(idx, &esum) - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn planar_neighborhood_is_planar() {
        let out = run(
            &[
                (0.0, 0.0, 0.0),
                (1.0, 0.0, 0.0),
                (1.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
            ],
            10,
        );
        let plan = DimId::from_name("Planarity");
        let scat = DimId::from_name("Scattering");
        for idx in 0..out.len() {
            assert!((out.get_f64(idx, &plan) - 1.0).abs() < 1e-6);
            assert!(out.get_f64(idx, &scat).abs() < 1e-6);
        }
    }

    #[test]
    fn feature_expansion_handles_groups_and_names() {
        assert_eq!(expand_features("dimensionality").len(), 4);
        assert_eq!(expand_features("all").len(), 11);
        assert_eq!(expand_features("Linearity, Density").len(), 2);
        assert_eq!(expand_features("bogus").len(), 0);
    }

    #[test]
    fn test_mode_from_u32_and_feature_expansion_more() {
        assert!(matches!(Mode::from_u32(0), Mode::Raw));
        assert!(matches!(Mode::from_u32(2), Mode::Normalized));
        assert!(matches!(Mode::from_u32(1), Mode::Sqrt));
        assert!(matches!(Mode::from_u32(99), Mode::Sqrt));

        let feats = expand_features("planarity,scattering,verticality,omnivariance,anisotropy,eigenentropy,eigenvaluesum,surfacevariation,demantkeverticality,density");
        assert_eq!(feats.len(), 10);
    }

    #[test]
    fn test_optimal_neighborhood() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::OptimalKNN, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));

        for i in 0..3 {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, i as f64);
            view.set_f64(idx, &DimId::Y, 0.0);
            view.set_f64(idx, &DimId::Z, 0.0);
            view.set_f64(idx, &DimId::OptimalKNN, 2.0);
        }

        let filter = CovarianceFeaturesFilter::new(3, 1, None, 1, Mode::Raw, true, "all");
        let index = SpatialIndex3d::new(&view);
        let n = filter.neighbors(&index, &view, 0).unwrap();
        assert_eq!(n.len(), 2);
    }

    #[test]
    fn test_radius_and_stride_neighborhood() {
        let view = view_with(&[
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (3.0, 0.0, 0.0),
        ]);
        let index = SpatialIndex3d::new(&view);

        let filter = CovarianceFeaturesFilter::new(3, 1, Some(1.5), 2, Mode::Raw, false, "all");
        let n = filter.neighbors(&index, &view, 1).unwrap();
        assert!(n.len() >= 2);

        let filter_less =
            CovarianceFeaturesFilter::new(3, 1, Some(1.5), 10, Mode::Raw, false, "all");
        assert!(filter_less.neighbors(&index, &view, 1).is_none());

        let filter_stride = CovarianceFeaturesFilter::new(2, 2, None, 1, Mode::Raw, false, "all");
        let n_stride = filter_stride.neighbors(&index, &view, 0).unwrap();
        assert!(n_stride.len() <= 2);
    }

    #[test]
    fn test_covariance_zero_and_eigenvalues_error() {
        let view = view_with(&[(0.0, 0.0, 0.0)]);
        let mut filter = CovarianceFeaturesFilter::new(3, 1, None, 1, Mode::Raw, false, "all");
        let res = filter.run_one(&view).unwrap();
        assert_eq!(res[0].len(), 1);

        let view_id = view_with(&[(1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 1.0)]);
        let mut filter_err = CovarianceFeaturesFilter::new(3, 1, None, 1, Mode::Raw, false, "all");
        let res_err = filter_err.run_one(&view_id);
        assert!(res_err.is_ok());
    }

    #[test]
    fn test_all_features() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::OptimalKNN, DimType::F64);
        layout.register(DimId::OptimalRadius, DimType::F64);
        for feat in ALL_FEATURES {
            layout.register(DimId::from_name(feat.dim_name()), DimType::F64);
        }
        let mut view = PointView::new(Rc::new(layout));
        for i in 0..4 {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, i as f64);
            view.set_f64(idx, &DimId::Y, i as f64 * 0.5);
            view.set_f64(idx, &DimId::Z, i as f64 * 0.2);
            view.set_f64(idx, &DimId::OptimalKNN, 3.0);
            view.set_f64(idx, &DimId::OptimalRadius, 2.5);
        }

        let mut filter_raw = CovarianceFeaturesFilter::new(3, 1, None, 1, Mode::Raw, false, "all");
        let out_raw = filter_raw.run_one(&view).unwrap().pop().unwrap();
        assert!(!out_raw.is_empty());

        let mut filter_norm =
            CovarianceFeaturesFilter::new(3, 1, None, 1, Mode::Normalized, false, "all");
        let out_norm = filter_norm.run_one(&view).unwrap().pop().unwrap();
        assert!(!out_norm.is_empty());
    }

    #[test]
    fn test_trait_and_streamable_methods() {
        let filter = CovarianceFeaturesFilter::new(3, 1, None, 1, Mode::Raw, false, "linearity");
        assert_eq!(filter.name(), "filters.covariancefeatures");
        assert!(Filter::as_any(&filter)
            .downcast_ref::<CovarianceFeaturesFilter>()
            .is_some());

        let dims = filter.output_dimensions();
        assert_eq!(dims.len(), 1);
        assert_eq!(dims[0].0, DimId::from_name("Linearity"));

        let mut filter_mut =
            CovarianceFeaturesFilter::new(3, 1, None, 1, Mode::Raw, false, "linearity");
        let mut scratch = PointView::new(Rc::new(PointLayout::new()));
        assert!(!Streamable::process_one(&mut filter_mut, &mut scratch, 0));
    }
}
