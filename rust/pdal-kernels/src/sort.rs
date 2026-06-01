use crate::KernelPipelinePlan;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};

pub fn build_sort_pipeline(args: &[String]) -> KernelPipelinePlan {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.sort: Missing value for positional argument 'input'.");
            return KernelPipelinePlan::Return(1);
        }
        println!("Usage:");
        println!("  pdal sort <input> <output> [--<stage>.<key>=<value> ...]");
        return KernelPipelinePlan::Return(0);
    }

    let mut parsed = SortArgs::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Err(code) = parse_sort_arg(arg, &mut iter, &mut parsed) {
            return KernelPipelinePlan::Return(code);
        }
    }

    let Some(input) = parsed.input else {
        eprintln!("PDAL: kernels.sort: Missing value for positional argument 'input'.");
        return KernelPipelinePlan::Return(1);
    };
    let Some(output) = parsed.output else {
        eprintln!("PDAL: kernels.sort: Missing value for positional argument 'output'.");
        return KernelPipelinePlan::Return(1);
    };

    let Some(reader) = parsed
        .reader_override
        .or_else(|| infer_reader_driver(&input).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.sort: Unable to infer reader driver for '{input}'.");
        return KernelPipelinePlan::Return(1);
    };
    let Some(writer) = infer_writer_driver(&output).map(str::to_string) else {
        eprintln!("PDAL: kernels.sort: Unable to infer writer driver for '{output}'.");
        return KernelPipelinePlan::Return(1);
    };

    let mut sort_stage = serde_json::json!({
        "type": "filters.sort",
        "dimensions": parsed.sort_dimension,
    });
    if let Some(order) = parsed.sort_order {
        sort_stage["order"] = serde_json::json!(order);
    }
    if let Some(algorithm) = parsed.sort_algorithm {
        sort_stage["algorithm"] = serde_json::json!(algorithm);
    }

    KernelPipelinePlan::Pipeline(serde_json::json!([
        { "type": reader, "filename": input },
        sort_stage,
        { "type": writer, "filename": output }
    ]))
}

struct SortArgs {
    input: Option<String>,
    output: Option<String>,
    reader_override: Option<String>,
    sort_dimension: String,
    sort_order: Option<String>,
    sort_algorithm: Option<String>,
}

impl Default for SortArgs {
    fn default() -> Self {
        Self {
            input: None,
            output: None,
            reader_override: None,
            sort_dimension: String::from("X"),
            sort_order: None,
            sort_algorithm: None,
        }
    }
}

fn parse_sort_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut SortArgs,
) -> Result<(), i32> {
    if arg == "--input" || arg == "-i" {
        parsed.input = Some(next_value(arg, iter)?);
    } else if arg == "--output" || arg == "-o" {
        parsed.output = Some(next_value(arg, iter)?);
    } else if arg == "--driver" {
        parsed.reader_override = Some(next_value("--driver", iter)?);
    } else if let Some(value) = arg.strip_prefix("--driver=") {
        parsed.reader_override = Some(value.to_string());
    } else if let Some(value) = arg.strip_prefix("--filters.sort.dimension=") {
        parsed.sort_dimension = value.to_string();
    } else if let Some(value) = arg.strip_prefix("--filters.sort.dimensions=") {
        parsed.sort_dimension = value.to_string();
    } else if let Some(value) = arg.strip_prefix("--filters.sort.order=") {
        parsed.sort_order = Some(value.to_string());
    } else if let Some(value) = arg.strip_prefix("--filters.sort.algorithm=") {
        parsed.sort_algorithm = Some(value.to_string());
    } else if arg.starts_with("--") {
        eprintln!("PDAL: kernels.sort: Unexpected argument '{arg}'.");
        return Err(1);
    } else if parsed.input.is_none() {
        parsed.input = Some(arg.to_string());
    } else if parsed.output.is_none() {
        parsed.output = Some(arg.to_string());
    } else {
        eprintln!("PDAL: kernels.sort: Unexpected argument '{arg}'.");
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
            eprintln!("PDAL: kernels.sort: Missing value for option '{option}'.");
            Err(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pipeline(args: &[&str]) -> serde_json::Value {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        match build_sort_pipeline(&args) {
            KernelPipelinePlan::Pipeline(value) => value,
            KernelPipelinePlan::Return(code) => panic!("unexpected return: {code}"),
        }
    }

    #[test]
    fn builds_default_sort_pipeline() {
        let value = pipeline(&["in.las", "out.las"]);
        assert_eq!(value[0]["type"], "readers.las");
        assert_eq!(value[1]["type"], "filters.sort");
        assert_eq!(value[1]["dimensions"], "X");
        assert_eq!(value[2]["type"], "writers.las");
    }

    #[test]
    fn honors_sort_options_and_reader_override() {
        let value = pipeline(&[
            "--driver",
            "readers.text",
            "--filters.sort.dimensions=GpsTime",
            "--filters.sort.order=DESC",
            "--filters.sort.algorithm=stable",
            "in.csv",
            "out.bpf",
        ]);
        assert_eq!(value[0]["type"], "readers.text");
        assert_eq!(value[1]["dimensions"], "GpsTime");
        assert_eq!(value[1]["order"], "DESC");
        assert_eq!(value[1]["algorithm"], "stable");
        assert_eq!(value[2]["type"], "writers.bpf");
    }

    #[test]
    fn rejects_missing_output() {
        let args = vec!["in.las".to_string()];
        assert!(matches!(
            build_sort_pipeline(&args),
            KernelPipelinePlan::Return(1)
        ));
    }
}
