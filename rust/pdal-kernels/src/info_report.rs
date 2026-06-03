use crate::QueryRequest;
use chrono::NaiveDate;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::point::{Bounds3D, DimId, DimType, PointId, PointView};
use pdal_core::srs::{SpatialReference, SrsTransform};
use serde_json::json;
use std::path::Path;

pub struct InfoDimensionStats {
    pub name: String,
    pub count: u64,
    pub minimum: f64,
    pub maximum: f64,
    pub average: f64,
    pub variance: f64,
    pub stddev: f64,
    pub values: Option<Vec<f64>>,
}

pub fn point_report(views: &[PointView], point_ids: &[PointId]) -> String {
    let mut output = String::from("{\n  \"points\":\n  {\n");
    if point_ids.len() == 1 {
        output.push_str("    \"point\":\n");
        if let Some((view, local_id)) = locate_point(views, point_ids[0]) {
            output.push_str(&point_json(view, local_id, 4));
            output.push('\n');
        } else {
            output.push_str("    null\n");
        }
    } else {
        output.push_str("    \"point\":\n    [\n");
        let mut emitted = 0;
        for point_id in point_ids {
            if let Some((view, local_id)) = locate_point(views, *point_id) {
                if emitted > 0 {
                    output.push_str(",\n");
                }
                output.push_str(&point_json(view, local_id, 6));
                emitted += 1;
            }
        }
        output.push_str("\n    ]\n");
    }
    output.push_str("  },\n  \"reader\": \"readers.las\"\n}\n");
    output
}

pub fn query_report(views: &[PointView], query: QueryRequest) -> String {
    let mut points = Vec::new();
    for view in views {
        if view.layout().dim(&DimId::X).is_none() || view.layout().dim(&DimId::Y).is_none() {
            continue;
        }
        for local_id in 0..view.len() {
            let dx = view.get_f64(local_id, &DimId::X) - query.x;
            let dy = view.get_f64(local_id, &DimId::Y) - query.y;
            let dz = query
                .z
                .filter(|_| view.layout().dim(&DimId::Z).is_some())
                .map(|z| view.get_f64(local_id, &DimId::Z) - z)
                .unwrap_or(0.0);
            points.push((
                dx.mul_add(dx, dy.mul_add(dy, dz * dz)),
                view.source_index(local_id),
                view,
                local_id,
            ));
        }
    }
    points.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });

    let mut output = String::from("{\n  \"points\":\n  {\n    \"point\":\n    [\n");
    for (idx, (_, _, view, local_id)) in points.iter().take(query.count).enumerate() {
        if idx > 0 {
            output.push_str(",\n");
        }
        output.push_str(&point_json(view, *local_id, 6));
    }
    output.push_str("\n    ]\n  },\n  \"reader\": \"readers.las\"\n}\n");
    output
}

pub fn schema_report(views: &[PointView]) -> String {
    format!("{{\n  \"schema\":\n{}\n}}\n", schema_body(views, 2))
}

pub fn schema_body(views: &[PointView], indent: usize) -> String {
    let pad = " ".repeat(indent);
    let list_pad = " ".repeat(indent + 2);
    let item_pad = " ".repeat(indent + 4);
    let value_pad = " ".repeat(indent + 6);
    let mut output = format!("{pad}{{\n{list_pad}\"dimensions\":\n{list_pad}[\n");
    if let Some(view) = views.first() {
        let layout = view.layout();
        for idx in 0..layout.dim_count() {
            if let Some((dim, ty)) = layout.dim_at(idx) {
                if idx > 0 {
                    output.push_str(",\n");
                }
                output.push_str(&format!(
                    "{item_pad}{{\n{value_pad}\"name\": \"{}\",\n{value_pad}\"size\": {},\n{value_pad}\"type\": \"{}\"\n{item_pad}}}",
                    dim.name(),
                    ty.size(),
                    dim_type_name(ty)
                ));
            }
        }
    }
    output.push_str(&format!("\n{list_pad}]\n{pad}}}"));
    output
}

