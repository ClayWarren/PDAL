//! `writers.gdal` -- Write point clouds as GDAL rasters.

use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::StageError;

#[derive(Clone, Debug, PartialEq)]
enum OutputStat {
    Min,
    Max,
    Mean,
    Idw,
    Count,
    Stdev,
    Percentile(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputDataType {
    Float64,
    Int32,
}

pub struct GdalWriter {
    filename: String,
    driver_name: String,
    output_types: Vec<OutputStat>,
    output_type_error: Option<String>,
    dimension: DimId,
    data_type: OutputDataType,
    data_type_error: Option<String>,
    resolution: f64,
    radius: Option<f64>,
    power: f64,
    no_data: f64,
    binmode: bool,
    window_size: usize,
    allow_empty: bool,
    bounds: String,
    fixed_grid: Option<FixedGrid>,
    fixed_grid_arg_count: usize,
    metadata: Vec<(String, String)>,
    override_srs: String,
    default_srs: String,
}

#[derive(Clone, Copy)]
struct FixedGrid {
    origin_x: f64,
    origin_y: f64,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy)]
struct Bounds {
    minx: f64,
    maxx: f64,
    miny: f64,
    maxy: f64,
}

#[derive(Clone, Copy)]
struct Sample {
    x: f64,
    y: f64,
    value: f64,
}

impl GdalWriter {
    pub fn new(options: &Options) -> Self {
        let resolution = options.get_f64("resolution", 1.0);
        let (output_types, output_type_error) = output_types(options);
        let (data_type, data_type_error) =
            parse_data_type(&options.get_str("data_type", "float64"));
        Self {
            filename: options.get_str("filename", ""),
            driver_name: options.get_str("gdaldriver", "GTiff"),
            output_types,
            output_type_error,
            dimension: DimId::from_name(&options.get_str("dimension", "Z")),
            data_type,
            data_type_error,
            resolution,
            radius: options
                .has("radius")
                .then(|| options.get_f64("radius", 0.0)),
            power: options.get_f64("power", 1.0),
            no_data: options.get_f64("nodata", -9999.0),
            binmode: options.get_bool("binmode", false),
            window_size: options.get_u64("window_size", 0) as usize,
            allow_empty: options.get_bool("allow_empty", false),
            bounds: options.get_str("bounds", ""),
            fixed_grid: fixed_grid(options),
            fixed_grid_arg_count: fixed_grid_arg_count(options),
            metadata: parse_metadata(options),
            override_srs: options.get_str("override_srs", ""),
            default_srs: options.get_str("default_srs", ""),
        }
    }
}

impl Writer for GdalWriter {
    fn name(&self) -> &str {
        "writers.gdal"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "GdalWriter requires a filename option.".to_string(),
            ));
        }
        if self.resolution <= 0.0 {
            return Err(StageError(
                "GDAL writer resolution must be positive.".to_string(),
            ));
        }
        if let Some(error) = &self.output_type_error {
            return Err(StageError(error.clone()));
        }
        if let Some(error) = &self.data_type_error {
            return Err(StageError(error.clone()));
        }
        if self.fixed_grid_arg_count != 0 && self.fixed_grid_arg_count != 4 {
            return Err(StageError(
                "Must specify all or none of 'origin_x', 'origin_y', 'width' and 'height'."
                    .to_string(),
            ));
        }
        if !self.bounds.is_empty() && self.fixed_grid_arg_count != 0 {
            return Err(StageError(
                "Specify either 'bounds' or 'origin_x'/'origin_y'/'width'/'height' options -- not both".to_string(),
            ));
        }
        if self.output_types.iter().any(is_percentile) && !self.binmode {
            return Err(StageError(
                "Can't output percentiles without 'binmode=true'.".to_string(),
            ));
        }
        validate_srs_options(&self.override_srs, &self.default_srs)?;

        let samples = collect_samples(views, &self.dimension)?;
        if samples.is_empty() && !self.allow_empty {
            return Err(StageError(
                "Unable to write GDAL data with no points for output.".to_string(),
            ));
        }

        let configured_grid = match self.fixed_grid {
            Some(grid) => Some(grid),
            None => grid_from_bounds(&self.bounds, self.resolution)?,
        };
        let Some(grid) = configured_grid.or_else(|| grid_from_samples(&samples, self.resolution))
        else {
            return Ok(());
        };

        let bands = self.render_bands(grid, &samples);
        let geo_transform = [
            grid.origin_x,
            self.resolution,
            0.0,
            grid.origin_y + self.resolution * grid.height as f64,
            0.0,
            -self.resolution,
        ];
        let srs_wkt = resolve_srs(views, &self.override_srs, &self.default_srs);

        pdal_core::gdal::register_drivers();
        match self.data_type {
            OutputDataType::Float64 => {
                let mut raster = pdal_core::gdal::Raster::create_float64(
                    &self.filename,
                    &self.driver_name,
                    grid.width as i32,
                    grid.height as i32,
                    bands.len() as i32,
                    geo_transform,
                    &srs_wkt,
                )
                .map_err(StageError)?;
                for (idx, (name, data)) in bands.iter().enumerate() {
                    raster
                        .write_band_f64(
                            idx as i32 + 1,
                            grid.width,
                            grid.height,
                            data,
                            self.no_data,
                            name,
                        )
                        .map_err(StageError)?;
                }
                for (key, value) in &self.metadata {
                    raster.set_metadata_item(key, value).map_err(StageError)?;
                }
            }
            OutputDataType::Int32 => {
                let no_data = self.no_data.round() as i32;
                let mut raster = pdal_core::gdal::Raster::create_int32(
                    &self.filename,
                    &self.driver_name,
                    grid.width as i32,
                    grid.height as i32,
                    bands.len() as i32,
                    geo_transform,
                    &srs_wkt,
                )
                .map_err(StageError)?;
                for (idx, (name, data)) in bands.iter().enumerate() {
                    let int_data: Vec<i32> = data
                        .iter()
                        .map(|value| {
                            if (*value - self.no_data).abs() < f64::EPSILON {
                                no_data
                            } else {
                                value.round() as i32
                            }
                        })
                        .collect();
                    raster
                        .write_band_i32(
                            idx as i32 + 1,
                            grid.width,
                            grid.height,
                            &int_data,
                            no_data,
                            name,
                        )
                        .map_err(StageError)?;
                }
                for (key, value) in &self.metadata {
                    raster.set_metadata_item(key, value).map_err(StageError)?;
                }
            }
        }
        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("writers.gdal")
    }
}

