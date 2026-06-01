use crate::stage_options::apply_writer_stage_option;
use crate::KernelPipelinePlan;
use pdal_core::driver::infer_writer_driver;

pub fn build_random_pipeline(args: &[String]) -> KernelPipelinePlan {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.random: Missing value for positional argument 'output'.");
            return KernelPipelinePlan::Return(1);
        }
        println!("Usage:");
        println!(
            "  pdal random <output> [--count=N] [--bounds=([minx,maxx],[miny,maxy],[minz,maxz])] \
             [--distribution=uniform|normal|random] [--compress]"
        );
        return KernelPipelinePlan::Return(0);
    }

    let mut parsed = RandomArgs::default();
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if let Err(code) = parse_random_arg(arg, &mut iter, &mut parsed) {
            return KernelPipelinePlan::Return(code);
        }
    }

    let mode = match parsed.distribution.to_lowercase().as_str() {
        "uniform" => "uniform",
        "normal" => "normal",
        "random" => "random",
        other => {
            eprintln!("PDAL: kernels.random: invalid distribution: {other}");
            return KernelPipelinePlan::Return(1);
        }
    };

    let Some(output) = parsed.output else {
        eprintln!("PDAL: kernels.random: Missing value for positional argument 'output'.");
        return KernelPipelinePlan::Return(1);
    };
    let Some(writer) = infer_writer_driver(&output).map(str::to_string) else {
        eprintln!("PDAL: kernels.random: Unable to infer writer driver for '{output}'.");
        return KernelPipelinePlan::Return(1);
    };

    let mut reader_stage = serde_json::Map::new();
    reader_stage.insert("type".to_string(), serde_json::json!("readers.faux"));
    reader_stage.insert("count".to_string(), serde_json::json!(parsed.count));
    reader_stage.insert("mode".to_string(), serde_json::json!(mode));
    if let Some(bounds) = parsed.bounds {
        reader_stage.insert("bounds".to_string(), serde_json::json!(bounds));
    } else {
        for (k, v) in [
            ("minx", 0.0),
            ("maxx", 1.0),
            ("miny", 0.0),
            ("maxy", 1.0),
            ("minz", 0.0),
            ("maxz", 1.0),
        ] {
            reader_stage.insert(k.to_string(), serde_json::json!(v));
        }
    }

    let mut writer_stage = serde_json::Map::new();
    writer_stage.insert("type".to_string(), serde_json::json!(writer));
    writer_stage.insert("filename".to_string(), serde_json::json!(output));
    if parsed.compress {
        writer_stage.insert("compression".to_string(), serde_json::json!(true));
    }
    writer_stage.extend(parsed.writer_options);

    KernelPipelinePlan::Pipeline(serde_json::json!([
        serde_json::Value::Object(reader_stage),
        serde_json::Value::Object(writer_stage),
    ]))
}

#[derive(Debug)]
struct RandomArgs {
    output: Option<String>,
    count: u64,
    bounds: Option<String>,
    distribution: String,
    compress: bool,
    writer_options: serde_json::Map<String, serde_json::Value>,
}

impl Default for RandomArgs {
    fn default() -> Self {
        Self {
            output: None,
            count: 1000,
            bounds: None,
            distribution: String::from("uniform"),
            compress: false,
            writer_options: serde_json::Map::new(),
        }
    }
}

fn parse_random_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut RandomArgs,
) -> Result<(), i32> {
    if arg == "--count" || arg.starts_with("--count=") {
        let value = option_value(arg, "--count", iter)?;
        match value.parse::<u64>() {
            Ok(count) => parsed.count = count,
            Err(_) => {
                eprintln!("PDAL: kernels.random: --count must be a non-negative integer.");
                return Err(1);
            }
        }
    } else if arg == "--bounds" || arg.starts_with("--bounds=") {
        parsed.bounds = Some(option_value(arg, "--bounds", iter)?);
    } else if arg == "--distribution" || arg.starts_with("--distribution=") {
        parsed.distribution = option_value(arg, "--distribution", iter)?;
    } else if arg == "--compress" || arg == "-z" {
        parsed.compress = true;
    } else if arg == "--mean"
        || arg.starts_with("--mean=")
        || arg == "--stdev"
        || arg.starts_with("--stdev=")
    {
        if split_value(arg).is_none() {
            iter.next();
        }
    } else if arg == "--output" || arg == "-o" {
        let Some(value) = iter.next() else {
            eprintln!("PDAL: kernels.random: Missing value for option '{arg}'.");
            return Err(1);
        };
        if parsed.output.is_some() {
            eprintln!("PDAL: kernels.random: Unexpected argument '{value}'.");
            return Err(1);
        }
        parsed.output = Some(value.clone());
    } else if arg.starts_with("--") {
        if !apply_writer_stage_option(arg, &mut parsed.writer_options) {
            eprintln!("PDAL: kernels.random: Unexpected argument '{arg}'.");
            return Err(1);
        }
    } else if parsed.output.is_none() {
        parsed.output = Some(arg.to_string());
    } else {
        eprintln!("PDAL: kernels.random: Unexpected argument '{arg}'.");
        return Err(1);
    }
    Ok(())
}

fn option_value<'a>(
    arg: &str,
    option: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<String, i32> {
    match split_value(arg) {
        Some(value) => Ok(value),
        None => match iter.next() {
            Some(value) => Ok(value.clone()),
            None => {
                eprintln!("PDAL: kernels.random: Missing value for option '{option}'.");
                Err(1)
            }
        },
    }
}

fn split_value(arg: &str) -> Option<String> {
    arg.split_once('=').map(|(_, value)| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pipeline(args: &[&str]) -> serde_json::Value {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        match build_random_pipeline(&args) {
            KernelPipelinePlan::Pipeline(value) => value,
            KernelPipelinePlan::Return(code) => panic!("unexpected return: {code}"),
        }
    }

    #[test]
    fn default_pipeline_matches_cpp_random_defaults() {
        let value = pipeline(&["out.las"]);
        assert_eq!(value[0]["type"], "readers.faux");
        assert_eq!(value[0]["count"], 1000);
        assert_eq!(value[0]["mode"], "uniform");
        assert_eq!(value[0]["minx"], 0.0);
        assert_eq!(value[1]["type"], "writers.las");
        assert_eq!(value[1]["filename"], "out.las");
    }

    #[test]
    fn parses_count_distribution_bounds_and_writer_options() {
        let value = pipeline(&[
            "--count=7",
            "--distribution",
            "normal",
            "--bounds",
            "([1,2],[3,4],[5,6])",
            "--writers.las.minor_version=4",
            "--compress",
            "out.laz",
        ]);
        assert_eq!(value[0]["count"], 7);
        assert_eq!(value[0]["mode"], "normal");
        assert_eq!(value[0]["bounds"], "([1,2],[3,4],[5,6])");
        assert_eq!(value[1]["compression"], true);
        assert_eq!(value[1]["minor_version"], 4);
    }

    #[test]
    fn rejects_invalid_distribution_and_missing_output() {
        let bad_distribution = vec!["--distribution=bogus".to_string(), "out.las".to_string()];
        assert!(matches!(
            build_random_pipeline(&bad_distribution),
            KernelPipelinePlan::Return(1)
        ));

        let empty: Vec<String> = Vec::new();
        assert!(matches!(
            build_random_pipeline(&empty),
            KernelPipelinePlan::Return(1)
        ));
    }
}
