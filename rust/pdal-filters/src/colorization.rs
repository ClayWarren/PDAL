//! `filters.colorization` -- assignments colors from a GDAL-readable datasource.

use pdal_core::gdal::{self, Raster};
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct BandInfo {
    pub name: String,
    pub band: u32,
    pub scale: f64,
}

pub struct ColorizationFilter {
    raster_path: String,
    bands: Vec<BandInfo>,
    raster: Option<Raster>,
}

impl ColorizationFilter {
    pub fn new(raster_path: &str, bands: Vec<BandInfo>) -> Self {
        Self {
            raster_path: raster_path.to_string(),
            bands,
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

impl Filter for ColorizationFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.colorization"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_raster()?;
        let mut output = input.clone();
        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }
        Ok(vec![output])
    }
}

impl Streamable for ColorizationFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        if self.ensure_raster().is_err() {
            return true;
        }

        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);

        let mut data = vec![0.0; 16];
        if let Some(ref r) = self.raster {
            if r.read_at(x, y, &mut data).is_ok() {
                for band_info in &self.bands {
                    let val = data[(band_info.band - 1) as usize] * band_info.scale;
                    let dim = DimId::from_name(&band_info.name);
                    view.set_f64(idx, &dim, val);
                }
            }
        }
        true
    }

    fn reset(&mut self) {
        self.raster = None;
    }
}