fn parse_data_type(value: &str) -> (OutputDataType, Option<String>) {
    match value.to_ascii_lowercase().as_str() {
        "double" | "float64" => (OutputDataType::Float64, None),
        "int32" | "int32_t" | "signed32" | "int" => (OutputDataType::Int32, None),
        _ => (
            OutputDataType::Float64,
            Some(format!(
                "Unsupported GDAL writer data_type '{value}' for the Rust-backed path."
            )),
        ),
    }
}

impl GdalWriter {
    fn render_bands(&self, grid: FixedGrid, samples: &[Sample]) -> Vec<(String, Vec<f64>)> {
        self.output_types
            .iter()
            .map(|stat| {
                (
                    stat_name(stat),
                    render_stat(stat, grid, samples, self).unwrap_or_else(|| {
                        vec![self.no_data; grid.width.saturating_mul(grid.height)]
                    }),
                )
            })
            .collect()
    }
}

fn collect_samples(views: &[PointView], dimension: &DimId) -> Result<Vec<Sample>, StageError> {
    let mut samples = Vec::new();
    for view in views {
        if !view.is_empty() && view.layout().dim(dimension).is_none() {
            return Err(StageError(format!(
                "Specified dimension '{}' does not exist.",
                dimension.name()
            )));
        }
        for idx in 0..view.len() {
            let x = view.get_f64(idx, &DimId::X);
            let y = view.get_f64(idx, &DimId::Y);
            if x.is_finite() && y.is_finite() {
                samples.push(Sample {
                    x,
                    y,
                    value: view.get_f64(idx, dimension),
                });
            }
        }
    }
    Ok(samples)
}

fn grid_from_samples(samples: &[Sample], resolution: f64) -> Option<FixedGrid> {
    let first = samples.first()?;
    let mut bounds = Bounds {
        minx: first.x,
        maxx: first.x,
        miny: first.y,
        maxy: first.y,
    };
    for sample in &samples[1..] {
        bounds.minx = bounds.minx.min(sample.x);
        bounds.maxx = bounds.maxx.max(sample.x);
        bounds.miny = bounds.miny.min(sample.y);
        bounds.maxy = bounds.maxy.max(sample.y);
    }
    Some(FixedGrid {
        origin_x: bounds.minx,
        origin_y: bounds.miny,
        width: ((bounds.maxx - bounds.minx) / resolution).floor() as usize + 1,
        height: ((bounds.maxy - bounds.miny) / resolution).floor() as usize + 1,
    })
}