pub fn stats_report(
    views: &[PointView],
    dimensions: Option<&[DimId]>,
    enumerate: Option<&[DimId]>,
    breakout: Option<&DimId>,
) -> String {
    format!(
        "{{\n  \"stats\":\n{}\n}}\n",
        stats_body(views, 2, dimensions, enumerate, breakout)
    )
}

pub fn stats_body(
    views: &[PointView],
    indent: usize,
    dimensions: Option<&[DimId]>,
    enumerate: Option<&[DimId]>,
    breakout: Option<&DimId>,
) -> String {
    let pad = " ".repeat(indent);
    let list_pad = " ".repeat(indent + 2);
    let item_pad = " ".repeat(indent + 4);
    let value_pad = " ".repeat(indent + 6);
    let stats = dimension_stats(views, dimensions, enumerate);
    let mut output = format!("{pad}{{\n");
    if let Some(dim) = breakout {
        output.push_str(&breakout_body(dim, list_pad.as_str(), item_pad.as_str()));
        output.push_str(",\n");
    }
    output.push_str(&format!("{list_pad}\"statistic\":\n{list_pad}[\n"));
    for (idx, stat) in stats.iter().enumerate() {
        if idx > 0 {
            output.push_str(",\n");
        }
        output.push_str(&format!(
            "{item_pad}{{\n{value_pad}\"average\": {},\n{value_pad}\"count\": {},\n{value_pad}\"maximum\": {},\n{value_pad}\"minimum\": {},\n{value_pad}\"name\": \"{}\",\n{value_pad}\"position\": {},\n{value_pad}\"stddev\": {}",
            format_number(stat.average),
            stat.count,
            format_number(stat.maximum),
            format_number(stat.minimum),
            stat.name,
            idx,
            format_number(stat.stddev)
        ));
        if let Some(values) = &stat.values {
            output.push_str(&format!(",\n{value_pad}\"values\":\n{value_pad}[\n"));
            for (value_idx, value) in values.iter().enumerate() {
                if value_idx > 0 {
                    output.push_str(",\n");
                }
                output.push_str(&format!("{value_pad}  {}", format_number(*value)));
            }
            output.push_str(&format!("\n{value_pad}]"));
        }
        output.push_str(&format!(
            ",\n{value_pad}\"variance\": {}\n{item_pad}}}",
            format_number(stat.variance)
        ));
    }
    output.push_str(&format!("\n{list_pad}]\n{pad}}}"));
    output
}

