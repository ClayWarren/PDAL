use crate::stage_options::{apply_cli_stage_options, parse_cli_stage_option, CliStageOption};

pub struct ParsedPipelineArgs {
    pub input: Option<String>,
    pub read_stdin: bool,
    pub validate_only: bool,
    pub metadata_file: Option<String>,
    pub progress_file: Option<String>,
    pub pointcloud_schema_file: Option<String>,
    pub serialization_file: Option<String>,
    pub summary_stdout: bool,
    pub stream_allowed: bool,
    pub stream_required: bool,
    pub stage_options: Vec<CliStageOption>,
}

pub enum PipelineArgsResult {
    Run(ParsedPipelineArgs),
    Return(i32),
}

pub fn parse_pipeline_args(args: &[String]) -> PipelineArgsResult {
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
        pointcloud_schema_file: None,
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
    } else if let Some(value) = arg.strip_prefix("--input=") {
        parsed.input = Some(value.to_string());
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
    } else if arg.starts_with("--dims=") {
    } else if arg == "--progress" {
        parsed.progress_file = Some(next_option_value(arg, iter)?.clone());
    } else if let Some(value) = arg.strip_prefix("--progress=") {
        parsed.progress_file = Some(value.to_string());
    } else if arg == "--pointcloudschema" {
        parsed.pointcloud_schema_file = Some(next_option_value(arg, iter)?.clone());
    } else if let Some(value) = arg.strip_prefix("--pointcloudschema=") {
        parsed.pointcloud_schema_file = Some(value.to_string());
    } else if arg == "--metadata" {
        parsed.metadata_file = Some(next_option_value("--metadata", iter)?.clone());
    } else if let Some(value) = arg.strip_prefix("--metadata=") {
        parsed.metadata_file = Some(value.to_string());
    } else if arg == "--pipeline-serialization" {
        parsed.serialization_file =
            Some(next_option_value("--pipeline-serialization", iter)?.clone());
    } else if let Some(value) = arg.strip_prefix("--pipeline-serialization=") {
        parsed.serialization_file = Some(value.to_string());
    } else if let Some(stage_option) = parse_cli_stage_option(arg) {
        parsed.stage_options.push(stage_option);
    } else if arg.starts_with("--") || arg.starts_with("-v") {
        eprintln!("PDAL: kernels.pipeline: Unknown option '{arg}'.");
        return Err(1);
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

pub fn validate_pipeline_json_shape(json: &str) -> Result<(), String> {
    pdal_core::pipeline_reader::parse_pipeline_descriptors(json).map(|_| ())
}

pub fn apply_stage_options_to_pipeline_json(
    json: &str,
    stage_options: &[CliStageOption],
) -> Result<String, String> {
    if stage_options.is_empty() {
        return Ok(json.to_string());
    }

    validate_pipeline_json_shape(json)?;

    let stripped = pdal_core::pipeline_reader::strip_json_comments(json);
    let mut value: serde_json::Value =
        serde_json::from_str(&stripped).map_err(|err| format!("Invalid pipeline JSON: {err}"))?;
    let stages = if let Some(stages) = value.as_array_mut() {
        stages
    } else if let Some(stages) = value
        .get_mut("pipeline")
        .and_then(serde_json::Value::as_array_mut)
    {
        stages
    } else {
        return Err("Pipeline JSON must be an array or an object with a 'pipeline' array.".into());
    };

    if !apply_cli_stage_options(stages, stage_options) {
        return Err("Unable to apply stage option to pipeline.".to_string());
    }
    serde_json::to_string(&value).map_err(|err| format!("Unable to serialize pipeline JSON: {err}"))
}

pub fn serialize_pipeline_json(json: &str) -> Result<String, String> {
    let stripped = pdal_core::pipeline_reader::strip_json_comments(json);
    let value: serde_json::Value =
        serde_json::from_str(&stripped).map_err(|err| format!("Invalid pipeline JSON: {err}"))?;
    let stages = if let Some(stages) = value.as_array() {
        stages
    } else if let Some(stages) = value.get("pipeline").and_then(serde_json::Value::as_array) {
        stages
    } else {
        return Err("Pipeline JSON must be an array or an object with a 'pipeline' array.".into());
    };

    let mut existing_tags = Vec::new();
    let mut serialized = Vec::with_capacity(stages.len());
    for (position, stage) in stages.iter().enumerate() {
        let mut object = serialized_stage_object(stage, position, stages.len())?;
        let stage_type = object
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("Pipeline stage {position} is missing a 'type'."))?
            .to_string();
        decode_typed_json_options(&mut object);
        if !object.contains_key("tag") {
            let tag = pdal_core::pipeline::generate_stage_tag(
                &stage_type,
                "",
                &existing_tags.iter().map(String::as_str).collect::<Vec<_>>(),
            );
            existing_tags.push(tag.clone());
            object.insert("tag".to_string(), serde_json::Value::String(tag));
        } else if let Some(tag) = object.get("tag").and_then(serde_json::Value::as_str) {
            existing_tags.push(tag.to_string());
        }
        serialized.push(serde_json::Value::Object(object));
    }

    let root = serde_json::json!({ "pipeline": serialized });
    serde_json::to_string_pretty(&root)
        .map(|text| text + "\n")
        .map_err(|err| format!("Unable to serialize pipeline JSON: {err}"))
}

