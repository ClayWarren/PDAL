use crate::registry::pipeline_from_json;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use std::ffi::CStr;
use std::os::raw::c_char;

pub(super) unsafe fn run_ground_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.ground: Missing value for positional argument 'input'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal ground <input> <output> [--<stage>.<key>=<value> ...]");
        return 0;
    }

    let mut input = None;
    let mut output = None;
    let mut reader_override = None;
    let mut smrf_stage = serde_json::json!({
        "type": "filters.smrf",
    });

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--input" || arg == "-i" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.ground: Missing value for option '{arg}'.");
                return 1;
            };
            input = Some(value.clone());
        } else if arg == "--output" || arg == "-o" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.ground: Missing value for option '{arg}'.");
                return 1;
            };
            output = Some(value.clone());
        } else if arg == "--driver" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.ground: Missing value for option '--driver'.");
                return 1;
            };
            reader_override = Some(value.clone());
        } else if let Some(value) = arg.strip_prefix("--driver=") {
            reader_override = Some(value.to_string());
        } else if arg.starts_with("--") {
            let Some(option) = parse_smrf_option(arg) else {
                return -1;
            };
            smrf_stage[option.key.as_str()] = parse_option_value(&option.value);
        } else if input.is_none() {
            input = Some(arg.clone());
        } else if output.is_none() {
            output = Some(arg.clone());
        } else {
            eprintln!("PDAL: kernels.ground: Unexpected argument '{arg}'.");
            return 1;
        }
    }

    let Some(input) = input else {
        eprintln!("PDAL: kernels.ground: Missing value for positional argument 'input'.");
        return 1;
    };
    let Some(output) = output else {
        eprintln!("PDAL: kernels.ground: Missing value for positional argument 'output'.");
        return 1;
    };
    let Some(reader) = reader_override.or_else(|| infer_reader_driver(&input).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.ground: Unable to infer reader driver for '{input}'.");
        return 1;
    };
    let Some(writer) = infer_writer_driver(&output).map(str::to_string) else {
        eprintln!("PDAL: kernels.ground: Unable to infer writer driver for '{output}'.");
        return 1;
    };

    execute_ground_pipeline(serde_json::json!([
        { "type": reader, "filename": input },
        smrf_stage,
        { "type": writer, "filename": output },
    ]))
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

struct SmrfOption {
    key: String,
    value: String,
}

fn parse_smrf_option(arg: &str) -> Option<SmrfOption> {
    let spec = arg.strip_prefix("--")?;
    let (lhs, value) = spec.split_once('=')?;
    let (stage, key) = lhs.rsplit_once('.')?;
    if stage != "filters.smrf" && stage != "smrf" {
        return None;
    }
    Some(SmrfOption {
        key: key.to_string(),
        value: value.to_string(),
    })
}

fn parse_option_value(value: &str) -> serde_json::Value {
    if let Ok(number) = value.parse::<u64>() {
        serde_json::json!(number)
    } else if let Ok(number) = value.parse::<f64>() {
        serde_json::json!(number)
    } else if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        serde_json::json!(value.eq_ignore_ascii_case("true"))
    } else {
        serde_json::json!(value)
    }
}

fn execute_ground_pipeline(value: serde_json::Value) -> i32 {
    let mut pipeline = match pipeline_from_json(&value.to_string()) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.ground: {err}");
            return 1;
        }
    };

    match pipeline.execute(Vec::new()) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("PDAL: kernels.ground: {err}");
            1
        }
    }
}
