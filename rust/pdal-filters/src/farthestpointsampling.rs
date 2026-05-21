use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct FarthestPointSamplingFilter {
    pub count: u64,
}

impl FarthestPointSamplingFilter {
    pub fn new(count: u64) -> Self {
        Self { count }
    }
}

impl Filter for FarthestPointSamplingFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.farthestpointsampling"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let size = view.len();
        if size == 0 {
            return Ok(Vec::new());
        }

        if size < self.count {
            let mut out = PointView::new(view.layout().clone());
            for i in 0..size {
                out.append_point(view, i);
            }
            return Ok(vec![out]);
        }

        let mut ids = vec![0; self.count as usize];
        ids[0] = 0;

        let mut min_dists = vec![0.0; size as usize];

        let x0 = view.get_f64(0, &DimId::X);
        let y0 = view.get_f64(0, &DimId::Y);
        let z0 = view.get_f64(0, &DimId::Z);

        for j in 0..size {
            let xj = view.get_f64(j, &DimId::X);
            let yj = view.get_f64(j, &DimId::Y);
            let zj = view.get_f64(j, &DimId::Z);
            let dx = xj - x0;
            let dy = yj - y0;
            let dz = zj - z0;
            min_dists[j as usize] = dx * dx + dy * dy + dz * dz;
        }

        for id in ids.iter_mut().skip(1) {
            let mut max_idx = 0;
            let mut max_val = -1.0;
            for (j, &dist) in min_dists.iter().enumerate() {
                if dist > max_val {
                    max_val = dist;
                    max_idx = j;
                }
            }
            *id = max_idx as u64;

            let xi = view.get_f64(max_idx as u64, &DimId::X);
            let yi = view.get_f64(max_idx as u64, &DimId::Y);
            let zi = view.get_f64(max_idx as u64, &DimId::Z);

            for j in 0..size {
                let xj = view.get_f64(j, &DimId::X);
                let yj = view.get_f64(j, &DimId::Y);
                let zj = view.get_f64(j, &DimId::Z);
                let dx = xj - xi;
                let dy = yj - yi;
                let dz = zj - zi;
                let d2 = dx * dx + dy * dy + dz * dz;
                if d2 < min_dists[j as usize] {
                    min_dists[j as usize] = d2;
                }
            }
        }

        let mut out_view = PointView::new(view.layout().clone());
        for id in ids {
            out_view.append_point(view, id);
        }

        Ok(vec![out_view])
    }
}

impl Streamable for FarthestPointSamplingFilter {
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

    fn view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Y, *y);
            view.set_f64(idx, &DimId::Z, *z);
        }
        view
    }

    #[test]
    fn empty_input_produces_no_views() {
        let mut filter = FarthestPointSamplingFilter::new(2);
        assert!(filter
            .run(std::slice::from_ref(&view(&[])))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn count_larger_than_input_keeps_every_point() {
        let input = view(&[(1.0, 0.0, 0.0), (2.0, 0.0, 0.0)]);
        let mut filter = FarthestPointSamplingFilter::new(5);

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);
        assert_eq!(out.len(), 2);
        assert_eq!(out.get_f64(0, &DimId::X), 1.0);
        assert_eq!(out.get_f64(1, &DimId::X), 2.0);
    }

    #[test]
    fn selects_iteratively_farthest_points() {
        let input = view(&[
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (10.0, 0.0, 0.0),
            (5.0, 0.0, 0.0),
        ]);
        let mut filter = FarthestPointSamplingFilter::new(3);

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);
        assert_eq!(out.len(), 3);
        assert_eq!(out.get_f64(0, &DimId::X), 0.0);
        assert_eq!(out.get_f64(1, &DimId::X), 10.0);
        assert_eq!(out.get_f64(2, &DimId::X), 5.0);
    }

    #[test]
    fn streaming_is_not_supported() {
        let mut filter = FarthestPointSamplingFilter::new(1);
        let mut input = view(&[(0.0, 0.0, 0.0)]);

        assert!(!filter.process_one(&mut input, 0));
    }
}
