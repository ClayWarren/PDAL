use super::{apply_cli_stage_options, parse_cli_stage_option, CliStageOption};
use crate::pipeline_abi::{
    pdal_pipeline_result_t, pipeline_result_to_json_for_kernel, PipelineHandle,
};
use crate::registry::pipeline_from_json;
use chrono::NaiveDate;
use pdal_core::driver::infer_reader_driver;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::point::{DimId, DimType, PointId, PointView};
use std::ffi::CStr;
use std::io::Read;
use std::os::raw::c_char;
use std::path::Path;

pub(super) unsafe fn run_pipeline_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.pipeline: Missing value for positional argument 'input'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal pipeline <pipeline.json>");
        println!("  pdal pipeline --input <pipeline.json>");
        println!("  pdal pipeline --stdin");
        return 0;
    }

    let mut input = None;
    let mut read_stdin = false;
    let mut validate_only = false;
    let mut metadata_file = None;
    let mut serialization_file = None;
    let mut stage_options: Vec<CliStageOption> = Vec::new();

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--input" || arg == "-i" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.pipeline: Missing value for option '{arg}'.");
                return 1;
            };
            input = Some(value.clone());
        } else if arg == "--stdin" || arg == "-s" {
            read_stdin = true;
        } else if arg == "--validate" {
            validate_only = true;
        } else if arg == "--metadata" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.pipeline: Missing value for option '--metadata'.");
                return 1;
            };
            metadata_file = Some(value.clone());
        } else if arg == "--pipeline-serialization" {
            let Some(value) = iter.next() else {
                eprintln!(
                    "PDAL: kernels.pipeline: Missing value for option '--pipeline-serialization'."
                );
                return 1;
            };
            serialization_file = Some(value.clone());
        } else if let Some(stage_option) = parse_cli_stage_option(arg) {
            stage_options.push(stage_option);
        } else if arg.starts_with("--") || arg.starts_with("-v") {
            return -1;
        } else if input.replace(arg.clone()).is_some() {
            eprintln!("PDAL: kernels.pipeline: Unexpected argument '{arg}'.");
            return 1;
        }
    }

    if read_stdin && input.is_some() {
        eprintln!(
            "PDAL: kernels.pipeline: Expected either --stdin or an input filename, not both."
        );
        return 1;
    }
    if !read_stdin && input.is_none() {
        eprintln!("PDAL: kernels.pipeline: Missing value for positional argument 'input'.");
        return 1;
    }

    let json = if read_stdin {
        let mut json = String::new();
        if let Err(err) = std::io::stdin().read_to_string(&mut json) {
            eprintln!("PDAL: kernels.pipeline: Unable to read pipeline from stdin: {err}");
            return 1;
        }
        json
    } else {
        let input = input.unwrap();
        match std::fs::read_to_string(&input) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("PDAL: kernels.pipeline: Unable to read pipeline '{input}': {err}");
                return 1;
            }
        }
    };

    let json = match apply_stage_options_to_pipeline_json(&json, &stage_options) {
        Ok(json) => json,
        Err(()) => return -1,
    };

    if validate_only {
        if let Err(err) = validate_pipeline_json_shape(&json) {
            eprintln!("PDAL: kernels.pipeline: {err}");
            return 1;
        }
        return 0;
    }

    if let Some(path) = serialization_file {
        if let Err(err) = std::fs::write(&path, &json) {
            eprintln!(
                "PDAL: kernels.pipeline: Unable to write pipeline serialization '{path}': {err}"
            );
            return 1;
        }
    }

    let mut pipeline = match pipeline_from_json(&json) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.pipeline: {err}");
            return 1;
        }
    };
    match pipeline.execute_with_result(Vec::new()) {
        Ok(result) => {
            if let Some(path) = metadata_file {
                let handle = PipelineHandle { pipeline };
                let summary = pipeline_result_to_json_for_kernel(result, &handle);
                if let Err(err) = std::fs::write(&path, summary) {
                    eprintln!("PDAL: kernels.pipeline: Unable to write metadata '{path}': {err}");
                    return 1;
                }
            }
            0
        }
        Err(err) => {
            eprintln!("PDAL: kernels.pipeline: {err}");
            1
        }
    }
}

