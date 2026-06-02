use pdal_core::driver::infer_reader_driver;
use pdal_core::point::{DimId, PointId};
use std::io::Read;

pub enum InfoKernelPlan {
    Run(InfoRunPlan),
    Return(i32),
}

pub enum InfoRunPlan {
    File {
        filename: String,
        driver: String,
        mode: InfoMode,
        pc_type: String,
        serialization_file: Option<String>,
    },
    PipelineJson {
        json: String,
        mode: InfoMode,
        pc_type: String,
        serialization_file: Option<String>,
    },
}

#[derive(Clone)]
pub enum InfoMode {
    Summary,
    Stats {
        dimensions: Option<Vec<DimId>>,
        enumerate: Option<Vec<DimId>>,
        breakout: Option<DimId>,
    },
    Schema,
    Metadata,
    All,
    Boundary,
    Stac,
    Points(Vec<PointId>),
    Query(QueryRequest),
}

impl InfoMode {
    pub fn needs_boundary(&self) -> bool {
        matches!(self, Self::All | Self::Boundary)
    }
}

#[derive(Clone, Copy)]
pub struct QueryRequest {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
    pub count: usize,
}

pub fn build_info_plan(args: &[String]) -> InfoKernelPlan {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.info: Missing value for positional argument 'input'.");
            return InfoKernelPlan::Return(1);
        }
        println!("Usage:");
        println!("  pdal info --summary <file>");
        return InfoKernelPlan::Return(0);
    }

    let parsed = match InfoArgs::parse(args) {
        Ok(parsed) => parsed,
        Err(code) => return InfoKernelPlan::Return(code),
    };

    if parsed.read_stdin && parsed.filename.is_some() {
        eprintln!("PDAL: kernels.info: Expected either --stdin or an input filename, not both.");
        return InfoKernelPlan::Return(1);
    }

    if parsed.read_stdin {
        let mut json = String::new();
        if let Err(err) = std::io::stdin().read_to_string(&mut json) {
            eprintln!("PDAL: kernels.info: Unable to read pipeline from stdin: {err}");
            return InfoKernelPlan::Return(1);
        }
        return InfoKernelPlan::Run(InfoRunPlan::PipelineJson {
            json,
            mode: parsed.mode,
            pc_type: parsed.pc_type,
            serialization_file: parsed.serialization_file,
        });
    }

    let Some(filename) = parsed.filename else {
        eprintln!("PDAL: kernels.info: Missing value for positional argument 'input'.");
        return InfoKernelPlan::Return(1);
    };
    let Some(driver) = parsed
        .driver_override
        .or_else(|| infer_reader_driver(&filename).map(str::to_string))
    else {
        eprintln!("PDAL: kernels.info: Unable to infer reader driver for '{filename}'.");
        return InfoKernelPlan::Return(1);
    };

    InfoKernelPlan::Run(InfoRunPlan::File {
        filename,
        driver,
        mode: parsed.mode,
        pc_type: parsed.pc_type,
        serialization_file: parsed.serialization_file,
    })
}

struct InfoArgs {
    filename: Option<String>,
    driver_override: Option<String>,
    mode: InfoMode,
    pc_type: String,
    serialization_file: Option<String>,
    read_stdin: bool,
}

impl InfoArgs {
    fn parse(args: &[String]) -> Result<Self, i32> {
        let mut parsed = Self {
            filename: None,
            driver_override: None,
            mode: InfoMode::Summary,
            pc_type: "lidar".to_string(),
            serialization_file: None,
            read_stdin: false,
        };

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            parse_info_arg(arg, &mut iter, &mut parsed)?;
        }
        Ok(parsed)
    }
}

