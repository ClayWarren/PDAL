use super::{apply_cli_stage_options, parse_cli_stage_option, CliStageOption};
use crate::pipeline_abi::{
    pdal_pipeline_result_t, pipeline_result_to_json_for_kernel, PipelineHandle,
};
use crate::registry::pipeline_from_json;
use pdal_core::driver::infer_reader_driver;
use std::ffi::CStr;
use std::io::Read;
use std::os::raw::c_char;

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
    let mut summary = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--summary" {
            summary = true;
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
        } else if arg.starts_with("--") || arg.starts_with("-p") {
            return -1;
        } else if filename.replace(arg.clone()).is_some() {
            eprintln!("PDAL: kernels.info: Expected exactly one input file.");
            return 1;
        }
    }

    if !summary {
        return -1;
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

    let mut pipeline = match pipeline_from_json(
        &serde_json::json!([{ "type": driver, "filename": filename }]).to_string(),
    ) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.info: {err}");
            return 1;
        }
    };

    match pipeline.execute_with_result(Vec::new()) {
        Ok(result) => {
            let handle = PipelineHandle { pipeline };
            println!("{}", pipeline_result_to_json_for_kernel(result, &handle));
            0
        }
        Err(err) => {
            eprintln!("PDAL: kernels.info: {err}");
            1
        }
    }
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
