use crate::stage_options::{apply_cli_stage_options, parse_cli_stage_option, CliStageOption};
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use std::fs;

pub enum TranslateKernelPlan {
    Run(TranslatePlan),
    Return(i32),
}

pub struct TranslatePlan {
    pub stages: Vec<serde_json::Value>,
    pub allowed_dims: Vec<String>,
    pub metadata_file: Option<String>,
    pub serialization_file: Option<String>,
    pub stream_allowed: bool,
    pub stream_required: bool,
}

pub fn build_translate_plan(args: &[String]) -> TranslateKernelPlan {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.translate: Missing value for positional argument 'input'.");
            return TranslateKernelPlan::Return(1);
        }
        println!("Usage:");
        println!("  pdal translate <input> <output> [filter ...] [--<stage>.<key>=<value> ...]");
        return TranslateKernelPlan::Return(0);
    }

    let parsed = match parse_translate_args(args) {
        Ok(parsed) => parsed,
        Err(code) => return TranslateKernelPlan::Return(code),
    };

    let Some(input) = parsed.input else {
        eprintln!("PDAL: kernels.translate: Missing value for positional argument 'input'.");
        return TranslateKernelPlan::Return(1);
    };
    let Some(output) = parsed.output else {
        eprintln!("PDAL: kernels.translate: Missing value for positional argument 'output'.");
        return TranslateKernelPlan::Return(1);
    };
    if parsed.filter_json.is_some() && !parsed.filters.is_empty() {
        eprintln!("PDAL: kernels.translate: Cannot set both --filter options and --json options");
        return TranslateKernelPlan::Return(1);
    }
    if input.eq_ignore_ascii_case(&output) && !parsed.overwrite {
        eprintln!(
            "PDAL: kernels.translate: Input and output filenames are equal and no --overwrite option was provided!"
        );
        return TranslateKernelPlan::Return(1);
    }
    let Some(reader) = parsed
        .reader_override
        .or_else(|| infer_reader_driver(&input).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.translate: Unable to infer reader driver for '{input}'.");
        return TranslateKernelPlan::Return(1);
    };
    let Some(writer) = parsed
        .writer_override
        .or_else(|| infer_writer_driver(&output).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.translate: Unable to infer writer driver for '{output}'.");
        return TranslateKernelPlan::Return(1);
    };

    let mut stages = if let Some(json) = parsed.filter_json {
        match translate_json_stages(&json, &input, &output, &reader, &writer) {
            Ok(stages) => stages,
            Err(message) => {
                eprintln!("PDAL: kernels.translate: {message}");
                return TranslateKernelPlan::Return(1);
            }
        }
    } else {
        let mut stages = Vec::new();
        stages.push(serde_json::json!({ "type": reader, "filename": input }));
        for filter in parsed.filters {
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

    let stage_options = match expand_translate_option_files(parsed.stage_options) {
        Ok(options) => options,
        Err(code) => return TranslateKernelPlan::Return(code),
    };
    if !apply_cli_stage_options(&mut stages, &stage_options) {
        eprintln!("PDAL: kernels.translate: Unable to apply stage option to pipeline.");
        return TranslateKernelPlan::Return(1);
    }

    TranslateKernelPlan::Run(TranslatePlan {
        stages,
        allowed_dims: parsed.allowed_dims,
        metadata_file: parsed.metadata_file,
        serialization_file: parsed.serialization_file,
        stream_allowed: parsed.stream_allowed,
        stream_required: parsed.stream_required,
    })
}

struct ParsedTranslateArgs {
    input: Option<String>,
    output: Option<String>,
    reader_override: Option<String>,
    writer_override: Option<String>,
    filters: Vec<String>,
    stage_options: Vec<CliStageOption>,
    metadata_file: Option<String>,
    serialization_file: Option<String>,
    filter_json: Option<String>,
    allowed_dims: Vec<String>,
    stream_allowed: bool,
    stream_required: bool,
    overwrite: bool,
}

fn parse_translate_args(args: &[String]) -> Result<ParsedTranslateArgs, i32> {
    let mut parsed = ParsedTranslateArgs {
        input: None,
        output: None,
        reader_override: None,
        writer_override: None,
        filters: Vec::new(),
        stage_options: Vec::new(),
        metadata_file: None,
        serialization_file: None,
        filter_json: None,
        allowed_dims: Vec::new(),
        stream_allowed: true,
        stream_required: false,
        overwrite: false,
    };

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        parse_translate_arg(arg, &mut iter, &mut parsed)?;
    }
    Ok(parsed)
}

