use crate::error::string_to_c_ptr;
use crate::registry::pipeline_from_json;
use crate::tile_abi::{tile_file, TileRequest};
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::kernel::{parse_stage_option, ParseStageResult};
use pdal_core::options::Options;
use pdal_kernels::{FauxPluginKernel, Kernel, KernelArgs};
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::c_char;

mod ground;
mod metrics;
mod pipeline;
mod split;
mod tindex;

const KERNEL_LIST_JSON: &str = concat!(
    r#"[
{"name":"chamfer","full_name":"kernels.chamfer","description":"Chamfer Kernel [DEPRECATED]"},
{"name":"delta","full_name":"kernels.delta","description":"Delta Kernel [DEPRECATED]"},
{"name":"density","full_name":"kernels.density","description":"Density Kernel [DEPRECATED]"},
{"name":"eval","full_name":"kernels.eval","description":"Eval Kernel [DEPRECATED]"},
{"name":"fauxplugin","full_name":"kernels.fauxplugin","description":"Faux Plugin Kernel"},
{"name":"ground","full_name":"kernels.ground","description":"Ground Kernel [DEPRECATED]"},
{"name":"hausdorff","full_name":"kernels.hausdorff","description":"Hausdorff Kernel [DEPRECATED]"},
{"name":"info","full_name":"kernels.info","description":"Info Kernel"},
{"name":"merge","full_name":"kernels.merge","description":"Merge Kernel"},
{"name":"pipeline","full_name":"kernels.pipeline","description":"Pipeline Kernel"},
{"name":"random","full_name":"kernels.random","description":"Random Kernel [DEPRECATED]"},
{"name":"sort","full_name":"kernels.sort","description":"Sort Kernel"},
{"name":"split","full_name":"kernels.split","description":"Split Kernel"},
{"name":"tile","full_name":"kernels.tile","description":"Tile Kernel"},
{"name":"tindex","full_name":"kernels.tindex","description":"TIndex Kernel"},
{"name":"translate","full_name":"kernels.translate","description":"Translate Kernel"}
]
"#,
    "\0"
);

#[no_mangle]
pub extern "C" fn pdal_rust_kernel_list_json() -> *const c_char {
    KERNEL_LIST_JSON.as_ptr().cast()
}

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
        "chamfer" => metrics::run_chamfer_kernel(argc, argv),
        "delta" => metrics::run_delta_kernel(argc, argv),
        "density" => run_density_kernel(argc, argv),
        "eval" => metrics::run_eval_kernel(argc, argv),
        "fauxplugin" => run_fauxplugin_kernel(argc, argv),
        "ground" => ground::run_ground_kernel(argc, argv),
        "hausdorff" => metrics::run_hausdorff_kernel(argc, argv),
        "info" => pipeline::run_info_kernel(argc, argv),
        "merge" => run_merge_kernel(argc, argv),
        "pipeline" => pipeline::run_pipeline_kernel(argc, argv),
        "random" => run_random_kernel(argc, argv),
        "sort" => run_sort_kernel(argc, argv),
        "split" => split::run_split_kernel(argc, argv),
        "tile" => run_tile_kernel(argc, argv),
        "tindex" => tindex::run_tindex_kernel(argc, argv),
        "translate" => run_translate_kernel(argc, argv),
        _ => -1,
    }
}

