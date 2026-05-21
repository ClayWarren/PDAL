use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct SeparateScanLineFilter {
    pub group_by: u64,
}

impl SeparateScanLineFilter {
    pub fn new(group_by: u64) -> Self {
        SeparateScanLineFilter { group_by }
    }
}

impl Filter for SeparateScanLineFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.separatescanline"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let edge_dim = DimId::from_name("EdgeOfFlightLine");
        let mut results = Vec::new();
        let mut v = input.make_new();

        let mut line_num = 1;
        for i in 0..input.len() {
            v.append_point(input, i);
            if input.get_f64(i, &edge_dim) as u8 != 0 {
                line_num += 1;
                if line_num > self.group_by {
                    results.push(v);
                    v = input.make_new();
                    line_num = 1;
                }
            }
        }

        if !v.is_empty() {
            results.push(v);
        }

        Ok(results)
    }
}

impl Streamable for SeparateScanLineFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view(edge_flags: &[f64]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::from_name("EdgeOfFlightLine"), DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (idx, edge) in edge_flags.iter().enumerate() {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, idx as f64);
            view.set_f64(id, &DimId::from_name("EdgeOfFlightLine"), *edge);
        }
        view
    }

    #[test]
    fn groups_scan_lines_by_edge_markers() {
        let input = view(&[0.0, 1.0, 0.0, 1.0, 0.0]);
        let mut filter = SeparateScanLineFilter::new(2);

        let out = filter.run(std::slice::from_ref(&input)).unwrap();

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].len(), 4);
        assert_eq!(out[0].get_f64(0, &DimId::X), 0.0);
        assert_eq!(out[0].get_f64(3, &DimId::X), 3.0);
        assert_eq!(out[1].len(), 1);
        assert_eq!(out[1].get_f64(0, &DimId::X), 4.0);
    }

    #[test]
    fn keeps_trailing_partial_group_and_is_not_streamable() {
        let input = view(&[0.0, 1.0, 0.0]);
        let mut filter = SeparateScanLineFilter::new(3);

        let out = filter.run(std::slice::from_ref(&input)).unwrap();

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 3);

        let mut stream_view = view(&[0.0]);
        assert!(!filter.process_one(&mut stream_view, 0));
        filter.reset();
    }
}