fn parse_translate_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut ParsedTranslateArgs,
) -> Result<(), i32> {
    if arg == "--input" || arg == "-i" {
        parsed.input = Some(next_value(arg, iter)?.to_string());
    } else if let Some(value) = arg.strip_prefix("--input=") {
        parsed.input = Some(value.to_string());
    } else if arg == "--output" || arg == "-o" {
        parsed.output = Some(next_value(arg, iter)?.to_string());
    } else if let Some(value) = arg.strip_prefix("--output=") {
        parsed.output = Some(value.to_string());
    } else if arg == "--reader" || arg == "-r" || arg == "--driver" {
        parsed.reader_override = Some(next_value(arg, iter)?.to_string());
    } else if let Some(value) = arg
        .strip_prefix("--reader=")
        .or_else(|| arg.strip_prefix("--driver="))
    {
        parsed.reader_override = Some(value.to_string());
    } else if arg == "--writer" || arg == "-w" {
        parsed.writer_override = Some(next_value(arg, iter)?.to_string());
    } else if let Some(value) = arg.strip_prefix("--writer=") {
        parsed.writer_override = Some(value.to_string());
    } else if arg == "--filter" || arg == "-f" {
        parsed.filters.push(next_value(arg, iter)?.to_string());
    } else if let Some(value) = arg.strip_prefix("--filter=") {
        parsed.filters.push(value.to_string());
    } else if arg == "--metadata" || arg == "-m" {
        parsed.metadata_file = Some(next_value(arg, iter)?.to_string());
    } else if let Some(value) = arg.strip_prefix("--metadata=") {
        parsed.metadata_file = Some(value.to_string());
    } else if arg == "--pipeline" || arg == "-p" {
        parsed.serialization_file = Some(next_value(arg, iter)?.to_string());
    } else if let Some(value) = arg.strip_prefix("--pipeline=") {
        parsed.serialization_file = Some(value.to_string());
    } else if arg == "--stream" {
        if !parsed.stream_allowed {
            eprintln!(
                "PDAL: kernels.translate: Can't specify both 'stream' and 'nostream' options."
            );
            return Err(1);
        }
        parsed.stream_allowed = true;
        parsed.stream_required = true;
    } else if arg == "--nostream" {
        if parsed.stream_required {
            eprintln!(
                "PDAL: kernels.translate: Can't specify both 'stream' and 'nostream' options."
            );
            return Err(1);
        }
        parsed.stream_allowed = false;
    } else if arg == "--overwrite" {
        parsed.overwrite = true;
    } else if arg == "--dims" {
        parsed.allowed_dims = parse_dim_names(next_value("--dims", iter)?);
    } else if let Some(value) = arg.strip_prefix("--dims=") {
        parsed.allowed_dims = parse_dim_names(value);
    } else if arg == "--json" {
        parsed.filter_json = Some(next_value("--json", iter)?.to_string());
    } else if let Some(value) = arg.strip_prefix("--json=") {
        parsed.filter_json = Some(value.to_string());
    } else if arg.starts_with("--") {
        match parse_cli_stage_option(arg) {
            Some(option) => parsed.stage_options.push(option),
            None => {
                eprintln!("PDAL: kernels.translate: Unknown option '{arg}'.");
                return Err(1);
            }
        }
    } else if parsed.input.is_none() {
        parsed.input = Some(arg.to_string());
    } else if parsed.output.is_none() {
        parsed.output = Some(arg.to_string());
    } else {
        parsed.filters.push(arg.to_string());
    }
    Ok(())
}