unsafe fn run_density_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.density: Missing value for positional argument 'input'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal density <input> <output.geojson> [--<stage>.<key>=<value> ...]");
        return 0;
    }

    let mut input = None;
    let mut output = None;
    let mut reader_override = None;
    let mut hexbin_stage = serde_json::json!({
        "type": "filters.hexbin",
    });

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--input" || arg == "-i" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.density: Missing value for option '{arg}'.");
                return 1;
            };
            input = Some(value.clone());
        } else if arg == "--output" || arg == "-o" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.density: Missing value for option '{arg}'.");
                return 1;
            };
            output = Some(value.clone());
        } else if arg == "--driver" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.density: Missing value for option '--driver'.");
                return 1;
            };
            reader_override = Some(value.clone());
        } else if let Some(value) = arg.strip_prefix("--driver=") {
            reader_override = Some(value.to_string());
        } else if arg == "--ogrdriver" || arg == "-f" {
            if iter.next().is_none() {
                eprintln!("PDAL: kernels.density: Missing value for option '{arg}'.");
                return 1;
            }
        } else if arg == "--edge_length" || arg == "--threshold" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.density: Missing value for option '{arg}'.");
                return 1;
            };
            hexbin_stage[arg.trim_start_matches("--")] = parse_option_value(value);
        } else if let Some(value) = arg.strip_prefix("--edge_length=") {
            hexbin_stage["edge_length"] = parse_option_value(value);
        } else if let Some(value) = arg.strip_prefix("--threshold=") {
            hexbin_stage["threshold"] = parse_option_value(value);
        } else if arg.starts_with("--") {
            let Some(option) = parse_cli_stage_option(arg) else {
                return -1;
            };
            if option.stage != "filters.hexbin" && option.stage != "hexbin" {
                return -1;
            }
            hexbin_stage[option.key.as_str()] = parse_option_value(&option.value);
        } else if input.is_none() {
            input = Some(arg.clone());
        } else if output.is_none() {
            output = Some(arg.clone());
        } else {
            eprintln!("PDAL: kernels.density: Unexpected argument '{arg}'.");
            return 1;
        }
    }

    let Some(input) = input else {
        eprintln!("PDAL: kernels.density: Missing value for positional argument 'input'.");
        return 1;
    };
    let Some(output) = output else {
        eprintln!("PDAL: kernels.density: Missing value for positional argument 'output'.");
        return 1;
    };
    let Some(reader) = reader_override.or_else(|| infer_reader_driver(&input).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.density: Unable to infer reader driver for '{input}'.");
        return 1;
    };

    hexbin_stage["density"] = serde_json::json!(output);
    execute_kernel_pipeline(
        "density",
        serde_json::json!([
            { "type": reader, "filename": input },
            hexbin_stage,
        ]),
    )
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
    // parse_stage_option strips the leading "--" itself; pass `arg` unstripped.
    // (Stripping it here too made every --writers.* option fail to parse,
    // breaking `pdal merge`/`pdal tile` stage-option forwarding.)
    let parsed = parse_stage_option(arg, true);
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

#[derive(Debug)]
pub(super) struct CliStageOption {
    pub(super) stage: String,
    pub(super) key: String,
    pub(super) value: String,
}

pub(super) fn parse_cli_stage_option(arg: &str) -> Option<CliStageOption> {
    let spec = arg.strip_prefix("--")?;
    let (lhs, value) = spec.split_once('=')?;
    let (stage, key) = lhs.rsplit_once('.')?;
    Some(CliStageOption {
        stage: stage.to_string(),
        key: key.to_string(),
        value: value.to_string(),
    })
}

pub(super) fn apply_cli_stage_options(
    stages: &mut [serde_json::Value],
    options: &[CliStageOption],
) -> bool {
    for option in options {
        let qualified = format!("filters.{}", option.stage);
        let mut applied = false;
        for entry in stages.iter_mut() {
            let entry_type = entry["type"].as_str();
            if entry_type == Some(option.stage.as_str()) || entry_type == Some(qualified.as_str()) {
                let value = parse_option_value(&option.value);
                match entry.get_mut(option.key.as_str()) {
                    Some(existing) => {
                        if let Some(values) = existing.as_array_mut() {
                            values.push(value);
                        } else {
                            let first = std::mem::take(existing);
                            *existing = serde_json::json!([first, value]);
                        }
                    }
                    None => entry[option.key.as_str()] = value,
                }
                applied = true;
            }
        }
        if !applied {
            return false;
        }
    }
    true
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

unsafe fn run_translate_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.translate: Missing value for positional argument 'input'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal translate <input> <output> [filter ...] [--<stage>.<key>=<value> ...]");
        return 0;
    }

    let mut input = None;
    let mut output = None;
    let mut reader_override = None;
    let mut writer_override = None;
    let mut filters = Vec::new();
    let mut stage_options = Vec::new();

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--input" || arg == "-i" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            input = Some(value.clone());
        } else if arg == "--output" || arg == "-o" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            output = Some(value.clone());
        } else if arg == "--reader" || arg == "-r" || arg == "--driver" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            reader_override = Some(value.clone());
        } else if arg == "--writer" || arg == "-w" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            writer_override = Some(value.clone());
        } else if arg == "--filter" || arg == "-f" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.translate: Missing value for option '{arg}'.");
                return 1;
            };
            filters.push(value.clone());
        } else if arg.starts_with("--") {
            match parse_cli_stage_option(arg) {
                Some(option) => stage_options.push(option),
                None => return -1,
            }
        } else if input.is_none() {
            input = Some(arg.clone());
        } else if output.is_none() {
            output = Some(arg.clone());
        } else {
            filters.push(arg.clone());
        }
    }

    let Some(input) = input else {
        eprintln!("PDAL: kernels.translate: Missing value for positional argument 'input'.");
        return 1;
    };
    let Some(output) = output else {
        eprintln!("PDAL: kernels.translate: Missing value for positional argument 'output'.");
        return 1;
    };
    let Some(reader) = reader_override.or_else(|| infer_reader_driver(&input).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.translate: Unable to infer reader driver for '{input}'.");
        return 1;
    };
    let Some(writer) = writer_override.or_else(|| infer_writer_driver(&output).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.translate: Unable to infer writer driver for '{output}'.");
        return 1;
    };

    let mut stages = Vec::new();
    stages.push(serde_json::json!({ "type": reader, "filename": input }));
    for filter in filters {
        let stage_type = if filter.contains('.') {
            filter
        } else {
            format!("filters.{filter}")
        };
        stages.push(serde_json::json!({ "type": stage_type }));
    }
    stages.push(serde_json::json!({ "type": writer, "filename": output }));

    let stage_options = match expand_translate_option_files(stage_options) {
        Ok(options) => options,
        Err(code) => return code,
    };
    if !apply_cli_stage_options(&mut stages, &stage_options) {
        return -1;
    }
    execute_kernel_pipeline("translate", serde_json::Value::Array(stages))
}

