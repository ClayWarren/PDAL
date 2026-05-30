use crate::pipeline_abi::{pipeline_result_to_json_for_kernel, PipelineHandle};
use crate::registry::pipeline_from_json;
use pdal_core::bounds::{parse_bounds2d, Bounds2D};
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::expr::ConditionalExpression;
use pdal_core::gdal::{LayerHandle, Vector};
use pdal_core::point::DimId;
use pdal_filters::hexer::{HexGrid, SQRT_3};
use pdal_native::geometry::Geometry;
use std::ffi::CStr;
use std::io::Read;
use std::os::raw::c_char;
use std::path::Path;

/// Defaults match the C++ TIndexKernel option defaults; `fast_boundary`
/// short-circuits to bbox output, and `boundary_expr` is rejected because
/// we don't have point-level expression filtering wired up here yet.
struct BoundaryOptions {
    density: i32,
    edge_length: f64,
    sample_size: u32,
    smooth: bool,
    fast_boundary: bool,
    where_expr: Option<String>,
}

impl Default for BoundaryOptions {
    fn default() -> Self {
        Self {
            density: 15,
            edge_length: 0.0,
            sample_size: 5000,
            smooth: true,
            fast_boundary: false,
            where_expr: None,
        }
    }
}

impl BoundaryOptions {
    fn exact(&self) -> bool {
        !self.fast_boundary
    }
}

struct CreateArgs {
    tindex_file: String,
    files: Vec<String>,
    driver_name: String,
    target_srs: String,
    assign_srs: String,
    override_source_srs: bool,
    path_prefix: Option<String>,
    write_absolute_path: bool,
    layer_name: String,
    location_field: String,
    lco_description: Option<String>,
    rich_boundary_options: bool,
    boundary: BoundaryOptions,
    stdin_requested: bool,
    input_methods: u8,
    filelists: Vec<String>,
    skip_different_srs: bool,
    unsupported_input: bool,
}

struct Entry {
    location: String,
    wkt: String,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
    /// Exact boundary WKT (MULTIPOLYGON) when rich-boundary is requested;
    /// `None` falls back to the axis-aligned bbox polygon.
    boundary_wkt: Option<String>,
}

struct MergeClip {
    bounds: Bounds2D,
    stage_key: &'static str,
    stage_value: String,
}

struct MergeArgs {
    tindex_file: String,
    output_file: String,
    location_field: String,
    target_srs: String,
    clip: Option<MergeClip>,
}

pub(super) unsafe fn run_tindex_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        if args.is_empty() {
            eprintln!("PDAL: kernels.tindex: Missing subcommand.");
            return 1;
        }
        print_usage();
        return 0;
    }

    match args[0].as_str() {
        "create" => run_create(&args[1..]),
        "merge" => run_merge(&args[1..]),
        _ => {
            eprintln!("PDAL: kernels.tindex: Expected 'create' or 'merge' subcommand.");
            1
        }
    }
}

fn print_usage() {
    println!("Usage:");
    println!("  pdal tindex create --tindex <output> <files...> [-f GeoJSON]");
    println!("  pdal tindex create --tindex <output> --filelist <path> [-f GeoJSON]");
    println!("  pdal tindex create --tindex <output> --glob <pattern> [-f GeoJSON]");
    println!("  pdal tindex merge --tindex <index> --filespec <output>");
}

fn run_create(args: &[String]) -> i32 {
    let args = match parse_create_args(args) {
        Ok(args) => args,
        Err(ParseResult::Error(message)) => {
            if message == INVALID_FILTER_STAGE_MESSAGE {
                println!("PDAL: kernels.tindex: {message}");
            }
            eprintln!("PDAL: kernels.tindex: {message}");
            return 1;
        }
        Err(ParseResult::Unsupported) => return -1,
    };

    if args.tindex_file == "/vsistdout/" {
        let (_, entries) = match collect_entries(&args) {
            Ok(entries) => entries,
            Err(()) => return 1,
        };
        print_geojson_tindex(&args, entries);
        return 0;
    }

    pdal_core::gdal::register_drivers();
    let dataset = match Vector::create(&args.tindex_file, &args.driver_name) {
        Ok(dataset) => dataset,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: Error creating tindex dataset: {err}");
            return 1;
        }
    };

    let (first_srs, entries) = match collect_entries(&args) {
        Ok(entries) => entries,
        Err(()) => return 1,
    };
    if args.driver_name.eq_ignore_ascii_case("ESRI Shapefile") && first_srs.len() > 254 {
        println!(
            "PDAL: kernels.tindex: ESRI Shapefile field 'srs' supports a maximum of 254 characters."
        );
    }
    let layer = match dataset.open_or_create_layer(&args.layer_name, &args.target_srs) {
        Ok(layer) => layer,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: Error creating layer: {err}");
            return 1;
        }
    };
    if create_fields(layer, &args.location_field).is_err() {
        return 1;
    }
    add_features(layer, &args.location_field, entries)
}

