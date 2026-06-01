use crate::error::pdal_string_free;
use crate::metrics_abi::{pdal_chamfer, pdal_delta_ex, pdal_eval, pdal_hausdorff};
use pdal_kernels::{
    build_chamfer_plan, build_delta_plan, build_eval_plan, build_hausdorff_plan, MetricPairPlan,
    MetricPlan,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub(super) unsafe fn run_hausdorff_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let plan = match build_hausdorff_plan(&args) {
        MetricPlan::Run(plan) => plan,
        MetricPlan::Return(code) => return code,
    };
    let (c_source, c_candidate) = match c_metric_paths(&plan) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    let mut hausdorff = 0.0;
    let mut modified = 0.0;
    if pdal_hausdorff(
        c_source.as_ptr(),
        c_candidate.as_ptr(),
        &mut hausdorff,
        &mut modified,
    ) < 0
    {
        print_last_error();
        return 1;
    }

    let report = serde_json::json!({
        "filenames": [plan.source, plan.candidate],
        "hausdorff": hausdorff,
        "modified_hausdorff": modified,
    });
    println!("{}", serde_json::to_string(&report).unwrap());
    0
}

pub(super) unsafe fn run_chamfer_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let plan = match build_chamfer_plan(&args) {
        MetricPlan::Run(plan) => plan,
        MetricPlan::Return(code) => return code,
    };
    let (c_source, c_candidate) = match c_metric_paths(&plan) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    let mut chamfer = 0.0;
    if pdal_chamfer(c_source.as_ptr(), c_candidate.as_ptr(), &mut chamfer) < 0 {
        print_last_error();
        return 1;
    }

    let report = serde_json::json!({
        "filenames": [plan.source, plan.candidate],
        "chamfer": chamfer,
    });
    println!("{}", serde_json::to_string(&report).unwrap());
    0
}

pub(super) unsafe fn run_delta_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let plan = match build_delta_plan(&args) {
        MetricPlan::Run(plan) => plan,
        MetricPlan::Return(code) => return code,
    };
    let (c_source, c_candidate) = match (
        CString::new(plan.source.as_str()),
        CString::new(plan.candidate.as_str()),
    ) {
        (Ok(c_source), Ok(c_candidate)) => (c_source, c_candidate),
        _ => {
            eprintln!("Error: a filename contains an interior NUL byte");
            return 1;
        }
    };
    let json = pdal_delta_ex(
        c_source.as_ptr(),
        c_candidate.as_ptr(),
        plan.detail,
        plan.all_dims,
    );
    if json.is_null() {
        print_last_error();
        return 1;
    }
    println!("{}", CStr::from_ptr(json).to_string_lossy());
    pdal_string_free(json);
    0
}

pub(super) unsafe fn run_eval_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let eval = match build_eval_plan(&args) {
        MetricPlan::Run(eval) => eval,
        MetricPlan::Return(code) => return code,
    };
    let (c_predicted, c_truth, c_labels, c_prediction_dim, c_truth_dim) = match (
        CString::new(eval.predicted.as_str()),
        CString::new(eval.truth.as_str()),
        CString::new(eval.labels.as_str()),
        CString::new(eval.prediction_dim.as_str()),
        CString::new(eval.truth_dim.as_str()),
    ) {
        (Ok(a), Ok(b), Ok(c), Ok(d), Ok(e)) => (a, b, c, d, e),
        _ => {
            eprintln!("Error: an argument contains an interior NUL byte");
            return 1;
        }
    };

    let json = pdal_eval(
        c_predicted.as_ptr(),
        c_truth.as_ptr(),
        c_labels.as_ptr(),
        c_prediction_dim.as_ptr(),
        c_truth_dim.as_ptr(),
    );
    if json.is_null() {
        print_last_error();
        return 1;
    }
    println!("{}", CStr::from_ptr(json).to_string_lossy());
    pdal_string_free(json);
    0
}

fn c_metric_paths(plan: &MetricPairPlan) -> Result<(CString, CString), i32> {
    match (
        CString::new(plan.source.as_str()),
        CString::new(plan.candidate.as_str()),
    ) {
        (Ok(c_source), Ok(c_candidate)) => Ok((c_source, c_candidate)),
        _ => {
            eprintln!("Error: a filename contains an interior NUL byte");
            Err(1)
        }
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

unsafe fn print_last_error() {
    let message = CStr::from_ptr(crate::error::pdal_last_error()).to_string_lossy();
    if !message.is_empty() {
        eprintln!("{message}");
    }
}
