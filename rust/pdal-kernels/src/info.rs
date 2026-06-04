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
    All {
        dimensions: Option<Vec<DimId>>,
        enumerate: Option<Vec<DimId>>,
        breakout: Option<DimId>,
    },
    Boundary,
    Stac,
    Points(Vec<PointId>),
    Query(QueryRequest),
}

impl InfoMode {
    pub fn needs_boundary(&self) -> bool {
        matches!(self, Self::All { .. } | Self::Boundary)
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
    requests: InfoRequests,
}

impl InfoArgs {
    fn parse(args: &[String]) -> Result<Self, i32> {
        let mut parsed = Self {
            filename: None,
            driver_override: None,
            // C++ InfoKernel defaults to stats when no display function is
            // requested (`m_showStats || functions == 0` in InfoKernel.cpp), and
            // installed `pdal info <file>` emits the stats.statistic block. Match
            // that: bare `pdal info` is stats, not summary.
            mode: InfoMode::Stats {
                dimensions: None,
                enumerate: None,
                breakout: None,
            },
            pc_type: "lidar".to_string(),
            serialization_file: None,
            read_stdin: false,
            requests: InfoRequests::default(),
        };

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            parse_info_arg(arg, &mut iter, &mut parsed)?;
        }
        validate_info_args(&parsed)?;
        Ok(parsed)
    }
}

#[derive(Default)]
struct InfoRequests {
    all: bool,
    summary: bool,
    stats: bool,
    schema: bool,
    metadata: bool,
    boundary: bool,
    stac: bool,
    point: bool,
    query: bool,
    dimensions: bool,
    enumerate: bool,
}

fn parse_info_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut InfoArgs,
) -> Result<(), i32> {
    if parse_info_mode_arg(arg, parsed) {
        return Ok(());
    }
    if parse_info_point_query_arg(arg, iter, parsed)? {
        return Ok(());
    }
    if parse_info_stats_arg(arg, iter, parsed)? {
        return Ok(());
    }
    if parse_info_io_arg(arg, iter, parsed)? {
        return Ok(());
    }
    if arg.starts_with("--") {
        return Err(-1);
    }
    set_info_filename(parsed, arg.to_string())
}

fn parse_info_mode_arg(arg: &str, parsed: &mut InfoArgs) -> bool {
    if arg == "--summary" {
        parsed.requests.summary = true;
        if !parsed.requests.all {
            parsed.mode = InfoMode::Summary;
        }
    } else if arg == "--stats" {
        parsed.requests.stats = true;
        if !parsed.requests.all {
            parsed.mode = InfoMode::Stats {
                dimensions: None,
                enumerate: None,
                breakout: None,
            };
        }
    } else if arg == "--schema" {
        parsed.requests.schema = true;
        if !parsed.requests.all {
            parsed.mode = InfoMode::Schema;
        }
    } else if arg == "--metadata" {
        parsed.requests.metadata = true;
        if !parsed.requests.all {
            parsed.mode = InfoMode::Metadata;
        }
    } else if arg == "--all" {
        parsed.requests.all = true;
        let (dimensions, enumerate, breakout) = stats_selection(&parsed.mode);
        parsed.mode = InfoMode::All {
            dimensions,
            enumerate,
            breakout,
        };
    } else if arg == "--boundary" {
        parsed.requests.boundary = true;
        if !parsed.requests.all {
            parsed.mode = InfoMode::Boundary;
        }
    } else if arg == "--stac" {
        parsed.requests.stac = true;
        if !parsed.requests.all {
            parsed.mode = InfoMode::Stac;
        }
    } else {
        return false;
    }
    true
}

