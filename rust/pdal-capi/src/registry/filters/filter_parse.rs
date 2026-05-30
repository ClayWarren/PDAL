//! Option-string parsers used only by `filters.*` construction.
//! Split out of `registry.rs` to keep `filters.rs` under ~1k LOC.

use super::*;
use crate::registry::{get_bool, get_f64};

pub(super) fn comma_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

/// Parse a simple `filters.assign` assignment statement of the form
/// `Dim[range]=value`, matching the C++ `AssignRange::parse`. The expression
/// form (the separate `value` option) is not handled here.
pub(super) fn parse_assign_range(spec: &str) -> Result<AssignRange, String> {
    let limit = parse_range_limit(spec)?;
    let rest = spec[limit.consumed..].trim_start();
    let value_str = rest
        .strip_prefix('=')
        .ok_or_else(|| "filters.assign: Missing '=' assignment separator.".to_string())?
        .trim();
    let value: f64 = value_str
        .parse()
        .map_err(|_| "filters.assign: Missing value to assign following '='.".to_string())?;
    Ok(AssignRange {
        dim_name: limit.dim_name,
        value,
        lower_bound: limit.lower_bound,
        upper_bound: limit.upper_bound,
        inclusive_lower: limit.inclusive_lower,
        inclusive_upper: limit.inclusive_upper,
        negate: limit.negate,
    })
}

/// Parse a `filters.assign` `condition` DimRange (`Dim[range]`).
pub(super) fn parse_assign_condition(spec: &str) -> Result<AssignCondition, String> {
    let limit = parse_range_limit(spec)?;
    if !spec[limit.consumed..].trim().is_empty() {
        return Err("filters.assign: Invalid characters following condition range.".to_string());
    }
    Ok(AssignCondition {
        dim_name: limit.dim_name,
        lower_bound: limit.lower_bound,
        upper_bound: limit.upper_bound,
        inclusive_lower: limit.inclusive_lower,
        inclusive_upper: limit.inclusive_upper,
        negate: limit.negate,
    })
}

/// Parse a `filters.smrf` `classbits` option (comma-separated
/// `synthetic|keypoint|withheld`) into the Classification-flag bit mask,
/// matching the C++ `Segmentation::PointClasses` stream operator.
pub(super) fn parse_classbits(value: &str) -> Result<u8, String> {
    use pdal_filters::smrf::{CLASSBIT_KEYPOINT, CLASSBIT_SYNTHETIC, CLASSBIT_WITHHELD};
    let mut bits = 0u8;
    for token in value.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        match token.to_ascii_lowercase().as_str() {
            "keypoint" => bits |= CLASSBIT_KEYPOINT,
            "synthetic" => bits |= CLASSBIT_SYNTHETIC,
            "withheld" => bits |= CLASSBIT_WITHHELD,
            other => {
                return Err(format!(
                    "filters.smrf: Invalid 'classbits' value: '{other}'."
                ));
            }
        }
    }
    Ok(bits)
}

pub(super) fn crop_filter_from_options(options: &Options) -> Result<CropFilter, StageError> {
    let mut bounds = Vec::new();
    for value in options.values("bounds") {
        bounds.push(parse_crop_bounds(value)?);
    }

    let polygons = options.values("polygon").to_vec();

    let mut centers = Vec::new();
    for value in options.values("point") {
        let point = parse_wkt_point_coords(value)?;
        match point.as_slice() {
            [x, y] => centers.push(CropCenter::new_2d(*x, *y)),
            [x, y, z] => centers.push(CropCenter::new_3d(*x, *y, *z)),
            _ => unreachable!("parse_wkt_point_coords validates coordinate count"),
        }
    }

    CropFilter::new(
        get_bool(options, "outside", false)?,
        bounds,
        polygons,
        centers,
        get_f64(options, "distance", 0.0)?,
    )
}

pub(super) fn parse_crop_bounds(value: &str) -> Result<(f64, f64, f64, f64, f64, f64), StageError> {
    if let Ok(parsed) = parse_bounds3d(value, 0) {
        let bounds = parsed.bounds;
        return Ok((
            bounds.minx,
            bounds.miny,
            bounds.minz,
            bounds.maxx,
            bounds.maxy,
            bounds.maxz,
        ));
    }

    let bounds = parse_bounds2d(value, 0).map_err(StageError)?.bounds;
    Ok((
        bounds.minx,
        bounds.miny,
        f64::MIN,
        bounds.maxx,
        bounds.maxy,
        f64::MAX,
    ))
}

