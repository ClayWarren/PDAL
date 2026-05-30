use crate::registry::pipeline_from_json;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use std::ffi::CStr;
use std::os::raw::c_char;

/// `pdal ground` -- progressive morphological ground segmentation.
///
/// Faithful port of the C++ `GroundKernel`: it builds a
/// reader -> [assign] -> [outlier] -> smrf -> [range] -> writer pipeline,
/// mapping the kernel's options onto `filters.smrf` and optionally inserting
/// `filters.assign` (reset), `filters.outlier` (denoise), and `filters.range`
/// (extract). The `--filters.smrf.<key>` passthrough is also accepted as a
/// superset convenience. Any option the C++ kernel did not understand now
/// errors (return 1) rather than falling back to the deleted C++ kernel.
#[allow(clippy::cognitive_complexity)]
pub(super) unsafe fn run_ground_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Usage:");
        println!("  pdal ground <input> <output> [options]");
        println!("    --max_window_size  Max window size [18.0]");
        println!("    --slope            Slope [0.15]");
        println!("    --cell_size        Cell size [1.0]");
        println!("    --scalar           Elevation scalar [1.25]");
        println!("    --threshold        Elevation threshold [0.5]");
        println!("    --cut              Cut net size [0.0]");
        println!("    --returns          Return types to consider [last,only]");
        println!("    --ignore           Range query to ignore (repeatable)");
        println!("    --reset            Reset classifications before segmenting");
        println!("    --denoise          Apply outlier removal before segmenting");
        println!("    --extract          Extract ground returns only");
        return 0;
    }

    let mut input = None;
    let mut output = None;
    let mut reader_override = None;

    // GroundKernel defaults. The C++ kernel always passes these to filters.smrf,
    // so we set them explicitly rather than relying on the smrf defaults.
    let mut max_window_size = 18.0_f64;
    let mut slope = 0.15_f64;
    let mut cell_size = 1.0_f64;
    let mut scalar = 1.25_f64;
    let mut threshold = 0.5_f64;
    let mut cut = 0.0_f64;
    let mut returns: Vec<String> = vec!["last".to_string(), "only".to_string()];
    let mut returns_set = false;
    let mut ignore: Vec<String> = Vec::new();
    let mut reset = false;
    let mut denoise = false;
    let mut extract = false;
    // Extra `--filters.smrf.<key>=<value>` overrides applied last.
    let mut smrf_overrides: Vec<(String, serde_json::Value)> = Vec::new();

    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        macro_rules! value_for {
            ($name:expr) => {{
                match split_value(arg) {
                    Some(v) => v,
                    None => match iter.next() {
                        Some(v) => v.clone(),
                        None => {
                            eprintln!(
                                "PDAL: kernels.ground: Missing value for option '{}'.",
                                $name
                            );
                            return 1;
                        }
                    },
                }
            }};
        }

        if arg == "--input" || arg == "-i" {
            input = Some(value_for!("--input"));
        } else if arg == "--output" || arg == "-o" {
            output = Some(value_for!("--output"));
        } else if arg == "--driver" || arg.starts_with("--driver=") {
            reader_override = Some(value_for!("--driver"));
        } else if arg == "--label" || arg.starts_with("--label=") {
            // Process label: accepted and ignored, matching the C++ basic switch.
            let _ = value_for!("--label");
        } else if arg == "--developer-debug"
            || arg == "--developer-debug=true"
            || arg == "--developer-debug=false"
        {
            // Accepted and ignored.
        } else if option_matches(arg, "--max_window_size") {
            max_window_size = match value_for!("--max_window_size").parse() {
                Ok(v) => v,
                Err(_) => return invalid_value("max_window_size"),
            };
        } else if option_matches(arg, "--slope") {
            slope = match value_for!("--slope").parse() {
                Ok(v) => v,
                Err(_) => return invalid_value("slope"),
            };
        } else if option_matches(arg, "--cell_size") {
            cell_size = match value_for!("--cell_size").parse() {
                Ok(v) => v,
                Err(_) => return invalid_value("cell_size"),
            };
        } else if option_matches(arg, "--scalar") {
            scalar = match value_for!("--scalar").parse() {
                Ok(v) => v,
                Err(_) => return invalid_value("scalar"),
            };
        } else if option_matches(arg, "--threshold") {
            threshold = match value_for!("--threshold").parse() {
                Ok(v) => v,
                Err(_) => return invalid_value("threshold"),
            };
        } else if option_matches(arg, "--cut") {
            cut = match value_for!("--cut").parse() {
                Ok(v) => v,
                Err(_) => return invalid_value("cut"),
            };
        } else if option_matches(arg, "--max_distance") {
            // Declared by the C++ kernel but unused; accept and ignore.
            let _ = value_for!("--max_distance");
        } else if option_matches(arg, "--initial_distance") {
            let _ = value_for!("--initial_distance");
        } else if option_matches(arg, "--returns") {
            // Replaces the default set; accepts comma-separated or repeated.
            let value = value_for!("--returns");
            let parsed: Vec<String> = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !returns_set {
                returns.clear();
                returns_set = true;
            }
            returns.extend(parsed);
        } else if option_matches(arg, "--ignore") {
            ignore.push(value_for!("--ignore"));
        } else if arg == "--reset" || arg == "--reset=true" {
            reset = true;
        } else if arg == "--reset=false" {
            reset = false;
        } else if arg == "--denoise" || arg == "--denoise=true" {
            denoise = true;
        } else if arg == "--denoise=false" {
            denoise = false;
        } else if arg == "--extract" || arg == "--extract=true" {
            extract = true;
        } else if arg == "--extract=false" {
            extract = false;
        } else if let Some(over) = parse_smrf_override(arg) {
            smrf_overrides.push(over);
        } else if arg.starts_with("--") {
            eprintln!("PDAL: kernels.ground: Unknown option '{arg}'.");
            return 1;
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

    let mut smrf = serde_json::json!({
        "type": "filters.smrf",
        "window": max_window_size,
        "threshold": threshold,
        "slope": slope,
        "cell": cell_size,
        "cut": cut,
        "scalar": scalar,
        "returns": returns.join(","),
    });
    if !ignore.is_empty() {
        smrf["ignore"] =
            serde_json::Value::Array(ignore.into_iter().map(serde_json::Value::String).collect());
    }
    for (key, value) in smrf_overrides {
        smrf[key] = value;
    }

    let mut stages = vec![serde_json::json!({ "type": reader, "filename": input })];
    if reset {
        stages.push(serde_json::json!({
            "type": "filters.assign",
            "assignment": "Classification[:]=0",
        }));
    }
    if denoise {
        stages.push(serde_json::json!({ "type": "filters.outlier" }));
    }
    stages.push(smrf);
    if extract {
        stages.push(serde_json::json!({
            "type": "filters.range",
            "limits": "Classification[2:2]",
        }));
    }
    stages.push(serde_json::json!({ "type": writer, "filename": output }));

    execute_ground_pipeline(serde_json::Value::Array(stages))
}

