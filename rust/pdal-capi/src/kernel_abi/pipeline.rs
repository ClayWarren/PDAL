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
use std::fs::File;
use std::io::Read;
use std::io::Write;
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
    let mut progress_file = None;
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
        } else if arg == "--stream" || arg == "--nostream" {
            // Accepted for C++ shell parity. The Rust pipeline executor still
            // chooses its single execution path while stream-mode parity is
            // tracked separately in STATUS.md.
        } else if arg == "--dims" {
            let Some(_) = iter.next() else {
                eprintln!("PDAL: kernels.pipeline: Missing value for option '--dims'.");
                return 1;
            };
        } else if arg == "--progress" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.pipeline: Missing value for option '{arg}'.");
                return 1;
            };
            progress_file = Some(value.clone());
        } else if arg == "--pointcloudschema" {
            let Some(_) = iter.next() else {
                eprintln!("PDAL: kernels.pipeline: Missing value for option '{arg}'.");
                return 1;
            };
            return -1;
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

    let mut progress = match open_progress_file(progress_file.as_deref()) {
        Ok(progress) => progress,
        Err(()) => return 1,
    };

    if validate_only {
        let validation = validate_pipeline_for_kernel(&json);
        println!("{}", serde_json::to_string_pretty(&validation).unwrap());
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
            write_progress(&mut progress, "DONEPIPELINE", "pipeline");
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

fn open_progress_file(path: Option<&str>) -> Result<Option<File>, ()> {
    let Some(path) = path else {
        return Ok(None);
    };
    match std::fs::OpenOptions::new().write(true).open(path) {
        Ok(file) => {
            let mut progress = Some(file);
            write_progress(&mut progress, "READYPIPELINE", "pipeline");
            Ok(progress)
        }
        Err(_) => {
            eprintln!("Can't open progress file '{path}'.");
            Err(())
        }
    }
}

fn write_progress(file: &mut Option<File>, event: &str, text: &str) {
    if let Some(file) = file {
        let _ = writeln!(file, "{event}:{text}");
    }
}

fn validate_pipeline_for_kernel(json: &str) -> serde_json::Value {
    match validate_pipeline_json_shape(json).and_then(|_| {
        let pipeline = pipeline_from_json(json).map_err(|err| err.to_string())?;
        if !pipeline.has_reader() {
            return Err("Pipeline does not start with a reader.".to_string());
        }
        Ok(())
    }) {
        Ok(()) => serde_json::json!({
            "valid": true,
            "error_detail": "",
            "streamable": true,
        }),
        Err(err) => serde_json::json!({
            "valid": false,
            "error_detail": err,
            "streamable": false,
        }),
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
    let mut mode = InfoMode::Stats {
        dimensions: None,
        enumerate: None,
        breakout: None,
    };
    let mut pc_type = "lidar".to_string();
    let mut serialization_file = None;
    let mut read_stdin = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--summary" {
            mode = InfoMode::Summary;
        } else if arg == "--stats" {
            mode = InfoMode::Stats {
                dimensions: None,
                enumerate: None,
                breakout: None,
            };
        } else if arg == "--schema" {
            mode = InfoMode::Schema;
        } else if arg == "--metadata" {
            mode = InfoMode::Metadata;
        } else if arg == "--all" {
            mode = InfoMode::All;
        } else if arg == "--boundary" {
            mode = InfoMode::Boundary;
        } else if arg == "--stac" {
            mode = InfoMode::Stac;
        } else if arg == "-p" || arg == "--point" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.info: Missing value for option '{arg}'.");
                return 1;
            };
            let Some(point_ids) = parse_point_spec(value) else {
                return -1;
            };
            mode = InfoMode::Points(point_ids);
        } else if let Some(value) = arg.strip_prefix("-p=") {
            let Some(point_ids) = parse_point_spec(value) else {
                return -1;
            };
            mode = InfoMode::Points(point_ids);
        } else if let Some(value) = arg.strip_prefix("--point=") {
            let Some(point_ids) = parse_point_spec(value) else {
                return -1;
            };
            mode = InfoMode::Points(point_ids);
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
        } else if arg == "--dimensions" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.info: Missing value for option '--dimensions'.");
                return 1;
            };
            mode = InfoMode::Stats {
                dimensions: Some(parse_dimension_list(value)),
                enumerate: match mode.clone() {
                    InfoMode::Stats { enumerate, .. } => enumerate,
                    _ => None,
                },
                breakout: match mode.clone() {
                    InfoMode::Stats { breakout, .. } => breakout,
                    _ => None,
                },
            };
        } else if let Some(value) = arg.strip_prefix("--dimensions=") {
            mode = InfoMode::Stats {
                dimensions: Some(parse_dimension_list(value)),
                enumerate: match mode.clone() {
                    InfoMode::Stats { enumerate, .. } => enumerate,
                    _ => None,
                },
                breakout: match mode.clone() {
                    InfoMode::Stats { breakout, .. } => breakout,
                    _ => None,
                },
            };
        } else if arg == "--enumerate" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.info: Missing value for option '--enumerate'.");
                return 1;
            };
            mode = InfoMode::Stats {
                dimensions: match mode.clone() {
                    InfoMode::Stats { dimensions, .. } => dimensions,
                    _ => None,
                },
                enumerate: Some(parse_dimension_list(value)),
                breakout: match mode.clone() {
                    InfoMode::Stats { breakout, .. } => breakout,
                    _ => None,
                },
            };
        } else if let Some(value) = arg.strip_prefix("--enumerate=") {
            mode = InfoMode::Stats {
                dimensions: match mode.clone() {
                    InfoMode::Stats { dimensions, .. } => dimensions,
                    _ => None,
                },
                enumerate: Some(parse_dimension_list(value)),
                breakout: match mode.clone() {
                    InfoMode::Stats { breakout, .. } => breakout,
                    _ => None,
                },
            };
        } else if arg == "--breakout" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.info: Missing value for option '--breakout'.");
                return 1;
            };
            mode = InfoMode::Stats {
                dimensions: match mode.clone() {
                    InfoMode::Stats { dimensions, .. } => dimensions,
                    _ => None,
                },
                enumerate: match mode.clone() {
                    InfoMode::Stats { enumerate, .. } => enumerate,
                    _ => None,
                },
                breakout: Some(DimId::from_name(value)),
            };
        } else if let Some(value) = arg.strip_prefix("--breakout=") {
            mode = InfoMode::Stats {
                dimensions: match mode.clone() {
                    InfoMode::Stats { dimensions, .. } => dimensions,
                    _ => None,
                },
                enumerate: match mode.clone() {
                    InfoMode::Stats { enumerate, .. } => enumerate,
                    _ => None,
                },
                breakout: Some(DimId::from_name(value)),
            };
        } else if arg == "--pc_type" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.info: Missing value for option '--pc_type'.");
                return 1;
            };
            pc_type = value.clone();
        } else if let Some(value) = arg.strip_prefix("--pc_type=") {
            pc_type = value.to_string();
        } else if arg == "--pipeline-serialization" {
            let Some(path) = iter.next() else {
                eprintln!(
                    "PDAL: kernels.info: Missing value for option '--pipeline-serialization'."
                );
                return 1;
            };
            serialization_file = Some(path.clone());
        } else if let Some(path) = arg.strip_prefix("--pipeline-serialization=") {
            serialization_file = Some(path.to_string());
        } else if arg == "--driver" {
            let Some(driver) = iter.next() else {
                eprintln!("PDAL: kernels.info: Missing value for option '--driver'.");
                return 1;
            };
            driver_override = Some(driver.clone());
        } else if let Some(driver) = arg.strip_prefix("--driver=") {
            driver_override = Some(driver.to_string());
        } else if arg == "--stdin" || arg == "-s" {
            read_stdin = true;
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

    if read_stdin && filename.is_some() {
        eprintln!("PDAL: kernels.info: Expected either --stdin or an input filename, not both.");
        return 1;
    }

    if read_stdin {
        let mut json = String::new();
        if let Err(err) = std::io::stdin().read_to_string(&mut json) {
            eprintln!("PDAL: kernels.info: Unable to read pipeline from stdin: {err}");
            return 1;
        }
        return run_info_pipeline_json(&json, mode, &pc_type, serialization_file);
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
                println!("{}", pipeline_result_to_json_for_kernel(result, &handle));
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
                println!("{}", pipeline_result_to_json_for_kernel(result, &handle));
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

#[derive(Clone)]
enum InfoMode {
    Summary,
    Stats {
        dimensions: Option<Vec<DimId>>,
        enumerate: Option<Vec<DimId>>,
        breakout: Option<DimId>,
    },
    Schema,
    Metadata,
    All,
    Boundary,
    Stac,
    Points(Vec<PointId>),
    Query(QueryRequest),
}

impl InfoMode {
    fn needs_boundary(&self) -> bool {
        matches!(self, Self::All | Self::Boundary)
    }
}

#[derive(Clone, Copy)]
struct QueryRequest {
    x: f64,
    y: f64,
    z: Option<f64>,
    count: usize,
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

fn parse_dimension_list(value: &str) -> Vec<DimId> {
    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(DimId::from_name)
        .collect()
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

fn parse_point_spec(value: &str) -> Option<Vec<PointId>> {
    let mut ids = Vec::new();
    for part in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some((start, end)) = part.split_once('-') {
            let start = start.parse::<PointId>().ok()?;
            let end = end.parse::<PointId>().ok()?;
            if end < start {
                return None;
            }
            ids.extend(start..=end);
        } else {
            ids.push(part.parse::<PointId>().ok()?);
        }
    }
    (!ids.is_empty()).then_some(ids)
}

fn parse_query(value: &str) -> Option<QueryRequest> {
    let (coords, count) = value.split_once('/')?;
    let parts: Vec<&str> = coords.split(',').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    Some(QueryRequest {
        x: parts[0].parse().ok()?,
        y: parts[1].parse().ok()?,
        z: parts.get(2).map(|z| z.parse()).transpose().ok()?,
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

struct InfoDimensionStats {
    name: String,
    count: u64,
    minimum: f64,
    maximum: f64,
    average: f64,
    variance: f64,
    stddev: f64,
    values: Option<Vec<f64>>,
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
    use pdal_core::point::PointLayout;
    use pdal_core::srs::SpatialReference;
    use std::rc::Rc;

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

    #[test]
    fn stac_report_uses_requested_pointcloud_type() {
        let layout = Rc::new(PointLayout::new());
        let mut view = PointView::new(layout);
        view.set_spatial_reference(SpatialReference::new("EPSG:4326"));
        view.add_point();

        let report = stac_report(&[view], &MetadataNode::new("root"), "sample.las", "sonar");
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();

        assert_eq!(json["stac"]["properties"]["pc:type"], "sonar");
    }
}
