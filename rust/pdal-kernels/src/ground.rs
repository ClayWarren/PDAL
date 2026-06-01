use crate::stage_options::parse_option_value;
use crate::KernelPipelinePlan;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};

pub fn build_ground_pipeline(args: &[String]) -> KernelPipelinePlan {
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
        return KernelPipelinePlan::Return(0);
    }

    let parsed = match GroundArgs::parse(args) {
        Ok(parsed) => parsed,
        Err(code) => return KernelPipelinePlan::Return(code),
    };

    let Some(input) = parsed.input.as_deref() else {
        eprintln!("PDAL: kernels.ground: Missing value for positional argument 'input'.");
        return KernelPipelinePlan::Return(1);
    };
    let Some(output) = parsed.output.as_deref() else {
        eprintln!("PDAL: kernels.ground: Missing value for positional argument 'output'.");
        return KernelPipelinePlan::Return(1);
    };
    let Some(reader) = parsed
        .reader_override
        .clone()
        .or_else(|| infer_reader_driver(input).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.ground: Unable to infer reader driver for '{input}'.");
        return KernelPipelinePlan::Return(1);
    };
    let Some(writer) = infer_writer_driver(output).map(str::to_string) else {
        eprintln!("PDAL: kernels.ground: Unable to infer writer driver for '{output}'.");
        return KernelPipelinePlan::Return(1);
    };

    let smrf = parsed.smrf_stage();
    let mut stages = vec![serde_json::json!({ "type": reader, "filename": input })];
    if parsed.reset {
        stages.push(serde_json::json!({
            "type": "filters.assign",
            "assignment": "Classification[:]=0",
        }));
    }
    if parsed.denoise {
        stages.push(serde_json::json!({ "type": "filters.outlier" }));
    }
    stages.push(smrf);
    if parsed.extract {
        stages.push(serde_json::json!({
            "type": "filters.range",
            "limits": "Classification[2:2]",
        }));
    }
    stages.push(serde_json::json!({ "type": writer, "filename": output }));

    KernelPipelinePlan::Pipeline(serde_json::Value::Array(stages))
}

struct GroundArgs {
    input: Option<String>,
    output: Option<String>,
    reader_override: Option<String>,
    max_window_size: f64,
    slope: f64,
    cell_size: f64,
    scalar: f64,
    threshold: f64,
    cut: f64,
    returns: Vec<String>,
    returns_set: bool,
    ignore: Vec<String>,
    reset: bool,
    denoise: bool,
    extract: bool,
    smrf_overrides: Vec<(String, serde_json::Value)>,
}

impl Default for GroundArgs {
    fn default() -> Self {
        Self {
            input: None,
            output: None,
            reader_override: None,
            max_window_size: 18.0,
            slope: 0.15,
            cell_size: 1.0,
            scalar: 1.25,
            threshold: 0.5,
            cut: 0.0,
            returns: vec!["last".to_string(), "only".to_string()],
            returns_set: false,
            ignore: Vec::new(),
            reset: false,
            denoise: false,
            extract: false,
            smrf_overrides: Vec::new(),
        }
    }
}

impl GroundArgs {
    fn parse(args: &[String]) -> Result<Self, i32> {
        let mut parsed = Self::default();
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            parse_ground_arg(arg, &mut iter, &mut parsed)?;
        }
        Ok(parsed)
    }

    fn smrf_stage(&self) -> serde_json::Value {
        let mut smrf = serde_json::json!({
            "type": "filters.smrf",
            "window": self.max_window_size,
            "threshold": self.threshold,
            "slope": self.slope,
            "cell": self.cell_size,
            "cut": self.cut,
            "scalar": self.scalar,
            "returns": self.returns.join(","),
        });
        if !self.ignore.is_empty() {
            smrf["ignore"] = serde_json::Value::Array(
                self.ignore
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            );
        }
        for (key, value) in &self.smrf_overrides {
            smrf[key] = value.clone();
        }
        smrf
    }
}