enum ParseResult {
    Error(String),
    Unsupported,
}

fn parse_create_args(args: &[String]) -> Result<CreateArgs, ParseResult> {
    let mut parsed = CreateArgs {
        tindex_file: String::new(),
        files: Vec::new(),
        driver_name: "ESRI Shapefile".to_string(),
        target_srs: "EPSG:4326".to_string(),
        assign_srs: "EPSG:4326".to_string(),
        override_source_srs: false,
        path_prefix: None,
        write_absolute_path: false,
        layer_name: "pdal".to_string(),
        location_field: "location".to_string(),
        lco_description: None,
        rich_boundary_options: false,
        boundary: BoundaryOptions::default(),
        stdin_requested: false,
        input_methods: 0,
        filelists: Vec::new(),
        skip_different_srs: false,
        unsupported_input: false,
    };

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => parsed.tindex_file = next_value(&mut iter, "--tindex")?.clone(),
            "--filelist" => {
                parsed.input_methods += 1;
                let path = next_value(&mut iter, "--filelist")?;
                parsed.filelists.push(path.clone());
            }
            "--glob" => {
                parsed.input_methods += 1;
                let pattern = next_value(&mut iter, "--glob")?;
                parsed.files.extend(read_glob(pattern)?);
            }
            "--path_prefix" => parsed.path_prefix = Some(next_value(&mut iter, arg)?.clone()),
            "--write_absolute_path" => parsed.write_absolute_path = true,
            "--lyr_name" => parsed.layer_name = next_value(&mut iter, arg)?.clone(),
            "--tindex_name" => parsed.location_field = next_value(&mut iter, arg)?.clone(),
            "-f" | "--ogrdriver" => parsed.driver_name = next_value(&mut iter, arg)?.clone(),
            "--t_srs" => parsed.target_srs = next_value(&mut iter, arg)?.clone(),
            "--a_srs" => {
                parsed.assign_srs = next_value(&mut iter, arg)?.clone();
                parsed.override_source_srs = true;
            }
            "--lco" => apply_layer_creation_option(&mut parsed, next_value(&mut iter, arg)?)?,
            "--log" => {
                let _ = next_value(&mut iter, "--log")?;
            }
            "--stdin" | "-s" => {
                parsed.input_methods += 1;
                parsed.stdin_requested = true;
            }
            "--threshold" => {
                parsed.rich_boundary_options = true;
                let value = next_value(&mut iter, arg)?;
                parsed.boundary.density = parse_int(value, arg)?;
            }
            "--resolution" | "--edge_length" => {
                parsed.rich_boundary_options = true;
                let value = next_value(&mut iter, arg)?;
                parsed.boundary.edge_length = parse_float(value, arg)?;
            }
            "--sample_size" => {
                parsed.rich_boundary_options = true;
                let value = next_value(&mut iter, arg)?;
                parsed.boundary.sample_size = parse_uint(value, arg)?;
            }
            "--simplify" => {
                parsed.rich_boundary_options = true;
                let value = next_value(&mut iter, arg)?;
                parsed.boundary.smooth = parse_bool(value, arg)?;
            }
            "--fast_boundary" => {
                parsed.rich_boundary_options = true;
                let value = next_value(&mut iter, arg)?;
                parsed.boundary.fast_boundary = parse_bool(value, arg)?;
            }
            "--skip_different_srs" => {
                let value = next_value(&mut iter, arg)?;
                parsed.skip_different_srs = parse_bool(value, arg)?;
            }
            "--where" => {
                parsed.rich_boundary_options = true;
                parsed.boundary.where_expr = Some(next_value(&mut iter, arg)?.clone());
            }
            _ if let Some(value) = arg.strip_prefix("--filespec=") => {
                parsed.input_methods += 1;
                if is_glob_pattern(value) {
                    parsed.files.extend(read_glob(value)?);
                } else {
                    parsed.files.push(value.to_string());
                }
            }
            _ if let Some(pattern) = arg.strip_prefix("--glob=") => {
                parsed.input_methods += 1;
                parsed.files.extend(read_glob(pattern)?);
            }
            _ if let Some(path) = arg.strip_prefix("--filelist=") => {
                parsed.input_methods += 1;
                parsed.filelists.push(path.to_string());
            }
            _ if let Some(value) = arg.strip_prefix("--write_absolute_path=") => {
                parsed.write_absolute_path = matches!(
                    value.to_ascii_lowercase().as_str(),
                    "true" | "1" | "yes" | "on"
                );
            }
            _ if let Some(value) = arg.strip_prefix("--path_prefix=") => {
                parsed.path_prefix = Some(value.to_string());
            }
            _ if let Some(value) = arg.strip_prefix("--t_srs=") => {
                parsed.target_srs = value.to_string();
            }
            _ if let Some(value) = arg.strip_prefix("--a_srs=") => {
                parsed.assign_srs = value.to_string();
                parsed.override_source_srs = true;
            }
            _ if arg.starts_with("--log=") => {}
            _ if let Some(value) = arg.strip_prefix("--lco=") => {
                apply_layer_creation_option(&mut parsed, value)?;
            }
            _ if let Some(value) = arg.strip_prefix("--threshold=") => {
                parsed.rich_boundary_options = true;
                parsed.boundary.density = parse_int(value, "--threshold")?;
            }
            _ if let Some(value) = arg
                .strip_prefix("--resolution=")
                .or_else(|| arg.strip_prefix("--edge_length=")) =>
            {
                parsed.rich_boundary_options = true;
                parsed.boundary.edge_length = parse_float(value, "--resolution")?;
            }
            _ if let Some(value) = arg.strip_prefix("--sample_size=") => {
                parsed.rich_boundary_options = true;
                parsed.boundary.sample_size = parse_uint(value, "--sample_size")?;
            }
            _ if let Some(value) = arg.strip_prefix("--simplify=") => {
                parsed.rich_boundary_options = true;
                parsed.boundary.smooth = parse_bool(value, "--simplify")?;
            }
            _ if let Some(value) = arg.strip_prefix("--fast_boundary=") => {
                parsed.rich_boundary_options = true;
                parsed.boundary.fast_boundary = parse_bool(value, "--fast_boundary")?;
            }
            _ if let Some(value) = arg.strip_prefix("--skip_different_srs=") => {
                parsed.skip_different_srs = parse_bool(value, "--skip_different_srs")?;
            }
            _ if let Some(value) = arg.strip_prefix("--where=") => {
                parsed.rich_boundary_options = true;
                parsed.boundary.where_expr = Some(value.to_string());
            }
            _ if arg.starts_with("--filters.hexbin.smooth") => {
                return Err(ParseResult::Error(INVALID_FILTER_STAGE_MESSAGE.to_string()));
            }
            _ if arg.starts_with("--filters.") => return Err(ParseResult::Unsupported),
            _ if arg.starts_with('-') => return Err(ParseResult::Unsupported),
            _ if parsed.tindex_file.is_empty() => parsed.tindex_file = arg.clone(),
            _ if is_glob_pattern(arg) => {
                parsed.input_methods += 1;
                parsed.files.extend(read_glob(arg)?);
            }
            _ => {
                parsed.input_methods += 1;
                parsed.files.push(arg.clone());
            }
        }
    }
    if parsed.input_methods > 1 {
        return Err(ParseResult::Error(
            "Can't specify more than one source of tindex input files.".to_string(),
        ));
    }
    if parsed.path_prefix.is_some() && parsed.write_absolute_path {
        return Err(ParseResult::Error(
            "Can't specify both path_prefix and write_absolute_path.".to_string(),
        ));
    }
    if parsed.unsupported_input {
        return Err(ParseResult::Unsupported);
    }
    for path in &parsed.filelists {
        parsed.files.extend(read_filelist(path)?);
    }
    if parsed.stdin_requested {
        parsed.files.extend(read_stdin_files()?);
    }
    if parsed.tindex_file.is_empty() {
        return Err(ParseResult::Error(
            "tindex create requires --tindex <output>".to_string(),
        ));
    }
    if parsed.files.is_empty() {
        return Err(ParseResult::Error(
            "tindex create needs at least one input file".to_string(),
        ));
    }
    Ok(parsed)
}