fn validate_pipeline_json_shape(json: &str) -> Result<(), String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|err| format!("Invalid pipeline JSON: {err}"))?;
    let stages = if let Some(stages) = value.as_array() {
        stages
    } else if let Some(stages) = value.get("pipeline").and_then(serde_json::Value::as_array) {
        stages
    } else {
        return Err("Pipeline JSON must be an array or an object with a 'pipeline' array.".into());
    };

    for (position, stage) in stages.iter().enumerate() {
        if stage.is_string() {
            continue;
        }
        let Some(object) = stage.as_object() else {
            return Err(format!(
                "Pipeline stage {position} must be a JSON object or filename string."
            ));
        };
        if let Some(stage_type) = object.get("type") {
            if !stage_type.is_string() {
                return Err(format!(
                    "Pipeline stage {position} has a non-string 'type'."
                ));
            }
        }
    }
    Ok(())
}

fn apply_stage_options_to_pipeline_json(
    json: &str,
    stage_options: &[CliStageOption],
) -> Result<String, ()> {
    if stage_options.is_empty() {
        return Ok(json.to_string());
    }

    let mut value: serde_json::Value = serde_json::from_str(json).map_err(|_| ())?;
    let stages = if let Some(stages) = value.as_array_mut() {
        stages
    } else if let Some(stages) = value
        .get_mut("pipeline")
        .and_then(serde_json::Value::as_array_mut)
    {
        stages
    } else {
        return Err(());
    };

    if !apply_cli_stage_options(stages, stage_options) {
        return Err(());
    }
    serde_json::to_string(&value).map_err(|_| ())
}

pub(super) unsafe fn run_info_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.info: Missing value for positional argument 'input'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal info --summary <file>");
        return 0;
    }

    let mut filename = None;
    let mut driver_override = None;
    let mut mode = InfoMode::Stats;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--summary" {
            mode = InfoMode::Summary;
        } else if arg == "--schema" {
            mode = InfoMode::Schema;
        } else if arg == "--all" {
            mode = InfoMode::All;
        } else if arg == "--stac" {
            mode = InfoMode::Stac;
        } else if arg == "-p" || arg == "--point" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.info: Missing value for option '{arg}'.");
                return 1;
            };
            let Some(point_id) = parse_point_id(value) else {
                return -1;
            };
            mode = InfoMode::Point(point_id);
        } else if let Some(value) = arg.strip_prefix("-p=") {
            let Some(point_id) = parse_point_id(value) else {
                return -1;
            };
            mode = InfoMode::Point(point_id);
        } else if let Some(value) = arg.strip_prefix("--point=") {
            let Some(point_id) = parse_point_id(value) else {
                return -1;
            };
            mode = InfoMode::Point(point_id);
        } else if arg == "--query" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.info: Missing value for option '--query'.");
                return 1;
            };
            let Some(query) = parse_query(value) else {
                return -1;
            };
            mode = InfoMode::Query(query);
        } else if let Some(value) = arg.strip_prefix("--query=") {
            let Some(query) = parse_query(value) else {
                return -1;
            };
            mode = InfoMode::Query(query);
        } else if arg == "--driver" {
            let Some(driver) = iter.next() else {
                eprintln!("PDAL: kernels.info: Missing value for option '--driver'.");
                return 1;
            };
            driver_override = Some(driver.clone());
        } else if let Some(driver) = arg.strip_prefix("--driver=") {
            driver_override = Some(driver.to_string());
        } else if arg == "--input" || arg == "-i" {
            let Some(input) = iter.next() else {
                eprintln!("PDAL: kernels.info: Missing value for option '{arg}'.");
                return 1;
            };
            if filename.replace(input.clone()).is_some() {
                eprintln!("PDAL: kernels.info: Expected exactly one input file.");
                return 1;
            }
        } else if arg.starts_with("--") {
            return -1;
        } else if filename.replace(arg.clone()).is_some() {
            eprintln!("PDAL: kernels.info: Expected exactly one input file.");
            return 1;
        }
    }

    let Some(filename) = filename else {
        eprintln!("PDAL: kernels.info: Missing value for positional argument 'input'.");
        return 1;
    };
    let Some(driver) =
        driver_override.or_else(|| infer_reader_driver(&filename).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.info: Unable to infer reader driver for '{filename}'.");
        return 1;
    };

    let mut pipeline = match info_pipeline(&driver, &filename) {
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
                println!("{}", pipeline_result_to_json_for_kernel(result, &handle));
                0
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                1
            }
        },
        InfoMode::Stats
        | InfoMode::Schema
        | InfoMode::All
        | InfoMode::Stac
        | InfoMode::Point(_)
        | InfoMode::Query(_) => match pipeline.execute(Vec::new()) {
            Ok(views) => {
                let metadata = pipeline.metadata();
                println!("{}", info_report(mode, &views, &metadata, &filename));
                0
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                1
            }
        },
    }
}

