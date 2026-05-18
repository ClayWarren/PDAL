//! `filters.dem` -- filter points about an elevation surface.

use pdal_core::gdal::{self, Raster};
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct DEMFilter {
    dim_name: String,
    raster_path: String,
    band: i32,
    lower_bound: f64,
    upper_bound: f64,
    raster: Option<Raster>,
}

impl DEMFilter {
    pub fn new(
        dim_name: &str,
        raster_path: &str,
        band: i32,
        lower_bound: f64,
        upper_bound: f64,
    ) -> Self {
        Self {
            dim_name: dim_name.to_string(),
            raster_path: raster_path.to_string(),
            band,
            lower_bound,
            upper_bound,
            raster: None,
        }
    }

    fn ensure_raster(&mut self) -> Result<(), StageError> {
        if self.raster.is_none() {
            gdal::register_drivers();
            self.raster = Some(Raster::open(&self.raster_path).map_err(StageError)?);
        }
        Ok(())
    }
}

impl Filter for DEMFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.dem"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_raster()?;
        let mut output = input.make_new();
        for idx in 0..input.len() {
            if self.process_one(&mut input.clone(), idx) {
                // Hack to avoid mutable borrow on input
                output.append_point(input, idx);
            }
        }
        Ok(vec![output])
    }
}

impl Streamable for DEMFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        if self.ensure_raster().is_err() {
            return false;
        }

        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        let dim = DimId::from_name(&self.dim_name);
        let z = view.get_f64(idx, &dim);

        let mut data = vec![0.0; self.band as usize];
        if let Some(ref r) = self.raster {
            if r.read_at(x, y, &mut data).is_ok() {
                let v = data[self.band as usize - 1];
                let lb = v + self.lower_bound; // PDAL adds limits to raster value
                let ub = v + self.upper_bound;
                return z >= lb && z <= ub;
            }
        }
        false
    }

    fn reset(&mut self) {
        self.raster = None;
    }
}