const INVALID_FILTER_STAGE_MESSAGE: &str = "Argument references invalid/unused stage";

fn apply_layer_creation_option(args: &mut CreateArgs, value: &str) -> Result<(), ParseResult> {
    let Some((name, option)) = value.split_once('=') else {
        return Err(ParseResult::Unsupported);
    };
    if name.eq_ignore_ascii_case("DESCRIPTION") {
        args.lco_description = Some(option.to_string());
        Ok(())
    } else {
        Err(ParseResult::Unsupported)
    }
}

fn next_value<'a, I>(iter: &mut I, arg: &str) -> Result<&'a String, ParseResult>
where
    I: Iterator<Item = &'a String>,
{
    iter.next()
        .ok_or_else(|| ParseResult::Error(format!("{arg} requires a value")))
}

fn parse_int(value: &str, arg: &str) -> Result<i32, ParseResult> {
    value
        .parse::<i32>()
        .map_err(|_| ParseResult::Error(format!("{arg} requires an integer value, got '{value}'")))
}

fn parse_uint(value: &str, arg: &str) -> Result<u32, ParseResult> {
    value.parse::<u32>().map_err(|_| {
        ParseResult::Error(format!(
            "{arg} requires a non-negative integer value, got '{value}'"
        ))
    })
}

fn parse_float(value: &str, arg: &str) -> Result<f64, ParseResult> {
    value
        .parse::<f64>()
        .map_err(|_| ParseResult::Error(format!("{arg} requires a numeric value, got '{value}'")))
}

