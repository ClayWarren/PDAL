use crate::error::{set_last_error, string_to_c_ptr};
use pdal_core::point::{DimId, PointView};
use serde_json::{json, Map, Value};
use std::cmp::Ordering;
use std::ffi::{c_char, CStr};

#[derive(Clone)]
struct NearPoint {
    id: u64,
    dist: f64,
}

/// Build a filters.info-style summary over a point view.
///
/// # Safety
/// `view` must be null or a valid pointer returned by this C ABI. `point_spec`
/// and `query_spec` must be null or valid NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_info_summary_json(
    view: *const PointView,
    point_spec: *const c_char,
    query_spec: *const c_char,
) -> *mut c_char {
    let Some(view) = view.as_ref() else {
        set_last_error("pdal_info_summary_json received a null point view.");
        return std::ptr::null_mut();
    };

    match info_summary_json(view, opt_cstr(point_spec), opt_cstr(query_spec)) {
        Ok(text) => string_to_c_ptr(text),
        Err(err) => {
            set_last_error(&err);
            std::ptr::null_mut()
        }
    }
}

unsafe fn opt_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

fn info_summary_json(
    view: &PointView,
    point_spec: Option<String>,
    query_spec: Option<String>,
) -> Result<String, String> {
    let point_ids = match point_spec.as_deref().map(str::trim) {
        Some("") | None => Vec::new(),
        Some(spec) => parse_point_spec(spec)?,
    };
    let query = match query_spec.as_deref().map(str::trim) {
        Some("") | None => None,
        Some(spec) => Some(parse_query_spec(spec)?),
    };

    let mut out = Map::new();
    out.insert("num_points".to_string(), json!(view.len()));
    out.insert("dimensions".to_string(), json!(dimension_names(view)));
    out.insert("schema".to_string(), schema_json(view));
    out.insert("bbox".to_string(), bounds_json(view));
    if !view.spatial_reference().is_empty() {
        out.insert(
            "srs".to_string(),
            json!({
                "wkt": view.spatial_reference().wkt(),
                "epoch": view.spatial_reference().epoch(),
            }),
        );
    }

    let points = if !point_ids.is_empty() {
        selected_points(view, &point_ids)
    } else if let Some((x, y, z, count)) = query {
        query_points(view, x, y, z, count)
    } else {
        Vec::new()
    };
    if !points.is_empty() {
        out.insert("points".to_string(), Value::Array(points));
    }

    serde_json::to_string(&Value::Object(out)).map_err(|err| err.to_string())
}

fn parse_point_spec(spec: &str) -> Result<Vec<u64>, String> {
    let mut ids = Vec::new();
    for raw in spec.split(',') {
        let part = raw.trim();
        if part.is_empty() {
            continue;
        }
        let range: Vec<&str> = part.split('-').collect();
        match range.as_slice() {
            [one] => ids.push(parse_u64(one, "point")?),
            [low, high] => {
                let low = parse_u64(low, "point range")?;
                let high = parse_u64(high, "point range")?;
                if low > high {
                    return Err(format!("Invalid range in 'point' option: '{part}'"));
                }
                ids.extend(low..=high);
            }
            _ => return Err(format!("Invalid point range in 'point' option: {part}")),
        }
    }
    Ok(ids)
}

fn parse_u64(value: &str, label: &str) -> Result<u64, String> {
    value
        .trim()
        .parse()
        .map_err(|_| format!("Invalid integer '{}' in {label} option", value.trim()))
}

fn parse_query_spec(spec: &str) -> Result<(f64, f64, Option<f64>, usize), String> {
    let parts: Vec<&str> = spec.split('/').collect();
    if parts.len() > 2 {
        return Err(
            "Invalid point location specification. Syntax: --query=\"X,Y[/count]\"".to_string(),
        );
    }
    let count = if parts.len() == 2 {
        parse_u64(parts[1], "query count")? as usize
    } else {
        10
    };

    let tokens: Vec<&str> = parts[0]
        .split([',', '|', ' '])
        .filter(|value| !value.is_empty())
        .collect();
    if tokens.len() != 2 && tokens.len() != 3 {
        return Err(
            "Invalid point location specification. Syntax: --query=\"X,Y[/count]\"".to_string(),
        );
    }
    let x = parse_f64(tokens[0])?;
    let y = parse_f64(tokens[1])?;
    let z = if tokens.len() == 3 {
        Some(parse_f64(tokens[2])?)
    } else {
        None
    };
    Ok((x, y, z, count))
}

