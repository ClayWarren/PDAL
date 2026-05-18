use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct OutlierFilter {
    method: String,
    min_k: usize,
    radius: f64,
    mean_k: usize,
    multiplier: f64,
    class_label: f64,
}

impl OutlierFilter {
    pub fn new(
        method: String,
        min_k: usize,
        radius: f64,
        mean_k: usize,
        multiplier: f64,
        class_label: u8,
    ) -> Self {
        Self {
            method,
            min_k,
            radius,
            mean_k,
            multiplier,
            class_label: class_label as f64,
        }
    }

    fn radius_indices(&self, view: &PointView) -> (Vec<u64>, Vec<u64>) {
        let index = SpatialIndex3d::new(view);
        let mut inliers = Vec::new();
        let mut outliers = Vec::new();

        for idx in 0..view.len() {
            if index.radius(idx, self.radius).len() > self.min_k {
                inliers.push(idx);
            } else {
                outliers.push(idx);
            }
        }

        (inliers, outliers)
    }

    fn statistical_indices(&self, view: &PointView) -> (Vec<u64>, Vec<u64>) {
        let index = SpatialIndex3d::new(view);
        let count = self.mean_k + 1;
        let mut distances = Vec::new();

        for idx in 0..view.len() {
            let neighbors = index.knn(idx, count);
            let mut mean = 0.0;
            for (j, (_, sqr_dist)) in neighbors.iter().enumerate().skip(1) {
                let delta = sqr_dist.sqrt() - mean;
                mean += delta / j as f64;
            }
            distances.push(mean);
        }

        if distances.len() < 2 {
            return ((0..view.len()).collect(), Vec::new());
        }

        let mut n = 0usize;
        let mut m1 = 0.0;
        let mut m2 = 0.0;
        for distance in &distances {
            let n1 = n;
            n += 1;
            let delta = distance - m1;
            let delta_n = delta / n as f64;
            m1 += delta_n;
            m2 += delta * delta_n * n1 as f64;
        }

        let stdev = (m2 / (n as f64 - 1.0)).sqrt();
        let threshold = m1 + self.multiplier * stdev;
        let mut inliers = Vec::new();
        let mut outliers = Vec::new();
        for (idx, distance) in distances.into_iter().enumerate() {
            if distance < threshold {
                inliers.push(idx as u64);
            } else {
                outliers.push(idx as u64);
            }
        }

        (inliers, outliers)
    }
}

impl Filter for OutlierFilter {
    fn name(&self) -> &str {
        "filters.outlier"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let method = self.method.to_ascii_lowercase();
        let (inliers, outliers) = match method.as_str() {
            "statistical" => self.statistical_indices(view),
            "radius" => self.radius_indices(view),
            _ => return Ok(vec![output]),
        };

        if inliers.is_empty() {
            return Ok(vec![output]);
        }

        for idx in outliers {
            output.set_f64(idx, &DimId::Classification, self.class_label);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for OutlierFilter {
    fn process_one(&mut self, _view: &pdal_core::point::PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn grid_with_outlier() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));

        for i in 0..5 {
            for j in 0..5 {
                let idx = view.add_point();
                view.set_f64(idx, &DimId::X, i as f64 * 0.5);
                view.set_f64(idx, &DimId::Y, j as f64 * 0.5);
                view.set_f64(idx, &DimId::Z, 0.0);
            }
        }
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, 1000.0);
        view.set_f64(idx, &DimId::Y, 1000.0);
        view.set_f64(idx, &DimId::Z, 1000.0);

        view
    }

    #[test]
    fn radius_mode_labels_noise_class() {
        let view = grid_with_outlier();
        let mut filter = OutlierFilter::new("radius".to_string(), 2, 1.0, 8, 2.0, 18);
        let out = filter.run(&view).unwrap().remove(0);

        for idx in 0..25 {
            assert_eq!(out.get_f64(idx, &DimId::Classification), 0.0);
        }
        assert_eq!(out.get_f64(25, &DimId::Classification), 18.0);
    }

    #[test]
    fn unknown_method_leaves_classification_unchanged() {
        let view = grid_with_outlier();
        let mut filter = OutlierFilter::new("wat".to_string(), 2, 1.0, 8, 2.0, 18);
        let out = filter.run(&view).unwrap().remove(0);

        for idx in 0..out.len() {
            assert_eq!(out.get_f64(idx, &DimId::Classification), 0.0);
        }
    }
}