fn parse_bool(value: &str, arg: &str) -> Result<bool, ParseResult> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(ParseResult::Error(format!(
            "{arg} requires a boolean value, got '{value}'"
        ))),
    }
}

fn read_glob(pattern: &str) -> Result<Vec<String>, ParseResult> {
    let entries = glob::glob(pattern).map_err(|err| ParseResult::Error(format!("{err}")))?;
    let mut files = Vec::new();
    for entry in entries {
        match entry {
            Ok(path) => files.push(path.to_string_lossy().into_owned()),
            Err(_) => return Err(ParseResult::Unsupported),
        }
    }
    if files.is_empty() {
        return Err(ParseResult::Error(format!(
            "glob pattern '{pattern}' did not match any files"
        )));
    }
    Ok(files)
}

fn is_glob_pattern(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}

fn read_stdin_files() -> Result<Vec<String>, ParseResult> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| ParseResult::Error(format!("unable to read stdin file list: {err}")))?;
    let files = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Err(ParseResult::Error(
            "stdin contained no tindex input files".to_string(),
        ));
    }
    Ok(files)
}

fn read_filelist(path: &str) -> Result<Vec<String>, ParseResult> {
    let input = std::fs::read_to_string(path)
        .map_err(|err| ParseResult::Error(format!("unable to read filelist '{path}': {err}")))?;
    let files = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Err(ParseResult::Error(format!(
            "filelist '{path}' contained no tindex input files"
        )));
    }
    Ok(files)
}

fn collect_entries(args: &CreateArgs) -> Result<(String, Vec<Entry>), ()> {
    let mut first_srs = String::new();
    let mut entries = Vec::new();
    for file in &args.files {
        let mut entry = create_entry(file, args)?;
        if entry.wkt.is_empty() || args.override_source_srs {
            entry.wkt.clone_from(&args.assign_srs);
        }
        if first_srs.is_empty() && !entry.wkt.is_empty() {
            first_srs.clone_from(&entry.wkt);
        } else if !first_srs.is_empty() && !entry.wkt.is_empty() && entry.wkt != first_srs {
            print!(
                "PDAL: kernels.tindex: SRS of file '{}' does not match the SRS of other files in the tileindex",
                entry.location
            );
            if args.skip_different_srs {
                println!(". Skipping this file");
                continue;
            }
            println!();
        }
        if let Some(prefix) = &args.path_prefix {
            entry.location = format!("{prefix}{}", entry.location);
        }
        entries.push(entry);
    }
    Ok((first_srs, entries))
}

