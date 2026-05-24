use crate::error::pdal_string_free;
use crate::metrics_abi::{pdal_chamfer, pdal_delta, pdal_eval, pdal_hausdorff};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub(super) unsafe fn run_hausdorff_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.hausdorff: Missing value for positional argument 'source'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal hausdorff <source> <candidate>");
        return 0;
    }

    let (c_source, c_candidate, source, candidate) = match c_metric_paths("hausdorff", &args) {
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
        "filenames": [source, candidate],
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
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.chamfer: Missing value for positional argument 'source'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal chamfer <source> <candidate>");
        return 0;
    }

    let (c_source, c_candidate, source, candidate) = match c_metric_paths("chamfer", &args) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    let mut chamfer = 0.0;
    if pdal_chamfer(c_source.as_ptr(), c_candidate.as_ptr(), &mut chamfer) < 0 {
        print_last_error();
        return 1;
    }

    let report = serde_json::json!({
        "filenames": [source, candidate],
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
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.delta: Missing value for positional argument 'source'.");
            return 1;
        }
        println!("Usage:");
        println!("  pdal delta <source> <candidate>");
        return 0;
    }

    let (c_source, c_candidate, _, _) = match c_metric_paths("delta", &args) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    let json = pdal_delta(c_source.as_ptr(), c_candidate.as_ptr());
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
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.eval: Missing value for positional argument 'predicted'.");
            return 1;
        }
        println!("Usage:");
        println!(
            "  pdal eval <predicted> <truth> --labels=<l1,l2,...> \
             [--prediction_dim=Classification] [--truth_dim=Classification]"
        );
        return 0;
    }

    let eval = match parse_eval_args(&args) {
        Ok(eval) => eval,
        Err(message) => {
            eprintln!("Error: {message}");
            return 1;
        }
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

struct EvalArgs {
    predicted: String,
    truth: String,
    labels: String,
    prediction_dim: String,
    truth_dim: String,
}

fn parse_eval_args(args: &[String]) -> Result<EvalArgs, String> {
    let mut predicted = None;
    let mut truth = None;
    let mut labels = String::new();
    let mut prediction_dim = String::from("Classification");
    let mut truth_dim = String::from("Classification");
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(rest) = arg.strip_prefix("--") {
            let (key, value) = match rest.split_once('=') {
                Some(pair) => pair,
                None => {
                    let Some(value) = iter.next() else {
                        return Err(format!("option '{arg}' requires a value"));
                    };
                    (rest, value.as_str())
                }
            };
            match key {
                "predicted" => predicted = Some(value.to_string()),
                "truth" => truth = Some(value.to_string()),
                "labels" => labels = value.to_string(),
                "prediction_dim" => prediction_dim = value.to_string(),
                "truth_dim" => truth_dim = value.to_string(),
                _ => return Err(format!("unknown eval option '--{key}'")),
            }
        } else if predicted.is_none() {
            predicted = Some(arg.clone());
        } else if truth.is_none() {
            truth = Some(arg.clone());
        } else {
            return Err("eval expects a predicted path and a truth path".to_string());
        }
    }

    let (Some(predicted), Some(truth)) = (predicted, truth) else {
        return Err("eval expects a predicted path and a truth path".to_string());
    };
    if labels.is_empty() {
        return Err("eval requires --labels=<comma-separated classification labels>".to_string());
    }

    Ok(EvalArgs {
        predicted,
        truth,
        labels,
        prediction_dim,
        truth_dim,
    })
}

fn parse_source_candidate_args(command: &str, args: &[String]) -> Result<(String, String), String> {
    let mut source = None;
    let mut candidate = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--source" {
            let Some(value) = iter.next() else {
                return Err("--source requires a filename".to_string());
            };
            source = Some(value.clone());
        } else if let Some(value) = arg.strip_prefix("--source=") {
            source = Some(value.to_string());
        } else if arg == "--candidate" {
            let Some(value) = iter.next() else {
                return Err("--candidate requires a filename".to_string());
            };
            candidate = Some(value.clone());
        } else if let Some(value) = arg.strip_prefix("--candidate=") {
            candidate = Some(value.to_string());
        } else if arg.starts_with("--") {
            return Err(format!("unknown {command} option '{arg}'"));
        } else if source.is_none() {
            source = Some(arg.clone());
        } else if candidate.is_none() {
            candidate = Some(arg.clone());
        } else {
            return Err(format!("{command} expects exactly two filenames"));
        }
    }

    match (source, candidate) {
        (Some(source), Some(candidate)) => Ok((source, candidate)),
        _ => Err(format!("{command} expects exactly two filenames")),
    }
}

fn c_metric_paths(
    command: &str,
    args: &[String],
) -> Result<(CString, CString, String, String), i32> {
    let (source, candidate) = match parse_source_candidate_args(command, args) {
        Ok(paths) => paths,
        Err(message) => {
            eprintln!("Error: {message}");
            return Err(1);
        }
    };

    match (
        CString::new(source.as_str()),
        CString::new(candidate.as_str()),
    ) {
        (Ok(c_source), Ok(c_candidate)) => Ok((c_source, c_candidate, source, candidate)),
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