pub fn stac_report(
    views: &[PointView],
    metadata: &MetadataNode,
    filename: &str,
    pc_type: &str,
) -> String {
    if views
        .first()
        .is_none_or(|view| view.spatial_reference().is_empty())
    {
        return "{\n  \"stac\":\n  {\n    \"message\": \"Failed to create STAC Feature with missing key. 'EPSG:4326'\",\n    \"status\": \"error\"\n  }\n}\n".to_string();
    }

    let Some(bounds) = aggregate_bounds_3d(views) else {
        return "{\n  \"stac\":\n  {\n    \"message\": \"Failed to create STAC Feature with missing key. 'bbox'\",\n    \"status\": \"error\"\n  }\n}\n".to_string();
    };
    let source_srs = views
        .iter()
        .map(PointView::spatial_reference)
        .find(|srs| !srs.is_empty())
        .map(|srs| srs.wkt())
        .unwrap_or("");
    let projected = stac_projected_geometry(&bounds, source_srs);
    let (wkt2, projjson) = stac_projection_metadata(source_srs);
    let mut properties = json!({
        "datetime": stac_datetime(metadata),
        "pc:count": views.iter().map(PointView::len).sum::<u64>(),
        "pc:encoding": stac_encoding(filename),
        "pc:type": pc_type,
        "pc:schemas": stac_schema_json(views),
        "pc:statistics": stac_statistics_json(views),
        "proj:bbox": source_bbox_json(&bounds),
        "proj:geometry": source_geometry_json(&bounds),
        "proj:wkt2": wkt2,
    });
    if !projjson.is_null() {
        properties["proj:projjson"] = projjson;
    }
    let value = json!({
        "stac": {
            "type": "Feature",
            "stac_version": "1.0.0",
            "stac_extensions": [
                "https://stac-extensions.github.io/pointcloud/v1.0.0/schema.json",
                "https://stac-extensions.github.io/projection/v1.1.0/schema.json"
            ],
            "id": stac_id(filename),
            "bbox": projected.bbox,
            "geometry": projected.geometry,
            "properties": properties,
            "assets": {
                "data": {
                    "href": filename,
                    "title": "Pointcloud data"
                }
            },
            "links": [
                {
                    "href": filename,
                    "rel": "derived_from"
                }
            ]
        }
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()) + "\n"
}

struct StacProjectedGeometry {
    bbox: serde_json::Value,
    geometry: serde_json::Value,
}

fn aggregate_bounds_3d(views: &[PointView]) -> Option<Bounds3D> {
    views
        .iter()
        .filter_map(PointView::calculate_bounds_3d)
        .fold(None, |acc, bounds| {
            Some(match acc {
                Some(existing) => Bounds3D {
                    minx: existing.minx.min(bounds.minx),
                    maxx: existing.maxx.max(bounds.maxx),
                    miny: existing.miny.min(bounds.miny),
                    maxy: existing.maxy.max(bounds.maxy),
                    minz: existing.minz.min(bounds.minz),
                    maxz: existing.maxz.max(bounds.maxz),
                },
                None => bounds,
            })
        })
}

fn stac_projected_geometry(bounds: &Bounds3D, source_srs: &str) -> StacProjectedGeometry {
    let source_corners = [
        (bounds.minx, bounds.miny, bounds.minz),
        (bounds.minx, bounds.maxy, bounds.minz),
        (bounds.maxx, bounds.maxy, bounds.maxz),
        (bounds.maxx, bounds.miny, bounds.maxz),
        (bounds.minx, bounds.miny, bounds.minz),
    ];
    let mut projected = source_corners;
    if let Ok(transform) = SrsTransform::new(
        &SpatialReference::new(source_srs),
        &SpatialReference::new("EPSG:4326"),
    ) {
        for (x, y, z) in &mut projected {
            let _ = transform.transform(x, y, z);
        }
    }
    let mut minx = f64::INFINITY;
    let mut maxx = f64::NEG_INFINITY;
    let mut miny = f64::INFINITY;
    let mut maxy = f64::NEG_INFINITY;
    let mut minz = f64::INFINITY;
    let mut maxz = f64::NEG_INFINITY;
    for (x, y, z) in &projected {
        minx = minx.min(*x);
        maxx = maxx.max(*x);
        miny = miny.min(*y);
        maxy = maxy.max(*y);
        minz = minz.min(*z);
        maxz = maxz.max(*z);
    }
    StacProjectedGeometry {
        bbox: json!([minx, miny, minz, maxx, maxy, maxz]),
        geometry: json!({
            "type": "Polygon",
            "coordinates": [projected.iter().map(|(x, y, z)| json!([x, y, z])).collect::<Vec<_>>()],
        }),
    }
}

fn source_bbox_json(bounds: &Bounds3D) -> serde_json::Value {
    json!([
        bounds.minx,
        bounds.miny,
        bounds.minz,
        bounds.maxx,
        bounds.maxy,
        bounds.maxz
    ])
}

fn source_geometry_json(bounds: &Bounds3D) -> serde_json::Value {
    json!({
        "type": "Polygon",
        "coordinates": [[
            [bounds.minx, bounds.miny, bounds.minz],
            [bounds.minx, bounds.maxy, bounds.minz],
            [bounds.maxx, bounds.maxy, bounds.maxz],
            [bounds.maxx, bounds.miny, bounds.maxz],
            [bounds.minx, bounds.miny, bounds.minz],
        ]]
    })
}

fn stac_schema_json(views: &[PointView]) -> serde_json::Value {
    let Some(view) = views.first() else {
        return json!([]);
    };
    let layout = view.layout();
    json!((0..layout.dim_count())
        .filter_map(|idx| {
            layout.dim_at(idx).map(|(dim, ty)| {
                json!({
                    "name": dim.name(),
                    "size": ty.size(),
                    "type": dim_type_name(ty),
                })
            })
        })
        .collect::<Vec<_>>())
}

fn stac_statistics_json(views: &[PointView]) -> serde_json::Value {
    json!(dimension_stats(views, None, None)
        .into_iter()
        .enumerate()
        .map(|(position, stat)| {
            json!({
                "average": stat.average,
                "count": stat.count,
                "maximum": stat.maximum,
                "minimum": stat.minimum,
                "name": stat.name,
                "position": position,
                "stddev": stat.stddev,
                "variance": stat.variance,
            })
        })
        .collect::<Vec<_>>())
}

fn stac_projection_metadata(source_srs: &str) -> (String, serde_json::Value) {
    if let Ok(input) = pdal_native::srs::user_input_to_wkt(source_srs) {
        let projjson = serde_json::from_str(&input.projjson).unwrap_or(serde_json::Value::Null);
        return (input.wkt2, projjson);
    }
    let wkt2 =
        pdal_native::srs::wkt_to_wkt2(source_srs, 0.0).unwrap_or_else(|_| source_srs.to_string());
    let projjson = pdal_native::srs::wkt_to_projjson(source_srs, 0.0)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(serde_json::Value::Null);
    (wkt2, projjson)
}

fn stac_id(filename: &str) -> String {
    Path::new(filename)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or(filename)
        .to_string()
}

fn dimension_stats(
    views: &[PointView],
    dimensions: Option<&[DimId]>,
    enumerate: Option<&[DimId]>,
) -> Vec<InfoDimensionStats> {
    let Some(first) = views.first() else {
        return Vec::new();
    };
    let layout = first.layout();
    let mut output = Vec::new();
    let mut selected = Vec::new();
    if let Some(dimensions) = dimensions {
        selected.extend(dimensions.iter().cloned());
    } else {
        for idx in 0..layout.dim_count() {
            if let Some((dim, _)) = layout.dim_at(idx) {
                selected.push(dim.clone());
            }
        }
    }

    for dim in &selected {
        let mut values = Vec::new();
        for view in views {
            if view.layout().dim(dim).is_none() {
                continue;
            }
            for point_idx in 0..view.len() {
                values.push(view.get_f64(point_idx, dim));
            }
        }
        if values.is_empty() {
            continue;
        }
        let count = values.len() as u64;
        let sum = values.iter().sum::<f64>();
        let average = sum / count as f64;
        let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let variance = if count > 1 {
            values
                .iter()
                .map(|value| {
                    let diff = value - average;
                    diff * diff
                })
                .sum::<f64>()
                / (count - 1) as f64
        } else {
            0.0
        };
        output.push(InfoDimensionStats {
            name: dim.name().to_string(),
            count,
            minimum,
            maximum,
            average,
            variance,
            stddev: variance.sqrt(),
            values: enumerate
                .is_some_and(|dims| dims.contains(dim))
                .then(|| unique_sorted_values(&values)),
        });
    }
    output
}

fn locate_point(views: &[PointView], point_id: PointId) -> Option<(&PointView, PointId)> {
    for view in views {
        for local_id in 0..view.len() {
            if view.source_index(local_id) == point_id {
                return Some((view, local_id));
            }
        }
    }
    None
}

fn point_json(view: &PointView, local_id: PointId, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let value_pad = " ".repeat(indent + 2);
    let mut fields = point_field_names(view);
    let point_id_pos = fields
        .iter()
        .position(|name| name.as_str() > "PointId")
        .unwrap_or(fields.len());
    fields.insert(point_id_pos, "PointId".to_string());

    let mut output = format!("{pad}{{\n");
    for (idx, name) in fields.iter().enumerate() {
        if idx > 0 {
            output.push_str(",\n");
        }
        let value = if name == "PointId" {
            view.source_index(local_id) as f64
        } else {
            view.get_f64(local_id, &DimId::from_name(name))
        };
        output.push_str(&format!(
            "{value_pad}\"{name}\": {}",
            format_point_value(value)
        ));
    }
    output.push_str(&format!("\n{pad}}}"));
    output
}

fn point_field_names(view: &PointView) -> Vec<String> {
    let layout = view.layout();
    let mut names = Vec::new();
    for idx in 0..layout.dim_count() {
        if let Some((dim, _)) = layout.dim_at(idx) {
            names.push(dim.name().to_string());
        }
    }
    names.sort();
    names
}

fn unique_sorted_values(values: &[f64]) -> Vec<f64> {
    let mut unique = values.to_vec();
    unique.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    unique.dedup_by(|a, b| a == b);
    unique
}

fn breakout_body(dim: &DimId, list_pad: &str, item_pad: &str) -> String {
    let value_pad = format!("{item_pad}  ");
    let expressions = [
        "(Withheld==1)",
        "(Keypoint==1)",
        "(Overlap==1)",
        "(Synthetic==1)",
    ];
    let mut output = format!(
        "{list_pad}\"breakout\":\n{list_pad}{{\n{item_pad}\"dimension\": \"{}\",\n{item_pad}\"statistic\":\n{item_pad}[\n",
        dim.name()
    );
    for (idx, expression) in expressions.iter().enumerate() {
        if idx > 0 {
            output.push_str(",\n");
        }
        output.push_str(&format!(
            "{value_pad}{{\n{value_pad}  \"expression\": \"{expression}\",\n{value_pad}  \"position\": {idx}\n{value_pad}}}"
        ));
    }
    output.push_str(&format!("\n{item_pad}]\n{list_pad}}}"));
    output
}

fn dim_type_name(ty: DimType) -> &'static str {
    match ty {
        DimType::U8 | DimType::U16 | DimType::U32 | DimType::U64 => "unsigned",
        DimType::I8 | DimType::I16 | DimType::I32 | DimType::I64 => "signed",
        DimType::F32 | DimType::F64 => "floating",
    }
}