fn create_entry(file: &str, args: &CreateArgs) -> Result<Entry, ()> {
    let summary = summary_for_file(file)?;
    let (minx, miny, maxx, maxy) = bounds_from_summary(file, &summary)?;
    let mut wkt = summary["metadata"]["pipeline"]["stage_0"]["srs"]["wkt"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if wkt.is_empty() {
        wkt = srs_for_file(file)?;
    }
    let location = tindex_location(file, args.write_absolute_path)?;

    let boundary_wkt = if args.rich_boundary_options && args.boundary.exact() {
        compute_exact_boundary(file, &args.boundary)?
    } else {
        None
    };

    Ok(Entry {
        location,
        wkt,
        minx,
        miny,
        maxx,
        maxy,
        boundary_wkt,
    })
}

/// Build an exact MULTIPOLYGON boundary for `file` using the Rust hexer port,
/// then run it through GEOS topology-preserving simplification when smoothing
/// is enabled (matching `pdal::Polygon::simplify`). Returns `Ok(None)` if the
/// file has too few points to populate at least one dense hex cell.
fn compute_exact_boundary(file: &str, opts: &BoundaryOptions) -> Result<Option<String>, ()> {
    let Some(driver) = infer_reader_driver(file) else {
        eprintln!("PDAL: kernels.tindex: Unable to infer reader driver for '{file}'.");
        return Err(());
    };
    let mut pipeline = match pipeline_from_json(
        &serde_json::json!([{ "type": driver, "filename": file }]).to_string(),
    ) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            return Err(());
        }
    };
    let views = match pipeline.execute(Vec::new()) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            return Err(());
        }
    };

    let mut grid = if opts.edge_length > 0.0 {
        HexGrid::with_height(opts.edge_length * SQRT_3, opts.density)
    } else {
        // Auto-edge-length sampling isn't ported yet; fall back to a small
        // edge derived from the leading-point spacing so we still produce a
        // boundary instead of throwing. The result will differ from
        // installed PDAL when edge_length is left unset.
        match estimate_edge_length(&views, opts.sample_size, opts.density) {
            Some(h) => HexGrid::with_height(h, opts.density),
            None => return Ok(None),
        }
    };
    let mut where_expr = match &opts.where_expr {
        Some(source) => match ConditionalExpression::parse(source) {
            Ok(expr) => Some(expr),
            Err(err) => {
                eprintln!("PDAL: kernels.tindex: Invalid where expression '{source}': {err}");
                return Err(());
            }
        },
        None => None,
    };

    for view in &views {
        if let Some(expr) = where_expr.as_mut() {
            if let Err(err) = expr.prepare(view.layout().as_ref()) {
                eprintln!("PDAL: kernels.tindex: Invalid where expression: {err}");
                return Err(());
            }
        }
        for idx in 0..view.len() {
            if let Some(expr) = &where_expr {
                if !expr.eval(view, idx) {
                    continue;
                }
            }
            let x = view.get_f64(idx, &DimId::X);
            let y = view.get_f64(idx, &DimId::Y);
            grid.add_xy(x, y);
        }
    }
    if grid.find_shapes().is_err() {
        return Ok(None);
    }
    grid.find_parent_paths();
    let wkt = grid.to_wkt(8);
    if !opts.smooth {
        return Ok(Some(wkt));
    }
    let tolerance = 1.1 * grid.height() / 2.0;
    match Geometry::from_wkt(&wkt)
        .and_then(|g| g.simplify(tolerance, true))
        .and_then(|g| g.to_wkt())
    {
        Ok(simplified) => Ok(Some(ensure_multipolygon(&simplified))),
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: GEOS simplify failed for '{file}': {err}");
            Ok(Some(wkt))
        }
    }
}

fn estimate_edge_length(
    views: &[pdal_core::point::PointView],
    sample_size: u32,
    density: i32,
) -> Option<f64> {
    let mut last: Option<(f64, f64)> = None;
    let mut total = 0.0_f64;
    let mut count = 0u64;
    let limit = sample_size.max(1) as u64;
    'outer: for view in views {
        for idx in 0..view.len() {
            let x = view.get_f64(idx, &DimId::X);
            let y = view.get_f64(idx, &DimId::Y);
            if let Some((px, py)) = last {
                let dx = x - px;
                let dy = y - py;
                total += (dx * dx + dy * dy).sqrt();
                count += 1;
                if count >= limit {
                    break 'outer;
                }
            }
            last = Some((x, y));
        }
    }
    if count == 0 {
        return None;
    }
    Some((density as f64 * total) / (count + 1) as f64)
}

fn ensure_multipolygon(wkt: &str) -> String {
    let trimmed = wkt.trim_start();
    if trimmed.starts_with('P') || trimmed.starts_with('p') {
        // Wrap a single POLYGON in a MULTIPOLYGON so output shape stays
        // consistent across smoothing branches.
        let after_keyword = trimmed
            .strip_prefix("POLYGON ")
            .or_else(|| trimmed.strip_prefix("POLYGON"))
            .unwrap_or(trimmed);
        format!("MULTIPOLYGON ({})", after_keyword.trim_start())
    } else {
        wkt.to_string()
    }
}