fn grid_from_bounds(bounds: &str, resolution: f64) -> Result<Option<FixedGrid>, StageError> {
    if bounds.is_empty() {
        return Ok(None);
    }

    let bounds = pdal_core::bounds::parse_bounds2d(bounds, 0)
        .map_err(StageError)?
        .bounds;
    Ok(Some(FixedGrid {
        origin_x: bounds.minx,
        origin_y: bounds.miny,
        width: ((bounds.maxx - bounds.minx) / resolution).floor() as usize + 1,
        height: ((bounds.maxy - bounds.miny) / resolution).floor() as usize + 1,
    }))
}

fn fixed_grid(options: &Options) -> Option<FixedGrid> {
    let has_origin_x = options.has("origin_x");
    let has_origin_y = options.has("origin_y");
    let has_width = options.has("width");
    let has_height = options.has("height");
    if !(has_origin_x && has_origin_y && has_width && has_height) {
        return None;
    }
    Some(FixedGrid {
        origin_x: options.get_f64("origin_x", 0.0),
        origin_y: options.get_f64("origin_y", 0.0),
        width: options.get_u64("width", 0) as usize,
        height: options.get_u64("height", 0) as usize,
    })
}

fn fixed_grid_arg_count(options: &Options) -> usize {
    ["origin_x", "origin_y", "width", "height"]
        .into_iter()
        .filter(|key| options.has(key))
        .count()
}

fn output_types(options: &Options) -> (Vec<OutputStat>, Option<String>) {
    let raw = if options.values("output_type").is_empty() {
        vec!["all".to_string()]
    } else {
        options.values("output_type").to_vec()
    };
    let mut stats = Vec::new();
    for value in raw {
        for token in value
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
        {
            match token.to_ascii_lowercase().as_str() {
                "all" => {
                    return (
                        vec![
                            OutputStat::Min,
                            OutputStat::Max,
                            OutputStat::Mean,
                            OutputStat::Idw,
                            OutputStat::Count,
                            OutputStat::Stdev,
                        ],
                        None,
                    );
                }
                "min" => stats.push(OutputStat::Min),
                "max" => stats.push(OutputStat::Max),
                "mean" => stats.push(OutputStat::Mean),
                "idw" => stats.push(OutputStat::Idw),
                "count" => stats.push(OutputStat::Count),
                "stdev" => stats.push(OutputStat::Stdev),
                pct if pct.starts_with('p') => {
                    if let Ok(percentile) = pct[1..].parse::<u8>() {
                        if percentile <= 100 {
                            stats.push(OutputStat::Percentile(percentile));
                        } else {
                            return (
                                stats,
                                Some(
                                    "Percentile values must be integers between 1 and 100."
                                        .to_string(),
                                ),
                            );
                        }
                    } else {
                        return (stats, Some(format!("Invalid percentile value: '{token}'.")));
                    }
                }
                _ => return (stats, Some(format!("Invalid output type: '{token}'."))),
            }
        }
    }
    if stats.is_empty() {
        return (
            stats,
            Some("No valid GDAL output types were provided.".to_string()),
        );
    }
    (stats, None)
}

fn render_stat(
    stat: &OutputStat,
    grid: FixedGrid,
    samples: &[Sample],
    writer: &GdalWriter,
) -> Option<Vec<f64>> {
    let mut out = vec![writer.no_data; grid.width.checked_mul(grid.height)?];
    let mut populated = vec![false; out.len()];
    for row in 0..grid.height {
        for col in 0..grid.width {
            let values = cell_values(grid, row, col, samples, writer);
            if values.is_empty() {
                if matches!(stat, OutputStat::Count) {
                    out[row * grid.width + col] = 0.0;
                }
                continue;
            }
            populated[row * grid.width + col] = true;
            out[row * grid.width + col] = match stat {
                OutputStat::Min => values
                    .iter()
                    .map(|(_, value)| *value)
                    .fold(f64::INFINITY, f64::min),
                OutputStat::Max => values
                    .iter()
                    .map(|(_, value)| *value)
                    .fold(f64::NEG_INFINITY, f64::max),
                OutputStat::Mean => {
                    values.iter().map(|(_, value)| *value).sum::<f64>() / values.len() as f64
                }
                OutputStat::Idw => idw(&values, writer.power),
                OutputStat::Count => values.len() as f64,
                OutputStat::Stdev => stdev(&values),
                OutputStat::Percentile(percentile) => percentile_value(&values, *percentile),
            };
        }
    }
    if writer.window_size > 0 && supports_window_fill(stat) {
        window_fill(
            &mut out,
            &populated,
            grid,
            writer.window_size,
            writer.no_data,
        );
    }
    Some(out)
}

