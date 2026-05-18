//! `filters.colorinterp` -- assigns RGB colors based on a dimension and a ramp.

use pdal_core::gdal::{self, Raster};
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct ColorinterpFilter {
    dim_name: String,
    ramp: String,
    min: f64,
    max: f64,
    clamp: bool,
    invert: bool,
    red_band: Vec<f64>,
    green_band: Vec<f64>,
    blue_band: Vec<f64>,
}

impl ColorinterpFilter {
    pub fn new(dim_name: &str, ramp: &str, min: f64, max: f64, clamp: bool, invert: bool) -> Self {
        Self {
            dim_name: dim_name.to_string(),
            ramp: ramp.to_string(),
            min,
            max,
            clamp,
            invert,
            red_band: Vec::new(),
            green_band: Vec::new(),
            blue_band: Vec::new(),
        }
    }

    fn ensure_bands(&mut self) -> Result<(), StageError> {
        if self.red_band.is_empty() {
            gdal::register_drivers();
            let raster = Raster::open(&self.ramp).map_err(StageError)?;
            let width = 256;
            let height = 1;

            self.red_band = vec![0.0; width * height];
            self.green_band = vec![0.0; width * height];
            self.blue_band = vec![0.0; width * height];

            raster
                .read_band(1, width, height, &mut self.red_band)
                .map_err(StageError)?;
            raster
                .read_band(2, width, height, &mut self.green_band)
                .map_err(StageError)?;
            raster
                .read_band(3, width, height, &mut self.blue_band)
                .map_err(StageError)?;
        }
        Ok(())
    }
}

impl Filter for ColorinterpFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.colorinterp"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_bands()?;
        let mut output = input.clone();
        for idx in 0..output.len() {
            self.process_one(&mut output, idx);
        }
        Ok(vec![output])
    }
}

impl Streamable for ColorinterpFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        if self.ensure_bands().is_err() {
            return false;
        }

        let dim = DimId::from_name(&self.dim_name);
        let mut v = view.get_f64(idx, &dim);

        if self.clamp {
            v = v.clamp(self.min, self.max);
        }

        if v < self.min || v > self.max {
            return true;
        }

        let factor = (v - self.min) / (self.max - self.min);
        let img_width = self.red_band.len();
        let mut position = (factor * (img_width as f64 - 1.0)).floor() as usize;
        position = position.min(img_width - 1);

        if self.invert {
            position = (img_width - 1) - position;
        }

        view.set_f64(
            idx,
            &DimId::Other("Red".to_string()),
            self.red_band[position],
        );
        view.set_f64(
            idx,
            &DimId::Other("Green".to_string()),
            self.green_band[position],
        );
        view.set_f64(
            idx,
            &DimId::Other("Blue".to_string()),
            self.blue_band[position],
        );

        true
    }

    fn reset(&mut self) {
        self.red_band.clear();
        self.green_band.clear();
        self.blue_band.clear();
    }
}