fn summary_for_file(file: &str) -> Result<serde_json::Value, ()> {
    let Some(driver) = infer_reader_driver(file) else {
        eprintln!("PDAL: kernels.tindex: Unable to infer reader driver for '{file}'.");
        return Err(());
    };
    let mut pipeline = match pipeline_from_json(
        &serde_json::json!([{ "type": driver, "filename": file }]).to_string(),
    ) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            return Err(());
        }
    };
    match pipeline.execute_with_result(Vec::new()) {
        Ok(result) => {
            let handle = PipelineHandle { pipeline };
            serde_json::from_str(&pipeline_result_to_json_for_kernel(result, &handle)).map_err(
                |err| {
                    eprintln!(
                        "PDAL: kernels.tindex: Unable to parse pipeline summary for '{file}': {err}"
                    );
                },
            )
        }
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            Err(())
        }
    }
}

fn srs_for_file(file: &str) -> Result<String, ()> {
    let Some(driver) = infer_reader_driver(file) else {
        eprintln!("PDAL: kernels.tindex: Unable to infer reader driver for '{file}'.");
        return Err(());
    };
    let mut pipeline = match pipeline_from_json(
        &serde_json::json!([{ "type": driver, "filename": file }]).to_string(),
    ) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            return Err(());
        }
    };
    match pipeline.execute(Vec::new()) {
        Ok(views) => Ok(views
            .iter()
            .map(|view| view.spatial_reference().wkt())
            .find(|wkt| !wkt.is_empty())
            .unwrap_or("")
            .to_string()),
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            Err(())
        }
    }
}

fn bounds_from_summary(
    file: &str,
    summary: &serde_json::Value,
) -> Result<(f64, f64, f64, f64), ()> {
    let Some(bounds) = summary.get("bounds_2d") else {
        eprintln!("PDAL: kernels.tindex: '{file}' produced no 2D bounds.");
        return Err(());
    };
    let Some(minx) = bounds["minx"].as_f64() else {
        return Err(());
    };
    let Some(maxx) = bounds["maxx"].as_f64() else {
        return Err(());
    };
    let Some(miny) = bounds["miny"].as_f64() else {
        return Err(());
    };
    let Some(maxy) = bounds["maxy"].as_f64() else {
        return Err(());
    };
    Ok((minx, miny, maxx, maxy))
}

fn tindex_location(file: &str, write_absolute_path: bool) -> Result<String, ()> {
    if !write_absolute_path {
        return Ok(file.to_string());
    }
    Path::new(file)
        .canonicalize()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|err| {
            eprintln!("PDAL: kernels.tindex: Unable to resolve absolute path for '{file}': {err}");
        })
}

fn create_fields(layer: LayerHandle, location_field: &str) -> Result<(), ()> {
    unsafe {
        for result in [
            Vector::create_string_field(layer, location_field),
            Vector::create_string_field(layer, "srs"),
            Vector::create_datetime_field(layer, "created"),
            Vector::create_datetime_field(layer, "modified"),
        ] {
            if let Err(err) = result {
                eprintln!("PDAL: kernels.tindex: Error creating tindex field: {err}");
                return Err(());
            }
        }
    }
    Ok(())
}

fn add_features(layer: LayerHandle, location_field: &str, entries: Vec<Entry>) -> i32 {
    for entry in entries {
        let poly_wkt = entry.boundary_wkt.clone().unwrap_or_else(|| {
            format!(
                "POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))",
                entry.minx,
                entry.miny,
                entry.maxx,
                entry.miny,
                entry.maxx,
                entry.maxy,
                entry.minx,
                entry.maxy,
                entry.minx,
                entry.miny
            )
        });
        let fields = vec![
            (location_field, entry.location.as_str()),
            ("srs", entry.wkt.as_str()),
        ];
        unsafe {
            if let Err(err) = Vector::add_feature(layer, &poly_wkt, &fields) {
                eprintln!(
                    "PDAL: kernels.tindex: Error adding feature for {}: {err}",
                    entry.location
                );
                return 1;
            }
        }
        println!("Indexed file {}", entry.location);
    }
    0
}

fn print_geojson_tindex(args: &CreateArgs, entries: Vec<Entry>) {
    let features = entries
        .into_iter()
        .map(|entry| {
            let geometry = entry
                .boundary_wkt
                .as_deref()
                .and_then(|wkt| {
                    Geometry::from_wkt(wkt)
                        .and_then(|g| g.to_gdal_geojson(8))
                        .ok()
                })
                .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "type": "Polygon",
                        "coordinates": [[
                            [entry.minx, entry.miny],
                            [entry.maxx, entry.miny],
                            [entry.maxx, entry.maxy],
                            [entry.minx, entry.maxy],
                            [entry.minx, entry.miny],
                        ]],
                    })
                });
            serde_json::json!({
                "type": "Feature",
                "properties": {
                    args.location_field.clone(): entry.location,
                    "srs": entry.wkt,
                },
                "geometry": geometry,
            })
        })
        .collect::<Vec<_>>();
    let mut output = serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
    });
    if let Some(description) = &args.lco_description {
        output["description"] = serde_json::Value::String(description.clone());
    }
    println!("{output}");
}

