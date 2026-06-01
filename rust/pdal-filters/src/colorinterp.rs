//! `filters.colorinterp` -- assigns RGB colors based on a dimension and a ramp.

use crate::colorinterp_ramps;
use pdal_core::gdal::{self, Raster};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

/// Per-pixel Red/Green/Blue band values, in raster scan order.
type RgbBands = (Vec<f64>, Vec<f64>, Vec<f64>);

/// Decode an embedded ramp PNG into per-pixel Red/Green/Blue bands, in the
/// same raster scan order GDAL's `readBand` returns. The built-in ramps are
/// 8-bit RGB; RGBA is also accepted (the alpha channel is dropped).
fn decode_ramp_png(bytes: &[u8]) -> Result<RgbBands, String> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let channels = match info.color_type {
        png::ColorType::Rgb => 3usize,
        png::ColorType::Rgba => 4usize,
        other => return Err(format!("Unsupported ramp color type {other:?}.")),
    };
    if info.bit_depth != png::BitDepth::Eight {
        return Err("Color ramp must be 8-bit.".to_string());
    }
    let pixels = (info.width as usize) * (info.height as usize);
    let data = &buf[..info.buffer_size()];
    let mut red = Vec::with_capacity(pixels);
    let mut green = Vec::with_capacity(pixels);
    let mut blue = Vec::with_capacity(pixels);
    for i in 0..pixels {
        red.push(data[i * channels] as f64);
        green.push(data[i * channels + 1] as f64);
        blue.push(data[i * channels + 2] as f64);
    }
    Ok((red, green, blue))
}

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

/// The element at sorted index `n/2`, matching C++ `std::nth_element` at
/// `begin + size()/2` (the upper median for even counts).
fn nth_median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    let n = v.len();
    let (_, m, _) = v.select_nth_unstable_by(n / 2, f64::total_cmp);
    *m
}

/// Sample standard deviation (`n - 1` denominator), matching PDAL's
/// `stats::Summary::sampleStddev`.
fn sample_stddev(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let var = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n as f64 - 1.0);
    var.sqrt()
}