fn serialized_stage_object(
    stage: &serde_json::Value,
    position: usize,
    len: usize,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    if let Some(object) = stage.as_object() {
        let mut object = object.clone();
        if !object.contains_key("type") {
            let filename = object
                .get("filename")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("Pipeline stage {position} is missing a 'type'."))?;
            object.insert(
                "type".to_string(),
                serde_json::Value::String(infer_stage_name(filename, position, len)?),
            );
        }
        return Ok(object);
    }

    let Some(filename) = stage.as_str() else {
        return Err(format!(
            "Pipeline stage {position} must be a JSON object or filename string."
        ));
    };
    let mut object = serde_json::Map::new();
    object.insert(
        "type".to_string(),
        serde_json::Value::String(infer_stage_name(filename, position, len)?),
    );
    object.insert(
        "filename".to_string(),
        serde_json::Value::String(filename.to_string()),
    );
    Ok(object)
}

fn infer_stage_name(filename: &str, position: usize, len: usize) -> Result<String, String> {
    if position + 1 == len {
        pdal_core::driver::infer_writer_driver(filename)
            .map(str::to_string)
            .ok_or_else(|| format!("Unable to infer writer for '{filename}'."))
    } else {
        pdal_core::driver::infer_reader_driver(filename)
            .map(str::to_string)
            .ok_or_else(|| format!("Unable to infer reader for '{filename}'."))
    }
}