fn run_merge(args: &[String]) -> i32 {
    let args = match parse_merge_args(args) {
        Ok(parsed) => parsed,
        Err(ParseResult::Error(message)) => {
            eprintln!("PDAL: kernels.tindex: {message}");
            return 1;
        }
        Err(ParseResult::Unsupported) => return -1,
    };

    let index_json = match std::fs::read_to_string(&args.tindex_file) {
        Ok(json) => json,
        Err(err) => {
            eprintln!(
                "PDAL: kernels.tindex: Unable to read tindex '{}': {err}",
                args.tindex_file
            );
            return 1;
        }
    };
    let index: serde_json::Value = match serde_json::from_str(&index_json) {
        Ok(index) => index,
        Err(err) => {
            eprintln!(
                "PDAL: kernels.tindex: Unable to parse GeoJSON tindex '{}': {err}",
                args.tindex_file
            );
            return 1;
        }
    };
    let Some(features) = index["features"].as_array() else {
        eprintln!("PDAL: kernels.tindex: tindex merge expects a GeoJSON FeatureCollection.");
        return 1;
    };
    if features.is_empty() {
        eprintln!("PDAL: kernels.tindex: tindex contains no features.");
        return 1;
    }

    let mut stages = Vec::new();
    let mut tags = Vec::new();
    let mut file_count = 0;
    for (index, feature) in features.iter().enumerate() {
        if let Some(clip) = &args.clip {
            let Some(feature_bounds) = feature_bounds_2d(feature) else {
                eprintln!("PDAL: kernels.tindex: Feature has invalid geometry.");
                return 1;
            };
            if !feature_bounds.overlaps(&clip.bounds) {
                continue;
            }
        }
        let Some(location) = feature["properties"][&args.location_field].as_str() else {
            eprintln!(
                "PDAL: kernels.tindex: Feature is missing '{}'.",
                args.location_field
            );
            return 1;
        };
        let Some(reader) = infer_reader_driver(location) else {
            eprintln!("PDAL: kernels.tindex: Unable to infer reader driver for '{location}'.");
            return 1;
        };
        let tag = format!("tindex_input_{index}");
        stages.push(serde_json::json!({
            "type": reader,
            "filename": location,
            "tag": tag.clone(),
        }));
        let mut input_tag = tag;
        let feature_srs = feature["properties"]["srs"].as_str().unwrap_or("");
        if !feature_srs.is_empty() && feature_srs != args.target_srs {
            let reprojection_tag = format!("tindex_reprojection_{index}");
            stages.push(serde_json::json!({
                "type": "filters.reprojection",
                "in_srs": feature_srs,
                "out_srs": &args.target_srs,
                "inputs": [input_tag],
                "tag": reprojection_tag,
            }));
            input_tag = reprojection_tag;
        }
        if let Some(clip) = &args.clip {
            let crop_tag = format!("tindex_crop_{index}");
            stages.push(serde_json::json!({
                "type": "filters.crop",
                (clip.stage_key): clip.stage_value,
                "inputs": [input_tag],
                "tag": crop_tag,
            }));
            tags.push(crop_tag);
        } else {
            tags.push(input_tag);
        }
        file_count += 1;
    }
    println!("Merge filecount: {file_count}");
    if stages.is_empty() {
        eprintln!("PDAL: kernels.tindex: No indexed files matched merge criteria.");
        return 1;
    }
    if stages.len() > 1 {
        stages.push(serde_json::json!({
            "type": "filters.merge",
            "inputs": tags,
        }));
    }
    let Some(writer) = infer_writer_driver(&args.output_file) else {
        eprintln!(
            "PDAL: kernels.tindex: Unable to infer writer driver for '{}'.",
            args.output_file
        );
        return 1;
    };
    stages.push(serde_json::json!({ "type": writer, "filename": args.output_file }));
    execute_pipeline(serde_json::Value::Array(stages))
}

