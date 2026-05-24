use crate::error::string_to_c_ptr;
use crate::registry::pipeline_from_json;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::kernel::{parse_stage_option, ParseStageResult};
use pdal_kernels::{FauxPluginKernel, Kernel, KernelArgs};
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub unsafe extern "C" fn pdal_kernel_parse_stage_option(
    input: *const c_char,
    allow_stage_prefix: bool,
    stage: *mut *mut c_char,
    option: *mut *mut c_char,
    value: *mut *mut c_char,
) -> i32 {
    let input = if input.is_null() {
        String::new()
    } else {
        CStr::from_ptr(input).to_string_lossy().into_owned()
    };
    let parsed = parse_stage_option(&input, allow_stage_prefix);

    if !stage.is_null() {
        *stage = string_to_c_ptr(parsed.stage);
    }
    if !option.is_null() {
        *option = string_to_c_ptr(parsed.option);
    }
    if !value.is_null() {
        *value = string_to_c_ptr(parsed.value);
    }

    match parsed.result {
        ParseStageResult::Ok => 0,
        ParseStageResult::Invalid => 1,
        ParseStageResult::Unknown => 2,
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_rust_kernel_run(
    kernel_name: *const c_char,
    argc: i32,
    argv: *const *const c_char,
) -> i32 {
    if kernel_name.is_null() || argc < 0 || (argc > 0 && argv.is_null()) {
        return -1;
    }

    let name = CStr::from_ptr(kernel_name).to_string_lossy().to_lowercase();
    let name = name.strip_prefix("kernels.").unwrap_or(&name);
    match name {
        "fauxplugin" => run_fauxplugin_kernel(argc, argv),
        "merge" => run_merge_kernel(argc, argv),
        "sort" => run_sort_kernel(argc, argv),
        _ => -1,
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

unsafe fn run_fauxplugin_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() {
        eprintln!("PDAL: kernels.fauxplugin: Missing value for positional argument 'fakearg'.");
        return 1;
    }

    let mut kernel = FauxPluginKernel::default();
    match kernel.run(&KernelArgs::new(args)) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

unsafe fn run_sort_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.sort: Missing value for positional argument 'input'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal sort <input> <output> [--<stage>.<key>=<value> ...]");
        return 0;
    }

    let mut input = None;
    let mut output = None;
    let mut reader_override = None;
    let mut sort_dimension = "X".to_string();
    let mut sort_order = None;
    let mut sort_algorithm = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--input" || arg == "-i" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.sort: Missing value for option '{arg}'.");
                return 1;
            };
            input = Some(value.clone());
        } else if arg == "--output" || arg == "-o" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.sort: Missing value for option '{arg}'.");
                return 1;
            };
            output = Some(value.clone());
        } else if arg == "--driver" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.sort: Missing value for option '--driver'.");
                return 1;
            };
            reader_override = Some(value.clone());
        } else if let Some(value) = arg.strip_prefix("--driver=") {
            reader_override = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("--filters.sort.dimension=") {
            sort_dimension = value.to_string();
        } else if let Some(value) = arg.strip_prefix("--filters.sort.dimensions=") {
            sort_dimension = value.to_string();
        } else if let Some(value) = arg.strip_prefix("--filters.sort.order=") {
            sort_order = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("--filters.sort.algorithm=") {
            sort_algorithm = Some(value.to_string());
        } else if arg.starts_with("--") {
            eprintln!("PDAL: kernels.sort: Unexpected argument '{arg}'.");
            return 1;
        } else if input.is_none() {
            input = Some(arg.clone());
        } else if output.is_none() {
            output = Some(arg.clone());
        } else {
            eprintln!("PDAL: kernels.sort: Unexpected argument '{arg}'.");
            return 1;
        }
    }

    let Some(input) = input else {
        eprintln!("PDAL: kernels.sort: Missing value for positional argument 'input'.");
        return 1;
    };
    let Some(output) = output else {
        eprintln!("PDAL: kernels.sort: Missing value for positional argument 'output'.");
        return 1;
    };

    let Some(reader) = reader_override.or_else(|| infer_reader_driver(&input).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.sort: Unable to infer reader driver for '{input}'.");
        return 1;
    };
    let Some(writer) = infer_writer_driver(&output).map(str::to_string) else {
        eprintln!("PDAL: kernels.sort: Unable to infer writer driver for '{output}'.");
        return 1;
    };

    let mut sort_stage = serde_json::json!({
        "type": "filters.sort",
        "dimensions": sort_dimension,
    });
    if let Some(order) = sort_order {
        sort_stage["order"] = serde_json::json!(order);
    }
    if let Some(algorithm) = sort_algorithm {
        sort_stage["algorithm"] = serde_json::json!(algorithm);
    }

    let pipeline_json = serde_json::json!([
        { "type": reader, "filename": input },
        sort_stage,
        { "type": writer, "filename": output }
    ])
    .to_string();

    let mut pipeline = match pipeline_from_json(&pipeline_json) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.sort: {err}");
            return 1;
        }
    };

    match pipeline.execute(Vec::new()) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("PDAL: kernels.sort: {err}");
            1
        }
    }
}

