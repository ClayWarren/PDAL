use crate::pipeline_abi::{pipeline_result_to_json_for_kernel, PipelineHandle};
use crate::registry::pipeline_from_json;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::gdal::{LayerHandle, Vector};
use std::ffi::CStr;
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
    };

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => parsed.tindex_file = next_value(&mut iter, "--tindex")?.clone(),
            "--filelist" => return Err(ParseResult::Unsupported),
            "--glob" => {
                let pattern = next_value(&mut iter, "--glob")?;
                parsed.files.extend(read_glob(pattern)?);
            }
            "--path_prefix" => parsed.path_prefix = Some(next_value(&mut iter, arg)?.clone()),
            "--write_absolute_path" => parsed.write_absolute_path = true,
            "--lyr_name" => parsed.layer_name = next_value(&mut iter, arg)?.clone(),
            "--tindex_name" => parsed.location_field = next_value(&mut iter, arg)?.clone(),
            "-f" | "--ogrdriver" => parsed.driver_name = next_value(&mut iter, arg)?.clone(),
            "--stdin" | "-s" | "--threshold" | "--resolution" | "--simplify" | "--where"
            | "--fast_boundary" => return Err(ParseResult::Unsupported),
            _ if arg.starts_with("--threshold=")
                || arg.starts_with("--resolution=")
                || arg.starts_with("--simplify=")
                || arg.starts_with("--where=")
                || arg.starts_with("--filters.") =>
            {
                return Err(ParseResult::Unsupported);
            }
            _ if arg.starts_with('-') => return Err(ParseResult::Unsupported),
            _ if parsed.tindex_file.is_empty() => parsed.tindex_file = arg.clone(),
            _ if arg.contains('*') || arg.contains('?') || arg.contains('[') => {
                return Err(ParseResult::Unsupported);
            }
            _ => parsed.files.push(arg.clone()),
        }
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
    let (index_file, output_file, location_field) = match parse_merge_args(args) {
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
    for (index, feature) in features.iter().enumerate() {
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

fn parse_merge_args(args: &[String]) -> Result<(String, String, String), ParseResult> {
    let mut tindex_file = None;
    let mut output_file = None;
    let mut location_field = "location".to_string();

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tindex" => tindex_file = Some(next_value(&mut iter, "--tindex")?.clone()),
            "--filespec" => output_file = Some(next_value(&mut iter, "--filespec")?.clone()),
            "--tindex_name" => location_field = next_value(&mut iter, "--tindex_name")?.clone(),
            "--lyr_name" | "--ogrdriver" | "-f" => {
                let _ = next_value(&mut iter, arg)?;
            }
            _ if arg.starts_with("--bounds") || arg.starts_with("--") => {
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
    Ok((tindex_file, output_file, location_field))
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
