use crate::pipeline_abi::{
    pdal_pipeline_result_t, pipeline_result_to_json_for_kernel, PipelineHandle,
};
use crate::registry::pipeline_from_json;
use chrono::NaiveDate;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_kernels::{build_info_plan, InfoKernelPlan, InfoMode, InfoRunPlan, QueryRequest};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::Path;

mod command;

pub(in crate::kernel_abi) use command::run_pipeline_kernel;

pub(super) unsafe fn run_info_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    let plan = match build_info_plan(&args) {
        InfoKernelPlan::Run(plan) => plan,
        InfoKernelPlan::Return(code) => return code,
    };

    let (filename, driver, mode, pc_type, serialization_file) = match plan {
        InfoRunPlan::PipelineJson {
            json,
            mode,
            pc_type,
            serialization_file,
        } => return run_info_pipeline_json(&json, mode, &pc_type, serialization_file),
        InfoRunPlan::File {
            filename,
            driver,
            mode,
            pc_type,
            serialization_file,
        } => (filename, driver, mode, pc_type, serialization_file),
    };

    if let Some(path) = serialization_file {
        let serialized = serde_json::json!({
            "pipeline": [
                {
                    "type": driver,
                    "filename": filename,
                }
            ]
        });
        let Ok(text) = serde_json::to_string_pretty(&serialized) else {
            eprintln!("PDAL: kernels.info: Unable to serialize pipeline.");
            return 1;
        };
        if let Err(err) = std::fs::write(&path, text + "\n") {
            eprintln!("PDAL: kernels.info: Unable to write pipeline serialization '{path}': {err}");
            return 1;
        }
    }

    let mut pipeline = match info_pipeline(&driver, &filename, mode.needs_boundary()) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.info: {err}");
            return 1;
        }
    };

    match mode {
        InfoMode::Summary => match pipeline.execute_with_result(Vec::new()) {
            Ok(result) => {
                let handle = PipelineHandle { pipeline };
                println!(
                    "{}",
                    info_summary_json(
                        pipeline_result_to_json_for_kernel(result, &handle),
                        Some(&filename),
                        Some(&driver),
                    )
                );
                0
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                1
            }
        },
        InfoMode::Stats { .. }
        | InfoMode::Schema
        | InfoMode::Metadata
        | InfoMode::All
        | InfoMode::Boundary
        | InfoMode::Stac
        | InfoMode::Points(_)
        | InfoMode::Query(_) => match pipeline.execute(Vec::new()) {
            Ok(views) => {
                let metadata = pipeline.metadata();
                println!(
                    "{}",
                    info_report(mode, &views, &metadata, &filename, &pc_type)
                );
                0
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                1
            }
        },
    }
}

fn run_info_pipeline_json(
    json: &str,
    mode: InfoMode,
    pc_type: &str,
    serialization_file: Option<String>,
) -> i32 {
    let json = if mode.needs_boundary() {
        match append_info_stage_to_pipeline_json(
            json,
            serde_json::json!({ "type": "filters.hexbin" }),
        ) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                return 1;
            }
        }
    } else {
        json.to_string()
    };

    if let Some(path) = serialization_file {
        if let Err(err) = std::fs::write(&path, &json) {
            eprintln!("PDAL: kernels.info: Unable to write pipeline serialization '{path}': {err}");
            return 1;
        }
    }

    let mut pipeline = match pipeline_from_json(&json) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.info: {err}");
            return 1;
        }
    };

    match mode {
        InfoMode::Summary => match pipeline.execute_with_result(Vec::new()) {
            Ok(result) => {
                let handle = PipelineHandle { pipeline };
                println!(
                    "{}",
                    info_summary_json(
                        pipeline_result_to_json_for_kernel(result, &handle),
                        Some("STDIN"),
                        None,
                    )
                );
                0
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                1
            }
        },
        _ => match pipeline.execute(Vec::new()) {
            Ok(views) => {
                let metadata = pipeline.metadata();
                println!("{}", info_report(mode, &views, &metadata, "STDIN", pc_type));
                0
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                1
            }
        },
    }
}

fn info_summary_json(summary: String, filename: Option<&str>, driver: Option<&str>) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&summary) else {
        return summary;
    };
    let Some(object) = value.as_object_mut() else {
        return summary;
    };
    if let Some(filename) = filename {
        object.insert("filename".to_string(), serde_json::json!(filename));
    }
    if let Some(driver) = driver {
        object.insert("driver".to_string(), serde_json::json!(driver));
    }
    serde_json::to_string(&value).unwrap_or(summary)
}

fn append_info_stage_to_pipeline_json(
    json: &str,
    stage: serde_json::Value,
) -> Result<String, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(json).map_err(|err| format!("Invalid pipeline JSON: {err}"))?;
    if let Some(stages) = value.as_array_mut() {
        stages.push(stage);
    } else if let Some(stages) = value
        .get_mut("pipeline")
        .and_then(serde_json::Value::as_array_mut)
    {
        stages.push(stage);
    } else {
        return Err("Pipeline JSON object must contain a 'pipeline' array.".to_string());
    }
    serde_json::to_string(&value).map_err(|err| err.to_string())
}

fn info_pipeline(
    driver: &str,
    filename: &str,
    include_boundary: bool,
) -> Result<pdal_core::pipeline::Pipeline, String> {
    let mut stages = vec![serde_json::json!({ "type": driver, "filename": filename })];
    if include_boundary {
        stages.push(serde_json::json!({ "type": "filters.hexbin" }));
    }
    pipeline_from_json(&serde_json::Value::Array(stages).to_string()).map_err(|err| err.to_string())
}

