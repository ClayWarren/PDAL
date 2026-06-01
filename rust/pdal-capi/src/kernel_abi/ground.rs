use crate::registry::pipeline_from_json;
use pdal_kernels::{build_ground_pipeline, KernelPipelinePlan};
use std::ffi::CStr;
use std::os::raw::c_char;

pub(super) unsafe fn run_ground_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    match build_ground_pipeline(&args) {
        KernelPipelinePlan::Pipeline(value) => execute_ground_pipeline(value),
        KernelPipelinePlan::Return(code) => code,
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
