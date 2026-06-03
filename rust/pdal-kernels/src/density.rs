use crate::stage_options::{parse_cli_stage_option, parse_option_value};
use crate::KernelPipelinePlan;
use pdal_core::driver::infer_reader_driver;
use std::io::Read;

pub fn build_density_pipeline(args: &[String]) -> KernelPipelinePlan {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.density: Missing value for positional argument 'input'.");
            return KernelPipelinePlan::Return(1);
        }
        println!("Usage:");
        println!("  pdal density <input> <output.geojson> [--<stage>.<key>=<value> ...]");
        return KernelPipelinePlan::Return(0);
    }

    let mut parsed = DensityArgs::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Err(code) = parse_density_arg(arg, &mut iter, &mut parsed) {
            return KernelPipelinePlan::Return(code);
        }
    }

    let Some(input) = parsed.input else {
        eprintln!("PDAL: kernels.density: Missing value for positional argument 'input'.");
        return KernelPipelinePlan::Return(1);
    };
    let Some(output) = parsed.output else {
        eprintln!("PDAL: kernels.density: Missing value for positional argument 'output'.");
        return KernelPipelinePlan::Return(1);
    };

    parsed.hexbin_stage["density"] = serde_json::json!(output);

    if input == "STDIN" || input == "-" {
        let mut json = String::new();
        if let Err(err) = std::io::stdin().read_to_string(&mut json) {
            eprintln!("PDAL: kernels.density: Unable to read pipeline from stdin: {err}");
            return KernelPipelinePlan::Return(1);
        }
        return append_density_stage(&json, parsed.hexbin_stage);
    }

    if input.ends_with(".json") {
        let json = match std::fs::read_to_string(&input) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("PDAL: kernels.density: Unable to read pipeline '{input}': {err}");
                return KernelPipelinePlan::Return(1);
            }
        };
        return append_density_stage(&json, parsed.hexbin_stage);
    }

    if input.ends_with(".xml") {
        return KernelPipelinePlan::Return(-1);
    }

    let Some(reader) = parsed
        .reader_override
        .or_else(|| infer_reader_driver(&input).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.density: Unable to infer reader driver for '{input}'.");
        return KernelPipelinePlan::Return(1);
    };

    KernelPipelinePlan::Pipeline(serde_json::json!([
        { "type": reader, "filename": input },
        parsed.hexbin_stage,
    ]))
}

struct DensityArgs {
    input: Option<String>,
    output: Option<String>,
    reader_override: Option<String>,
    hexbin_stage: serde_json::Value,
}

impl Default for DensityArgs {
    fn default() -> Self {
        Self {
            input: None,
            output: None,
            reader_override: None,
            hexbin_stage: serde_json::json!({
                "type": "filters.hexbin",
            }),
        }
    }
}