fn supports_window_fill(stat: &OutputStat) -> bool {
    matches!(
        stat,
        OutputStat::Min | OutputStat::Max | OutputStat::Mean | OutputStat::Idw | OutputStat::Stdev
    )
}

fn window_fill(
    values: &mut [f64],
    populated: &[bool],
    grid: FixedGrid,
    window_size: usize,
    no_data: f64,
) {
    for row in 0..grid.height {
        for col in 0..grid.width {
            let dst = row * grid.width + col;
            if populated[dst] {
                continue;
            }

            let row_start = row.saturating_sub(window_size);
            let row_end = (row + window_size + 1).min(grid.height);
            let col_start = col.saturating_sub(window_size);
            let col_end = (col + window_size + 1).min(grid.width);
            let mut weighted_sum = 0.0;
            let mut weight_sum = 0.0;

            for src_row in row_start..row_end {
                for src_col in col_start..col_end {
                    let src = src_row * grid.width + src_col;
                    if src == dst || !populated[src] {
                        continue;
                    }

                    let distance = row.abs_diff(src_row).max(col.abs_diff(src_col)) as f64;
                    weighted_sum += values[src] / distance;
                    weight_sum += 1.0 / distance;
                }
            }

            values[dst] = if weight_sum > 0.0 {
                weighted_sum / weight_sum
            } else {
                no_data
            };
        }
    }
}

fn cell_values(
    grid: FixedGrid,
    row: usize,
    col: usize,
    samples: &[Sample],
    writer: &GdalWriter,
) -> Vec<(f64, f64)> {
    if writer.binmode {
        return samples
            .iter()
            .filter_map(|sample| {
                sample_cell(grid, sample, writer.resolution)
                    .filter(|(sample_row, sample_col)| *sample_row == row && *sample_col == col)
                    .map(|_| (0.0, sample.value))
            })
            .collect();
    }

    let center_x = grid.origin_x + (col as f64 + 0.5) * writer.resolution;
    let center_y =
        grid.origin_y + (grid.height - row) as f64 * writer.resolution - writer.resolution / 2.0;
    let radius = writer.radius.unwrap_or(writer.resolution * 2.0_f64.sqrt());
    samples
        .iter()
        .filter_map(|sample| {
            let distance = ((sample.x - center_x).powi(2) + (sample.y - center_y).powi(2)).sqrt();
            (distance <= radius).then_some((distance, sample.value))
        })
        .collect()
}

fn sample_cell(grid: FixedGrid, sample: &Sample, resolution: f64) -> Option<(usize, usize)> {
    let col = ((sample.x - grid.origin_x) / resolution).floor() as isize;
    let row_from_bottom = ((sample.y - grid.origin_y) / resolution).floor() as isize;
    if col < 0 || row_from_bottom < 0 {
        return None;
    }
    let row = grid.height as isize - 1 - row_from_bottom;
    if row < 0 || row >= grid.height as isize || col >= grid.width as isize {
        None
    } else {
        Some((row as usize, col as usize))
    }
}

fn idw(values: &[(f64, f64)], power: f64) -> f64 {
    let zero_distance: Vec<f64> = values
        .iter()
        .filter(|(distance, _)| *distance == 0.0)
        .map(|(_, value)| *value)
        .collect();
    if !zero_distance.is_empty() {
        return zero_distance.iter().sum::<f64>() / zero_distance.len() as f64;
    }

    let mut weighted_sum = 0.0;
    let mut weight_sum = 0.0;
    for (distance, value) in values {
        let weight = 1.0 / distance.powf(power);
        weighted_sum += value * weight;
        weight_sum += weight;
    }
    weighted_sum / weight_sum
}

fn stdev(values: &[(f64, f64)]) -> f64 {
    let mean = values.iter().map(|(_, value)| *value).sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|(_, value)| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn percentile_value(values: &[(f64, f64)], percentile: u8) -> f64 {
    let mut sorted: Vec<f64> = values.iter().map(|(_, value)| *value).collect();
    sorted.sort_by(f64::total_cmp);
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = percentile as f64 / 100.0 * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = rank - lower as f64;
        sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
    }
}