pub struct ColorinterpFilter {
    dim_name: String,
    ramp: String,
    min: f64,
    max: f64,
    clamp: bool,
    invert: bool,
    mad: bool,
    mad_multiplier: f64,
    k: f64,
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
            mad: false,
            mad_multiplier: 1.4862,
            k: 0.0,
            red_band: Vec::new(),
            green_band: Vec::new(),
            blue_band: Vec::new(),
        }
    }

    /// Configure the auto-bounds parameters used when `min`/`max` are left
    /// `NaN` (the `mad`, `mad_multiplier`, and `k` options of the C++ stage).
    pub fn with_bounds_params(mut self, mad: bool, mad_multiplier: f64, k: f64) -> Self {
        self.mad = mad;
        self.mad_multiplier = mad_multiplier;
        self.k = k;
        self
    }

    /// Resolve `min`/`max` from the view when the user left them unset.
    ///
    /// Mirrors the C++ `ColorinterpFilter::filter` bounds logic: a non-zero
    /// `k` drives median +/- threshold (threshold from MAD or sample stddev);
    /// otherwise any NaN bound falls back to the dimension min/max.
    fn resolve_bounds(&mut self, view: &PointView) {
        let n = view.len();
        if n == 0 {
            return;
        }
        let dim = DimId::from_name(&self.dim_name);
        let values: Vec<f64> = (0..n).map(|i| view.get_f64(i, &dim)).collect();

        if self.k != 0.0 {
            let median = nth_median(&values);
            let threshold = if self.mad {
                let deviations: Vec<f64> = values.iter().map(|v| (v - median).abs()).collect();
                nth_median(&deviations) * self.mad_multiplier * self.k
            } else {
                sample_stddev(&values) * self.k
            };
            self.min = median - threshold;
            self.max = median + threshold;
        } else if self.min.is_nan() || self.max.is_nan() {
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for &v in &values {
                lo = lo.min(v);
                hi = hi.max(v);
            }
            if self.min.is_nan() {
                self.min = lo;
            }
            if self.max.is_nan() {
                self.max = hi;
            }
        }
    }

    fn ensure_bands(&mut self) -> Result<(), StageError> {
        if !self.red_band.is_empty() {
            return Ok(());
        }

        // A named built-in ramp (e.g. the default `pestel_shades`) is decoded
        // directly from its embedded PNG; only file/`/vsimem` paths need GDAL.
        if let Some(png_bytes) = colorinterp_ramps::ramp_png(&self.ramp) {
            let (red, green, blue) = decode_ramp_png(png_bytes).map_err(StageError)?;
            self.red_band = red;
            self.green_band = green;
            self.blue_band = blue;
            return Ok(());
        }

        {
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

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        // The C++ stage registers Red/Green/Blue (uint16); the pipeline must
        // prepare them or the per-point set_f64 would silently no-op.
        vec![
            (DimId::Red, DimType::U16),
            (DimId::Green, DimType::U16),
            (DimId::Blue, DimType::U16),
        ]
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.ensure_bands()?;
        let mut output = input.clone();
        self.resolve_bounds(&output);
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
    use pdal_core::point::PointLayout;
    use std::rc::Rc;

    // A 4-pixel ramp whose Red band is [1,2,3,4] and Green/Blue are 0 --
    // mirrors `gdal/1234_red_0_green_0_blue.tif` used by the C++ tests.
    fn write_ramp4(path: &str) {
        gdal::register_drivers();
        let mut raster =
            Raster::create_float64(path, "GTiff", 4, 1, 3, [0.0, 1.0, 0.0, 1.0, 0.0, -1.0], "")
                .unwrap();
        raster
            .write_band_f64(1, 4, 1, &[1.0, 2.0, 3.0, 4.0], -9999.0, "Red")
            .unwrap();
        raster
            .write_band_f64(2, 4, 1, &[0.0, 0.0, 0.0, 0.0], -9999.0, "Green")
            .unwrap();
        raster
            .write_band_f64(3, 4, 1, &[0.0, 0.0, 0.0, 0.0], -9999.0, "Blue")
            .unwrap();
    }

    // Z = 0..=99, matching the C++ FauxReader ramp mode (count 100, [0,99]).
    fn ramp_view_0_99() -> PointView {
        let values: Vec<f64> = (0..100).map(|i| i as f64).collect();
        view(&values)
    }

    fn run_red(filter: &mut ColorinterpFilter) -> Vec<i64> {
        let out = filter.run_one(&ramp_view_0_99()).unwrap().remove(0);
        (0..out.len())
            .map(|i| out.get_f64(i, &DimId::Red) as i64)
            .collect()
    }

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
    fn decodes_named_builtin_ramp_to_256_rgb_pixels() {
        // The default `pestel_shades` ramp must resolve without a file path.
        let mut filter = ColorinterpFilter::new("Z", "pestel_shades", 0.0, 10.0, false, false);
        filter.ensure_bands().unwrap();
        assert_eq!(filter.red_band.len(), 256);
        assert_eq!(filter.green_band.len(), 256);
        assert_eq!(filter.blue_band.len(), 256);
        // Case-insensitive lookup, like the C++ name match.
        let mut upper = ColorinterpFilter::new("Z", "PESTEL_SHADES", 0.0, 10.0, false, false);
        upper.ensure_bands().unwrap();
        assert_eq!(upper.red_band, filter.red_band);
    }

    #[test]
    fn autorange_matches_cpp_expectations() {
        // Mirrors ColorinterpFilterTest.autorange: NaN bounds -> [min,max] of Z,
        // and Red == (z / 25) + 1.
        let temp = tempfile::NamedTempFile::with_suffix(".tif").unwrap();
        write_ramp4(temp.path().to_str().unwrap());
        let mut filter = ColorinterpFilter::new(
            "Z",
            temp.path().to_str().unwrap(),
            f64::NAN,
            f64::NAN,
            false,
            false,
        );
        let reds = run_red(&mut filter);
        for (z, &r) in reds.iter().enumerate() {
            assert_eq!(r, (z as i64 / 25) + 1, "z={z}");
        }
    }

    #[test]
    fn k_stddev_bounds_match_cpp_expectations() {
        // Mirrors ColorinterpFilterTest.k (k=1, sample stddev bounds).
        let temp = tempfile::NamedTempFile::with_suffix(".tif").unwrap();
        write_ramp4(temp.path().to_str().unwrap());
        let mut filter = ColorinterpFilter::new(
            "Z",
            temp.path().to_str().unwrap(),
            f64::NAN,
            f64::NAN,
            false,
            false,
        )
        .with_bounds_params(false, 1.4862, 1.0);
        let reds = run_red(&mut filter);
        for (z, &r) in reds.iter().enumerate() {
            let z = z as i64;
            let ok = (z < 22 && r == 0)
                || (z < 36 && r == 1)
                || (z < 51 && r == 2)
                || (z < 65 && r == 3)
                || (z < 80 && r == 4)
                || (z >= 80 && r == 0);
            assert!(ok, "z={z} r={r}");
        }
    }

    #[test]
    fn mad_bounds_match_cpp_expectations() {
        // Mirrors ColorinterpFilterTest.mad (mad=true, k=1).
        let temp = tempfile::NamedTempFile::with_suffix(".tif").unwrap();
        write_ramp4(temp.path().to_str().unwrap());
        let mut filter = ColorinterpFilter::new(
            "Z",
            temp.path().to_str().unwrap(),
            f64::NAN,
            f64::NAN,
            false,
            false,
        )
        .with_bounds_params(true, 1.4862, 1.0);
        let reds = run_red(&mut filter);
        for (z, &r) in reds.iter().enumerate() {
            let z = z as i64;
            let ok = (z < 13 && r == 0)
                || (z < 32 && r == 1)
                || (z < 50 && r == 2)
                || (z < 69 && r == 3)
                || (z < 88 && r == 4)
                || (z >= 88 && r == 0);
            assert!(ok, "z={z} r={r}");
        }
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