fn parse_info_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut InfoArgs,
) -> Result<(), i32> {
    if arg == "--summary" {
        parsed.mode = InfoMode::Summary;
    } else if arg == "--stats" {
        parsed.mode = InfoMode::Stats {
            dimensions: None,
            enumerate: None,
            breakout: None,
        };
    } else if arg == "--schema" {
        parsed.mode = InfoMode::Schema;
    } else if arg == "--metadata" {
        parsed.mode = InfoMode::Metadata;
    } else if arg == "--all" {
        parsed.mode = InfoMode::All;
    } else if arg == "--boundary" {
        parsed.mode = InfoMode::Boundary;
    } else if arg == "--stac" {
        parsed.mode = InfoMode::Stac;
    } else if arg == "-p" || arg == "--point" {
        let Some(point_ids) = parse_point_spec(&next_value(iter, arg)?) else {
            return Err(-1);
        };
        parsed.mode = InfoMode::Points(point_ids);
    } else if let Some(value) = arg.strip_prefix("-p=") {
        let Some(point_ids) = parse_point_spec(value) else {
            return Err(-1);
        };
        parsed.mode = InfoMode::Points(point_ids);
    } else if let Some(value) = arg.strip_prefix("--point=") {
        let Some(point_ids) = parse_point_spec(value) else {
            return Err(-1);
        };
        parsed.mode = InfoMode::Points(point_ids);
    } else if arg == "--query" {
        let Some(query) = parse_query(&next_value(iter, "--query")?) else {
            return Err(-1);
        };
        parsed.mode = InfoMode::Query(query);
    } else if let Some(value) = arg.strip_prefix("--query=") {
        let Some(query) = parse_query(value) else {
            return Err(-1);
        };
        parsed.mode = InfoMode::Query(query);
    } else if arg == "--dimensions" {
        apply_stats_dimensions(parsed, Some(parse_dimension_list(&next_value(iter, arg)?)));
    } else if let Some(value) = arg.strip_prefix("--dimensions=") {
        apply_stats_dimensions(parsed, Some(parse_dimension_list(value)));
    } else if arg == "--enumerate" {
        apply_stats_enumerate(parsed, Some(parse_dimension_list(&next_value(iter, arg)?)));
    } else if let Some(value) = arg.strip_prefix("--enumerate=") {
        apply_stats_enumerate(parsed, Some(parse_dimension_list(value)));
    } else if arg == "--breakout" {
        apply_stats_breakout(parsed, Some(DimId::from_name(&next_value(iter, arg)?)));
    } else if let Some(value) = arg.strip_prefix("--breakout=") {
        apply_stats_breakout(parsed, Some(DimId::from_name(value)));
    } else if arg == "--pc_type" {
        parsed.pc_type = next_value(iter, "--pc_type")?;
    } else if let Some(value) = arg.strip_prefix("--pc_type=") {
        parsed.pc_type = value.to_string();
    } else if arg == "--pipeline-serialization" {
        parsed.serialization_file = Some(next_value(iter, "--pipeline-serialization")?);
    } else if let Some(path) = arg.strip_prefix("--pipeline-serialization=") {
        parsed.serialization_file = Some(path.to_string());
    } else if arg == "--driver" {
        parsed.driver_override = Some(next_value(iter, "--driver")?);
    } else if let Some(driver) = arg.strip_prefix("--driver=") {
        parsed.driver_override = Some(driver.to_string());
    } else if arg == "--stdin" || arg == "-s" {
        parsed.read_stdin = true;
    } else if arg == "--input" || arg == "-i" {
        if parsed.filename.replace(next_value(iter, arg)?).is_some() {
            eprintln!("PDAL: kernels.info: Expected exactly one input file.");
            return Err(1);
        }
    } else if arg.starts_with("--") {
        return Err(-1);
    } else if parsed.filename.replace(arg.to_string()).is_some() {
        eprintln!("PDAL: kernels.info: Expected exactly one input file.");
        return Err(1);
    }
    Ok(())
}

fn apply_stats_dimensions(parsed: &mut InfoArgs, dimensions: Option<Vec<DimId>>) {
    let (enumerate, breakout) = match parsed.mode.clone() {
        InfoMode::Stats {
            enumerate,
            breakout,
            ..
        } => (enumerate, breakout),
        _ => (None, None),
    };
    parsed.mode = InfoMode::Stats {
        dimensions,
        enumerate,
        breakout,
    };
}

fn apply_stats_enumerate(parsed: &mut InfoArgs, enumerate: Option<Vec<DimId>>) {
    let (dimensions, breakout) = match parsed.mode.clone() {
        InfoMode::Stats {
            dimensions,
            breakout,
            ..
        } => (dimensions, breakout),
        _ => (None, None),
    };
    parsed.mode = InfoMode::Stats {
        dimensions,
        enumerate,
        breakout,
    };
}

fn apply_stats_breakout(parsed: &mut InfoArgs, breakout: Option<DimId>) {
    let (dimensions, enumerate) = match parsed.mode.clone() {
        InfoMode::Stats {
            dimensions,
            enumerate,
            ..
        } => (dimensions, enumerate),
        _ => (None, None),
    };
    parsed.mode = InfoMode::Stats {
        dimensions,
        enumerate,
        breakout,
    };
}

fn next_value<'a>(
    iter: &mut impl Iterator<Item = &'a String>,
    option: &str,
) -> Result<String, i32> {
    match iter.next() {
        Some(value) => Ok(value.clone()),
        None => {
            eprintln!("PDAL: kernels.info: Missing value for option '{option}'.");
            Err(1)
        }
    }
}

fn parse_dimension_list(value: &str) -> Vec<DimId> {
    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(DimId::from_name)
        .collect()
}

fn parse_point_spec(value: &str) -> Option<Vec<PointId>> {
    let mut ids = Vec::new();
    for part in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some((start, end)) = part.split_once('-') {
            let start = start.parse::<PointId>().ok()?;
            let end = end.parse::<PointId>().ok()?;
            if end < start {
                return None;
            }
            ids.extend(start..=end);
        } else {
            ids.push(part.parse::<PointId>().ok()?);
        }
    }
    (!ids.is_empty()).then_some(ids)
}

