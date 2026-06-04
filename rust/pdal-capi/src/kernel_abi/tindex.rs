use crate::pipeline_abi::{pipeline_result_to_json_for_kernel, PipelineHandle};
use crate::registry::pipeline_from_json;
use pdal_core::driver::infer_reader_driver;
use pdal_core::gdal::{LayerHandle, Vector};
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_kernels::{
    parse_tindex_create_args, print_tindex_usage, BoundaryOptions, TindexCreateArgs as CreateArgs,
    TindexParseResult, INVALID_TINDEX_FILTER_STAGE_MESSAGE,
};
use pdal_native::geometry::Geometry;
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::Path;

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
        print_tindex_usage();
        return 0;
    }

    match args[0].as_str() {
        "create" => run_create(&args[1..]),
        "merge" => merge::run_merge(&args[1..]),
        _ => {
            eprintln!("PDAL: kernels.tindex: Expected 'create' or 'merge' subcommand.");
            1
        }
    }
}

fn run_create(args: &[String]) -> i32 {
    let args = match parse_tindex_create_args(args) {
        Ok(args) => args,
        Err(TindexParseResult::Error(message)) => {
            if message == INVALID_TINDEX_FILTER_STAGE_MESSAGE {
                println!("PDAL: kernels.tindex: {message}");
            }
            eprintln!("PDAL: kernels.tindex: {message}");
            return 1;
        }
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
    let dataset = match open_or_create_tindex_dataset(&args.tindex_file, &args.driver_name) {
        Ok(dataset) => dataset,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: Error opening tindex dataset: {err}");
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
    let layer = match dataset.open_or_create_layer_with_options(
        &args.layer_name,
        &args.target_srs,
        &args.lco_options,
    ) {
        Ok(layer) => layer,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: Error creating layer: {err}");
            return 1;
        }
    };
    if create_fields(layer, &args.location_field).is_err() {
        return 1;
    }
    let entries = match filter_existing_entries(&dataset, &args, entries) {
        Ok(entries) => entries,
        Err(()) => return 1,
    };
    if entries.is_empty() {
        eprintln!("PDAL: Couldn't index any files.");
        return 1;
    }
    add_features(layer, &args.location_field, entries)
}

fn open_or_create_tindex_dataset(path: &str, driver_name: &str) -> Result<Vector, String> {
    if Path::new(path).exists() {
        Vector::open_update(path)
    } else {
        Vector::create(path, driver_name)
    }
}

fn filter_existing_entries(
    dataset: &Vector,
    args: &CreateArgs,
    entries: Vec<Entry>,
) -> Result<Vec<Entry>, ()> {
    let existing =
        match dataset.get_string_features_by_layer(&args.layer_name, &args.location_field, "") {
            Ok(values) => values
                .into_iter()
                .map(|(_, location)| location)
                .collect::<HashSet<_>>(),
            Err(err) => {
                eprintln!("PDAL: kernels.tindex: Error reading existing tindex entries: {err}");
                return Err(());
            }
        };
    Ok(entries
        .into_iter()
        .filter(|entry| !existing.contains(&entry.location))
        .collect())
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
        eprintln!("PDAL: kernels.tindex: unable to infer reader driver for '{file}'.");
        return Err(());
    };
    let mut hexbin_stage = serde_json::json!({
        "type": "filters.hexbin",
        "threshold": opts.density,
        "sample_size": opts.sample_size,
    });
    if opts.edge_length > 0.0 {
        hexbin_stage["edge_length"] = serde_json::json!(opts.edge_length);
    }
    if let Some(where_expr) = &opts.where_expr {
        hexbin_stage["where"] = serde_json::json!(where_expr);
    }

    let mut pipeline = match pipeline_from_json(
        &serde_json::json!([
            { "type": driver, "filename": file },
            hexbin_stage
        ])
        .to_string(),
    ) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: {err}");
            return Err(());
        }
    };
    if let Err(err) = pipeline.execute(Vec::new()) {
        eprintln!("PDAL: kernels.tindex: {err}");
        return Err(());
    }
    boundary_from_hexbin_metadata(file, opts, &pipeline.metadata())
}

fn boundary_from_hexbin_metadata(
    file: &str,
    opts: &BoundaryOptions,
    metadata: &MetadataNode,
) -> Result<Option<String>, ()> {
    let Some(hexbin) = metadata.find_child("stage_1") else {
        return Ok(None);
    };
    let raw = hexbin
        .find_child("hex_boundary_raw")
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_string)
        .unwrap_or_else(|| "MULTIPOLYGON EMPTY".to_string());
    if raw == "MULTIPOLYGON EMPTY" {
        return Ok(None);
    }
    if !opts.smooth {
        return Ok(Some(raw));
    }
    let estimated_edge = hexbin
        .find_child("estimated_edge")
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_f64)
        .unwrap_or(0.0);
    let tolerance = 1.1 * estimated_edge / 2.0;
    match Geometry::from_wkt(&raw)
        .and_then(|g| g.simplify(tolerance, true))
        .and_then(|g| g.to_wkt())
    {
        Ok(simplified) => Ok(Some(ensure_multipolygon(&simplified))),
        Err(err) => {
            eprintln!("PDAL: kernels.tindex: GEOS simplify failed for '{file}': {err}");
            Ok(Some(raw))
        }
    }
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
        eprintln!("PDAL: kernels.tindex: unable to infer reader driver for '{file}'.");
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
        eprintln!("PDAL: kernels.tindex: unable to infer reader driver for '{file}'.");
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

mod merge;

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
