use super::argv_to_vec;
use crate::pipeline_abi::{pipeline_result_to_json_for_kernel, PipelineHandle};
use crate::registry::pipeline_from_json;
use pdal_core::point::DimId;
use pdal_kernels::{
    apply_stage_options_to_pipeline_json, parse_pipeline_args, serialize_pipeline_json,
    validate_pipeline_json_shape, PipelineArgsResult,
};
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
        Err(err) => {
            eprintln!("PDAL: kernels.pipeline: {err}");
            return 1;
        }
    };

    let mut progress = match open_progress_file(parsed.progress_file.as_deref()) {
        Ok(progress) => progress,
        Err(()) => return 1,
    };
    let progress_targets = progress_file_targets(&json);

    if parsed.validate_only {
        let validation = validate_pipeline_for_kernel(&json);
        println!("{}", serde_json::to_string_pretty(&validation).unwrap());
        return 0;
    }

    if let Some(path) = parsed.serialization_file {
        match serialize_pipeline_json(&json) {
            Ok(serialized) => {
                if let Err(err) = std::fs::write(&path, serialized) {
                    eprintln!(
                        "PDAL: kernels.pipeline: Unable to write pipeline serialization '{path}': {err}"
                    );
                    return 1;
                }
            }
            Err(err) => {
                eprintln!("PDAL: kernels.pipeline: {err}");
                return 1;
            }
        }
    }

    let mut pipeline = match pipeline_from_json(&json) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.pipeline: {err}");
            return 1;
        }
    };
    pipeline.set_allowed_dims(
        parsed
            .allowed_dims
            .iter()
            .map(|name| DimId::from_name(name))
            .collect(),
    );
    write_ready_progress(&mut progress, &progress_targets);
    // When no metadata summary is requested, try chunked streaming first
    // (bounded peak memory). `Ok(None)` means the pipeline is not streaming-
    // eligible -- fall through to the materializing path with no side effects.
    if parsed.stream_allowed
        && parsed.metadata_file.is_none()
        && parsed.pointcloud_schema_file.is_none()
        && !parsed.summary_stdout
    {
        match pipeline.execute_streaming() {
            Ok(Some(_)) => {
                write_done_progress(&mut progress, &progress_targets);
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
            write_done_progress(&mut progress, &progress_targets);
            if let Some(path) = parsed.pointcloud_schema_file {
                let xml = pdal_core::xml_schema::point_cloud_schema_xml(&result.output_views);
                if let Err(err) = std::fs::write(&path, xml) {
                    eprintln!(
                        "PDAL: kernels.pipeline: Unable to write PointCloudSchema '{path}': {err}"
                    );
                    return 1;
                }
            }
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

fn open_progress_file(path: Option<&str>) -> Result<Option<File>, ()> {
    let Some(path) = path else {
        return Ok(None);
    };
    match std::fs::OpenOptions::new().write(true).open(path) {
        Ok(file) => Ok(Some(file)),
        Err(_) => {
            eprintln!("Can't open progress file '{path}'.");
            Err(())
        }
    }
}

fn write_ready_progress(file: &mut Option<File>, targets: &[String]) {
    if targets.is_empty() {
        write_progress(file, "READYPIPELINE", "pipeline");
    } else {
        for target in targets {
            write_progress(file, "READYFILE", target);
        }
    }
}

fn write_done_progress(file: &mut Option<File>, targets: &[String]) {
    if targets.is_empty() {
        write_progress(file, "DONEPIPELINE", "pipeline");
    } else {
        for target in targets {
            write_progress(file, "DONEFILE", target);
        }
    }
}

fn write_progress(file: &mut Option<File>, event: &str, text: &str) {
    if let Some(file) = file {
        let _ = writeln!(file, "{event}:{text}");
    }
}

pub(super) fn progress_file_targets(json: &str) -> Vec<String> {
    let Ok(descriptors) = pdal_core::pipeline_reader::parse_pipeline_descriptors(json) else {
        return Vec::new();
    };
    let Some(stages) = descriptors.as_array() else {
        return Vec::new();
    };

    stages
        .iter()
        .filter(|stage| stage["role"] == "writer")
        .filter_map(|stage| stage["filename"].as_str())
        .filter(|filename| !filename.is_empty())
        .map(str::to_string)
        .collect()
}

pub(super) fn validate_pipeline_for_kernel(json: &str) -> serde_json::Value {
    match validate_pipeline_json_shape(json).and_then(|_| {
        let pipeline = pipeline_from_json(json).map_err(|err| err.to_string())?;
        if !pipeline.roots_are_readers() {
            return Err("Pipeline does not start with a reader.".to_string());
        }
        Ok(pipeline.validation_streamable())
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