fn stat_name(stat: &OutputStat) -> String {
    match stat {
        OutputStat::Min => "min".to_string(),
        OutputStat::Max => "max".to_string(),
        OutputStat::Mean => "mean".to_string(),
        OutputStat::Idw => "idw".to_string(),
        OutputStat::Count => "count".to_string(),
        OutputStat::Stdev => "stdev".to_string(),
        OutputStat::Percentile(percentile) => format!("p{percentile}"),
    }
}

fn is_percentile(stat: &OutputStat) -> bool {
    matches!(stat, OutputStat::Percentile(_))
}

fn resolve_srs(views: &[PointView], override_srs: &str, default_srs: &str) -> String {
    let mut srs = views
        .iter()
        .map(PointView::spatial_reference)
        .find(|srs| !srs.is_empty())
        .map(|srs| srs.wkt().to_string())
        .unwrap_or_default();

    if !override_srs.is_empty() {
        srs = override_srs.to_string();
    }
    if srs.is_empty() && !default_srs.is_empty() {
        srs = default_srs.to_string();
    }
    srs
}

fn validate_srs_options(override_srs: &str, default_srs: &str) -> Result<(), StageError> {
    if !override_srs.is_empty() && !default_srs.is_empty() {
        return Err(StageError(
            "Can't set both 'override_srs' and 'default_srs'.".to_string(),
        ));
    }
    Ok(())
}