fn info_report(
    mode: InfoMode,
    views: &[PointView],
    metadata: &MetadataNode,
    filename: &str,
    pc_type: &str,
) -> String {
    match mode {
        InfoMode::Stats {
            dimensions,
            enumerate,
            breakout,
        } => stats_report(
            views,
            dimensions.as_deref(),
            enumerate.as_deref(),
            breakout.as_ref(),
        ),
        InfoMode::Schema => schema_report(views),
        InfoMode::Metadata => metadata_report(metadata),
        InfoMode::All => {
            let mut output = String::from("{\n");
            output.push_str("  \"schema\":\n");
            output.push_str(&schema_body(views, 2));
            output.push_str(",\n");
            output.push_str("  \"stats\":\n");
            output.push_str(&stats_body(views, 2, None, None, None));
            output.push_str(",\n");
            output.push_str("  \"metadata\": ");
            let metadata_json = crate::metadata_abi::metadata_node_to_json_flat(metadata);
            output.push_str(
                &serde_json::to_string_pretty(&metadata_json).unwrap_or_else(|_| "{}".into()),
            );
            output.push_str(",\n");
            output.push_str("  \"boundary\": ");
            output.push_str(&boundary_value(metadata).to_string());
            output.push_str(",\n");
            output.push_str("  \"stac\": ");
            let stac_json = serde_json::from_str::<serde_json::Value>(&stac_report(
                views, metadata, filename, pc_type,
            ))
            .ok()
            .and_then(|value| value.get("stac").cloned())
            .unwrap_or_else(|| serde_json::json!({}));
            output.push_str(
                &serde_json::to_string_pretty(&stac_json).unwrap_or_else(|_| "{}".into()),
            );
            output.push_str("\n}\n");
            output
        }
        InfoMode::Stac => stac_report(views, metadata, filename, pc_type),
        InfoMode::Boundary => boundary_report(metadata),
        InfoMode::Points(point_ids) => point_report(views, &point_ids),
        InfoMode::Query(query) => query_report(views, query),
        InfoMode::Summary => String::new(),
    }
}

fn boundary_report(metadata: &MetadataNode) -> String {
    let value = serde_json::json!({
        "boundary": boundary_value(metadata),
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()) + "\n"
}

fn boundary_value(metadata: &MetadataNode) -> serde_json::Value {
    let hexbin = metadata.find_child("stage_1");
    let boundary = hexbin
        .and_then(|stage| stage.find_child("hex_boundary_raw"))
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_string)
        .unwrap_or_else(|| "MULTIPOLYGON EMPTY".to_string());
    let estimated_edge = hexbin
        .and_then(|stage| stage.find_child("estimated_edge"))
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_f64)
        .unwrap_or(0.0);
    let geometry = pdal_native::geometry::Geometry::from_wkt(&boundary).and_then(|geometry| {
        if estimated_edge > 0.0 && boundary != "MULTIPOLYGON EMPTY" {
            geometry.simplify(1.1 * estimated_edge / 2.0, true)
        } else {
            Ok(geometry)
        }
    });
    let boundary = geometry
        .as_ref()
        .ok()
        .and_then(|geometry| geometry.to_wkt_precision(8).ok())
        .unwrap_or(boundary);
    let boundary_json = geometry
        .and_then(|geometry| geometry.to_gdal_geojson(8))
        .unwrap_or_else(|_| "{}".to_string());

    serde_json::json!({
        "boundary": boundary,
        "boundary_json": boundary_json,
    })
}

fn metadata_report(metadata: &MetadataNode) -> String {
    let value = serde_json::json!({
        "metadata": crate::metadata_abi::metadata_node_to_json_flat(metadata),
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()) + "\n"
}

fn stac_report(
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

    format!(
        "{{\n  \"stac\":\n  {{\n    \"properties\":\n    {{\n      \"datetime\": \"{}\",\n      \"pc:count\": {},\n      \"pc:encoding\": \"{}\",\n      \"pc:type\": \"{}\"\n    }}\n  }}\n}}\n",
        stac_datetime(metadata),
        views.iter().map(PointView::len).sum::<u64>(),
        stac_encoding(filename),
        pc_type
    )
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

fn point_report(views: &[PointView], point_ids: &[PointId]) -> String {
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

fn query_report(views: &[PointView], query: QueryRequest) -> String {
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

fn schema_report(views: &[PointView]) -> String {
    format!("{{\n  \"schema\":\n{}\n}}\n", schema_body(views, 2))
}

fn schema_body(views: &[PointView], indent: usize) -> String {
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

fn stats_report(
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

fn stats_body(
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

mod stats;
use stats::{breakout_body, dim_type_name, dimension_stats, format_number, format_point_value};

pub(super) unsafe fn argv_to_vec(
    argc: i32,
    argv: *const *const c_char,
) -> Result<Vec<String>, i32> {
    let mut args = Vec::new();
    for i in 0..argc {
        let arg = *argv.add(i as usize);
        if arg.is_null() {
            return Err(1);
        }
        args.push(CStr::from_ptr(arg).to_string_lossy().into_owned());
    }
    Ok(args)
}

#[allow(dead_code)]
fn _assert_result_abi_shape(_: pdal_pipeline_result_t) {}

#[cfg(test)]
mod tests;