fn parse_dim_names(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

fn next_value<'a>(
    option: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<&'a str, i32> {
    match iter.next() {
        Some(value) => Ok(value),
        None => {
            eprintln!("PDAL: kernels.translate: Missing value for option '{option}'.");
            Err(1)
        }
    }
}

pub fn translate_json_stages(
    json_arg: &str,
    input: &str,
    output: &str,
    reader: &str,
    writer: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let json = fs::read_to_string(json_arg).unwrap_or_else(|_| json_arg.to_string());
    pdal_core::pipeline_reader::parse_pipeline_descriptors(&json)?;

    let stripped = pdal_core::pipeline_reader::strip_json_comments(&json);
    let value: serde_json::Value =
        serde_json::from_str(&stripped).map_err(|err| format!("Invalid pipeline JSON: {err}"))?;
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
    has_stage_filename(stage) && position == 0 && len > 1
}

fn is_writer_stage(stage: &serde_json::Value, position: usize, len: usize) -> bool {
    if let Some(driver) = stage.get("type").and_then(serde_json::Value::as_str) {
        return driver.starts_with("writers.");
    }
    has_stage_filename(stage) && position + 1 == len
}

fn has_stage_filename(stage: &serde_json::Value) -> bool {
    if stage.as_str().is_some()
        || stage
            .get("filename")
            .and_then(serde_json::Value::as_str)
            .is_some()
    {
        return true;
    }
    stage
        .get("filename")
        .and_then(serde_json::Value::as_object)
        .and_then(|object| object.get("path"))
        .and_then(serde_json::Value::as_str)
        .is_some()
}