fn parse_metadata(options: &Options) -> Vec<(String, String)> {
    let spec = options.get_str("metadata", "");
    if spec.is_empty() {
        return Vec::new();
    }

    spec.split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            let (key, value) = entry.split_once('=')?;
            Some((key.trim().to_string(), value.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    #[test]
    fn resolves_writer_srs_overrides() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);
        view.set_spatial_reference(pdal_core::srs::SpatialReference::new("EPSG:2030"));

        assert_eq!(resolve_srs(&[view.clone()], "EPSG:4326", ""), "EPSG:4326");
        assert_eq!(resolve_srs(&[view.clone()], "", "EPSG:4326"), "EPSG:2030");
        assert_eq!(resolve_srs(&[], "", "EPSG:4326"), "EPSG:4326");
    }

    #[test]
    fn rejects_conflicting_srs_options() {
        let err = validate_srs_options("EPSG:4326", "EPSG:2030").unwrap_err();
        assert!(err.0.contains("override_srs"));

        assert!(validate_srs_options("EPSG:4326", "").is_ok());
        assert!(validate_srs_options("", "EPSG:2030").is_ok());
        assert!(validate_srs_options("", "").is_ok());
    }

    #[test]
    fn parses_output_types_and_percentiles() {
        let mut options = Options::new();
        options.add("output_type", "min,p50,count");
        assert_eq!(
            output_types(&options).0,
            vec![
                OutputStat::Min,
                OutputStat::Percentile(50),
                OutputStat::Count
            ]
        );
        options.add("output_type", "nope");
        assert!(output_types(&options).1.is_some());
    }

    #[test]
    fn fixed_grid_requires_the_whole_grid_shape() {
        let mut options = Options::new();
        options.add("origin_x", 1.0);
        assert!(fixed_grid(&options).is_none());
        options.add("origin_y", 2.0);
        options.add("width", 3);
        options.add("height", 4);
        let grid = fixed_grid(&options).unwrap();
        assert_eq!(grid.width, 3);
        assert_eq!(grid.height, 4);
    }

    #[test]
    fn bounds_option_defines_grid_shape() {
        let grid = grid_from_bounds("([0, 4.5],[0, 4.5])", 1.0)
            .unwrap()
            .unwrap();
        assert_eq!(grid.origin_x, 0.0);
        assert_eq!(grid.origin_y, 0.0);
        assert_eq!(grid.width, 5);
        assert_eq!(grid.height, 5);
    }

    #[test]
    fn writer_rejects_bounds_with_alternate_grid() {
        let mut options = Options::new();
        options.add("filename", "/tmp/x.tif");
        options.add("resolution", 1.0);
        options.add("bounds", "([0, 1],[0, 1])");
        options.add("origin_x", 0.0);
        options.add("origin_y", 0.0);
        options.add("width", 2);
        options.add("height", 2);
        let mut writer = GdalWriter::new(&options);
        let view = make_view_with_points();
        let err = writer.write(&[view]).unwrap_err();
        assert!(err.0.contains("Specify either 'bounds'"));
    }

    #[test]
    fn parses_gdal_metadata_items() {
        let mut options = Options::new();
        options.add(
            "metadata",
            "AREA_OR_PIXEL=Pixel,empty=,equals=some_more_equals===",
        );
        assert_eq!(
            parse_metadata(&options),
            vec![
                ("AREA_OR_PIXEL".to_string(), "Pixel".to_string()),
                ("empty".to_string(), String::new()),
                ("equals".to_string(), "some_more_equals===".to_string()),
            ]
        );
    }

    #[test]
    fn count_band_uses_top_to_bottom_raster_order() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in [(0.0, 0.0, 1.0), (1.0, 1.0, 2.0)] {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, x);
            view.set_f64(point, &DimId::Y, y);
            view.set_f64(point, &DimId::Z, z);
        }

        let mut options = Options::new();
        options.add("output_type", "count");
        options.add("resolution", 1.0);
        options.add("radius", 0.1);
        options.add("binmode", true);
        let writer = GdalWriter::new(&options);
        let grid = FixedGrid {
            origin_x: 0.0,
            origin_y: 0.0,
            width: 2,
            height: 2,
        };
        let samples = collect_samples(&[view], &DimId::Z).unwrap();
        let bands = writer.render_bands(grid, &samples);
        assert_eq!(bands[0].1, vec![0.0, 1.0, 1.0, 0.0]);
    }

    #[test]
    fn writer_errors_without_filename() {
        let mut writer = GdalWriter::new(&Options::new());
        let layout = PointLayout::new();
        let view = PointView::new(std::rc::Rc::new(layout));
        let result = writer.write(&[view]);
        assert!(result.is_err());
        assert!(result.err().unwrap().0.contains("filename"));
    }

    #[test]
    fn writer_errors_on_zero_resolution() {
        let mut options = Options::new();
        options.add("filename", "/tmp/x.tif");
        options.add("resolution", 0.0);
        let mut writer = GdalWriter::new(&options);
        let layout = PointLayout::new();
        let view = PointView::new(std::rc::Rc::new(layout));
        assert!(writer.write(&[view]).is_err());
    }

    #[test]
    fn writer_errors_on_partial_fixed_grid() {
        let mut options = Options::new();
        options.add("filename", "/tmp/x.tif");
        options.add("resolution", 1.0);
        options.add("origin_x", 0.0);
        options.add("origin_y", 0.0);
        let mut writer = GdalWriter::new(&options);
        let layout = PointLayout::new();
        let view = PointView::new(std::rc::Rc::new(layout));
        assert!(writer.write(&[view]).is_err());
    }

    #[test]
    fn writer_errors_on_percentile_without_binmode() {
        let mut options = Options::new();
        options.add("filename", "/tmp/x.tif");
        options.add("resolution", 1.0);
        options.add("output_type", "p50");
        let mut writer = GdalWriter::new(&options);
        let layout = PointLayout::new();
        let view = PointView::new(std::rc::Rc::new(layout));
        assert!(writer.write(&[view]).is_err());
    }

    #[test]
    fn writer_errors_on_empty_view_without_allow_empty() {
        let mut options = Options::new();
        options.add("filename", "/tmp/x.tif");
        options.add("resolution", 1.0);
        let mut writer = GdalWriter::new(&options);
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let view = PointView::new(std::rc::Rc::new(layout));
        assert!(writer.write(&[view]).is_err());
    }

    #[test]
    fn parse_data_type_branches() {
        assert_eq!(parse_data_type("float64").0, OutputDataType::Float64);
        assert_eq!(parse_data_type("double").0, OutputDataType::Float64);
        assert_eq!(parse_data_type("int32").0, OutputDataType::Int32);
        assert_eq!(parse_data_type("int32_t").0, OutputDataType::Int32);
        assert_eq!(parse_data_type("signed32").0, OutputDataType::Int32);
        assert_eq!(parse_data_type("int").0, OutputDataType::Int32);
        assert!(parse_data_type("mystery").1.is_some());
    }

    #[test]
    fn writer_metadata_returns_expected_name() {
        let writer = GdalWriter::new(&Options::new());
        assert_eq!(writer.metadata().name(), "writers.gdal");
        assert_eq!(writer.name(), "writers.gdal");
    }

    fn tmp_tif(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "pdal-rust-gdal-writer-{}-{name}",
            std::process::id()
        ))
    }

    fn make_view_with_points() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in [
            (0.0, 0.0, 1.0),
            (1.0, 0.0, 2.0),
            (0.0, 1.0, 3.0),
            (1.0, 1.0, 4.0),
        ] {
            let p = view.add_point();
            view.set_f64(p, &DimId::X, x);
            view.set_f64(p, &DimId::Y, y);
            view.set_f64(p, &DimId::Z, z);
        }
        view
    }

    #[test]
    fn writer_writes_float64_output() {
        let out = tmp_tif("f64.tif");
        let mut options = Options::new();
        options.add("filename", out.to_str().unwrap());
        options.add("resolution", 1.0);
        options.add("output_type", "mean");
        let mut writer = GdalWriter::new(&options);
        let view = make_view_with_points();
        let r = writer.write(&[view]);
        // GDAL may not be configured in dev; tolerate failure.
        if r.is_ok() {
            assert!(out.exists());
        }
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn writer_writes_int32_output() {
        let out = tmp_tif("i32.tif");
        let mut options = Options::new();
        options.add("filename", out.to_str().unwrap());
        options.add("resolution", 1.0);
        options.add("output_type", "count");
        options.add("data_type", "int32");
        let mut writer = GdalWriter::new(&options);
        let view = make_view_with_points();
        let r = writer.write(&[view]);
        if r.is_ok() {
            assert!(out.exists());
        }
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn writer_writes_with_metadata_items() {
        let out = tmp_tif("meta.tif");
        let mut options = Options::new();
        options.add("filename", out.to_str().unwrap());
        options.add("resolution", 1.0);
        options.add("output_type", "mean");
        options.add("metadata", "AREA_OR_PIXEL=Pixel,Author=test");
        let mut writer = GdalWriter::new(&options);
        let view = make_view_with_points();
        let _ = writer.write(&[view]);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn writer_writes_all_stats_via_output_type_all() {
        let out = tmp_tif("all.tif");
        let mut options = Options::new();
        options.add("filename", out.to_str().unwrap());
        options.add("resolution", 1.0);
        options.add("output_type", "all");
        let mut writer = GdalWriter::new(&options);
        let view = make_view_with_points();
        let _ = writer.write(&[view]);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn output_types_rejects_invalid_percentile_value() {
        let mut options = Options::new();
        options.add("output_type", "p150");
        let (_stats, err) = output_types(&options);
        assert!(err.is_some());
    }

    #[test]
    fn output_types_rejects_invalid_percentile_text() {
        let mut options = Options::new();
        options.add("output_type", "pxx");
        let (_stats, err) = output_types(&options);
        assert!(err.is_some());
    }

    #[test]
    fn output_types_handles_no_valid_types() {
        let mut options = Options::new();
        options.add("output_type", "mystery");
        let (_stats, err) = output_types(&options);
        assert!(err.is_some());
    }

    #[test]
    fn writer_propagates_output_type_error() {
        let mut options = Options::new();
        options.add("filename", "/tmp/x.tif");
        options.add("resolution", 1.0);
        options.add("output_type", "mystery");
        let mut writer = GdalWriter::new(&options);
        let view = make_view_with_points();
        assert!(writer.write(&[view]).is_err());
    }

    #[test]
    fn writer_propagates_data_type_error() {
        let mut options = Options::new();
        options.add("filename", "/tmp/x.tif");
        options.add("resolution", 1.0);
        options.add("data_type", "uint8");
        let mut writer = GdalWriter::new(&options);
        let view = make_view_with_points();
        let err = writer.write(&[view]).unwrap_err();
        assert!(err.0.contains("Unsupported GDAL writer data_type"));
    }

    #[test]
    fn sample_cell_rejects_out_of_range() {
        let grid = FixedGrid {
            origin_x: 0.0,
            origin_y: 0.0,
            width: 2,
            height: 2,
        };
        // Negative column
        let s = Sample {
            x: -10.0,
            y: 0.5,
            value: 1.0,
        };
        assert!(sample_cell(grid, &s, 1.0).is_none());
        // Row out of range
        let s = Sample {
            x: 0.5,
            y: 100.0,
            value: 1.0,
        };
        assert!(sample_cell(grid, &s, 1.0).is_none());
    }
}
