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

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
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
    fn process_one(&mut self, _view: &pdal_core::point::PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}