fn parse_f64(value: &str) -> Result<f64, String> {
    value.parse().map_err(|_| {
        "Invalid point location specification. Syntax: --query=\"X,Y[/count]\"".to_string()
    })
}

fn dimension_names(view: &PointView) -> Vec<String> {
    (0..view.layout().dim_count())
        .filter_map(|idx| {
            view.layout()
                .dim_at(idx)
                .map(|(dim, _)| dim.name().to_string())
        })
        .collect()
}

fn schema_json(view: &PointView) -> Value {
    Value::Array(
        (0..view.layout().dim_count())
            .filter_map(|idx| {
                view.layout().dim_at(idx).map(|(dim, ty)| {
                    json!({
                        "name": dim.name(),
                        "size": ty.size(),
                    })
                })
            })
            .collect(),
    )
}

fn bounds_json(view: &PointView) -> Value {
    if let Some(bounds) = view.calculate_bounds_3d() {
        json!({
            "minx": bounds.minx,
            "maxx": bounds.maxx,
            "miny": bounds.miny,
            "maxy": bounds.maxy,
            "minz": bounds.minz,
            "maxz": bounds.maxz,
        })
    } else {
        json!({})
    }
}

fn selected_points(view: &PointView, ids: &[u64]) -> Vec<Value> {
    ids.iter()
        .filter(|&&id| id < view.len())
        .map(|&id| point_json(view, id))
        .collect()
}

fn query_points(view: &PointView, x: f64, y: f64, z: Option<f64>, count: usize) -> Vec<Value> {
    let mut near: Vec<NearPoint> = (0..view.len())
        .map(|id| NearPoint {
            id,
            dist: query_distance(view, id, x, y, z),
        })
        .collect();
    near.sort_by(|a, b| {
        a.dist
            .partial_cmp(&b.dist)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    near.truncate(count);
    near.into_iter()
        .map(|point| point_json(view, point.id))
        .collect()
}

fn query_distance(view: &PointView, id: u64, x: f64, y: f64, z: Option<f64>) -> f64 {
    let dx = view.get_f64(id, &DimId::X) - x;
    let dy = view.get_f64(id, &DimId::Y) - y;
    let dz = z.map_or(0.0, |z| view.get_f64(id, &DimId::Z) - z);
    dx * dx + dy * dy + dz * dz
}

fn point_json(view: &PointView, id: u64) -> Value {
    let mut point = Map::new();
    for idx in 0..view.layout().dim_count() {
        if let Some((dim, _)) = view.layout().dim_at(idx) {
            point.insert(dim.name().to_string(), json!(view.get_f64(id, dim)));
        }
    }
    point.insert("PointId".to_string(), json!(id));
    Value::Object(point)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Red, DimType::U16);
        let mut view = PointView::new(Rc::new(layout));
        for idx in 0..3 {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, idx as f64);
            view.set_f64(id, &DimId::Y, idx as f64 * 2.0);
            view.set_f64(id, &DimId::Z, 10.0 - idx as f64);
            view.set_f64(id, &DimId::Red, 100.0 + idx as f64);
        }
        view
    }

    #[test]
    fn reports_point_ranges_bounds_and_schema() {
        let summary = info_summary_json(&view(), Some("0-1".to_string()), None).unwrap();
        let json: Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(json["num_points"], 3);
        assert_eq!(json["points"].as_array().unwrap().len(), 2);
        assert_eq!(json["points"][1]["Red"], 101.0);
        assert_eq!(json["bbox"]["maxz"], 10.0);
        assert_eq!(json["schema"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn reports_nearest_query_points() {
        let summary = info_summary_json(&view(), None, Some("1,2/1".to_string())).unwrap();
        let json: Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(json["points"][0]["PointId"], 1);
    }
}
