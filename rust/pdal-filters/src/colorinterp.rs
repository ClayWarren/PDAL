//! `filters.colorinterp` -- assigns RGB colors based on a dimension and a ramp.

use pdal_core::gdal::{self, Raster};
use pdal_core::point::{DimId, PointLayout, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub fn validate_prepared(
    dim_name: &str,
    min: f64,
    max: f64,
    layout: &PointLayout,
) -> Result<(), String> {
    let dim = DimId::from_name(dim_name);
    if layout.dim(&dim).is_none() {
        return Err(format!("Dimension '{dim_name}' does not exist."));
    }
    if !min.is_nan() && !max.is_nan() && max <= min {
        return Err("Specified 'minimum' value must be less than 'maximum' value.".to_string());
    }
    Ok(())
}

pub fn pipeline_streamable(min: f64, max: f64) -> bool {
    !min.is_nan() && !max.is_nan()
}

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
            let width = raster.width().max(0) as usize;
            let height = raster.height().max(0) as usize;
            if width == 0 || height == 0 {
                return Err(StageError("Color ramp has no pixels.".to_string()));
            }

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

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
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
        let mut position = (factor * img_width as f64).floor() as usize;
        position = position.min(img_width - 1);

        if self.invert {
            position = (img_width - 1) - position;
        }

        view.set_f64(idx, &DimId::Red, self.red_band[position]);
        view.set_f64(idx, &DimId::Green, self.green_band[position]);
        view.set_f64(idx, &DimId::Blue, self.blue_band[position]);

        true
    }

    fn reset(&mut self) {
        self.red_band.clear();
        self.green_band.clear();
        self.blue_band.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::gdal::Raster;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn write_ramp(path: &str) {
        gdal::register_drivers();
        let mut raster =
            Raster::create_float64(path, "GTiff", 2, 1, 3, [0.0, 1.0, 0.0, 1.0, 0.0, -1.0], "")
                .unwrap();
        raster
            .write_band_f64(1, 2, 1, &[10.0, 20.0], -9999.0, "Red")
            .unwrap();
        raster
            .write_band_f64(2, 2, 1, &[30.0, 40.0], -9999.0, "Green")
            .unwrap();
        raster
            .write_band_f64(3, 2, 1, &[50.0, 60.0], -9999.0, "Blue")
            .unwrap();
    }

    fn view(values: &[f64]) -> PointView {
        let mut layout = PointLayout::new();
        for dim in [DimId::Z, DimId::Red, DimId::Green, DimId::Blue] {
            layout.register(dim, DimType::F64);
        }
        let mut view = PointView::new(Rc::new(layout));
        for value in values {
            let id = view.add_point();
            view.set_f64(id, &DimId::Z, *value);
        }
        view
    }

    #[test]
    fn assigns_rgb_from_ramp_and_supports_clamp_invert_reset() {
        let temp = tempfile::NamedTempFile::with_suffix(".tif").unwrap();
        write_ramp(temp.path().to_str().unwrap());
        let mut filter =
            ColorinterpFilter::new("Z", temp.path().to_str().unwrap(), 0.0, 10.0, true, true);

        let output = filter.run_one(&view(&[-10.0, 10.0])).unwrap().remove(0);

        assert_eq!(output.get_f64(0, &DimId::Red), 20.0);
        assert_eq!(output.get_f64(0, &DimId::Green), 40.0);
        assert_eq!(output.get_f64(0, &DimId::Blue), 60.0);
        assert_eq!(output.get_f64(1, &DimId::Red), 10.0);
        filter.reset();
    }

    #[test]
    fn rejects_missing_dimension_and_invalid_bounds() {
        let layout = PointLayout::new();
        assert!(validate_prepared("Z", 0.0, 0.0, &layout)
            .unwrap_err()
            .contains("does not exist"));

        let mut layout = PointLayout::new();
        layout.register(DimId::Z, pdal_core::point::DimType::F64);
        assert!(validate_prepared("Z", 1.0, 1.0, &layout)
            .unwrap_err()
            .contains("minimum"));
    }

    #[test]
    fn requires_finite_bounds_for_streaming() {
        assert!(!pipeline_streamable(0.0, f64::NAN));
        assert!(pipeline_streamable(0.0, 1.0));
    }

    #[test]
    fn leaves_out_of_range_points_unchanged_without_clamp() {
        let temp = tempfile::NamedTempFile::with_suffix(".tif").unwrap();
        write_ramp(temp.path().to_str().unwrap());
        let mut filter =
            ColorinterpFilter::new("Z", temp.path().to_str().unwrap(), 0.0, 10.0, false, false);
        let mut input = view(&[50.0]);

        assert!(filter.process_one(&mut input, 0));
        assert_eq!(input.get_f64(0, &DimId::Red), 0.0);
    }
}
