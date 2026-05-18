use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct SkewnessBalancingFilter {
    ground_class: u8,
    other_class: u8,
    only_ground: bool,
}

impl SkewnessBalancingFilter {
    pub fn new(ground_class: u8, other_class: u8, only_ground: bool) -> Self {
        Self {
            ground_class,
            other_class,
            only_ground,
        }
    }
}

impl Filter for SkewnessBalancingFilter {
    fn name(&self) -> &str {
        "filters.skewnessbalancing"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut indices: Vec<PointId> = (0..view.len()).collect();
        indices.sort_by(|left, right| {
            view.get_f64(*left, &DimId::Z)
                .partial_cmp(&view.get_f64(*right, &DimId::Z))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut output = view.make_new();
        for idx in indices {
            output.append_point(view, idx);
        }

        self.process_ground(&mut output);
        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for SkewnessBalancingFilter {
    fn process_one(
        &mut self,
        _view: &pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        false
    }
}

impl SkewnessBalancingFilter {
    fn process_ground(&self, view: &mut PointView) {
        if view.is_empty() {
            return;
        }

        let mut n = 0.0;
        let mut mean = 0.0;
        let mut m2 = 0.0;
        let mut m3 = 0.0;
        let mut last_positive = 0;
        let mut skewness = 0.0;
        let mut last_skewness = f64::NAN;

        for i in 0..view.len() {
            let z = view.get_f64(i, &DimId::Z);
            let previous_n = n;
            n += 1.0;
            let delta = z - mean;
            let delta_n = delta / n;
            let term1 = delta * delta_n * previous_n;
            mean += delta_n;
            m3 += term1 * delta_n * (n - 2.0) - 3.0 * delta_n * m2;
            m2 += term1;
            skewness = n.sqrt() * m3 / m2.powf(1.5);

            if skewness > 0.0 && last_skewness <= 0.0 {
                set_class(view, last_positive, i - 1, self.ground_class);
                last_positive = i;
            }
            last_skewness = skewness;
        }

        if last_positive == 0 && skewness <= 0.0 {
            set_class(view, last_positive, view.len() - 1, self.ground_class);
        } else if !self.only_ground {
            set_class(view, last_positive, view.len() - 1, self.other_class);
        }
    }
}

fn set_class(view: &mut PointView, first: PointId, last: PointId, class_label: u8) {
    if first > last {
        return;
    }
    for idx in first..=last {
        view.set_f64(idx, &DimId::Classification, class_label as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn view(zs: &[f64]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for z in zs {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::Z, *z);
        }
        view
    }

    #[test]
    fn marks_other_when_no_ground_break_is_found() {
        let view = view(&[0.0, 0.0, 0.0, 0.0]);
        let mut filter = SkewnessBalancingFilter::new(2, 1, false);
        let output = filter.run(&view).unwrap().remove(0);
        for idx in 0..output.len() {
            assert_eq!(output.get_f64(idx, &DimId::Classification), 1.0);
        }
    }

    #[test]
    fn leaves_other_points_when_only_ground_is_set() {
        let view = view(&[0.0, 0.0, 0.0, 100.0]);
        let mut filter = SkewnessBalancingFilter::new(2, 1, true);
        let output = filter.run(&view).unwrap().remove(0);
        assert_eq!(output.get_f64(3, &DimId::Classification), 0.0);
    }
}