fn parse_ground_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut GroundArgs,
) -> Result<(), i32> {
    if arg == "--input" || arg == "-i" {
        parsed.input = Some(next_value("--input", iter)?);
    } else if arg == "--output" || arg == "-o" {
        parsed.output = Some(next_value("--output", iter)?);
    } else if arg == "--driver" || arg.starts_with("--driver=") {
        parsed.reader_override = Some(value_for(arg, "--driver", iter)?);
    } else if arg == "--label" || arg.starts_with("--label=") {
        let _ = value_for(arg, "--label", iter)?;
    } else if arg == "--developer-debug"
        || arg == "--developer-debug=true"
        || arg == "--developer-debug=false"
    {
    } else if option_matches(arg, "--max_window_size") {
        parsed.max_window_size = parse_f64(
            "max_window_size",
            &value_for(arg, "--max_window_size", iter)?,
        )?;
    } else if option_matches(arg, "--slope") {
        parsed.slope = parse_f64("slope", &value_for(arg, "--slope", iter)?)?;
    } else if option_matches(arg, "--cell_size") {
        parsed.cell_size = parse_f64("cell_size", &value_for(arg, "--cell_size", iter)?)?;
    } else if option_matches(arg, "--scalar") {
        parsed.scalar = parse_f64("scalar", &value_for(arg, "--scalar", iter)?)?;
    } else if option_matches(arg, "--threshold") {
        parsed.threshold = parse_f64("threshold", &value_for(arg, "--threshold", iter)?)?;
    } else if option_matches(arg, "--cut") {
        parsed.cut = parse_f64("cut", &value_for(arg, "--cut", iter)?)?;
    } else if option_matches(arg, "--max_distance") || option_matches(arg, "--initial_distance") {
        let _ = value_for(arg, arg.split_once('=').map_or(arg, |(name, _)| name), iter)?;
    } else if option_matches(arg, "--returns") {
        let returns = value_for(arg, "--returns", iter)?;
        let returns = returns
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if !parsed.returns_set {
            parsed.returns.clear();
            parsed.returns_set = true;
        }
        parsed.returns.extend(returns);
    } else if option_matches(arg, "--ignore") {
        parsed.ignore.push(value_for(arg, "--ignore", iter)?);
    } else if arg == "--reset" || arg == "--reset=true" {
        parsed.reset = true;
    } else if arg == "--reset=false" {
        parsed.reset = false;
    } else if arg == "--denoise" || arg == "--denoise=true" {
        parsed.denoise = true;
    } else if arg == "--denoise=false" {
        parsed.denoise = false;
    } else if arg == "--extract" || arg == "--extract=true" {
        parsed.extract = true;
    } else if arg == "--extract=false" {
        parsed.extract = false;
    } else if let Some(override_option) = parse_smrf_override(arg) {
        parsed.smrf_overrides.push(override_option);
    } else if arg.starts_with("--") {
        eprintln!("PDAL: kernels.ground: Unknown option '{arg}'.");
        return Err(1);
    } else if parsed.input.is_none() {
        parsed.input = Some(arg.to_string());
    } else if parsed.output.is_none() {
        parsed.output = Some(arg.to_string());
    } else {
        eprintln!("PDAL: kernels.ground: Unexpected argument '{arg}'.");
        return Err(1);
    }
    Ok(())
}

fn value_for<'a>(
    arg: &str,
    option: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<String, i32> {
    if let Some((_, value)) = arg.split_once('=') {
        Ok(value.to_string())
    } else {
        next_value(option, iter)
    }
}

fn next_value<'a>(
    option: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<String, i32> {
    match iter.next() {
        Some(value) => Ok(value.clone()),
        None => {
            eprintln!("PDAL: kernels.ground: Missing value for option '{option}'.");
            Err(1)
        }
    }
}

fn parse_f64(name: &str, value: &str) -> Result<f64, i32> {
    value.parse().map_err(|_| {
        eprintln!("PDAL: kernels.ground: Invalid value for option '--{name}'.");
        1
    })
}

fn option_matches(arg: &str, name: &str) -> bool {
    arg == name
        || arg
            .strip_prefix(name)
            .is_some_and(|rest| rest.starts_with('='))
}

fn parse_smrf_override(arg: &str) -> Option<(String, serde_json::Value)> {
    let spec = arg.strip_prefix("--")?;
    let (lhs, value) = spec.split_once('=')?;
    let (stage, key) = lhs.rsplit_once('.')?;
    if stage != "filters.smrf" && stage != "smrf" {
        return None;
    }
    Some((key.to_string(), parse_option_value(value)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    fn pipeline(args: &[&str]) -> serde_json::Value {
        match build_ground_pipeline(&strings(args)) {
            KernelPipelinePlan::Pipeline(value) => value,
            KernelPipelinePlan::Return(code) => panic!("unexpected return code {code}"),
        }
    }

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

    #[test]
    fn builds_default_ground_pipeline() {
        assert_eq!(
            pipeline(&["in.las", "out.las"]),
            serde_json::json!([
                { "type": "readers.las", "filename": "in.las" },
                {
                    "type": "filters.smrf",
                    "window": 18.0,
                    "threshold": 0.5,
                    "slope": 0.15,
                    "cell": 1.0,
                    "cut": 0.0,
                    "scalar": 1.25,
                    "returns": "last,only",
                },
                { "type": "writers.las", "filename": "out.las" },
            ])
        );
    }

    #[test]
    fn builds_optional_ground_stages_and_smrf_overrides() {
        assert_eq!(
            pipeline(&[
                "in.las",
                "out.las",
                "--reset",
                "--denoise",
                "--extract",
                "--returns=first",
                "--ignore=Classification[7:7]",
                "--filters.smrf.window=12",
            ]),
            serde_json::json!([
                { "type": "readers.las", "filename": "in.las" },
                { "type": "filters.assign", "assignment": "Classification[:]=0" },
                { "type": "filters.outlier" },
                {
                    "type": "filters.smrf",
                    "window": 12,
                    "threshold": 0.5,
                    "slope": 0.15,
                    "cell": 1.0,
                    "cut": 0.0,
                    "scalar": 1.25,
                    "returns": "first",
                    "ignore": ["Classification[7:7]"],
                },
                { "type": "filters.range", "limits": "Classification[2:2]" },
                { "type": "writers.las", "filename": "out.las" },
            ])
        );
    }
}