fn format_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let digits_before_decimal = value.abs().log10().floor().max(0.0) as i32 + 1;
    let decimals = (10 - digits_before_decimal).max(0) as usize;
    let mut text = format!("{value:.decimals$}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn format_point_value(value: f64) -> String {
    format_number(value)
}

fn stac_datetime(metadata: &MetadataNode) -> String {
    let year = metadata_value_u64(metadata, "creation_year");
    let doy = metadata_value_u64(metadata, "creation_doy");
    if let (Some(year), Some(doy)) = (year, doy) {
        if let Some(date) = NaiveDate::from_yo_opt(year as i32, doy as u32) {
            return date.format("%Y-%m-%dT00:00:00Z").to_string();
        }
    }
    "1970-01-01T00:00:00Z".to_string()
}

fn metadata_value_u64(node: &MetadataNode, name: &str) -> Option<u64> {
    if node.name() == name {
        return node.value().map(MetadataValue::as_u64);
    }
    node.children()
        .iter()
        .find_map(|child| metadata_value_u64(child, name))
}

fn stac_encoding(filename: &str) -> String {
    Path::new(filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{}", extension.to_ascii_lowercase()))
        .unwrap_or_else(|| "?".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::metadata::MetadataValue;
    use pdal_core::point::PointLayout;
    use pdal_core::srs::SpatialReference;
    use std::rc::Rc;

    fn sample_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Intensity, DimType::U16);
        layout.register(DimId::ReturnNumber, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for (source, x, y, z, intensity, return_number) in [
            (10, 1.0, 4.0, 7.0, 100.0, 1.0),
            (11, 2.0, 5.0, 8.0, 200.0, 1.0),
            (12, 3.0, 6.0, 9.0, 200.0, 2.0),
        ] {
            let point = view.add_point();
            view.set_source_index(point, source);
            view.set_f64(point, &DimId::X, x);
            view.set_f64(point, &DimId::Y, y);
            view.set_f64(point, &DimId::Z, z);
            view.set_f64(point, &DimId::Intensity, intensity);
            view.set_f64(point, &DimId::ReturnNumber, return_number);
        }
        view
    }

    #[test]
    fn point_report_returns_single_point_or_null() {
        let view = sample_view();
        let report = point_report(&[view], &[11]);
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();

        assert_eq!(json["points"]["point"]["PointId"], 11.0);
        assert_eq!(json["points"]["point"]["X"], 2.0);
        assert_eq!(json["reader"], "readers.las");

        let missing = point_report(&[], &[7]);
        let json: serde_json::Value = serde_json::from_str(&missing).unwrap();
        assert!(json["points"]["point"].is_null());
    }

    #[test]
    fn point_report_lists_only_found_points() {
        let view = sample_view();
        let report = point_report(&[view], &[12, 99, 10]);
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();
        let points = json["points"]["point"].as_array().unwrap();

        assert_eq!(points.len(), 2);
        assert_eq!(points[0]["PointId"], 12.0);
        assert_eq!(points[1]["PointId"], 10.0);
    }

    #[test]
    fn query_report_uses_xy_and_optional_z_distance() {
        let view = sample_view();
        let report = query_report(
            &[view],
            QueryRequest {
                x: 2.0,
                y: 5.0,
                z: Some(8.4),
                count: 2,
            },
        );
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();
        let points = json["points"]["point"].as_array().unwrap();

        assert_eq!(points.len(), 2);
        assert_eq!(points[0]["PointId"], 11.0);
        assert_eq!(points[1]["PointId"], 12.0);
    }

    #[test]
    fn query_report_skips_views_without_xy() {
        let layout = Rc::new(PointLayout::new());
        let mut view = PointView::new(layout);
        view.add_point();
        let report = query_report(
            &[view],
            QueryRequest {
                x: 0.0,
                y: 0.0,
                z: None,
                count: 3,
            },
        );
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();

        assert!(json["points"]["point"].as_array().unwrap().is_empty());
    }

    #[test]
    fn schema_report_lists_registered_dimensions() {
        let view = sample_view();
        let report = schema_report(&[view]);
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();
        let dims = json["schema"]["dimensions"].as_array().unwrap();

        assert_eq!(dims[0]["name"], "X");
        assert_eq!(dims[0]["type"], "floating");
        assert_eq!(dims[3]["name"], "Intensity");
        assert_eq!(dims[3]["type"], "unsigned");
    }

    #[test]
    fn stats_report_filters_enumerates_and_breaks_out_dimensions() {
        let view = sample_view();
        let report = stats_report(
            &[view],
            Some(&[DimId::Intensity, DimId::Other("Missing".into())]),
            Some(&[DimId::Intensity]),
            Some(&DimId::Classification),
        );
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();
        let stats = json["stats"]["statistic"].as_array().unwrap();

        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0]["name"], "Intensity");
        assert_eq!(stats[0]["count"], 3);
        assert_eq!(stats[0]["minimum"], 100.0);
        assert_eq!(stats[0]["maximum"], 200.0);
        assert_eq!(stats[0]["values"].as_array().unwrap().len(), 2);
        assert_eq!(json["stats"]["breakout"]["dimension"], "Classification");
        assert_eq!(
            json["stats"]["breakout"]["statistic"][0]["expression"],
            "(Withheld==1)"
        );
    }

    #[test]
    fn empty_schema_and_stats_reports_are_valid_json() {
        let schema = schema_report(&[]);
        let schema_json: serde_json::Value = serde_json::from_str(&schema).unwrap();
        assert!(schema_json["schema"]["dimensions"]
            .as_array()
            .unwrap()
            .is_empty());

        let stats = stats_report(&[], None, None, None);
        let stats_json: serde_json::Value = serde_json::from_str(&stats).unwrap();
        assert!(stats_json["stats"]["statistic"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn schema_report_names_signed_dimensions() {
        let mut layout = PointLayout::new();
        layout.register(DimId::ScanAngleRank, DimType::I8);
        let view = PointView::new(Rc::new(layout));
        let report = schema_report(&[view]);
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();

        assert_eq!(json["schema"]["dimensions"][0]["type"], "signed");
    }

    #[test]
    fn single_value_stats_have_zero_sample_variance() {
        let mut layout = PointLayout::new();
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        let point = view.add_point();
        view.set_f64(point, &DimId::Z, 12.25);

        let report = stats_report(&[view], Some(&[DimId::Z]), None, None);
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();
        let stat = &json["stats"]["statistic"][0];

        assert_eq!(stat["average"], 12.25);
        assert_eq!(stat["variance"], 0.0);
        assert_eq!(stat["stddev"], 0.0);
    }

    #[test]
    fn stac_report_errors_without_spatial_reference() {
        let layout = Rc::new(PointLayout::new());
        let mut view = PointView::new(layout);
        view.add_point();
        let report = stac_report(&[view], &MetadataNode::new("root"), "sample", "lidar");
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();

        assert_eq!(json["stac"]["status"], "error");
    }

    #[test]
    fn stac_report_defaults_datetime_and_unknown_extension() {
        let mut view = sample_view();
        view.set_spatial_reference(SpatialReference::new("EPSG:4326"));

        let report = stac_report(&[view], &MetadataNode::new("root"), "sample", "lidar");
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();

        assert_eq!(json["stac"]["type"], "Feature");
        assert_eq!(json["stac"]["geometry"]["type"], "Polygon");
        assert_eq!(json["stac"]["bbox"].as_array().unwrap().len(), 6);
        assert_eq!(
            json["stac"]["properties"]["datetime"],
            "1970-01-01T00:00:00Z"
        );
        assert_eq!(json["stac"]["properties"]["pc:encoding"], "?");
        assert_eq!(json["stac"]["properties"]["pc:schemas"][0]["name"], "X");
        assert_eq!(json["stac"]["properties"]["pc:statistics"][0]["name"], "X");
        assert_eq!(json["stac"]["properties"]["proj:bbox"][0], 1.0);
        assert_eq!(json["stac"]["assets"]["data"]["href"], "sample");
    }

    #[test]
    fn stac_report_uses_requested_pointcloud_type() {
        let mut view = sample_view();
        view.set_spatial_reference(SpatialReference::new("EPSG:4326"));

        let report = stac_report(&[view], &MetadataNode::new("root"), "sample.las", "sonar");
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();

        assert_eq!(json["stac"]["properties"]["pc:type"], "sonar");
    }

    #[test]
    fn stac_report_uses_las_creation_day() {
        let mut view = sample_view();
        view.set_spatial_reference(SpatialReference::new("EPSG:4326"));
        let mut metadata = MetadataNode::new("root");
        metadata.add_value("creation_year", MetadataValue::U64(2026));
        metadata.add_value("creation_doy", MetadataValue::U64(32));

        let report = stac_report(&[view], &metadata, "sample.las", "lidar");
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();

        assert_eq!(
            json["stac"]["properties"]["datetime"],
            "2026-02-01T00:00:00Z"
        );
    }
}