fn invalid_value(name: &str) -> i32 {
    eprintln!("PDAL: kernels.ground: Invalid value for option '--{name}'.");
    1
}

fn option_matches(arg: &str, name: &str) -> bool {
    arg == name
        || arg
            .strip_prefix(name)
            .is_some_and(|rest| rest.starts_with('='))
}

/// Split an `--option=value` argument into its value, or `None` for the
/// space-separated `--option value` form.
fn split_value(arg: &str) -> Option<String> {
    arg.split_once('=').map(|(_, v)| v.to_string())
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

/// Parse a `--filters.smrf.<key>=<value>` (or `--smrf.<key>=<value>`) override.
fn parse_smrf_override(arg: &str) -> Option<(String, serde_json::Value)> {
    let spec = arg.strip_prefix("--")?;
    let (lhs, value) = spec.split_once('=')?;
    let (stage, key) = lhs.rsplit_once('.')?;
    if stage != "filters.smrf" && stage != "smrf" {
        return None;
    }
    Some((key.to_string(), parse_option_value(value)))
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

#[cfg(test)]
mod tests {
    use super::option_matches;

    #[test]
    fn option_matches_exact_or_equals_forms_only() {
        assert!(option_matches("--slope", "--slope"));
        assert!(option_matches("--slope=0.2", "--slope"));

        assert!(!option_matches("--slope_bad=0.2", "--slope"));
        assert!(!option_matches("--thresholded=1.0", "--threshold"));
        assert!(!option_matches(
            "--ignoreme=Classification[7:7]",
            "--ignore"
        ));
    }
}