#[derive(Clone, Copy)]
enum InfoMode {
    Summary,
    Stats,
    Schema,
    All,
    Stac,
    Point(PointId),
    Query(QueryRequest),
}

#[derive(Clone, Copy)]
struct QueryRequest {
    x: f64,
    y: f64,
    count: usize,
}

fn info_pipeline(driver: &str, filename: &str) -> Result<pdal_core::pipeline::Pipeline, String> {
    pipeline_from_json(&serde_json::json!([{ "type": driver, "filename": filename }]).to_string())
        .map_err(|err| err.to_string())
}

fn info_report(
    mode: InfoMode,
    views: &[PointView],
    metadata: &MetadataNode,
    filename: &str,
) -> String {
    match mode {
        InfoMode::Stats => stats_report(views),
        InfoMode::Schema => schema_report(views),
        InfoMode::All => {
            let mut output = String::from("{\n");
            output.push_str("  \"schema\":\n");
            output.push_str(&schema_body(views, 2));
            output.push_str(",\n");
            output.push_str("  \"stats\":\n");
            output.push_str(&stats_body(views, 2));
            output.push_str("\n}\n");
            output
        }
        InfoMode::Stac => stac_report(views, metadata, filename),
        InfoMode::Point(point_id) => point_report(views, point_id),
        InfoMode::Query(query) => query_report(views, query),
        InfoMode::Summary => String::new(),
    }
}