fn decode_typed_json_options(object: &mut serde_json::Map<String, serde_json::Value>) {
    for key in [
        "filename",
        "spatialreference",
        "default_srs",
        "override_srs",
        "in_srs",
        "out_srs",
        "a_srs",
    ] {
        let Some(value) = object.get_mut(key) else {
            continue;
        };
        let Some(text) = value.as_str() else {
            continue;
        };
        let trimmed = text.trim_start();
        if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
            *value = parsed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pipeline_input_and_stage_options() {
        let args = vec![
            "--input".to_string(),
            "pipeline.json".to_string(),
            "--metadata".to_string(),
            "meta.json".to_string(),
            "--filters.sort.dimension=Y".to_string(),
        ];
        let PipelineArgsResult::Run(parsed) = parse_pipeline_args(&args) else {
            panic!("expected runnable pipeline args");
        };
        assert_eq!(parsed.input.as_deref(), Some("pipeline.json"));
        assert_eq!(parsed.metadata_file.as_deref(), Some("meta.json"));
        assert_eq!(parsed.stage_options.len(), 1);
        assert_eq!(parsed.stage_options[0].stage, "filters.sort");
    }

    #[test]
    fn parses_pointcloud_schema_output_path() {
        let args = vec![
            "pipeline.json".to_string(),
            "--pointcloudschema".to_string(),
            "schema.xml".to_string(),
        ];
        let PipelineArgsResult::Run(parsed) = parse_pipeline_args(&args) else {
            panic!("expected runnable pipeline args");
        };

        assert_eq!(parsed.input.as_deref(), Some("pipeline.json"));
        assert_eq!(parsed.pointcloud_schema_file.as_deref(), Some("schema.xml"));
    }

    #[test]
    fn parses_equals_forms_for_public_value_switches() {
        let args = vec![
            "--input=pipeline.json".to_string(),
            "--metadata=meta.json".to_string(),
            "--pipeline-serialization=serial.json".to_string(),
            "--progress=progress.txt".to_string(),
            "--pointcloudschema=schema.xml".to_string(),
            "--dims=X,Y".to_string(),
        ];
        let PipelineArgsResult::Run(parsed) = parse_pipeline_args(&args) else {
            panic!("expected runnable pipeline args");
        };

        assert_eq!(parsed.input.as_deref(), Some("pipeline.json"));
        assert_eq!(parsed.metadata_file.as_deref(), Some("meta.json"));
        assert_eq!(parsed.serialization_file.as_deref(), Some("serial.json"));
        assert_eq!(parsed.progress_file.as_deref(), Some("progress.txt"));
        assert_eq!(parsed.pointcloud_schema_file.as_deref(), Some("schema.xml"));
    }

    #[test]
    fn rejects_stdin_and_input_together() {
        let args = vec!["--stdin".to_string(), "pipeline.json".to_string()];
        assert!(matches!(
            parse_pipeline_args(&args),
            PipelineArgsResult::Return(1)
        ));
    }

    #[test]
    fn rejects_unknown_options_as_rust_errors() {
        let args = vec!["pipeline.json".to_string(), "--bogus".to_string()];
        assert!(matches!(
            parse_pipeline_args(&args),
            PipelineArgsResult::Return(1)
        ));
    }

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
    fn applies_cli_stage_options_to_commented_pipeline_json() {
        let json = r#"{
            // accepted by C++ PipelineReaderJSON
            "pipeline": [
                {"type":"readers.faux", "count":4},
                {"type":"filters.sort", "dimension":"X"},
                {"type":"writers.null"}
            ]
        }"#;
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
    fn rejects_unmatched_cli_stage_options() {
        let json = r#"{"pipeline":[{"type":"readers.faux"},{"type":"writers.null"}]}"#;
        let options = vec![CliStageOption {
            stage: "filters.sort".to_string(),
            key: "dimension".to_string(),
            value: "Y".to_string(),
        }];

        assert!(apply_stage_options_to_pipeline_json(json, &options)
            .unwrap_err()
            .contains("Unable to apply stage option"));
    }

    #[test]
    fn rejects_invalid_stage_metadata_before_applying_cli_stage_options() {
        let json = r#"[
            {"type":"readers.faux","tag":"A"},
            {"type":"readers.faux","inputs":["A"]}
        ]"#;
        let options = vec![CliStageOption {
            stage: "readers.faux".to_string(),
            key: "count".to_string(),
            value: "4".to_string(),
        }];

        assert!(apply_stage_options_to_pipeline_json(json, &options)
            .unwrap_err()
            .contains("Inputs not permitted"));
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
    fn serializes_filename_stages_with_inferred_types_and_generated_tags() {
        let serialized = serialize_pipeline_json(r#"["in.las","in2.las","out.las"]"#).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(parsed["pipeline"][0]["type"], "readers.las");
        assert_eq!(parsed["pipeline"][0]["tag"], "readers_las1");
        assert_eq!(parsed["pipeline"][1]["tag"], "readers_las2");
        assert_eq!(parsed["pipeline"][2]["type"], "writers.las");
        assert_eq!(parsed["pipeline"][2]["tag"], "writers_las1");
    }

    #[test]
    fn serializes_commented_pipeline_json() {
        let serialized = serialize_pipeline_json(
            r#"[
                // input
                "in.las",
                {"type":"filters.decimation", "step":2},
                "out.las"
            ]"#,
        )
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(parsed["pipeline"][0]["type"], "readers.las");
        assert_eq!(parsed["pipeline"][1]["type"], "filters.decimation");
        assert_eq!(parsed["pipeline"][2]["type"], "writers.las");
    }

    #[test]
    fn serializes_typed_json_option_strings_as_objects() {
        let json = r#"{
            "pipeline": [
                {
                    "type": "readers.las",
                    "filename": "{\"path\":\"/tmp/in.las\",\"headers\":{\"k\":\"v\"}}"
                },
                {
                    "type": "filters.reprojection",
                    "out_srs": "{\"$schema\":\"https://proj.org/schemas/v0.7/projjson.schema.json\",\"type\":\"GeographicCRS\"}"
                },
                {
                    "type": "writers.las",
                    "filename": "/tmp/out.las"
                }
            ]
        }"#;

        let serialized = serialize_pipeline_json(json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed["pipeline"][0]["filename"]["path"], "/tmp/in.las");
        assert_eq!(
            parsed["pipeline"][1]["out_srs"]["$schema"],
            "https://proj.org/schemas/v0.7/projjson.schema.json"
        );
        assert_eq!(parsed["pipeline"][2]["filename"], "/tmp/out.las");
    }

    #[test]
    fn validate_shape_accepts_object_valued_options() {
        let json = r#"[{"type":"readers.ept","filename":"ept.json"},{"type":"writers.ept_addon","addons":{"Z":"Z"}}]"#;

        assert!(validate_pipeline_json_shape(json).is_ok());
    }

    #[test]
    fn validate_shape_accepts_commented_pipeline_json() {
        let json = r#"{
            "pipeline": [
                {"type":"readers.faux"}, // source
                {"type":"writers.null"}
            ]
        }"#;

        assert!(validate_pipeline_json_shape(json).is_ok());
    }

    #[test]
    fn validate_shape_rejects_non_stage_entries() {
        let json = r#"[{"type":"readers.faux"}, 7]"#;

        assert!(validate_pipeline_json_shape(json).is_err());
    }

    #[test]
    fn validate_shape_rejects_invalid_stage_metadata() {
        for json in [
            r#"[{"type":7,"filename":"in.las"}]"#,
            r#"[{"type":"readers.faux","tag":7}]"#,
            r#"[{"type":"readers.faux","tag":"1bad"}]"#,
            r#"[{"type":"readers.faux","tag":"A"},{"type":"readers.faux","tag":"A"}]"#,
            r#"[{"type":"readers.faux","tag":"A"},{"type":"readers.faux","inputs":["A"]}]"#,
            r#"[{"type":"readers.faux"},{"type":"filters.merge","inputs":["missing"]}]"#,
        ] {
            assert!(validate_pipeline_json_shape(json).is_err(), "{json}");
        }
    }
}
