//! `writers.gdal` -- Write point clouds as GDAL rasters.

use pdal_core::gdal::RasterDataType;
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
    Float32,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
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
        let mut raster = pdal_core::gdal::Raster::create_typed(
            &self.filename,
            &self.driver_name,
            grid.width as i32,
            grid.height as i32,
            bands.len() as i32,
            geo_transform,
            &srs_wkt,
            self.data_type.gdal_type(),
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
        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("writers.gdal")
    }
}

fn parse_data_type(value: &str) -> (OutputDataType, Option<String>) {
    match value.to_ascii_lowercase().as_str() {
        "double" | "float64" => (OutputDataType::Float64, None),
        "float" | "float32" => (OutputDataType::Float32, None),
        "int8" | "signed8" => (OutputDataType::Int8, None),
        "uint8" | "unsigned8" | "byte" => (OutputDataType::UInt8, None),
        "int16" | "int16_t" | "signed16" => (OutputDataType::Int16, None),
        "uint16" | "uint16_t" | "unsigned16" => (OutputDataType::UInt16, None),
        "int32" | "int32_t" | "signed32" | "int" => (OutputDataType::Int32, None),
        "uint32" | "uint32_t" | "unsigned32" => (OutputDataType::UInt32, None),
        "int64" | "int64_t" | "signed64" => (OutputDataType::Int64, None),
        "uint64" | "uint64_t" | "unsigned64" => (OutputDataType::UInt64, None),
        _ => (
            OutputDataType::Float64,
            Some(format!(
                "Unsupported GDAL writer data_type '{value}' for the Rust-backed path."
            )),
        ),
    }
}

impl OutputDataType {
    fn gdal_type(self) -> RasterDataType {
        match self {
            OutputDataType::Float64 => RasterDataType::Float64,
            OutputDataType::Float32 => RasterDataType::Float32,
            OutputDataType::Int8 => RasterDataType::Int8,
            OutputDataType::UInt8 => RasterDataType::UInt8,
            OutputDataType::Int16 => RasterDataType::Int16,
            OutputDataType::UInt16 => RasterDataType::UInt16,
            OutputDataType::Int32 => RasterDataType::Int32,
            OutputDataType::UInt32 => RasterDataType::UInt32,
            OutputDataType::Int64 => RasterDataType::Int64,
            OutputDataType::UInt64 => RasterDataType::UInt64,
        }
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
#[path = "gdal_writer_tests.rs"]
mod gdal_writer_tests;