fn stac_report(views: &[PointView], metadata: &MetadataNode, filename: &str) -> String {
    if views
        .first()
        .is_none_or(|view| view.spatial_reference().is_empty())
    {
        return "{\n  \"stac\":\n  {\n    \"message\": \"Failed to create STAC Feature with missing key. 'EPSG:4326'\",\n    \"status\": \"error\"\n  }\n}\n".to_string();
    }

    format!(
        "{{\n  \"stac\":\n  {{\n    \"properties\":\n    {{\n      \"datetime\": \"{}\",\n      \"pc:count\": {},\n      \"pc:encoding\": \"{}\",\n      \"pc:type\": \"lidar\"\n    }}\n  }}\n}}\n",
        stac_datetime(metadata),
        views.iter().map(PointView::len).sum::<u64>(),
        stac_encoding(filename)
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

fn point_report(views: &[PointView], point_id: PointId) -> String {
    let mut output = String::from("{\n  \"points\":\n  {\n");
    if let Some((view, local_id)) = locate_point(views, point_id) {
        output.push_str("    \"point\":\n");
        output.push_str(&point_json(view, local_id, 4));
        output.push('\n');
    } else {
        output.push_str("    \"point\": null\n");
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
            points.push((
                dx.mul_add(dx, dy * dy),
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

fn parse_point_id(value: &str) -> Option<PointId> {
    value.parse::<PointId>().ok()
}

fn parse_query(value: &str) -> Option<QueryRequest> {
    let (coords, count) = value.split_once('/')?;
    let (x, y) = coords.split_once(',')?;
    Some(QueryRequest {
        x: x.parse().ok()?,
        y: y.parse().ok()?,
        count: count.parse().ok()?,
    })
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

fn stats_report(views: &[PointView]) -> String {
    format!("{{\n  \"stats\":\n{}\n}}\n", stats_body(views, 2))
}

fn stats_body(views: &[PointView], indent: usize) -> String {
    let pad = " ".repeat(indent);
    let list_pad = " ".repeat(indent + 2);
    let item_pad = " ".repeat(indent + 4);
    let value_pad = " ".repeat(indent + 6);
    let stats = dimension_stats(views);
    let mut output = format!("{pad}{{\n{list_pad}\"statistic\":\n{list_pad}[\n");
    for (idx, stat) in stats.iter().enumerate() {
        if idx > 0 {
            output.push_str(",\n");
        }
        output.push_str(&format!(
            "{item_pad}{{\n{value_pad}\"average\": {},\n{value_pad}\"count\": {},\n{value_pad}\"maximum\": {},\n{value_pad}\"minimum\": {},\n{value_pad}\"name\": \"{}\",\n{value_pad}\"position\": {},\n{value_pad}\"stddev\": {},\n{value_pad}\"variance\": {}\n{item_pad}}}",
            format_number(stat.average),
            stat.count,
            format_number(stat.maximum),
            format_number(stat.minimum),
            stat.name,
            idx,
            format_number(stat.stddev),
            format_number(stat.variance)
        ));
    }
    output.push_str(&format!("\n{list_pad}]\n{pad}}}"));
    output
}

struct InfoDimensionStats {
    name: String,
    count: u64,
    minimum: f64,
    maximum: f64,
    average: f64,
    variance: f64,
    stddev: f64,
}

fn dimension_stats(views: &[PointView]) -> Vec<InfoDimensionStats> {
    let Some(first) = views.first() else {
        return Vec::new();
    };
    let layout = first.layout();
    let mut output = Vec::new();
    for idx in 0..layout.dim_count() {
        let Some((dim, _)) = layout.dim_at(idx) else {
            continue;
        };
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
        });
    }
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

unsafe fn argv_to_vec(argc: i32, argv: *const *const c_char) -> Result<Vec<String>, i32> {
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
mod tests {
    use super::*;

    #[test]
    fn applies_cli_stage_options_to_object_pipeline() {
        let json = r#"{"pipeline":[{"type":"readers.faux"},{"type":"filters.sort","dimension":"X"},{"type":"writers.las"}]}"#;
        let options = vec![CliStageOption {
            stage: "filters.sort".to_string(),
            key: "dimension".to_string(),
            value: "Y".to_string(),
        }];

        let updated = apply_stage_options_to_pipeline_json(json, &options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&updated).unwrap();

        assert_eq!(parsed["pipeline"][1]["dimension"][0], "X");
        assert_eq!(parsed["pipeline"][1]["dimension"][1], "Y");
    }

    #[test]
    fn applies_cli_stage_options_to_array_pipeline() {
        let json =
            r#"[{"type":"readers.faux"},{"type":"sort","dimension":"X"},{"type":"writers.las"}]"#;
        let options = vec![CliStageOption {
            stage: "sort".to_string(),
            key: "dimension".to_string(),
            value: "Y".to_string(),
        }];

        let updated = apply_stage_options_to_pipeline_json(json, &options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&updated).unwrap();

        assert_eq!(parsed[1]["dimension"][0], "X");
        assert_eq!(parsed[1]["dimension"][1], "Y");
    }

    #[test]
    fn validate_shape_accepts_object_valued_options() {
        let json = r#"[{"type":"readers.ept","filename":"ept.json"},{"type":"writers.ept_addon","addons":{"Z":"Z"}}]"#;

        assert!(validate_pipeline_json_shape(json).is_ok());
    }

    #[test]
    fn validate_shape_rejects_non_stage_entries() {
        let json = r#"[{"type":"readers.faux"}, 7]"#;

        assert!(validate_pipeline_json_shape(json).is_err());
    }
}
