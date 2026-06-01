use crate::stage_options::apply_writer_stage_option;
use crate::KernelPipelinePlan;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};

pub fn build_merge_pipeline(args: &[String]) -> KernelPipelinePlan {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.merge: Missing value for positional argument 'files'.");
            return KernelPipelinePlan::Return(1);
        }
        println!("Usage:");
        println!("  pdal merge <input> [input ...] <output> [--<stage>.<key>=<value> ...]");
        return KernelPipelinePlan::Return(0);
    }

    let mut parsed = MergeArgs::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Err(code) = parse_merge_arg(arg, &mut iter, &mut parsed) {
            return KernelPipelinePlan::Return(code);
        }
    }

    if parsed.positional.len() < 2 {
        eprintln!("PDAL: kernels.merge: Missing value for positional argument 'files'.");
        return KernelPipelinePlan::Return(1);
    }

    let output = parsed.positional.last().cloned().unwrap_or_default();
    let inputs = &parsed.positional[..parsed.positional.len() - 1];
    let Some(writer) = infer_writer_driver(&output).map(str::to_string) else {
        eprintln!("PDAL: kernels.merge: Unable to infer writer driver for '{output}'.");
        return KernelPipelinePlan::Return(1);
    };

    let mut stages = Vec::new();
    let mut tags = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let Some(reader) = parsed
            .reader_override
            .clone()
            .or_else(|| infer_reader_driver(input).map(str::to_string))
        else {
            eprintln!("PDAL: kernels.merge: Unable to infer reader driver for '{input}'.");
            return KernelPipelinePlan::Return(1);
        };
        let tag = format!("merge_input_{index}");
        stages.push(serde_json::json!({
            "type": reader,
            "filename": input,
            "tag": tag,
        }));
        tags.push(tag);
    }

    stages.push(serde_json::json!({
        "type": "filters.merge",
        "inputs": tags,
    }));

    let mut writer_stage = serde_json::Map::new();
    writer_stage.insert("type".to_string(), serde_json::json!(writer));
    writer_stage.insert("filename".to_string(), serde_json::json!(output));
    writer_stage.extend(parsed.writer_options);
    stages.push(serde_json::Value::Object(writer_stage));

    KernelPipelinePlan::Pipeline(serde_json::Value::Array(stages))
}

#[derive(Default)]
struct MergeArgs {
    positional: Vec<String>,
    reader_override: Option<String>,
    writer_options: serde_json::Map<String, serde_json::Value>,
}

fn parse_merge_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut MergeArgs,
) -> Result<(), i32> {
    if arg == "--driver" {
        parsed.reader_override = Some(next_value("--driver", iter)?);
    } else if let Some(value) = arg.strip_prefix("--driver=") {
        parsed.reader_override = Some(value.to_string());
    } else if arg == "--files" || arg == "-f" {
        parsed.positional.push(next_value(arg, iter)?);
    } else if arg.starts_with("--") {
        if !apply_writer_stage_option(arg, &mut parsed.writer_options) {
            eprintln!("PDAL: kernels.merge: Unexpected argument '{arg}'.");
            return Err(1);
        }
    } else {
        parsed.positional.push(arg.to_string());
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
            eprintln!("PDAL: kernels.merge: Missing value for option '{option}'.");
            Err(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pipeline(args: &[&str]) -> serde_json::Value {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        match build_merge_pipeline(&args) {
            KernelPipelinePlan::Pipeline(value) => value,
            KernelPipelinePlan::Return(code) => panic!("unexpected return: {code}"),
        }
    }

    #[test]
    fn builds_merge_pipeline_with_tags() {
        let value = pipeline(&["a.las", "b.las", "out.las"]);
        assert_eq!(value[0]["tag"], "merge_input_0");
        assert_eq!(value[1]["tag"], "merge_input_1");
        assert_eq!(value[2]["type"], "filters.merge");
        assert_eq!(
            value[2]["inputs"],
            serde_json::json!(["merge_input_0", "merge_input_1"])
        );
        assert_eq!(value[3]["type"], "writers.las");
    }

    #[test]
    fn honors_reader_override_and_writer_options() {
        let value = pipeline(&[
            "--driver=readers.text",
            "--writers.las.minor_version=4",
            "a.csv",
            "-f",
            "b.csv",
            "out.las",
        ]);
        assert_eq!(value[0]["type"], "readers.text");
        assert_eq!(value[1]["type"], "readers.text");
        assert_eq!(value[3]["minor_version"], 4);
    }

    #[test]
    fn rejects_missing_inputs() {
        let args = vec!["out.las".to_string()];
        assert!(matches!(
            build_merge_pipeline(&args),
            KernelPipelinePlan::Return(1)
        ));
    }
}