unsafe fn run_merge_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.merge: Missing value for positional argument 'files'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal merge <input> [input ...] <output> [--<stage>.<key>=<value> ...]");
        return 0;
    }

    let mut positional = Vec::new();
    let mut reader_override = None;
    let mut writer_options = serde_json::Map::new();

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--driver" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.merge: Missing value for option '--driver'.");
                return 1;
            };
            reader_override = Some(value.clone());
        } else if let Some(value) = arg.strip_prefix("--driver=") {
            reader_override = Some(value.to_string());
        } else if arg == "--files" || arg == "-f" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.merge: Missing value for option '{arg}'.");
                return 1;
            };
            positional.push(value.clone());
        } else if arg.starts_with("--") {
            if !apply_writer_stage_option(arg, &mut writer_options) {
                eprintln!("PDAL: kernels.merge: Unexpected argument '{arg}'.");
                return 1;
            }
        } else {
            positional.push(arg.clone());
        }
    }

    if positional.len() < 2 {
        eprintln!("PDAL: kernels.merge: Missing value for positional argument 'files'.");
        return 1;
    }

    let output = positional.last().cloned().unwrap_or_default();
    let inputs = &positional[..positional.len() - 1];
    let Some(writer) = infer_writer_driver(&output).map(str::to_string) else {
        eprintln!("PDAL: kernels.merge: Unable to infer writer driver for '{output}'.");
        return 1;
    };

    let mut stages = Vec::new();
    let mut tags = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let Some(reader) = reader_override
            .clone()
            .or_else(|| infer_reader_driver(input).map(str::to_string))
        else {
            eprintln!("PDAL: kernels.merge: Unable to infer reader driver for '{input}'.");
            return 1;
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
    writer_stage.extend(writer_options);
    stages.push(serde_json::Value::Object(writer_stage));

    execute_kernel_pipeline("merge", serde_json::Value::Array(stages))
}

fn apply_writer_stage_option(
    arg: &str,
    writer_options: &mut serde_json::Map<String, serde_json::Value>,
) -> bool {
    let Some(value) = arg.strip_prefix("--") else {
        return false;
    };
    let parsed = parse_stage_option(value, true);
    if parsed.result != ParseStageResult::Ok || !parsed.stage.starts_with("writers.") {
        return false;
    }
    writer_options.insert(parsed.option, parse_option_value(&parsed.value));
    true
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

fn execute_kernel_pipeline(name: &str, value: serde_json::Value) -> i32 {
    let mut pipeline = match pipeline_from_json(&value.to_string()) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.{name}: {err}");
            return 1;
        }
    };

    match pipeline.execute(Vec::new()) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("PDAL: kernels.{name}: {err}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn rust_kernel_run_reports_unsupported_kernels() {
        let name = CString::new("kernels.missing").unwrap();

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

        assert_eq!(result, -1);
    }

    #[test]
    fn rust_kernel_run_dispatches_fauxplugin() {
        let name = CString::new("fauxplugin").unwrap();
        let arg = CString::new("7").unwrap();
        let argv = [arg.as_ptr()];

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

        assert_eq!(result, 0);
    }

    #[test]
    fn rust_kernel_run_requires_fauxplugin_arg() {
        let name = CString::new("kernels.fauxplugin").unwrap();

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

        assert_eq!(result, 1);
    }

    #[test]
    fn rust_kernel_run_reports_sort_missing_input() {
        let name = CString::new("sort").unwrap();

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

        assert_eq!(result, 1);
    }

    #[test]
    fn rust_kernel_run_reports_merge_missing_files() {
        let name = CString::new("merge").unwrap();

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

        assert_eq!(result, 1);
    }
}
