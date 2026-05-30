//! `filters.hag_dem` -- compute height above ground using a DEM raster.

use pdal_core::gdal::{self, Raster};
use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct HagDemFilter {
    raster_path: String,
    band: i32,
    zero_ground: bool,
    min_clamp: f64,
    max_clamp: f64,
    nodata_height: f64,
    ground_class: u8,
    raster: Option<Raster>,
}

impl HagDemFilter {
    pub fn new(
        raster_path: &str,
        band: i32,
        zero_ground: bool,
        min_clamp: f64,
        max_clamp: f64,
        nodata_height: f64,
        ground_class: u8,
    ) -> Self {
        Self {
            raster_path: raster_path.to_string(),
            band,
            zero_ground,
            min_clamp,
            max_clamp,
            nodata_height,
            ground_class,
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

impl Filter for HagDemFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.hag_dem"
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::HeightAboveGround, DimType::F64)]
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_raster()?;
        let mut output = input.clone();
        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }
        Ok(vec![output])
    }
}

impl Streamable for HagDemFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        if self.zero_ground {
            let class = view.get_f64(idx, &DimId::Classification) as u8;
            if class == self.ground_class {
                view.set_f64(idx, &DimId::HeightAboveGround, 0.0);
                return true;
            }
        }

        if self.ensure_raster().is_err() {
            return true;
        }

        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        let z = view.get_f64(idx, &DimId::Z);

        let mut data = vec![0.0; self.band as usize];
        if let Some(ref r) = self.raster {
            if r.read_at(x, y, &mut data).is_ok() {
                let val = data[self.band as usize - 1];
                let mut hag = z - val;
                hag = hag.clamp(self.min_clamp, self.max_clamp);
                view.set_f64(idx, &DimId::HeightAboveGround, hag);
            } else {
                view.set_f64(idx, &DimId::HeightAboveGround, self.nodata_height);
            }
        }
        true
    }

    fn reset(&mut self) {
        self.raster = None;
    }
}
