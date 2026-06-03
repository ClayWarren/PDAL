use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub enum SplitKernelPlan {
    Run(SplitPlan),
    Return(i32),
}

pub struct SplitPlan {
    pub input: String,
    pub output: PathBuf,
    pub reader: String,
    pub writer: String,
    pub filter: serde_json::Value,
}

pub fn build_split_plan(args: &[String]) -> SplitKernelPlan {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.split: Missing value for positional argument 'input'.");
            return SplitKernelPlan::Return(1);
        }
        println!("Usage:");
        println!(
            "  pdal split <input> <output> [--length=N | --capacity=N] [--origin_x=X] [--origin_y=Y]"
        );
        return SplitKernelPlan::Return(0);
    }

    let split = match SplitArgs::parse(args) {
        Ok(split) => split,
        Err(message) => {
            eprintln!("PDAL: kernels.split: {message}");
            return SplitKernelPlan::Return(1);
        }
    };

    let Some(reader) = split
        .reader_driver
        .clone()
        .or_else(|| infer_reader_driver(&split.input).map(str::to_string))
    else {
        eprintln!(
            "PDAL: kernels.split: Unable to infer reader driver for '{}'.",
            split.input
        );
        return SplitKernelPlan::Return(1);
    };

    let output_name = split.output.to_string_lossy();
    let Some(writer) = infer_writer_driver(&output_name).map(str::to_string) else {
        eprintln!(
            "PDAL: kernels.split: Unable to infer writer driver for '{}'.",
            split.output.display()
        );
        return SplitKernelPlan::Return(1);
    };

    let filter = if let Some(length) = split.length {
        let mut filter = serde_json::json!({
            "type": "filters.splitter",
            "length": length,
        });
        if let Some(origin_x) = split.origin_x {
            filter["origin_x"] = serde_json::json!(origin_x);
        }
        if let Some(origin_y) = split.origin_y {
            filter["origin_y"] = serde_json::json!(origin_y);
        }
        filter
    } else {
        serde_json::json!({
            "type": "filters.chipper",
            "capacity": split.capacity.unwrap_or(100000),
        })
    };

    SplitKernelPlan::Run(SplitPlan {
        input: split.input,
        output: split.output,
        reader,
        writer,
        filter,
    })
}

struct SplitArgs {
    input: String,
    output: PathBuf,
    reader_driver: Option<String>,
    length: Option<f64>,
    capacity: Option<u64>,
    origin_x: Option<f64>,
    origin_y: Option<f64>,
}

impl SplitArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut positional = Vec::new();
        let mut input = None;
        let mut output = None;
        let mut reader_driver = None;
        let mut length = None;
        let mut capacity = None;
        let mut origin_x = None;
        let mut origin_y = None;

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if let Some(value) = arg.strip_prefix("--length=") {
                length = Some(parse_f64_option("length", value)?);
            } else if let Some(value) = arg.strip_prefix("--capacity=") {
                capacity = Some(parse_u64_option("capacity", value)?);
            } else if let Some(value) = arg.strip_prefix("--origin_x=") {
                origin_x = Some(parse_f64_option("origin_x", value)?);
            } else if let Some(value) = arg.strip_prefix("--origin_y=") {
                origin_y = Some(parse_f64_option("origin_y", value)?);
            } else if arg == "--length" {
                length = Some(parse_f64_option("length", next_value(arg, &mut iter)?)?);
            } else if arg == "--capacity" {
                capacity = Some(parse_u64_option("capacity", next_value(arg, &mut iter)?)?);
            } else if arg == "--origin_x" {
                origin_x = Some(parse_f64_option("origin_x", next_value(arg, &mut iter)?)?);
            } else if arg == "--origin_y" {
                origin_y = Some(parse_f64_option("origin_y", next_value(arg, &mut iter)?)?);
            } else if arg == "--input" || arg == "-i" {
                input = Some(next_value(arg, &mut iter)?.to_string());
            } else if let Some(value) = arg.strip_prefix("--input=") {
                input = Some(value.to_string());
            } else if arg == "--output" || arg == "-o" {
                output = Some(next_value(arg, &mut iter)?.to_string());
            } else if let Some(value) = arg.strip_prefix("--output=") {
                output = Some(value.to_string());
            } else if arg == "--driver" {
                reader_driver = Some(next_value("--driver", &mut iter)?.to_string());
            } else if let Some(value) = arg.strip_prefix("--driver=") {
                reader_driver = Some(value.to_string());
            } else if arg.starts_with("--") {
                return Err(format!("unknown option '{arg}' for split"));
            } else {
                positional.push(arg.clone());
            }
        }

        if input.is_none() && !positional.is_empty() {
            input = Some(positional.remove(0));
        }
        if output.is_none() && !positional.is_empty() {
            output = Some(positional.remove(0));
        }
        if input.is_none() || output.is_none() || !positional.is_empty() {
            return Err("split expects an input path and an output path".to_string());
        }
        if length.is_some() && capacity.is_some() {
            return Err("can't specify both length and capacity".to_string());
        }
        if length.is_none() && (origin_x.is_some() || origin_y.is_some()) {
            return Err("origin_x and origin_y require length mode".to_string());
        }

        let input = input.unwrap();
        let output = output.unwrap();
        Ok(Self {
            output: split_output_path(&input, &output),
            input,
            reader_driver,
            length,
            capacity,
            origin_x,
            origin_y,
        })
    }
}

