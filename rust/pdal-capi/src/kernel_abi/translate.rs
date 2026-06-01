use super::{apply_cli_stage_options, argv_to_vec, parse_cli_stage_option, CliStageOption};
use crate::registry::pipeline_from_json;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use std::fs;
use std::os::raw::c_char;

pub(super) unsafe fn run_translate_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.translate: Missing value for positional argument 'input'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal translate <input> <output> [filter ...] [--<stage>.<key>=<value> ...]");
        return 0;
    }

    let mut input = None;
    let mut output = None;
    let mut reader_override = None;
    let mut writer_override = None;
    let mut filters = Vec::new();
    let mut stage_options = Vec::new();
    let mut metadata_file = None;
    let mut serialization_file = None;
    let mut filter_json = None;
    let mut stream_allowed = true;
    let mut stream_required = false;
    let mut overwrite = false;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--input" || arg == "-i" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            input = Some(value.clone());
        } else if arg == "--output" || arg == "-o" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            output = Some(value.clone());
        } else if arg == "--reader" || arg == "-r" || arg == "--driver" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            reader_override = Some(value.clone());
        } else if arg == "--writer" || arg == "-w" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            writer_override = Some(value.clone());
        } else if arg == "--filter" || arg == "-f" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            filters.push(value.clone());
        } else if arg == "--metadata" || arg == "-m" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            metadata_file = Some(value.clone());
        } else if arg == "--pipeline" || arg == "-p" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            serialization_file = Some(value.clone());
        } else if arg == "--stream" {
            if !stream_allowed {
                eprintln!(
                    "PDAL: kernels.translate: Can't specify both 'stream' and 'nostream' options."
                );
                return 1;
            }
            stream_allowed = true;
            stream_required = true;
        } else if arg == "--nostream" {
            if stream_required {
                eprintln!(
                    "PDAL: kernels.translate: Can't specify both 'stream' and 'nostream' options."
                );
                return 1;
            }
            stream_allowed = false;
        } else if arg == "--overwrite" {
            overwrite = true;
        } else if arg == "--dims" {
            let Some(_) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '--dims'.");
                return 1;
            };
        } else if arg == "--json" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '--json'.");
                return 1;
            };
            filter_json = Some(value.clone());
        } else if let Some(value) = arg.strip_prefix("--json=") {
            filter_json = Some(value.to_string());
        } else if arg.starts_with("--") {
            match parse_cli_stage_option(arg) {
                Some(option) => stage_options.push(option),
                None => return -1,
            }
        } else if input.is_none() {
            input = Some(arg.clone());
        } else if output.is_none() {
            output = Some(arg.clone());
        } else {
            filters.push(arg.clone());
        }
    }

    let Some(input) = input else {
        eprintln!("PDAL: kernels.translate: Missing value for positional argument 'input'.");
        return 1;
    };
    let Some(output) = output else {
        eprintln!("PDAL: kernels.translate: Missing value for positional argument 'output'.");
        return 1;
    };
    if filter_json.is_some() && !filters.is_empty() {
        eprintln!("PDAL: kernels.translate: Cannot set both --filter options and --json options");
        return 1;
    }
    if input == output && !overwrite {
        eprintln!(
            "PDAL: kernels.translate: Input and output filenames are equal and no --overwrite option was provided!"
        );
        return 1;
    }
    let Some(reader) = reader_override.or_else(|| infer_reader_driver(&input).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.translate: Unable to infer reader driver for '{input}'.");
        return 1;
    };
    let Some(writer) = writer_override.or_else(|| infer_writer_driver(&output).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.translate: Unable to infer writer driver for '{output}'.");
        return 1;
    };

    let mut stages = if let Some(json) = filter_json {
        match translate_json_stages(&json, &input, &output, &reader, &writer) {
            Ok(stages) => stages,
            Err(message) => {
                eprintln!("PDAL: kernels.translate: {message}");
                return 1;
            }
        }
    } else {
        let mut stages = Vec::new();
        stages.push(serde_json::json!({ "type": reader, "filename": input }));
        for filter in filters {
            let stage_type = if filter.contains('.') {
                filter
            } else {
                format!("filters.{filter}")
            };
            stages.push(serde_json::json!({ "type": stage_type }));
        }
        stages.push(serde_json::json!({ "type": writer, "filename": output }));
        stages
    };

    let stage_options = match expand_translate_option_files(stage_options) {
        Ok(options) => options,
        Err(code) => return code,
    };
    if !apply_cli_stage_options(&mut stages, &stage_options) {
        return -1;
    }
    execute_translate_pipeline(
        stages,
        metadata_file,
        serialization_file,
        stream_allowed,
        stream_required,
    )
}

