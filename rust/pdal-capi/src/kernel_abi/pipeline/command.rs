use super::super::{apply_cli_stage_options, parse_cli_stage_option, CliStageOption};
use super::argv_to_vec;
use crate::pipeline_abi::{pipeline_result_to_json_for_kernel, PipelineHandle};
use crate::registry::pipeline_from_json;
use std::fs::File;
use std::io::{Read, Write};
use std::os::raw::c_char;

pub(in crate::kernel_abi) unsafe fn run_pipeline_kernel(
    argc: i32,
    argv: *const *const c_char,
) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    let parsed = match parse_pipeline_args(&args) {
        PipelineArgsResult::Run(parsed) => parsed,
        PipelineArgsResult::Return(code) => return code,
    };

    let json = if parsed.read_stdin {
        let mut json = String::new();
        if let Err(err) = std::io::stdin().read_to_string(&mut json) {
            eprintln!("PDAL: kernels.pipeline: Unable to read pipeline from stdin: {err}");
            return 1;
        }
        json
    } else {
        let input = parsed.input.expect("input validated");
        match std::fs::read_to_string(&input) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("PDAL: kernels.pipeline: Unable to read pipeline '{input}': {err}");
                return 1;
            }
        }
    };

    let json = match apply_stage_options_to_pipeline_json(&json, &parsed.stage_options) {
        Ok(json) => json,
        Err(()) => return -1,
    };

    let mut progress = match open_progress_file(parsed.progress_file.as_deref()) {
        Ok(progress) => progress,
        Err(()) => return 1,
    };

    if parsed.validate_only {
        let validation = validate_pipeline_for_kernel(&json);
        println!("{}", serde_json::to_string_pretty(&validation).unwrap());
        return 0;
    }

    if let Some(path) = parsed.serialization_file {
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
    // When no metadata summary is requested, try chunked streaming first
    // (bounded peak memory). `Ok(None)` means the pipeline is not streaming-
    // eligible -- fall through to the materializing path with no side effects.
    if parsed.stream_allowed && parsed.metadata_file.is_none() && !parsed.summary_stdout {
        match pipeline.execute_streaming() {
            Ok(Some(_)) => {
                write_progress(&mut progress, "DONEPIPELINE", "pipeline");
                return 0;
            }
            Ok(None) if parsed.stream_required => {
                eprintln!(
                    "PDAL: kernels.pipeline: Attempting to use stream mode with a stage that doesn't support streaming."
                );
                return 1;
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("PDAL: kernels.pipeline: {err}");
                return 1;
            }
        }
    }
    match pipeline.execute_with_result(Vec::new()) {
        Ok(result) => {
            write_progress(&mut progress, "DONEPIPELINE", "pipeline");
            if parsed.metadata_file.is_some() || parsed.summary_stdout {
                let handle = PipelineHandle { pipeline };
                let summary = pipeline_result_to_json_for_kernel(result, &handle);
                if let Some(path) = parsed.metadata_file {
                    if let Err(err) = std::fs::write(&path, &summary) {
                        eprintln!(
                            "PDAL: kernels.pipeline: Unable to write metadata '{path}': {err}"
                        );
                        return 1;
                    }
                }
                if parsed.summary_stdout {
                    println!("{summary}");
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

struct ParsedPipelineArgs {
    input: Option<String>,
    read_stdin: bool,
    validate_only: bool,
    metadata_file: Option<String>,
    progress_file: Option<String>,
    serialization_file: Option<String>,
    summary_stdout: bool,
    stream_allowed: bool,
    stream_required: bool,
    stage_options: Vec<CliStageOption>,
}

enum PipelineArgsResult {
    Run(ParsedPipelineArgs),
    Return(i32),
}

fn parse_pipeline_args(args: &[String]) -> PipelineArgsResult {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.pipeline: Missing value for positional argument 'input'.");
            return PipelineArgsResult::Return(1);
        }
        println!("Usage:");
        println!("  pdal pipeline <pipeline.json>");
        println!("  pdal pipeline --input <pipeline.json>");
        println!("  pdal pipeline --stdin");
        return PipelineArgsResult::Return(0);
    }

    let mut parsed = ParsedPipelineArgs {
        input: None,
        read_stdin: false,
        validate_only: false,
        metadata_file: None,
        progress_file: None,
        serialization_file: None,
        summary_stdout: false,
        stream_allowed: true,
        stream_required: false,
        stage_options: Vec::new(),
    };

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Err(code) = parse_pipeline_arg(arg, &mut iter, &mut parsed) {
            return PipelineArgsResult::Return(code);
        }
    }

    if parsed.read_stdin && parsed.input.is_some() {
        eprintln!(
            "PDAL: kernels.pipeline: Expected either --stdin or an input filename, not both."
        );
        return PipelineArgsResult::Return(1);
    }
    if !parsed.read_stdin && parsed.input.is_none() {
        eprintln!("PDAL: kernels.pipeline: Missing value for positional argument 'input'.");
        return PipelineArgsResult::Return(1);
    }

    PipelineArgsResult::Run(parsed)
}

fn parse_pipeline_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut ParsedPipelineArgs,
) -> Result<(), i32> {
    if arg == "--input" || arg == "-i" {
        parsed.input = Some(next_option_value(arg, iter)?.clone());
    } else if arg == "--stdin" || arg == "-s" {
        parsed.read_stdin = true;
    } else if arg == "--validate" {
        parsed.validate_only = true;
    } else if arg == "--showjson" {
        parsed.summary_stdout = true;
    } else if arg == "--stream" {
        if !parsed.stream_allowed {
            eprintln!("PDAL: kernels.pipeline: Can't execute with 'stream' and 'nostream' options");
            return Err(1);
        }
        parsed.stream_allowed = true;
        parsed.stream_required = true;
    } else if arg == "--nostream" {
        if parsed.stream_required {
            eprintln!("PDAL: kernels.pipeline: Can't execute with 'stream' and 'nostream' options");
            return Err(1);
        }
        parsed.stream_allowed = false;
    } else if arg == "--dims" {
        next_option_value("--dims", iter)?;
    } else if arg == "--progress" {
        parsed.progress_file = Some(next_option_value(arg, iter)?.clone());
    } else if arg == "--pointcloudschema" {
        next_option_value(arg, iter)?;
        return Err(-1);
    } else if arg == "--metadata" {
        parsed.metadata_file = Some(next_option_value("--metadata", iter)?.clone());
    } else if arg == "--pipeline-serialization" {
        parsed.serialization_file =
            Some(next_option_value("--pipeline-serialization", iter)?.clone());
    } else if let Some(stage_option) = parse_cli_stage_option(arg) {
        parsed.stage_options.push(stage_option);
    } else if arg.starts_with("--") || arg.starts_with("-v") {
        return Err(-1);
    } else if parsed.input.replace(arg.to_string()).is_some() {
        eprintln!("PDAL: kernels.pipeline: Unexpected argument '{arg}'.");
        return Err(1);
    }
    Ok(())
}

fn next_option_value<'a>(
    option: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<&'a String, i32> {
    match iter.next() {
        Some(value) => Ok(value),
        None => {
            eprintln!("PDAL: kernels.pipeline: Missing value for option '{option}'.");
            Err(1)
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

pub(super) fn validate_pipeline_for_kernel(json: &str) -> serde_json::Value {
    match validate_pipeline_json_shape(json).and_then(|_| {
        let pipeline = pipeline_from_json(json).map_err(|err| err.to_string())?;
        if !pipeline.has_reader() {
            return Err("Pipeline does not start with a reader.".to_string());
        }
        Ok(pipeline.streamable())
    }) {
        Ok(streamable) => serde_json::json!({
            "valid": true,
            "error_detail": "",
            "streamable": streamable,
        }),
        Err(err) => serde_json::json!({
            "valid": false,
            "error_detail": err,
            "streamable": false,
        }),
    }
}

pub(super) fn validate_pipeline_json_shape(json: &str) -> Result<(), String> {
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

pub(super) fn apply_stage_options_to_pipeline_json(
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
