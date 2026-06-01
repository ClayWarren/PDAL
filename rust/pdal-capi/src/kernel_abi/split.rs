use crate::registry::{create_writer, pipeline_from_json};
use pdal_core::options::Options;
use pdal_kernels::{build_split_plan, numbered_split_output, SplitKernelPlan};
use std::ffi::CStr;
use std::os::raw::c_char;

pub(super) unsafe fn run_split_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    let split = match build_split_plan(&args) {
        SplitKernelPlan::Run(split) => split,
        SplitKernelPlan::Return(code) => return code,
    };

    let stages = serde_json::json!([
        { "type": split.reader, "filename": split.input },
        split.filter,
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
        let filename = numbered_split_output(&split.output, index + 1);
        let mut options = Options::new();
        options.add("filename", filename.display());
        let mut writer = match create_writer(&split.writer, &options) {
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
