#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetricPlan<T> {
    Run(T),
    Return(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricPairPlan {
    pub source: String,
    pub candidate: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeltaPlan {
    pub source: String,
    pub candidate: String,
    pub detail: bool,
    pub all_dims: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvalPlan {
    pub predicted: String,
    pub truth: String,
    pub labels: String,
    pub prediction_dim: String,
    pub truth_dim: String,
}

pub fn build_hausdorff_plan(args: &[String]) -> MetricPlan<MetricPairPlan> {
    build_pair_plan("hausdorff", args)
}

pub fn build_chamfer_plan(args: &[String]) -> MetricPlan<MetricPairPlan> {
    build_pair_plan("chamfer", args)
}

pub fn build_delta_plan(args: &[String]) -> MetricPlan<DeltaPlan> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.delta: Missing value for positional argument 'source'.");
            return MetricPlan::Return(1);
        }
        println!("Usage:");
        println!("  pdal delta <source> <candidate>");
        return MetricPlan::Return(0);
    }

    match parse_delta_args(args) {
        Ok(plan) => MetricPlan::Run(plan),
        Err(message) => {
            eprintln!("Error: {message}");
            MetricPlan::Return(1)
        }
    }
}

pub fn build_eval_plan(args: &[String]) -> MetricPlan<EvalPlan> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.eval: Missing value for positional argument 'predicted'.");
            return MetricPlan::Return(1);
        }
        println!("Usage:");
        println!(
            "  pdal eval <predicted> <truth> --labels=<l1,l2,...> \
             [--prediction_dim=Classification] [--truth_dim=Classification]"
        );
        return MetricPlan::Return(0);
    }

    match parse_eval_args(args) {
        Ok(plan) => MetricPlan::Run(plan),
        Err(message) => {
            eprintln!("Error: {message}");
            MetricPlan::Return(1)
        }
    }
}

fn build_pair_plan(command: &str, args: &[String]) -> MetricPlan<MetricPairPlan> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.{command}: Missing value for positional argument 'source'.");
            return MetricPlan::Return(1);
        }
        println!("Usage:");
        println!("  pdal {command} <source> <candidate>");
        return MetricPlan::Return(0);
    }

    match parse_source_candidate_args(command, args) {
        Ok((source, candidate)) => MetricPlan::Run(MetricPairPlan { source, candidate }),
        Err(message) => {
            eprintln!("Error: {message}");
            MetricPlan::Return(1)
        }
    }
}

fn parse_delta_args(args: &[String]) -> Result<DeltaPlan, String> {
    let mut source = None;
    let mut candidate = None;
    let mut detail = false;
    let mut all_dims = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--detail" => detail = true,
            "--alldims" => all_dims = true,
            _ if arg.starts_with("--detail=") => {
                detail = parse_bool_flag("detail", arg)?;
            }
            _ if arg.starts_with("--alldims=") => {
                all_dims = parse_bool_flag("alldims", arg)?;
            }
            "--source" => source = Some(next_value("--source", &mut iter)?.to_string()),
            "--candidate" => candidate = Some(next_value("--candidate", &mut iter)?.to_string()),
            _ if let Some(value) = arg.strip_prefix("--source=") => {
                source = Some(value.to_string());
            }
            _ if let Some(value) = arg.strip_prefix("--candidate=") => {
                candidate = Some(value.to_string());
            }
            _ if arg.starts_with("--") => return Err(format!("unknown delta option '{arg}'")),
            _ if source.is_none() => source = Some(arg.clone()),
            _ if candidate.is_none() => candidate = Some(arg.clone()),
            _ => return Err("delta expects exactly two filenames".to_string()),
        }
    }

    match (source, candidate) {
        (Some(source), Some(candidate)) => Ok(DeltaPlan {
            source,
            candidate,
            detail,
            all_dims,
        }),
        _ => Err("delta expects exactly two filenames".to_string()),
    }
}

fn parse_eval_args(args: &[String]) -> Result<EvalPlan, String> {
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
                None => (rest, next_value(arg, &mut iter)?),
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

    Ok(EvalPlan {
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
            source = Some(next_value("--source", &mut iter)?.to_string());
        } else if let Some(value) = arg.strip_prefix("--source=") {
            source = Some(value.to_string());
        } else if arg == "--candidate" {
            candidate = Some(next_value("--candidate", &mut iter)?.to_string());
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

fn parse_bool_flag(name: &str, arg: &str) -> Result<bool, String> {
    let value = arg
        .split_once('=')
        .map(|(_, value)| value)
        .ok_or_else(|| format!("--{name} expects a boolean value"))?;
    match value {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!("--{name} expects a boolean value")),
    }
}

fn next_value<'a>(
    option: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<&'a str, String> {
    iter.next()
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a filename"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn pair_plan_accepts_positionals_and_options() {
        assert_eq!(
            build_chamfer_plan(&strings(&["a.las", "b.las"])),
            MetricPlan::Run(MetricPairPlan {
                source: "a.las".to_string(),
                candidate: "b.las".to_string(),
            })
        );
        assert_eq!(
            build_hausdorff_plan(&strings(&["--source=a.las", "--candidate", "b.las"])),
            MetricPlan::Run(MetricPairPlan {
                source: "a.las".to_string(),
                candidate: "b.las".to_string(),
            })
        );
    }

    #[test]
    fn delta_plan_tracks_flags() {
        assert_eq!(
            build_delta_plan(&strings(&["--detail", "--alldims", "a.las", "b.las"])),
            MetricPlan::Run(DeltaPlan {
                source: "a.las".to_string(),
                candidate: "b.las".to_string(),
                detail: true,
                all_dims: true,
            })
        );
    }

    #[test]
    fn delta_plan_accepts_boolean_flag_values() {
        assert_eq!(
            build_delta_plan(&strings(&[
                "--detail=true",
                "--alldims=false",
                "a.las",
                "b.las"
            ])),
            MetricPlan::Run(DeltaPlan {
                source: "a.las".to_string(),
                candidate: "b.las".to_string(),
                detail: true,
                all_dims: false,
            })
        );

        assert!(matches!(
            build_delta_plan(&strings(&["--detail=maybe", "a.las", "b.las"])),
            MetricPlan::Return(1)
        ));
    }

    #[test]
    fn eval_plan_defaults_dimensions() {
        assert_eq!(
            build_eval_plan(&strings(&["pred.las", "truth.las", "--labels=2,3"])),
            MetricPlan::Run(EvalPlan {
                predicted: "pred.las".to_string(),
                truth: "truth.las".to_string(),
                labels: "2,3".to_string(),
                prediction_dim: "Classification".to_string(),
                truth_dim: "Classification".to_string(),
            })
        );
    }

    #[test]
    fn eval_requires_labels() {
        assert!(matches!(
            build_eval_plan(&strings(&["pred.las", "truth.las"])),
            MetricPlan::Return(1)
        ));
    }
}