pub fn expand_translate_option_files(
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

pub fn parse_option_file(stage: &str, text: &str) -> Result<Vec<CliStageOption>, String> {
    let trimmed = text.trim();
    let stripped = pdal_core::pipeline_reader::strip_json_comments(text);
    let json_trimmed = stripped.trim();
    if json_trimmed.starts_with('{') {
        let value: serde_json::Value =
            serde_json::from_str(json_trimmed).map_err(|_| "Unexpected argument".to_string())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(args: &[&str]) -> TranslatePlan {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        match build_translate_plan(&args) {
            TranslateKernelPlan::Run(plan) => plan,
            TranslateKernelPlan::Return(code) => panic!("unexpected return: {code}"),
        }
    }

    fn scratch_file(name: &str, text: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("pdal-translate-{name}-{}", std::process::id()));
        std::fs::write(&path, text).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn builds_basic_translate_plan() {
        let plan = plan(&["in.las", "out.las", "sort"]);
        assert_eq!(plan.stages[0]["type"], "readers.las");
        assert_eq!(plan.stages[1]["type"], "filters.sort");
        assert_eq!(plan.stages[2]["type"], "writers.las");
    }

    #[test]
    fn builds_translate_plan_with_named_options() {
        let plan = plan(&[
            "--input",
            "in.las",
            "--output",
            "out.copc.laz",
            "--reader",
            "readers.las",
            "--writer",
            "writers.copc",
            "--filter",
            "filters.range",
            "--metadata",
            "metadata.json",
            "--pipeline",
            "pipeline.json",
            "--nostream",
            "--filters.range.limits=Classification[2:2]",
        ]);

        assert_eq!(plan.stages[0]["type"], "readers.las");
        assert_eq!(plan.stages[1]["type"], "filters.range");
        assert_eq!(
            plan.stages[1]["limits"],
            serde_json::Value::String("Classification[2:2]".to_string())
        );
        assert_eq!(plan.stages[2]["type"], "writers.copc");
        assert_eq!(plan.metadata_file.as_deref(), Some("metadata.json"));
        assert_eq!(plan.serialization_file.as_deref(), Some("pipeline.json"));
        assert!(!plan.stream_allowed);
        assert!(!plan.stream_required);
    }

    #[test]
    fn builds_translate_plan_with_equals_form_options() {
        let plan = plan(&[
            "--input=in.csv",
            "--output=out.laz",
            "--reader=readers.text",
            "--writer=writers.las",
            "--filter=filters.sort",
            "--metadata=metadata.json",
            "--pipeline=pipeline.json",
            "--dims=X,Y",
        ]);

        assert_eq!(plan.stages[0]["type"], "readers.text");
        assert_eq!(plan.stages[0]["filename"], "in.csv");
        assert_eq!(plan.stages[1]["type"], "filters.sort");
        assert_eq!(plan.stages[2]["type"], "writers.las");
        assert_eq!(plan.stages[2]["filename"], "out.laz");
        assert_eq!(plan.metadata_file.as_deref(), Some("metadata.json"));
        assert_eq!(plan.serialization_file.as_deref(), Some("pipeline.json"));
        assert_eq!(plan.allowed_dims, vec!["X", "Y"]);
    }

    #[test]
    fn stream_option_requires_streaming() {
        let plan = plan(&["in.las", "out.las", "--stream"]);
        assert!(plan.stream_allowed);
        assert!(plan.stream_required);
    }

    #[test]
    fn rejects_same_input_output_without_overwrite() {
        let args = vec!["same.las".to_string(), "same.las".to_string()];
        assert!(matches!(
            build_translate_plan(&args),
            TranslateKernelPlan::Return(1)
        ));

        let args = vec!["SAME.las".to_string(), "same.las".to_string()];
        assert!(matches!(
            build_translate_plan(&args),
            TranslateKernelPlan::Return(1)
        ));
    }

    #[test]
    fn rejects_missing_unknown_and_conflicting_options() {
        for args in [
            vec!["--input"],
            vec!["in.las"],
            vec!["in.las", "out.las", "--stream", "--nostream"],
            vec!["in.las", "out.las", "--json", "[]", "range"],
            vec!["in.unknown", "out.las"],
            vec!["in.las", "out.unknown"],
            vec!["in.las", "out.las", "--not-a-stage-option"],
            vec!["in.las", "out.las", "--filters.sort.dimension=X"],
        ] {
            let args = strings(&args);
            assert!(
                matches!(build_translate_plan(&args), TranslateKernelPlan::Return(1)),
                "{args:?}"
            );
        }
    }

    #[test]
    fn allows_overwrite_for_equal_input_and_output() {
        let plan = plan(&["same.las", "same.las", "--overwrite"]);
        assert_eq!(plan.stages[0]["filename"], "same.las");
        assert_eq!(plan.stages[1]["filename"], "same.las");
    }

    #[test]
    fn translate_json_replaces_existing_reader_and_writer() {
        let json = r#"[
            {"type":"readers.ept","filename":"old.ept","tag":"reader"},
            {"type":"filters.decimation","step":2},
            {"type":"writers.bpf","filename":"old.bpf","tag":"writer"}
        ]"#;
        let stages =
            translate_json_stages(json, "in.las", "out.laz", "readers.las", "writers.las").unwrap();

        assert_eq!(stages[0]["type"], "readers.las");
        assert_eq!(stages[0]["filename"], "in.las");
        assert_eq!(stages[0]["tag"], "reader");
        assert_eq!(stages[1]["type"], "filters.decimation");
        assert_eq!(stages[2]["type"], "writers.las");
        assert_eq!(stages[2]["filename"], "out.laz");
        assert_eq!(stages[2]["tag"], "writer");
    }

    #[test]
    fn translate_json_accepts_pipeline_object_and_inserts_missing_io() {
        let json = r#"{"pipeline":[{"type":"filters.sort","dimension":"X"}]}"#;
        let stages =
            translate_json_stages(json, "in.las", "out.las", "readers.las", "writers.las").unwrap();

        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0]["type"], "readers.las");
        assert_eq!(stages[1]["type"], "filters.sort");
        assert_eq!(stages[2]["type"], "writers.las");
    }

    #[test]
    fn translate_json_accepts_commented_pipeline_object() {
        let json = r#"{
            "pipeline": [
                // filter provided by --json
                {"type":"filters.range","limits":"Z[0:10]"}
            ]
        }"#;
        let stages =
            translate_json_stages(json, "in.las", "out.las", "readers.las", "writers.las").unwrap();

        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0]["type"], "readers.las");
        assert_eq!(stages[1]["type"], "filters.range");
        assert_eq!(stages[1]["limits"], "Z[0:10]");
        assert_eq!(stages[2]["type"], "writers.las");
    }

    #[test]
    fn translate_json_replaces_filename_only_endpoint_stages() {
        let json = r#"["old.las", {"type":"filters.head"}, {"filename":"old.bpf"}]"#;
        let stages =
            translate_json_stages(json, "in.laz", "out.bpf", "readers.las", "writers.bpf").unwrap();

        assert_eq!(stages[0]["type"], "readers.las");
        assert_eq!(stages[0]["filename"], "in.laz");
        assert_eq!(stages[2]["type"], "writers.bpf");
        assert_eq!(stages[2]["filename"], "out.bpf");
    }

    #[test]
    fn translate_json_replaces_filespec_endpoint_stages() {
        let json = r#"[
            {"filename":{"path":"old-input.las","driver":"readers.las"}},
            {"type":"filters.head"},
            {"filename":{"path":"old-output.bpf","driver":"writers.bpf"}}
        ]"#;
        let stages =
            translate_json_stages(json, "in.laz", "out.bpf", "readers.las", "writers.bpf").unwrap();

        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0]["type"], "readers.las");
        assert_eq!(stages[0]["filename"], "in.laz");
        assert_eq!(stages[1]["type"], "filters.head");
        assert_eq!(stages[2]["type"], "writers.bpf");
        assert_eq!(stages[2]["filename"], "out.bpf");
    }

    #[test]
    fn translate_json_reports_invalid_pipeline_json() {
        for json in ["not-json", r#"{"pipeline":{}}"#] {
            assert!(
                translate_json_stages(json, "in.las", "out.las", "readers.las", "writers.las")
                    .is_err()
            );
        }
    }

    #[test]
    fn translate_json_rejects_invalid_stage_metadata() {
        let json = r#"[
            {"type":"readers.faux","tag":"A"},
            {"type":"readers.faux","inputs":["A"]}
        ]"#;

        let err = translate_json_stages(json, "in.las", "out.las", "readers.las", "writers.las")
            .unwrap_err();
        assert!(err.contains("Inputs not permitted"));
    }

    #[test]
    fn option_files_parse_text_and_json_forms() {
        let text = parse_option_file("filters.range", "--limits=Classification[2:2]").unwrap();
        assert_eq!(text[0].stage, "filters.range");
        assert_eq!(text[0].key, "limits");
        assert_eq!(text[0].value, "Classification[2:2]");

        let json = parse_option_file("filters.range", r#"{"limits":"Z[0:10]","ignored":false}"#);
        assert!(json.is_err());

        let json = parse_option_file("filters.range", r#"{"limits":true}"#).unwrap();
        assert_eq!(json[0].value, "true");
    }

    #[test]
    fn option_files_parse_commented_json_form() {
        let json = parse_option_file(
            "filters.range",
            r#"
                // accepted at the JSON option-file boundary
                {
                    /* range limits */
                    "limits": "Z[0:10]"
                }
            "#,
        )
        .unwrap();

        assert_eq!(json.len(), 1);
        assert_eq!(json[0].key, "limits");
        assert_eq!(json[0].value, "Z[0:10]");
    }

    #[test]
    fn expands_option_file_arguments() {
        let path = scratch_file("option-file", "--limits=Classification[2:2]");
        let options = vec![CliStageOption {
            stage: "filters.range".to_string(),
            key: "option_file".to_string(),
            value: path,
        }];

        let expanded = expand_translate_option_files(options).unwrap();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].key, "limits");
    }

    #[test]
    fn option_files_reject_bad_content_and_missing_files() {
        let bad = scratch_file("bad-option-file", "--limits");
        let options = vec![CliStageOption {
            stage: "filters.range".to_string(),
            key: "option_file".to_string(),
            value: bad,
        }];
        assert!(expand_translate_option_files(options).is_err());

        let missing = vec![CliStageOption {
            stage: "filters.range".to_string(),
            key: "option_file".to_string(),
            value: "/definitely/missing/pdal-option-file".to_string(),
        }];
        assert!(expand_translate_option_files(missing).is_err());
    }

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }
}