fn expand_translate_option_files(options: Vec<CliStageOption>) -> Result<Vec<CliStageOption>, i32> {
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

fn parse_option_file(stage: &str, text: &str) -> Result<Vec<CliStageOption>, String> {
    let trimmed = text.trim();
    if trimmed.starts_with('{') {
        let value: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|_| "Unexpected argument".to_string())?;
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

unsafe fn run_random_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.random: Missing value for positional argument 'output'.");
            return 1;
        }
        println!("Usage:");
        println!(
            "  pdal random <output> [--count=N] [--bounds=([minx,maxx],[miny,maxy],[minz,maxz])] \
             [--distribution=uniform|normal|random] [--compress]"
        );
        return 0;
    }

    let mut output = None;
    let mut count = 1000_u64;
    let mut bounds: Option<String> = None;
    // C++ RandomKernel default distribution is "uniform".
    let mut distribution = String::from("uniform");
    let mut compress = false;
    let mut writer_options = serde_json::Map::new();

    // Consume the value for an option given either as `--opt value` or
    // `--opt=value`; `None` signals a missing value.
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        let split = |arg: &str| arg.split_once('=').map(|(_, v)| v.to_string());
        if arg == "--count" || arg.starts_with("--count=") {
            let value = match split(arg) {
                Some(v) => v,
                None => match iter.next() {
                    Some(v) => v.clone(),
                    None => {
                        eprintln!("PDAL: kernels.random: Missing value for option '--count'.");
                        return 1;
                    }
                },
            };
            match value.parse::<u64>() {
                Ok(parsed) => count = parsed,
                Err(_) => {
                    eprintln!("PDAL: kernels.random: --count must be a non-negative integer.");
                    return 1;
                }
            }
        } else if arg == "--bounds" || arg.starts_with("--bounds=") {
            bounds = Some(match split(arg) {
                Some(v) => v,
                None => match iter.next() {
                    Some(v) => v.clone(),
                    None => {
                        eprintln!("PDAL: kernels.random: Missing value for option '--bounds'.");
                        return 1;
                    }
                },
            });
        } else if arg == "--distribution" || arg.starts_with("--distribution=") {
            distribution = match split(arg) {
                Some(v) => v,
                None => match iter.next() {
                    Some(v) => v.clone(),
                    None => {
                        eprintln!(
                            "PDAL: kernels.random: Missing value for option '--distribution'."
                        );
                        return 1;
                    }
                },
            };
        } else if arg == "--compress" || arg == "-z" {
            compress = true;
        } else if arg == "--mean" || arg.starts_with("--mean=") || arg == "--stdev"
            || arg.starts_with("--stdev=")
        {
            // Declared by the C++ kernel but unused in its execute(); accept
            // (consuming any separate value) and ignore for parity.
            if split(arg).is_none() {
                iter.next();
            }
        } else if arg == "--output" || arg == "-o" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.random: Missing value for option '{arg}'.");
                return 1;
            };
            output = Some(value.clone());
        } else if arg.starts_with("--") {
            if !apply_writer_stage_option(arg, &mut writer_options) {
                eprintln!("PDAL: kernels.random: Unexpected argument '{arg}'.");
                return 1;
            }
        } else if output.is_none() {
            output = Some(arg.clone());
        } else {
            eprintln!("PDAL: kernels.random: Unexpected argument '{arg}'.");
            return 1;
        }
    }

    // Map the distribution to a readers.faux mode, matching C++ RandomKernel.
    let mode = match distribution.to_lowercase().as_str() {
        "uniform" => "uniform",
        "normal" => "normal",
        "random" => "random",
        other => {
            eprintln!("PDAL: kernels.random: invalid distribution: {other}");
            return 1;
        }
    };

    let Some(output) = output else {
        eprintln!("PDAL: kernels.random: Missing value for positional argument 'output'.");
        return 1;
    };
    let Some(writer) = infer_writer_driver(&output).map(str::to_string) else {
        eprintln!("PDAL: kernels.random: Unable to infer writer driver for '{output}'.");
        return 1;
    };

    let mut reader_stage = serde_json::Map::new();
    reader_stage.insert("type".to_string(), serde_json::json!("readers.faux"));
    reader_stage.insert("count".to_string(), serde_json::json!(count));
    reader_stage.insert("mode".to_string(), serde_json::json!(mode));
    if let Some(bounds) = bounds {
        reader_stage.insert("bounds".to_string(), serde_json::json!(bounds));
    } else {
        // readers.faux's own defaults are the unit cube; spell them out so the
        // pipeline is identical regardless of the reader's default handling.
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
    if compress {
        writer_stage.insert("compression".to_string(), serde_json::json!(true));
    }
    writer_stage.extend(writer_options);

    execute_kernel_pipeline(
        "random",
        serde_json::json!([
            serde_json::Value::Object(reader_stage),
            serde_json::Value::Object(writer_stage),
        ]),
    )
}