fn parse_query(value: &str) -> Option<QueryRequest> {
    let (coords, count) = value.split_once('/')?;
    let parts: Vec<&str> = coords.split(',').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    Some(QueryRequest {
        x: parts[0].parse().ok()?,
        y: parts[1].parse().ok()?,
        z: parts.get(2).map(|z| z.parse()).transpose().ok()?,
        count: count.parse().ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn file_plan_infers_reader_and_defaults_summary_mode() {
        match build_info_plan(&strings(&["in.las"])) {
            InfoKernelPlan::Run(InfoRunPlan::File {
                filename,
                driver,
                mode,
                pc_type,
                serialization_file,
            }) => {
                assert_eq!(filename, "in.las");
                assert_eq!(driver, "readers.las");
                assert!(matches!(mode, InfoMode::Summary));
                assert_eq!(pc_type, "lidar");
                assert_eq!(serialization_file, None);
            }
            _ => panic!("expected file info plan"),
        }
    }

    #[test]
    fn stats_options_preserve_each_other() {
        match build_info_plan(&strings(&[
            "--stats",
            "--dimensions=X,Y",
            "--enumerate=Classification",
            "--breakout=ReturnNumber",
            "in.las",
        ])) {
            InfoKernelPlan::Run(InfoRunPlan::File { mode, .. }) => match mode {
                InfoMode::Stats {
                    dimensions,
                    enumerate,
                    breakout,
                } => {
                    assert_eq!(dimensions.unwrap().len(), 2);
                    assert_eq!(enumerate.unwrap().len(), 1);
                    assert_eq!(breakout.unwrap().name(), "ReturnNumber");
                }
                _ => panic!("expected stats mode"),
            },
            _ => panic!("expected file info plan"),
        }
    }

    #[test]
    fn point_and_query_modes_parse() {
        match build_info_plan(&strings(&["--point=1,3-4", "in.las"])) {
            InfoKernelPlan::Run(InfoRunPlan::File {
                mode: InfoMode::Points(ids),
                ..
            }) => assert_eq!(ids, vec![1, 3, 4]),
            _ => panic!("expected points mode"),
        }

        match build_info_plan(&strings(&["--query=1,2,3/5", "in.las"])) {
            InfoKernelPlan::Run(InfoRunPlan::File {
                mode: InfoMode::Query(query),
                ..
            }) => {
                assert_eq!(query.x, 1.0);
                assert_eq!(query.y, 2.0);
                assert_eq!(query.z, Some(3.0));
                assert_eq!(query.count, 5);
            }
            _ => panic!("expected query mode"),
        }
    }

    #[test]
    fn named_modes_and_driver_options_parse() {
        for (flag, expected_boundary) in [
            ("--schema", false),
            ("--metadata", false),
            ("--all", true),
            ("--boundary", true),
            ("--stac", false),
        ] {
            match build_info_plan(&strings(&[
                flag,
                "--driver=readers.ply",
                "--pc_type",
                "mesh",
                "--pipeline-serialization=pipe.json",
                "--input",
                "mesh.ply",
            ])) {
                InfoKernelPlan::Run(InfoRunPlan::File {
                    filename,
                    driver,
                    mode,
                    pc_type,
                    serialization_file,
                }) => {
                    assert_eq!(filename, "mesh.ply");
                    assert_eq!(driver, "readers.ply");
                    assert_eq!(mode.needs_boundary(), expected_boundary);
                    assert_eq!(pc_type, "mesh");
                    assert_eq!(serialization_file.as_deref(), Some("pipe.json"));
                }
                _ => panic!("expected file plan for {flag}"),
            }
        }
    }

    #[test]
    fn short_options_and_two_dimensional_query_parse() {
        match build_info_plan(&strings(&[
            "-p",
            "2",
            "--driver",
            "readers.las",
            "-i",
            "in.las",
        ])) {
            InfoKernelPlan::Run(InfoRunPlan::File {
                mode: InfoMode::Points(ids),
                driver,
                ..
            }) => {
                assert_eq!(ids, vec![2]);
                assert_eq!(driver, "readers.las");
            }
            _ => panic!("expected point mode"),
        }

        match build_info_plan(&strings(&["--query", "1,2/3", "in.las"])) {
            InfoKernelPlan::Run(InfoRunPlan::File {
                mode: InfoMode::Query(query),
                ..
            }) => {
                assert_eq!(query.x, 1.0);
                assert_eq!(query.y, 2.0);
                assert_eq!(query.z, None);
                assert_eq!(query.count, 3);
            }
            _ => panic!("expected query mode"),
        }
    }

    #[test]
    fn info_rejects_invalid_inputs_and_values() {
        for args in [
            vec!["--point=3-1", "in.las"],
            vec!["--point=bad", "in.las"],
            vec!["--query=1/2", "in.las"],
            vec!["--query=1,2,3,4/5", "in.las"],
            vec!["--query=1,2/bad", "in.las"],
            vec!["--input"],
            vec!["--driver"],
            vec!["--stdin", "in.las"],
            vec!["--input", "a.las", "--input", "b.las"],
            vec!["a.las", "b.las"],
            vec!["in.unknown"],
            vec!["--unknown", "in.las"],
        ] {
            let args = strings(&args);
            assert!(
                matches!(build_info_plan(&args), InfoKernelPlan::Return(_)),
                "{args:?}"
            );
        }
    }
}