fn next_value<'a>(
    option: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<&'a str, String> {
    iter.next()
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_f64_option(name: &str, value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("--{name} must be numeric"))
}

fn parse_u64_option(name: &str, value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("--{name} must be a non-negative integer"))
}

fn split_output_path(input: &str, output: &str) -> PathBuf {
    let output_path = Path::new(output);
    if output.ends_with(std::path::MAIN_SEPARATOR) || output_path.is_dir() {
        let filename = Path::new(input)
            .file_name()
            .unwrap_or_else(|| OsStr::new(input));
        output_path.join(filename)
    } else {
        output_path.to_path_buf()
    }
}

pub fn numbered_split_output(path: &Path, index: usize) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    let suffix = format!("{stem}_{index}");
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if !extension.is_empty() => {
            path.with_file_name(format!("{suffix}.{extension}"))
        }
        _ => path.with_file_name(suffix),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(args: &[&str]) -> SplitPlan {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        match build_split_plan(&args) {
            SplitKernelPlan::Run(plan) => plan,
            SplitKernelPlan::Return(code) => panic!("unexpected return: {code}"),
        }
    }

    #[test]
    fn builds_default_capacity_split_plan() {
        let plan = plan(&["in.las", "out.las"]);
        assert_eq!(plan.reader, "readers.las");
        assert_eq!(plan.writer, "writers.las");
        assert_eq!(plan.filter["type"], "filters.chipper");
        assert_eq!(plan.filter["capacity"], 100000);
    }

    #[test]
    fn builds_length_split_plan_with_origin() {
        let plan = plan(&[
            "--length=100",
            "--origin_x",
            "1.5",
            "--origin_y=2.5",
            "in.las",
            "out.las",
        ]);
        assert_eq!(plan.filter["type"], "filters.splitter");
        assert_eq!(plan.filter["length"], 100.0);
        assert_eq!(plan.filter["origin_x"], 1.5);
        assert_eq!(plan.filter["origin_y"], 2.5);
    }

    #[test]
    fn accepts_input_output_equals_forms() {
        let plan = plan(&["--input=in.las", "--output=out.las"]);
        assert_eq!(plan.input, "in.las");
        assert_eq!(plan.output, PathBuf::from("out.las"));
    }

    #[test]
    fn rejects_conflicting_modes() {
        let args = vec![
            "--length=1".to_string(),
            "--capacity=1".to_string(),
            "in.las".to_string(),
            "out.las".to_string(),
        ];
        assert!(matches!(
            build_split_plan(&args),
            SplitKernelPlan::Return(1)
        ));
    }

    #[test]
    fn numbers_outputs_before_extension() {
        assert_eq!(
            numbered_split_output(Path::new("out.las"), 2),
            PathBuf::from("out_2.las")
        );
    }
}