pub(super) fn translate_json_stages(
    json_arg: &str,
    input: &str,
    output: &str,
    reader: &str,
    writer: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let json = fs::read_to_string(json_arg).unwrap_or_else(|_| json_arg.to_string());
    let value: serde_json::Value =
        serde_json::from_str(&json).map_err(|err| format!("Invalid pipeline JSON: {err}"))?;
    let stages = if let Some(array) = value.as_array() {
        array.clone()
    } else if let Some(array) = value.get("pipeline").and_then(serde_json::Value::as_array) {
        array.clone()
    } else {
        return Err("Pipeline JSON object must contain a 'pipeline' array.".to_string());
    };

    let mut rewritten = Vec::new();
    let mut replaced_reader = false;
    let mut replaced_writer = false;
    for (position, stage) in stages.iter().enumerate() {
        if !replaced_reader && is_reader_stage(stage, position, stages.len()) {
            rewritten.push(replacement_stage(stage, reader, input));
            replaced_reader = true;
        } else if !replaced_writer && is_writer_stage(stage, position, stages.len()) {
            rewritten.push(replacement_stage(stage, writer, output));
            replaced_writer = true;
        } else {
            rewritten.push(stage.clone());
        }
    }

    if !replaced_reader {
        rewritten.insert(0, serde_json::json!({ "type": reader, "filename": input }));
    }
    if !replaced_writer {
        rewritten.push(serde_json::json!({ "type": writer, "filename": output }));
    }

    Ok(rewritten)
}

fn replacement_stage(
    original: &serde_json::Value,
    driver: &str,
    filename: &str,
) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    object.insert(
        "type".to_string(),
        serde_json::Value::String(driver.to_string()),
    );
    object.insert(
        "filename".to_string(),
        serde_json::Value::String(filename.to_string()),
    );
    if let Some(tag) = original.get("tag").cloned() {
        object.insert("tag".to_string(), tag);
    }
    serde_json::Value::Object(object)
}

fn is_reader_stage(stage: &serde_json::Value, position: usize, len: usize) -> bool {
    if let Some(driver) = stage.get("type").and_then(serde_json::Value::as_str) {
        return driver.starts_with("readers.");
    }
    let has_filename = stage.as_str().is_some()
        || stage
            .get("filename")
            .and_then(serde_json::Value::as_str)
            .is_some();
    has_filename && position == 0 && len > 1
}

fn is_writer_stage(stage: &serde_json::Value, position: usize, len: usize) -> bool {
    if let Some(driver) = stage.get("type").and_then(serde_json::Value::as_str) {
        return driver.starts_with("writers.");
    }
    let has_filename = stage.as_str().is_some()
        || stage
            .get("filename")
            .and_then(serde_json::Value::as_str)
            .is_some();
    has_filename && position + 1 == len
}