fn parse_merge_args(args: &[String]) -> Result<MergeArgs, ParseResult> {
    let mut tindex_file = None;
    let mut output_file = None;
    let mut location_field = "location".to_string();
    let mut target_srs = "EPSG:4326".to_string();
    let mut clip = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => tindex_file = Some(next_value(&mut iter, "--tindex")?.clone()),
            "--filespec" => output_file = Some(next_value(&mut iter, "--filespec")?.clone()),
            "--tindex_name" => location_field = next_value(&mut iter, "--tindex_name")?.clone(),
            "--bounds" => {
                let value = next_value(&mut iter, "--bounds")?;
                clip = Some(parse_merge_bounds(value)?);
            }
            "--polygon" => {
                let value = next_value(&mut iter, "--polygon")?;
                clip = Some(parse_merge_polygon(value)?);
            }
            "--t_srs" => {
                target_srs = next_value(&mut iter, "--t_srs")?.clone();
            }
            "--log" => {
                let _ = next_value(&mut iter, "--log")?;
            }
            "--lyr_name" | "--ogrdriver" | "-f" => {
                let _ = next_value(&mut iter, arg)?;
            }
            _ if let Some(value) = arg.strip_prefix("--bounds=") => {
                clip = Some(parse_merge_bounds(value)?);
            }
            _ if let Some(value) = arg.strip_prefix("--polygon=") => {
                clip = Some(parse_merge_polygon(value)?);
            }
            _ if let Some(value) = arg.strip_prefix("--t_srs=") => {
                target_srs = value.to_string();
            }
            _ if arg.starts_with("--log=") => {}
            _ if arg.starts_with("--") => {
                return Err(ParseResult::Unsupported);
            }
            _ if tindex_file.is_none() => tindex_file = Some(arg.clone()),
            _ if output_file.is_none() => output_file = Some(arg.clone()),
            _ => return Err(ParseResult::Error("too many merge arguments".to_string())),
        }
    }

    let Some(tindex_file) = tindex_file else {
        return Err(ParseResult::Error(
            "merge requires --tindex <index>".to_string(),
        ));
    };
    let Some(output_file) = output_file else {
        return Err(ParseResult::Error(
            "merge requires --filespec <output>".to_string(),
        ));
    };
    Ok(MergeArgs {
        tindex_file,
        output_file,
        location_field,
        target_srs,
        clip,
    })
}

fn parse_merge_bounds(value: &str) -> Result<MergeClip, ParseResult> {
    let bounds = parse_bounds2d(value, 0)
        .map(|parsed| parsed.bounds)
        .map_err(|err| ParseResult::Error(format!("Invalid bounds: {err}")))?;
    Ok(MergeClip {
        bounds,
        stage_key: "bounds",
        stage_value: value.to_string(),
    })
}

fn parse_merge_polygon(value: &str) -> Result<MergeClip, ParseResult> {
    let geometry = Geometry::from_wkt(value)
        .map_err(|err| ParseResult::Error(format!("Invalid polygon: {err}")))?;
    let (minx, maxx, miny, maxy, _, _) = geometry
        .bounds()
        .map_err(|err| ParseResult::Error(format!("Invalid polygon bounds: {err}")))?;
    Ok(MergeClip {
        bounds: Bounds2D {
            minx,
            maxx,
            miny,
            maxy,
        },
        stage_key: "polygon",
        stage_value: value.to_string(),
    })
}

fn feature_bounds_2d(feature: &serde_json::Value) -> Option<Bounds2D> {
    let geometry = feature.get("geometry")?;
    match geometry.get("type")?.as_str()? {
        "Polygon" => bounds_from_positions(geometry.get("coordinates")?.get(0)?.as_array()?),
        "MultiPolygon" => {
            let polygons = geometry.get("coordinates")?.as_array()?;
            let mut output: Option<Bounds2D> = None;
            for polygon in polygons {
                let ring = polygon.get(0)?.as_array()?;
                let bounds = bounds_from_positions(ring)?;
                if let Some(out) = output.as_mut() {
                    out.grow_bounds(&bounds);
                } else {
                    output = Some(bounds);
                }
            }
            output
        }
        _ => None,
    }
}

fn bounds_from_positions(positions: &[serde_json::Value]) -> Option<Bounds2D> {
    let mut iter = positions.iter();
    let first = iter.next()?.as_array()?;
    let mut bounds = Bounds2D::empty();
    bounds.grow_point(first.first()?.as_f64()?, first.get(1)?.as_f64()?);
    for position in iter {
        let coords = position.as_array()?;
        bounds.grow_point(coords.first()?.as_f64()?, coords.get(1)?.as_f64()?);
    }
    Some(bounds)
}

fn execute_pipeline(pipeline_json: serde_json::Value) -> i32 {
    let mut pipeline = match pipeline_from_json(&pipeline_json.to_string()) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            return 1;
        }
    };
    match pipeline.execute_with_result(Vec::new()) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            1
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