/// Parse a WKT POINT string into `[x, y, z]`.
///
/// Accepts `"POINT Z (x y z)"`, `"POINT (x y z)"`, and `"POINT (x y)"` (z=0).
/// Returns an error if the string is not a valid WKT point.
pub(super) fn parse_wkt_point(wkt: &str) -> Result<[f64; 3], StageError> {
    let parts = parse_wkt_point_coords(wkt)?;
    match parts.as_slice() {
        [x, y] => Ok([*x, *y, 0.0]),
        [x, y, z] => Ok([*x, *y, *z]),
        _ => unreachable!("parse_wkt_point_coords validates coordinate count"),
    }
}

pub(super) fn parse_wkt_point_coords(wkt: &str) -> Result<Vec<f64>, StageError> {
    let s = wkt.trim();
    let s = s
        .strip_prefix("POINT")
        .ok_or_else(|| StageError(format!("viewpoint must be a WKT POINT string, got '{wkt}'")))?;
    // Skip optional Z / ZM dimensionality keyword.
    let s = s.trim_start();
    let s = s
        .strip_prefix("ZM")
        .or_else(|| s.strip_prefix("Z"))
        .or_else(|| s.strip_prefix("M"))
        .map(|s| s.trim_start())
        .unwrap_or(s);
    let s = s
        .strip_prefix('(')
        .ok_or_else(|| StageError(format!("viewpoint must be a WKT POINT string, got '{wkt}'")))?;
    let s = s
        .strip_suffix(')')
        .ok_or_else(|| StageError(format!("viewpoint must be a WKT POINT string, got '{wkt}'")))?;
    let parts: Vec<f64> = s
        .split_whitespace()
        .map(|p| {
            p.parse().map_err(|_| {
                StageError(format!(
                    "viewpoint must be a WKT POINT string with numeric coordinates, got '{wkt}'"
                ))
            })
        })
        .collect::<Result<Vec<f64>, StageError>>()?;
    match parts.len() {
        2 | 3 => Ok(parts),
        _ => Err(StageError(format!(
            "viewpoint must have 2 or 3 coordinates, got {} in '{wkt}'",
            parts.len()
        ))),
    }
}

pub(super) fn sort_order(value: &str) -> Result<SortOrder, StageError> {
    match value.to_ascii_lowercase().as_str() {
        "" | "asc" | "ascending" => Ok(SortOrder::Asc),
        "desc" | "descending" => Ok(SortOrder::Desc),
        _ => Err(StageError(format!(
            "filters.sort order must be 'asc' or 'desc', got '{value}'."
        ))),
    }
}

pub(super) fn sort_algorithm(value: &str) -> Result<SortAlgorithm, StageError> {
    match value.to_ascii_lowercase().as_str() {
        "" | "normal" => Ok(SortAlgorithm::Normal),
        "stable" => Ok(SortAlgorithm::Stable),
        _ => Err(StageError(format!(
            "filters.sort algorithm must be 'normal' or 'stable', got '{value}'."
        ))),
    }
}

pub(super) fn covariance_mode(value: &str) -> CovarianceMode {
    match value.to_ascii_lowercase().as_str() {
        "raw" => CovarianceMode::Raw,
        "normalized" => CovarianceMode::Normalized,
        _ => CovarianceMode::Sqrt,
    }
}

pub(super) fn nn_distance_mode(value: &str) -> Result<NNDistanceMode, StageError> {
    match value.to_ascii_lowercase().as_str() {
        "kth" | "k" => Ok(NNDistanceMode::Kth),
        "avg" | "average" => Ok(NNDistanceMode::Average),
        _ => Err(StageError(format!(
            "filters.nndistance mode must be 'kth' or 'avg', got '{value}'."
        ))),
    }
}

pub(super) fn m3c2_orientation(value: &str) -> Result<M3C2NormalOrientation, StageError> {
    match value.to_ascii_lowercase().as_str() {
        "up" => Ok(M3C2NormalOrientation::Up),
        "down" => Ok(M3C2NormalOrientation::Down),
        "none" => Ok(M3C2NormalOrientation::None),
        _ => Err(StageError(format!(
            "filters.m3c2 orientation must be 'up', 'down', or 'none', got '{value}'."
        ))),
    }
}