fn parse_density_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut DensityArgs,
) -> Result<(), i32> {
    if arg == "--input" || arg == "-i" {
        parsed.input = Some(next_value(arg, iter)?);
    } else if let Some(value) = arg.strip_prefix("--input=") {
        parsed.input = Some(value.to_string());
    } else if arg == "--output" || arg == "-o" {
        parsed.output = Some(next_value(arg, iter)?);
    } else if let Some(value) = arg.strip_prefix("--output=") {
        parsed.output = Some(value.to_string());
    } else if arg == "--driver" {
        parsed.reader_override = Some(next_value("--driver", iter)?);
    } else if let Some(value) = arg.strip_prefix("--driver=") {
        parsed.reader_override = Some(value.to_string());
    } else if arg == "--ogrdriver" || arg == "-f" {
        parsed.hexbin_stage["ogrdriver"] = serde_json::json!(next_value(arg, iter)?);
    } else if let Some(value) = arg.strip_prefix("--ogrdriver=") {
        parsed.hexbin_stage["ogrdriver"] = serde_json::json!(value);
    } else if let Some(value) = arg.strip_prefix("-f=") {
        parsed.hexbin_stage["ogrdriver"] = serde_json::json!(value);
    } else if arg == "--lyr_name" {
        parsed.hexbin_stage["lyr_name"] = serde_json::json!(next_value("--lyr_name", iter)?);
    } else if let Some(value) = arg.strip_prefix("--lyr_name=") {
        parsed.hexbin_stage["lyr_name"] = serde_json::json!(value);
    } else if matches!(
        arg,
        "--edge_length"
            | "--threshold"
            | "--sample_size"
            | "--hole_cull_area_tolerance"
            | "--h3_resolution"
    ) {
        let value = next_value(arg, iter)?;
        parsed.hexbin_stage[arg.trim_start_matches("--")] = parse_option_value(&value);
    } else if let Some(value) = arg.strip_prefix("--edge_length=") {
        parsed.hexbin_stage["edge_length"] = parse_option_value(value);
    } else if let Some(value) = arg.strip_prefix("--threshold=") {
        parsed.hexbin_stage["threshold"] = parse_option_value(value);
    } else if let Some(value) = arg.strip_prefix("--sample_size=") {
        parsed.hexbin_stage["sample_size"] = parse_option_value(value);
    } else if let Some(value) = arg.strip_prefix("--hole_cull_area_tolerance=") {
        parsed.hexbin_stage["hole_cull_area_tolerance"] = parse_option_value(value);
    } else if let Some(value) = arg.strip_prefix("--h3_resolution=") {
        parsed.hexbin_stage["h3_resolution"] = parse_option_value(value);
    } else if arg == "--smooth" || arg == "--h3_grid" {
        parsed.hexbin_stage[arg.trim_start_matches("--")] = serde_json::json!(true);
    } else if let Some(value) = arg.strip_prefix("--smooth=") {
        parsed.hexbin_stage["smooth"] = parse_option_value(value);
    } else if let Some(value) = arg.strip_prefix("--h3_grid=") {
        parsed.hexbin_stage["h3_grid"] = parse_option_value(value);
    } else if arg.starts_with("--") {
        let Some(option) = parse_cli_stage_option(arg) else {
            return Err(-1);
        };
        if option.stage != "filters.hexbin" && option.stage != "hexbin" {
            return Err(-1);
        }
        parsed.hexbin_stage[option.key.as_str()] = parse_option_value(&option.value);
    } else if parsed.input.is_none() {
        parsed.input = Some(arg.to_string());
    } else if parsed.output.is_none() {
        parsed.output = Some(arg.to_string());
    } else {
        eprintln!("PDAL: kernels.density: Unexpected argument '{arg}'.");
        return Err(1);
    }
    Ok(())
}

fn next_value<'a>(
    option: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<String, i32> {
    match iter.next() {
        Some(value) => Ok(value.clone()),
        None => {
            eprintln!("PDAL: kernels.density: Missing value for option '{option}'.");
            Err(1)
        }
    }
}

fn append_density_stage(json: &str, stage: serde_json::Value) -> KernelPipelinePlan {
    match append_stage_to_pipeline_json(json, stage) {
        Ok(value) => KernelPipelinePlan::Pipeline(value),
        Err(err) => {
            eprintln!("PDAL: kernels.density: {err}");
            KernelPipelinePlan::Return(1)
        }
    }
}

