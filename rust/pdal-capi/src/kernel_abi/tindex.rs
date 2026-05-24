use crate::pipeline_abi::{pipeline_result_to_json_for_kernel, PipelineHandle};
use crate::registry::pipeline_from_json;
use pdal_core::bounds::{parse_bounds2d, Bounds2D};
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::gdal::{LayerHandle, Vector};
use std::ffi::CStr;
use std::io::Read;
use std::os::raw::c_char;
use std::path::Path;

struct CreateArgs {
    tindex_file: String,
    files: Vec<String>,
    driver_name: String,
    path_prefix: Option<String>,
    write_absolute_path: bool,
    layer_name: String,
    location_field: String,
    input_methods: u8,
    unsupported_input: bool,
}

struct Entry {
    location: String,
    wkt: String,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
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
    let layer = match dataset.open_or_create_layer(&args.layer_name, &first_srs) {
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
        path_prefix: None,
        write_absolute_path: false,
        layer_name: "pdal".to_string(),
        location_field: "location".to_string(),
        input_methods: 0,
        unsupported_input: false,
    };

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => parsed.tindex_file = next_value(&mut iter, "--tindex")?.clone(),
            "--filelist" => {
                parsed.input_methods += 1;
                let _ = next_value(&mut iter, "--filelist")?;
                parsed.unsupported_input = true;
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
            "--log" => {
                let _ = next_value(&mut iter, "--log")?;
            }
            "--stdin" | "-s" => {
                parsed.input_methods += 1;
                parsed.files.extend(read_stdin_files()?);
            }
            "--threshold" | "--resolution" | "--simplify" | "--where" | "--fast_boundary" => {
                return Err(ParseResult::Unsupported);
            }
            _ if let Some(value) = arg.strip_prefix("--filespec=") => {
                parsed.input_methods += 1;
                parsed.files.push(value.to_string());
            }
            _ if let Some(pattern) = arg.strip_prefix("--glob=") => {
                parsed.input_methods += 1;
                parsed.files.extend(read_glob(pattern)?);
            }
            _ if arg.starts_with("--filelist=") => {
                parsed.input_methods += 1;
                parsed.unsupported_input = true;
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
            _ if arg.starts_with("--log=") => {}
            _ if arg.starts_with("--threshold=")
                || arg.starts_with("--resolution=")
                || arg.starts_with("--simplify=")
                || arg.starts_with("--where=") =>
            {
                return Err(ParseResult::Unsupported);
            }
            _ if arg.starts_with("--filters.hexbin.smooth") => {
                return Err(ParseResult::Error(INVALID_FILTER_STAGE_MESSAGE.to_string()));
            }
            _ if arg.starts_with("--filters.") => return Err(ParseResult::Unsupported),
            _ if arg.starts_with('-') => return Err(ParseResult::Unsupported),
            _ if parsed.tindex_file.is_empty() => parsed.tindex_file = arg.clone(),
            _ if arg.contains('*') || arg.contains('?') || arg.contains('[') => {
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

fn next_value<'a, I>(iter: &mut I, arg: &str) -> Result<&'a String, ParseResult>
where
    I: Iterator<Item = &'a String>,
{
    iter.next()
        .ok_or_else(|| ParseResult::Error(format!("{arg} requires a value")))
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

fn collect_entries(args: &CreateArgs) -> Result<(String, Vec<Entry>), ()> {
    let mut first_srs = String::new();
    let mut entries = Vec::new();
    for file in &args.files {
        let mut entry = create_entry(file, args)?;
        if first_srs.is_empty() && !entry.wkt.is_empty() {
            first_srs.clone_from(&entry.wkt);
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
    let wkt = summary["metadata"]["pipeline"]["stage_0"]["srs"]["wkt"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let location = tindex_location(file, args.write_absolute_path)?;
    Ok(Entry {
        location,
        wkt,
        minx,
        miny,
        maxx,
        maxy,
    })
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
        let poly_wkt = format!(
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
        );
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

fn run_merge(args: &[String]) -> i32 {
    let (index_file, output_file, location_field, bounds) = match parse_merge_args(args) {
        Ok(parsed) => parsed,
        Err(ParseResult::Error(message)) => {
            eprintln!("PDAL: kernels.tindex: {message}");
            return 1;
        }
        Err(ParseResult::Unsupported) => return -1,
    };

    let index_json = match std::fs::read_to_string(&index_file) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: Unable to read tindex '{index_file}': {err}");
            return 1;
        }
    };
    let index: serde_json::Value = match serde_json::from_str(&index_json) {
        Ok(index) => index,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: Unable to parse GeoJSON tindex '{index_file}': {err}");
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
        if let Some(bounds) = &bounds {
            let Some(feature_bounds) = feature_bounds_2d(feature) else {
                eprintln!("PDAL: kernels.tindex: Feature has invalid geometry.");
                return 1;
            };
            if !feature_bounds.overlaps(bounds) {
                continue;
            }
        }
        let Some(location) = feature["properties"][&location_field].as_str() else {
            eprintln!("PDAL: kernels.tindex: Feature is missing '{location_field}'.");
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
            "tag": tag,
        }));
        tags.push(tag);
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
    let Some(writer) = infer_writer_driver(&output_file) else {
        eprintln!("PDAL: kernels.tindex: Unable to infer writer driver for '{output_file}'.");
        return 1;
    };
    stages.push(serde_json::json!({ "type": writer, "filename": output_file }));
    execute_pipeline(serde_json::Value::Array(stages))
}

fn parse_merge_args(
    args: &[String],
) -> Result<(String, String, String, Option<Bounds2D>), ParseResult> {
    let mut tindex_file = None;
    let mut output_file = None;
    let mut location_field = "location".to_string();
    let mut bounds = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => tindex_file = Some(next_value(&mut iter, "--tindex")?.clone()),
            "--filespec" => output_file = Some(next_value(&mut iter, "--filespec")?.clone()),
            "--tindex_name" => location_field = next_value(&mut iter, "--tindex_name")?.clone(),
            "--bounds" => {
                let value = next_value(&mut iter, "--bounds")?;
                bounds = Some(parse_merge_bounds(value)?);
            }
            "--log" => {
                let _ = next_value(&mut iter, "--log")?;
            }
            "--lyr_name" | "--ogrdriver" | "-f" => {
                let _ = next_value(&mut iter, arg)?;
            }
            _ if let Some(value) = arg.strip_prefix("--bounds=") => {
                bounds = Some(parse_merge_bounds(value)?);
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
    Ok((tindex_file, output_file, location_field, bounds))
}

fn parse_merge_bounds(value: &str) -> Result<Bounds2D, ParseResult> {
    parse_bounds2d(value, 0)
        .map(|parsed| parsed.bounds)
        .map_err(|err| ParseResult::Error(format!("Invalid bounds: {err}")))
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
