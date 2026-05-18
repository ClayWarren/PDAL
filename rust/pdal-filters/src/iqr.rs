use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct IqrFilter {
    multiplier: f64,
    dim: DimId,
}

impl IqrFilter {
    pub fn new(multiplier: f64, dim: DimId) -> Self {
        Self { multiplier, dim }
    }
}

impl Filter for IqrFilter {
    fn name(&self) -> &str {
        "filters.iqr"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut values = Vec::with_capacity(view.len() as usize);
        for idx in 0..view.len() {
            values.push(view.get_f64(idx, &self.dim));
        }
        if values.is_empty() {
            return Ok(vec![view.make_new()]);
        }

        let pc25 = percentile(values.clone(), 0.25);
        let pc75 = percentile(values, 0.75);
        let iqr = pc75 - pc25;
        let low = pc25 - self.multiplier * iqr;
        let high = pc75 + self.multiplier * iqr;

        let mut output = view.make_new();
        for idx in 0..view.len() {
            let value = view.get_f64(idx, &self.dim);
            if value > low && value < high {
                output.append_point(view, idx);
            }
        }
        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for IqrFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

fn percentile(mut values: Vec<f64>, percent: f64) -> f64 {
    let idx = (values.len() as f64 * percent) as usize;
    values.select_nth_unstable_by(idx, |left, right| left.total_cmp(right));
    values[idx]
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn drops_high_outlier() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for value in [1.0, 2.0, 3.0, 4.0, 100.0] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, value);
        }

        let mut filter = IqrFilter::new(1.5, DimId::X);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.len(), 4);
    }
}