fn append_stage_to_pipeline_json(
    json: &str,
    stage: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let stripped = pdal_core::pipeline_reader::strip_json_comments(json);
    let mut value: serde_json::Value =
        serde_json::from_str(&stripped).map_err(|err| format!("Invalid pipeline JSON: {err}"))?;
    match &mut value {
        serde_json::Value::Array(stages) => {
            stages.push(stage);
            Ok(value)
        }
        serde_json::Value::Object(object) => {
            let Some(pipeline) = object.get_mut("pipeline") else {
                return Err("Pipeline JSON object is missing a 'pipeline' array.".to_string());
            };
            let Some(stages) = pipeline.as_array_mut() else {
                return Err("Pipeline JSON object has a non-array 'pipeline' member.".to_string());
            };
            stages.push(stage);
            Ok(value)
        }
        _ => Err("Pipeline JSON must be an array or object.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pipeline(args: &[&str]) -> serde_json::Value {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        match build_density_pipeline(&args) {
            KernelPipelinePlan::Pipeline(value) => value,
            KernelPipelinePlan::Return(code) => panic!("unexpected return: {code}"),
        }
    }

    #[test]
    fn builds_file_pipeline() {
        let value = pipeline(&["in.las", "out.geojson"]);
        assert_eq!(value[0]["type"], "readers.las");
        assert_eq!(value[1]["type"], "filters.hexbin");
        assert_eq!(value[1]["density"], "out.geojson");
    }

    #[test]
    fn honors_hexbin_options_and_reader_override() {
        let value = pipeline(&[
            "--driver=readers.text",
            "--edge_length=12.5",
            "--threshold",
            "7",
            "--filters.hexbin.sample_size=42",
            "--ogrdriver",
            "GPKG",
            "--lyr_name=tiles",
            "in.csv",
            "out.gpkg",
        ]);
        assert_eq!(value[0]["type"], "readers.text");
        assert_eq!(value[1]["edge_length"], 12.5);
        assert_eq!(value[1]["threshold"], 7);
        assert_eq!(value[1]["sample_size"], 42);
        assert_eq!(value[1]["ogrdriver"], "GPKG");
        assert_eq!(value[1]["lyr_name"], "tiles");
    }

    #[test]
    fn accepts_input_output_equals_forms() {
        let value = pipeline(&["--input=in.las", "--output=out.geojson"]);
        assert_eq!(value[0]["filename"], "in.las");
        assert_eq!(value[1]["density"], "out.geojson");
    }

    #[test]
    fn accepts_cpp_density_switch_names() {
        let value = pipeline(&[
            "--sample_size=123",
            "--hole_cull_area_tolerance",
            "4.5",
            "--smooth=false",
            "--h3_grid",
            "--h3_resolution=8",
            "in.las",
            "out.geojson",
        ]);
        assert_eq!(value[1]["sample_size"], 123);
        assert_eq!(value[1]["hole_cull_area_tolerance"], 4.5);
        assert_eq!(value[1]["smooth"], false);
        assert_eq!(value[1]["h3_grid"], true);
        assert_eq!(value[1]["h3_resolution"], 8);
    }

    #[test]
    fn appends_hexbin_stage_to_pipeline_json() {
        let mut parsed = DensityArgs {
            input: Some("pipeline.json".to_string()),
            output: Some("out.geojson".to_string()),
            ..Default::default()
        };
        parsed.hexbin_stage["density"] = serde_json::json!("out.geojson");
        let value = match append_density_stage(
            r#"[{"type":"readers.faux","count":1}]"#,
            parsed.hexbin_stage,
        ) {
            KernelPipelinePlan::Pipeline(value) => value,
            KernelPipelinePlan::Return(code) => panic!("unexpected return: {code}"),
        };
        assert_eq!(value.as_array().unwrap().len(), 2);
        assert_eq!(value[1]["density"], "out.geojson");
    }

    #[test]
    fn appends_hexbin_stage_to_commented_pipeline_json() {
        let value = match append_density_stage(
            r#"{
                "pipeline": [
                    // source pipeline
                    {"type":"readers.faux","count":1}
                ]
            }"#,
            serde_json::json!({"type":"filters.hexbin", "density":"out.geojson"}),
        ) {
            KernelPipelinePlan::Pipeline(value) => value,
            KernelPipelinePlan::Return(code) => panic!("unexpected return: {code}"),
        };

        assert_eq!(value["pipeline"].as_array().unwrap().len(), 2);
        assert_eq!(value["pipeline"][1]["density"], "out.geojson");
    }

    #[test]
    fn routes_xml_pipeline_input_to_cpp_fallback() {
        let args = vec!["in.xml".to_string(), "out.geojson".to_string()];
        assert!(matches!(
            build_density_pipeline(&args),
            KernelPipelinePlan::Return(-1)
        ));
    }
}
