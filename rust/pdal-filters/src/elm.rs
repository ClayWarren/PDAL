use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::BTreeMap;

pub struct ElmFilter {
    cell: f64,
    class_label: u8,
    threshold: f64,
}

impl ElmFilter {
    pub fn new(cell: f64, class_label: u8, threshold: f64) -> Self {
        Self {
            cell,
            class_label,
            threshold,
        }
    }
}

impl Filter for ElmFilter {
    fn name(&self) -> &str {
        "filters.elm"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }
        if view.is_empty() {
            return Ok(vec![output]);
        }

        let bounds = bounds_2d(view);
        let rows = (((bounds.max_y - bounds.min_y) / self.cell) + 1.0) as u64;
        let mut cells: BTreeMap<u64, Vec<(f64, PointId)>> = BTreeMap::new();
        for id in 0..view.len() {
            let x = view.get_f64(id, &DimId::X);
            let y = view.get_f64(id, &DimId::Y);
            let z = view.get_f64(id, &DimId::Z);
            let c = ((x - bounds.min_x).floor() / self.cell) as u64;
            let r = ((y - bounds.min_y).floor() / self.cell) as u64;
            cells.entry(c * rows + r).or_default().push((z, id));
        }

        for ids in cells.values_mut() {
            if ids.len() <= 1 {
                continue;
            }
            ids.sort_by(|left, right| {
                left.0
                    .total_cmp(&right.0)
                    .then_with(|| left.1.cmp(&right.1))
            });
            for idx in 0..ids.len() - 1 {
                if (ids[idx].0 - ids[idx + 1].0).abs() < self.threshold {
                    break;
                }
                output.set_f64(ids[idx].1, &DimId::Classification, self.class_label as f64);
            }
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for ElmFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

struct Bounds2d {
    min_x: f64,
    min_y: f64,
    max_y: f64,
}

fn bounds_2d(view: &PointView) -> Bounds2d {
    let mut bounds = Bounds2d {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_y: f64::NEG_INFINITY,
    };
    for id in 0..view.len() {
        bounds.min_x = bounds.min_x.min(view.get_f64(id, &DimId::X));
        bounds.min_y = bounds.min_y.min(view.get_f64(id, &DimId::Y));
        bounds.max_y = bounds.max_y.max(view.get_f64(id, &DimId::Y));
    }
    bounds
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn marks_isolated_low_points_in_cell() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for z in [-10.0, 0.0, 0.2] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, 0.0);
            view.set_f64(idx, &DimId::Y, 0.0);
            view.set_f64(idx, &DimId::Z, z);
        }

        let mut filter = ElmFilter::new(10.0, 7, 1.0);
        let out = filter.run(std::slice::from_ref(&view)).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::Classification), 7.0);
        assert_eq!(out.get_f64(1, &DimId::Classification), 0.0);
    }
}