fn execute_translate_pipeline(
    stages: Vec<serde_json::Value>,
    metadata_file: Option<String>,
    serialization_file: Option<String>,
    stream_allowed: bool,
    stream_required: bool,
) -> i32 {
    let stage_types = translate_stage_types(&stages);
    let pipeline_json = serde_json::Value::Array(stages);
    if let Some(path) = serialization_file {
        if let Err(err) = std::fs::write(&path, pipeline_json.to_string()) {
            eprintln!(
                "PDAL: kernels.translate: Unable to write pipeline serialization '{path}': {err}"
            );
            return 1;
        }
        return 0;
    }

    let mut pipeline = match pipeline_from_json(&pipeline_json.to_string()) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.translate: {err}");
            return 1;
        }
    };

    if stream_allowed && metadata_file.is_none() {
        match pipeline.execute_streaming() {
            Ok(Some(_)) => return 0,
            Ok(None) if stream_required => {
                eprintln!("PDAL: kernels.translate: Pipeline is not streamable.");
                return 1;
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("PDAL: kernels.translate: {err}");
                return 1;
            }
        }
    }

    match pipeline.execute_with_result(Vec::new()) {
        Ok(result) => {
            if let Some(path) = metadata_file {
                let handle = crate::pipeline_abi::PipelineHandle { pipeline };
                let mut summary = serde_json::from_str::<serde_json::Value>(
                    &crate::pipeline_abi::pipeline_result_to_json_for_kernel(result, &handle),
                )
                .unwrap_or_else(|_| serde_json::json!({}));
                summary["stages"] = serde_json::Value::Array(
                    stage_types
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                );
                let summary =
                    serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string());
                if let Err(err) = std::fs::write(&path, summary) {
                    eprintln!("PDAL: kernels.translate: Unable to write metadata '{path}': {err}");
                    return 1;
                }
            }
            0
        }
        Err(err) => {
            eprintln!("PDAL: kernels.translate: {err}");
            1
        }
    }
}

fn translate_stage_types(stages: &[serde_json::Value]) -> Vec<String> {
    stages
        .iter()
        .filter_map(|stage| {
            stage
                .get("type")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect()
}

pub(super) fn expand_translate_option_files(
    options: Vec<CliStageOption>,
) -> Result<Vec<CliStageOption>, i32> {
    let mut expanded = Vec::new();
    for option in options {
        if option.key != "option_file" {
            expanded.push(option);
            continue;
        }
        let text = match fs::read_to_string(&option.value) {
            Ok(text) => text,
            Err(_) => {
                eprintln!("Can't read {}", option.value);
                return Err(1);
            }
        };
        let loaded = match parse_option_file(&option.stage, &text) {
            Ok(loaded) => loaded,
            Err(message) => {
                eprintln!("{message}");
                return Err(1);
            }
        };
        expanded.extend(loaded);
    }
    Ok(expanded)
}

pub(super) fn parse_option_file(stage: &str, text: &str) -> Result<Vec<CliStageOption>, String> {
    let trimmed = text.trim();
    if trimmed.starts_with('{') {
        let value: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|_| "Unexpected argument".to_string())?;
        let object = value
            .as_object()
            .ok_or_else(|| "Unexpected argument".to_string())?;
        return object
            .iter()
            .map(|(key, value)| {
                validate_translate_option_file_key(stage, key)?;
                Ok(CliStageOption {
                    stage: stage.to_string(),
                    key: key.clone(),
                    value: option_file_value_to_string(value)?,
                })
            })
            .collect();
    }

    trimmed
        .split_whitespace()
        .map(|arg| {
            let Some(spec) = arg.strip_prefix("--") else {
                return Err("Unexpected argument".to_string());
            };
            let Some((key, value)) = spec.split_once('=') else {
                return Err("Unexpected argument".to_string());
            };
            validate_translate_option_file_key(stage, key)?;
            Ok(CliStageOption {
                stage: stage.to_string(),
                key: key.to_string(),
                value: value.to_string(),
            })
        })
        .collect()
}

fn validate_translate_option_file_key(stage: &str, key: &str) -> Result<(), String> {
    if stage == "filters.range" && key == "limits" {
        return Ok(());
    }
    Err("Unexpected argument".to_string())
}

fn option_file_value_to_string(value: &serde_json::Value) -> Result<String, String> {
    match value {
        serde_json::Value::String(value) => Ok(value.clone()),
        serde_json::Value::Bool(value) => Ok(value.to_string()),
        serde_json::Value::Number(value) => Ok(value.to_string()),
        _ => Err("Unexpected argument".to_string()),
    }
}
