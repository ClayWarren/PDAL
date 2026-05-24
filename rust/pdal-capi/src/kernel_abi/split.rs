use crate::registry::{create_writer, pipeline_from_json};
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::options::Options;
use std::ffi::{CStr, OsStr};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

pub(super) unsafe fn run_split_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.split: Missing value for positional argument 'input'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal split <input> <output> [--length=N | --capacity=N] [--origin_x=X] [--origin_y=Y]");
        return 0;
    }

    let split = match SplitArgs::parse(&args) {
        Ok(split) => split,
        Err(message) => {
            eprintln!("PDAL: kernels.split: {message}");
            return 1;
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
        return 1;
    };

    let output_name = split.output.to_string_lossy();
    let Some(writer_name) = infer_writer_driver(&output_name).map(str::to_string) else {
        eprintln!(
            "PDAL: kernels.split: Unable to infer writer driver for '{}'.",
            split.output.display()
        );
        return 1;
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

    let stages = serde_json::json!([
        { "type": reader, "filename": split.input },
        filter,
    ]);
    let mut pipeline = match pipeline_from_json(&stages.to_string()) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.split: {err}");
            return 1;
        }
    };
    let views = match pipeline.execute(Vec::new()) {
        Ok(views) => views,
        Err(err) => {
            eprintln!("PDAL: kernels.split: {err}");
            return 1;
        }
    };

    for (index, view) in views.iter().enumerate() {
        let filename = numbered_output(&split.output, index + 1);
        let mut options = Options::new();
        options.add("filename", filename.display());
        let mut writer = match create_writer(&writer_name, &options) {
            Ok(writer) => writer,
            Err(err) => {
                eprintln!("PDAL: kernels.split: {err}");
                return 1;
            }
        };
        if let Err(err) = writer.write(std::slice::from_ref(view)) {
            eprintln!("PDAL: kernels.split: {err}");
            return 1;
        }
    }

    0
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
                let Some(value) = iter.next() else {
                    return Err("--length requires a value".to_string());
                };
                length = Some(parse_f64_option("length", value)?);
            } else if arg == "--capacity" {
                let Some(value) = iter.next() else {
                    return Err("--capacity requires a value".to_string());
                };
                capacity = Some(parse_u64_option("capacity", value)?);
            } else if arg == "--origin_x" {
                let Some(value) = iter.next() else {
                    return Err("--origin_x requires a value".to_string());
                };
                origin_x = Some(parse_f64_option("origin_x", value)?);
            } else if arg == "--origin_y" {
                let Some(value) = iter.next() else {
                    return Err("--origin_y requires a value".to_string());
                };
                origin_y = Some(parse_f64_option("origin_y", value)?);
            } else if arg == "--input" || arg == "-i" {
                let Some(value) = iter.next() else {
                    return Err(format!("{arg} requires an input path"));
                };
                input = Some(value.clone());
            } else if arg == "--output" || arg == "-o" {
                let Some(value) = iter.next() else {
                    return Err(format!("{arg} requires an output path"));
                };
                output = Some(value.clone());
            } else if arg == "--driver" {
                let Some(value) = iter.next() else {
                    return Err("--driver requires a reader driver name".to_string());
                };
                reader_driver = Some(value.clone());
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

fn numbered_output(path: &Path, index: usize) -> PathBuf {
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