unsafe fn run_tile_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.tile: Missing value for positional argument 'input'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal tile <input> <output-template> [--length=N] [--origin_x=X] [--origin_y=Y] [--buffer=N]");
        return 0;
    }

    let mut input = None;
    let mut output = None;
    let mut length = 1000.0;
    let mut origin_x = f64::NAN;
    let mut origin_y = f64::NAN;
    let mut buffer = 0.0;
    let mut out_srs = None;
    let mut writer_options = Options::new();

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--input" || arg == "-i" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.tile: Missing value for option '{arg}'.");
                return 1;
            };
            input = Some(value.clone());
        } else if arg == "--output" || arg == "-o" {
            let Some(value) = iter.next() else {
                eprintln!("PDAL: kernels.tile: Missing value for option '{arg}'.");
                return 1;
            };
            output = Some(value.clone());
        } else if let Some(rest) = arg.strip_prefix("--") {
            let (key, value) = match rest.split_once('=') {
                Some(pair) => pair,
                None => {
                    let Some(value) = iter.next() else {
                        eprintln!("PDAL: kernels.tile: Missing value for option '{arg}'.");
                        return 1;
                    };
                    (rest, value.as_str())
                }
            };
            let target = match key {
                "length" => &mut length,
                "origin_x" => &mut origin_x,
                "origin_y" => &mut origin_y,
                "buffer" => &mut buffer,
                "out_srs" => {
                    out_srs = Some(value.to_string());
                    continue;
                }
                _ => {
                    let option_text = format!("--{key}={value}");
                    let parsed = parse_stage_option(&option_text, true);
                    if parsed.result == ParseStageResult::Ok && parsed.stage == "writers.text" {
                        writer_options.add(&parsed.option, parsed.value);
                        continue;
                    }
                    return -1;
                }
            };
            match value.parse::<f64>() {
                Ok(parsed) => *target = parsed,
                Err(_) => {
                    eprintln!("PDAL: kernels.tile: Option '--{key}' expects a number.");
                    return 1;
                }
            }
        } else if input.is_none() {
            input = Some(arg.clone());
        } else if output.is_none() {
            output = Some(arg.clone());
        } else {
            eprintln!("PDAL: kernels.tile: Unexpected argument '{arg}'.");
            return 1;
        }
    }

    let Some(input) = input else {
        eprintln!("PDAL: kernels.tile: Missing value for positional argument 'input'.");
        return 1;
    };
    let Some(output) = output else {
        eprintln!("PDAL: kernels.tile: Missing value for positional argument 'output'.");
        return 1;
    };
    if CString::new(input.as_str()).is_err() || CString::new(output.as_str()).is_err() {
        eprintln!("PDAL: kernels.tile: Path contains an interior NUL byte.");
        return 1;
    }

    let request = TileRequest {
        input_path: &input,
        output_template: &output,
        length,
        origin_x,
        origin_y,
        buffer,
        out_srs: out_srs.as_deref(),
        writer_options: &writer_options,
    };
    let count = match tile_file(request) {
        Ok(count) => count,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    println!("Wrote {count} tile(s).");
    0
}

#[cfg(test)]
mod tests;
