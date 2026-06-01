use crate::error::string_to_c_ptr;
use crate::registry::pipeline_from_json;
use crate::tile_abi::{tile_file, TileRequest};
use pdal_core::kernel::{parse_stage_option, ParseStageResult};
use pdal_kernels::{
    build_density_pipeline, build_merge_pipeline, build_random_pipeline, build_sort_pipeline,
    build_tile_plan, FauxPluginKernel, Kernel, KernelArgs, KernelPipelinePlan, TileKernelPlan,
    KERNEL_LIST_JSON,
};
use std::ffi::CStr;
use std::os::raw::c_char;

mod ground;
mod metrics;
mod pipeline;
mod split;
mod tindex;
mod translate;

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
        "translate" => translate::run_translate_kernel(argc, argv),
        _ => -1,
    }
}

unsafe fn run_density_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    match build_density_pipeline(&args) {
        KernelPipelinePlan::Pipeline(value) => execute_kernel_pipeline("density", value),
        KernelPipelinePlan::Return(code) => code,
    }
}

pub(super) unsafe fn argv_to_vec(
    argc: i32,
    argv: *const *const c_char,
) -> Result<Vec<String>, i32> {
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

    match build_sort_pipeline(&args) {
        KernelPipelinePlan::Pipeline(value) => execute_kernel_pipeline("sort", value),
        KernelPipelinePlan::Return(code) => code,
    }
}

unsafe fn run_merge_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    match build_merge_pipeline(&args) {
        KernelPipelinePlan::Pipeline(value) => execute_kernel_pipeline("merge", value),
        KernelPipelinePlan::Return(code) => code,
    }
}

pub(super) fn parse_option_value(value: &str) -> serde_json::Value {
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

pub(super) fn execute_kernel_pipeline(name: &str, value: serde_json::Value) -> i32 {
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

unsafe fn run_random_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    match build_random_pipeline(&args) {
        KernelPipelinePlan::Pipeline(value) => execute_kernel_pipeline("random", value),
        KernelPipelinePlan::Return(code) => code,
    }
}

unsafe fn run_tile_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    let plan = match build_tile_plan(&args) {
        TileKernelPlan::Run(plan) => plan,
        TileKernelPlan::Return(code) => return code,
    };

    let request = TileRequest {
        input_path: &plan.input,
        output_template: &plan.output,
        length: plan.length,
        origin_x: plan.origin_x,
        origin_y: plan.origin_y,
        buffer: plan.buffer,
        out_srs: plan.out_srs.as_deref(),
        writer_options: &plan.writer_options,
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
