use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct MadFilter {
    multiplier: f64,
    dim: DimId,
    mad_multiplier: f64,
}

impl MadFilter {
    pub fn new(multiplier: f64, dim: DimId, mad_multiplier: f64) -> Self {
        Self {
            multiplier,
            dim,
            mad_multiplier,
        }
    }
}

impl Filter for MadFilter {
    fn name(&self) -> &str {
        "filters.mad"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut values = Vec::with_capacity(view.len() as usize);
        for idx in 0..view.len() {
            values.push(view.get_f64(idx, &self.dim));
        }
        if values.is_empty() {
            return Ok(vec![view.make_new()]);
        }

        let center = median(values.clone());
        for value in &mut values {
            *value = (*value - center).abs();
        }
        let mad = median(values) * self.mad_multiplier;

        let mut output = view.make_new();
        for idx in 0..view.len() {
            let value = (view.get_f64(idx, &self.dim) - center).abs();
            if value / mad < self.multiplier {
                output.append_point(view, idx);
            }
        }
        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for MadFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

fn median(mut values: Vec<f64>) -> f64 {
    let idx = values.len() / 2;
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

        let mut filter = MadFilter::new(2.0, DimId::X, 1.4862);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.len(), 4);
    }
}