fn parse_info_point_query_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut InfoArgs,
) -> Result<bool, i32> {
    if arg == "-p" || arg == "--point" {
        let value = next_value(iter, arg)?;
        apply_info_point(parsed, &value)?;
    } else if let Some(value) = arg.strip_prefix("-p=") {
        apply_info_point(parsed, value)?;
    } else if let Some(value) = arg.strip_prefix("--point=") {
        apply_info_point(parsed, value)?;
    } else if arg == "--query" {
        let value = next_value(iter, "--query")?;
        apply_info_query(parsed, &value)?;
    } else if let Some(value) = arg.strip_prefix("--query=") {
        apply_info_query(parsed, value)?;
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn apply_info_point(parsed: &mut InfoArgs, value: &str) -> Result<(), i32> {
    let Some(point_ids) = parse_point_spec(value) else {
        return Err(-1);
    };
    parsed.requests.point = true;
    parsed.mode = InfoMode::Points(point_ids);
    Ok(())
}

fn apply_info_query(parsed: &mut InfoArgs, value: &str) -> Result<(), i32> {
    let Some(query) = parse_query(value) else {
        return Err(-1);
    };
    parsed.requests.query = true;
    parsed.mode = InfoMode::Query(query);
    Ok(())
}

fn parse_info_stats_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut InfoArgs,
) -> Result<bool, i32> {
    if arg == "--dimensions" {
        parsed.requests.dimensions = true;
        apply_stats_dimensions(parsed, Some(parse_dimension_list(&next_value(iter, arg)?)));
    } else if let Some(value) = arg.strip_prefix("--dimensions=") {
        parsed.requests.dimensions = true;
        apply_stats_dimensions(parsed, Some(parse_dimension_list(value)));
    } else if arg == "--enumerate" {
        parsed.requests.enumerate = true;
        apply_stats_enumerate(parsed, Some(parse_dimension_list(&next_value(iter, arg)?)));
    } else if let Some(value) = arg.strip_prefix("--enumerate=") {
        parsed.requests.enumerate = true;
        apply_stats_enumerate(parsed, Some(parse_dimension_list(value)));
    } else if arg == "--breakout" {
        apply_stats_breakout(parsed, Some(DimId::from_name(&next_value(iter, arg)?)));
    } else if let Some(value) = arg.strip_prefix("--breakout=") {
        apply_stats_breakout(parsed, Some(DimId::from_name(value)));
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn parse_info_io_arg<'a>(
    arg: &str,
    iter: &mut impl Iterator<Item = &'a String>,
    parsed: &mut InfoArgs,
) -> Result<bool, i32> {
    if arg == "--pc_type" {
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
        set_info_filename(parsed, next_value(iter, arg)?)?;
    } else {
        return Ok(false);
    }
    Ok(true)
}

fn set_info_filename(parsed: &mut InfoArgs, filename: String) -> Result<(), i32> {
    if parsed.filename.replace(filename).is_some() {
        eprintln!("PDAL: kernels.info: Expected exactly one input file.");
        return Err(1);
    }
    Ok(())
}

fn validate_info_args(parsed: &InfoArgs) -> Result<(), i32> {
    let requests = &parsed.requests;
    let stac_requested = requests.stac || requests.all;
    if stac_requested && requests.query {
        eprintln!("PDAL: kernels.info: 'query' option incompatible with 'stac' option.");
        return Err(1);
    }
    if stac_requested && requests.point {
        eprintln!("PDAL: kernels.info: 'point' option incompatible with 'stac' option.");
        return Err(1);
    }
    if requests.point && requests.query {
        eprintln!("PDAL: kernels.info: 'point' option incompatible with 'query' option.");
        return Err(1);
    }

    let other_summary_function = requests.all
        || requests.stats
        || requests.schema
        || requests.metadata
        || requests.boundary
        || requests.stac
        || requests.point
        || requests.query
        || parsed.serialization_file.is_some();
    if requests.summary && other_summary_function {
        eprintln!(
            "PDAL: kernels.info: 'summary' option incompatible with other specified options."
        );
        return Err(1);
    }

    let stats_active =
        requests.stats || requests.all || !other_summary_function && !requests.summary;
    if !stats_active && requests.enumerate {
        eprintln!("PDAL: kernels.info: 'enumerate' option requires 'stats' option.");
        return Err(1);
    }
    if !stats_active && requests.dimensions {
        eprintln!("PDAL: kernels.info: 'dimensions' option requires 'stats' option.");
        return Err(1);
    }

    Ok(())
}

fn apply_stats_dimensions(parsed: &mut InfoArgs, dimensions: Option<Vec<DimId>>) {
    let (_, enumerate, breakout) = stats_selection(&parsed.mode);
    apply_stats_selection(parsed, dimensions, enumerate, breakout);
}

fn apply_stats_enumerate(parsed: &mut InfoArgs, enumerate: Option<Vec<DimId>>) {
    let (dimensions, _, breakout) = stats_selection(&parsed.mode);
    apply_stats_selection(parsed, dimensions, enumerate, breakout);
}

fn apply_stats_breakout(parsed: &mut InfoArgs, breakout: Option<DimId>) {
    let (dimensions, enumerate, _) = stats_selection(&parsed.mode);
    apply_stats_selection(parsed, dimensions, enumerate, breakout);
}

fn stats_selection(mode: &InfoMode) -> (Option<Vec<DimId>>, Option<Vec<DimId>>, Option<DimId>) {
    match mode.clone() {
        InfoMode::Stats {
            dimensions,
            enumerate,
            breakout,
        }
        | InfoMode::All {
            dimensions,
            enumerate,
            breakout,
        } => (dimensions, enumerate, breakout),
        _ => (None, None, None),
    }
}

fn apply_stats_selection(
    parsed: &mut InfoArgs,
    dimensions: Option<Vec<DimId>>,
    enumerate: Option<Vec<DimId>>,
    breakout: Option<DimId>,
) {
    if parsed.requests.all {
        parsed.mode = InfoMode::All {
            dimensions,
            enumerate,
            breakout,
        };
    } else {
        parsed.mode = InfoMode::Stats {
            dimensions,
            enumerate,
            breakout,
        };
    }
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
    fn file_plan_infers_reader_and_defaults_stats_mode() {
        // Bare `pdal info <file>` defaults to stats, matching C++ InfoKernel
        // (`functions == 0` -> showStats) and installed `pdal info`.
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
                assert!(matches!(mode, InfoMode::Stats { .. }));
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
    fn all_mode_survives_later_display_flags_and_preserves_stats_options() {
        for args in [
            vec!["--all", "--schema", "--dimensions=X", "in.las"],
            vec![
                "--dimensions=X",
                "--enumerate=Classification",
                "--all",
                "in.las",
            ],
        ] {
            match build_info_plan(&strings(&args)) {
                InfoKernelPlan::Run(InfoRunPlan::File {
                    mode:
                        InfoMode::All {
                            dimensions,
                            enumerate,
                            ..
                        },
                    ..
                }) => {
                    assert_eq!(dimensions.unwrap(), vec![DimId::X]);
                    if args.iter().any(|arg| arg.starts_with("--enumerate")) {
                        assert_eq!(enumerate.unwrap(), vec![DimId::Classification]);
                    }
                }
                _ => panic!("expected all mode for {args:?}"),
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
            vec!["--summary", "--metadata", "in.las"],
            vec!["--summary", "--pipeline-serialization=pipe.json", "in.las"],
            vec!["--stac", "--query=1,2/3", "in.las"],
            vec!["--stac", "--point=1", "in.las"],
            vec!["--all", "--point=1", "in.las"],
            vec!["--point=1", "--query=1,2/3", "in.las"],
            vec!["--schema", "--dimensions=X", "in.las"],
            vec!["--metadata", "--enumerate=Classification", "in.las"],
        ] {
            let args = strings(&args);
            assert!(
                matches!(build_info_plan(&args), InfoKernelPlan::Return(_)),
                "{args:?}"
            );
        }
    }
}
