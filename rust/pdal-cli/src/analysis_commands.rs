use super::*;
use std::ffi::CString;
use std::io::Read;
use std::path::Path;

struct DeltaArgs {
    source: String,
    candidate: String,
    detail: bool,
    all_dims: bool,
}

fn parse_delta_args(args: &[String]) -> Result<DeltaArgs, String> {
    let mut source = None;
    let mut candidate = None;
    let mut detail = false;
    let mut all_dims = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--detail" => detail = true,
            "--alldims" => all_dims = true,
            "--source" => {
                let Some(value) = iter.next() else {
                    return Err("--source requires a filename".to_string());
                };
                source = Some(value.clone());
            }
            "--candidate" => {
                let Some(value) = iter.next() else {
                    return Err("--candidate requires a filename".to_string());
                };
                candidate = Some(value.clone());
            }
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

    let (Some(source), Some(candidate)) = (source, candidate) else {
        return Err("delta expects exactly two filenames".to_string());
    };

    Ok(DeltaArgs {
        source,
        candidate,
        detail,
        all_dims,
    })
}

fn parse_source_candidate_args<'a>(
    command: &str,
    args: &'a [String],
) -> Result<(&'a str, &'a str), String> {
    let mut source = None;
    let mut candidate = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--source" {
            let Some(value) = iter.next() else {
                return Err("--source requires a filename".to_string());
            };
            source = Some(value.as_str());
        } else if let Some(value) = arg.strip_prefix("--source=") {
            source = Some(value);
        } else if arg == "--candidate" {
            let Some(value) = iter.next() else {
                return Err("--candidate requires a filename".to_string());
            };
            candidate = Some(value.as_str());
        } else if let Some(value) = arg.strip_prefix("--candidate=") {
            candidate = Some(value);
        } else if arg.starts_with("--") {
            return Err(format!("unknown {command} option '{arg}'"));
        } else if source.is_none() {
            source = Some(arg.as_str());
        } else if candidate.is_none() {
            candidate = Some(arg.as_str());
        } else {
            return Err(format!("{command} expects exactly two filenames"));
        }
    }

    match (source, candidate) {
        (Some(source), Some(candidate)) => Ok((source, candidate)),
        _ => Err(format!("{command} expects exactly two filenames")),
    }
}

struct TindexCreateArgs {
    tindex_file: String,
    files: Vec<String>,
    driver_name: String,
    path_prefix: Option<String>,
    write_absolute_path: bool,
    layer_name: String,
    location_field: String,
}

struct TindexEntry {
    location: String,
    wkt: String,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
}

fn parse_tindex_create_args(args: &[String]) -> Result<TindexCreateArgs, String> {
    let mut parsed = TindexCreateArgs {
        tindex_file: String::new(),
        files: Vec::new(),
        driver_name: "ESRI Shapefile".to_string(),
        path_prefix: None,
        write_absolute_path: false,
        layer_name: "pdal".to_string(),
        location_field: "location".to_string(),
    };

    let mut args_iter = args.iter();
    while let Some(arg) = args_iter.next() {
        parse_tindex_create_arg(arg, &mut args_iter, &mut parsed)?;
    }
    if parsed.tindex_file.is_empty() {
        return Err("tindex create requires --tindex <output>".to_string());
    }
    if parsed.files.is_empty() {
        return Err("tindex create needs at least one input file".to_string());
    }
    Ok(parsed)
}

fn parse_tindex_create_arg<'a, I>(
    arg: &str,
    args_iter: &mut I,
    parsed: &mut TindexCreateArgs,
) -> Result<(), String>
where
    I: Iterator<Item = &'a String>,
{
    match arg {
        "--tindex" => parsed.tindex_file = next_tindex_value(args_iter, "--tindex")?.clone(),
        "--filelist" => {
            let path = next_tindex_value(args_iter, "--filelist")?;
            parsed.files.extend(read_tindex_filelist(path)?);
        }
        "--glob" => {
            let pattern = next_tindex_value(args_iter, "--glob")?;
            parsed.files.extend(read_tindex_glob(pattern)?);
        }
        "--stdin" | "-s" => parsed.files.extend(read_tindex_stdin()?),
        "--path_prefix" => {
            parsed.path_prefix = Some(next_tindex_value(args_iter, "--path_prefix")?.clone());
        }
        "--write_absolute_path" => parsed.write_absolute_path = true,
        "--lyr_name" => parsed.layer_name = next_tindex_value(args_iter, "--lyr_name")?.clone(),
        "--tindex_name" => {
            parsed.location_field = next_tindex_value(args_iter, "--tindex_name")?.clone();
        }
        "--fast_boundary" => {}
        "-f" | "--ogrdriver" => {
            parsed.driver_name = next_tindex_value(args_iter, arg)?.clone();
        }
        _ if arg.starts_with('-') => return Err(format!("unknown tindex option '{arg}'")),
        _ if parsed.tindex_file.is_empty() => parsed.tindex_file = arg.to_string(),
        _ => parsed.files.push(arg.to_string()),
    }
    Ok(())
}

fn next_tindex_value<'a, I>(args_iter: &mut I, arg: &str) -> Result<&'a String, String>
where
    I: Iterator<Item = &'a String>,
{
    args_iter
        .next()
        .ok_or_else(|| format!("{arg} requires a value"))
}

fn read_tindex_filelist(path: &str) -> Result<Vec<String>, String> {
    let contents = pdal_io::source::read_to_string(path)
        .map_err(|err| format!("unable to read file list '{path}': {err}"))?;
    Ok(nonempty_lines(&contents))
}

fn read_tindex_glob(pattern: &str) -> Result<Vec<String>, String> {
    let entries =
        glob::glob(pattern).map_err(|err| format!("invalid glob pattern '{pattern}': {err}"))?;
    let mut files = Vec::new();
    for entry in entries {
        match entry {
            Ok(path) => files.push(path.to_string_lossy().into_owned()),
            Err(err) => return Err(format!("reading glob match for '{pattern}': {err}")),
        }
    }
    if files.is_empty() {
        return Err(format!("glob pattern '{pattern}' did not match any files"));
    }
    Ok(files)
}

fn read_tindex_stdin() -> Result<Vec<String>, String> {
    let mut contents = String::new();
    std::io::stdin()
        .read_to_string(&mut contents)
        .map_err(|err| format!("unable to read tindex input list from stdin: {err}"))?;
    Ok(nonempty_lines(&contents))
}

fn nonempty_lines(contents: &str) -> Vec<String> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn tindex_bounds(file: &str, summary: &serde_json::Value) -> Result<(f64, f64, f64, f64), ()> {
    let Some(bounds) = summary.get("bounds_2d") else {
        eprintln!("Error: '{file}' produced no 2D bounds");
        return Err(());
    };
    let Some(minx) = bounds["minx"].as_f64() else {
        eprintln!("Error: '{file}' produced invalid minx bounds");
        return Err(());
    };
    let Some(maxx) = bounds["maxx"].as_f64() else {
        eprintln!("Error: '{file}' produced invalid maxx bounds");
        return Err(());
    };
    let Some(miny) = bounds["miny"].as_f64() else {
        eprintln!("Error: '{file}' produced invalid miny bounds");
        return Err(());
    };
    let Some(maxy) = bounds["maxy"].as_f64() else {
        eprintln!("Error: '{file}' produced invalid maxy bounds");
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
            eprintln!("Error: unable to resolve absolute path for '{file}': {err}");
        })
}

fn create_tindex_fields(
    layer: pdal_core::gdal::LayerHandle,
    location_field: &str,
) -> Result<(), ()> {
    unsafe {
        for result in [
            pdal_core::gdal::Vector::create_string_field(layer, location_field),
            pdal_core::gdal::Vector::create_string_field(layer, "srs"),
            pdal_core::gdal::Vector::create_datetime_field(layer, "created"),
            pdal_core::gdal::Vector::create_datetime_field(layer, "modified"),
        ] {
            if let Err(err) = result {
                eprintln!("Error creating tindex field: {err}");
                return Err(());
            }
        }
    }
    Ok(())
}

fn add_tindex_features(
    layer: pdal_core::gdal::LayerHandle,
    location_field: &str,
    entries: Vec<TindexEntry>,
) -> i32 {
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
            if let Err(e) = pdal_core::gdal::Vector::add_feature(layer, &poly_wkt, &fields) {
                eprintln!("Error adding feature for {}: {}", entry.location, e);
                return 1;
            }
        }
        println!("Indexed file {}", entry.location);
    }
    0
}

impl App {
    pub(super) fn run_tile(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!(
                "  pdal tile <input> <output-template> \
                 [--length=N] [--origin_x=X] [--origin_y=Y] [--buffer=N]"
            );
            println!("  the output template must contain a single '#' placeholder");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let mut positional: Vec<&str> = Vec::new();
        let mut input: Option<&str> = None;
        let mut output: Option<&str> = None;
        let mut length = 1000.0_f64;
        let mut origin_x = f64::NAN;
        let mut origin_y = f64::NAN;
        let mut buffer = 0.0_f64;
        let mut args = self.command_args.iter();
        while let Some(arg) = args.next() {
            if arg == "--input" || arg == "-i" {
                let Some(value) = args.next() else {
                    eprintln!("Error: {arg} requires an input path");
                    return 1;
                };
                input = Some(value);
            } else if arg == "--output" || arg == "-o" {
                let Some(value) = args.next() else {
                    eprintln!("Error: {arg} requires an output template");
                    return 1;
                };
                output = Some(value);
            } else if let Some(rest) = arg.strip_prefix("--") {
                let (key, value) = match rest.split_once('=') {
                    Some(pair) => pair,
                    None => {
                        let Some(value) = args.next() else {
                            eprintln!("Error: option '{arg}' requires a value");
                            return 1;
                        };
                        (rest, value.as_str())
                    }
                };
                let target = match key {
                    "length" => &mut length,
                    "origin_x" => &mut origin_x,
                    "origin_y" => &mut origin_y,
                    "buffer" => &mut buffer,
                    _ => {
                        eprintln!("Error: unknown tile option '--{key}'");
                        return 1;
                    }
                };
                match value.parse::<f64>() {
                    Ok(parsed) => *target = parsed,
                    Err(_) => {
                        eprintln!("Error: tile option '--{key}' expects a number");
                        return 1;
                    }
                }
            } else if input.is_none() {
                input = Some(arg);
            } else if output.is_none() {
                output = Some(arg);
            } else {
                positional.push(arg);
            }
        }
        if input.is_none() && !positional.is_empty() {
            input = Some(positional.remove(0));
        }
        if output.is_none() && !positional.is_empty() {
            output = Some(positional.remove(0));
        }
        if !positional.is_empty() {
            eprintln!("Error: tile expects an input path and an output template");
            return 1;
        }
        let (Some(input), Some(output)) = (input, output) else {
            eprintln!("Error: tile expects an input path and an output template");
            return 1;
        };

        let (c_input, c_template) = match (CString::new(input), CString::new(output)) {
            (Ok(input), Ok(template)) => (input, template),
            _ => {
                eprintln!("Error: a path contains an interior NUL byte");
                return 1;
            }
        };

        let count = unsafe {
            pdal_capi::pdal_tile(
                c_input.as_ptr(),
                c_template.as_ptr(),
                length,
                origin_x,
                origin_y,
                buffer,
            )
        };
        if count < 0 {
            self.output_last_error();
            return 1;
        }
        println!("Wrote {count} tile(s).");
        0
    }

    pub(super) fn run_hausdorff(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal hausdorff <source> <candidate>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        let (source, candidate) = match parse_source_candidate_args("hausdorff", &self.command_args)
        {
            Ok(paths) => paths,
            Err(message) => {
                eprintln!("Error: {message}");
                return 1;
            }
        };
        let (c_source, c_candidate) = match (CString::new(source), CString::new(candidate)) {
            (Ok(source), Ok(candidate)) => (source, candidate),
            _ => {
                eprintln!("Error: a filename contains an interior NUL byte");
                return 1;
            }
        };

        let mut hausdorff = 0.0f64;
        let mut modified = 0.0f64;
        let status = unsafe {
            pdal_capi::pdal_hausdorff(
                c_source.as_ptr(),
                c_candidate.as_ptr(),
                &mut hausdorff,
                &mut modified,
            )
        };
        if status < 0 {
            self.output_last_error();
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

    pub(super) fn run_chamfer(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal chamfer <source> <candidate>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        let (source, candidate) = match parse_source_candidate_args("chamfer", &self.command_args) {
            Ok(paths) => paths,
            Err(message) => {
                eprintln!("Error: {message}");
                return 1;
            }
        };
        let (c_source, c_candidate) = match (CString::new(source), CString::new(candidate)) {
            (Ok(source), Ok(candidate)) => (source, candidate),
            _ => {
                eprintln!("Error: a filename contains an interior NUL byte");
                return 1;
            }
        };

        let mut chamfer = 0.0f64;
        let status = unsafe {
            pdal_capi::pdal_chamfer(c_source.as_ptr(), c_candidate.as_ptr(), &mut chamfer)
        };
        if status < 0 {
            self.output_last_error();
            return 1;
        }

        let report = serde_json::json!({
            "filenames": [source, candidate],
            "chamfer": chamfer,
        });
        println!("{}", serde_json::to_string(&report).unwrap());
        0
    }

    pub(super) fn run_delta(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal delta <source> <candidate> [--detail] [--alldims]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        let args = match parse_delta_args(&self.command_args) {
            Ok(args) => args,
            Err(message) => {
                eprintln!("Error: {message}");
                return 1;
            }
        };
        let (c_source, c_candidate) = match (
            CString::new(args.source.as_str()),
            CString::new(args.candidate.as_str()),
        ) {
            (Ok(source), Ok(candidate)) => (source, candidate),
            _ => {
                eprintln!("Error: a filename contains an interior NUL byte");
                return 1;
            }
        };

        let json_ptr = unsafe {
            pdal_capi::pdal_delta_ex(
                c_source.as_ptr(),
                c_candidate.as_ptr(),
                args.detail,
                args.all_dims,
            )
        };
        if json_ptr.is_null() {
            self.output_last_error();
            return 1;
        }
        if let Some(json) = safe_cstr(json_ptr) {
            println!("{}", json);
        }
        unsafe { pdal_capi::pdal_string_free(json_ptr) };
        0
    }

    pub(super) fn run_tindex(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal tindex create --tindex <output> <files...> [-f <driver>]");
            println!("  pdal tindex create --tindex <output> --filelist <path> [-f <driver>]");
            println!("  pdal tindex create --tindex <output> --glob <pattern> [-f <driver>]");
            println!("  pdal tindex create --tindex <output> --stdin [-f <driver>]");
            println!("  pdal tindex create --tindex <output> --path_prefix <prefix> <files...>");
            println!("  pdal tindex create --tindex <output> --lyr_name <name> <files...>");
            println!("  pdal tindex create <output> <files...> [-f <driver>]");
            println!("  pdal tindex merge --tindex <index> --filespec <output>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let subcommand = &self.command_args[0];
        if subcommand == "merge" {
            return self.run_tindex_merge();
        } else if subcommand != "create" {
            eprintln!("Error: expected 'create' or 'merge' subcommand");
            return 1;
        }

        if self.command_args.len() < 2 {
            eprintln!("Error: tindex {subcommand} needs more arguments");
            return 1;
        }

        self.run_tindex_create()
    }

    fn run_tindex_create(&self) -> i32 {
        let args = match parse_tindex_create_args(&self.command_args[1..]) {
            Ok(args) => args,
            Err(message) => {
                eprintln!("Error: {message}");
                return 1;
            }
        };

        pdal_core::gdal::register_drivers();
        let dataset = match pdal_core::gdal::Vector::create(&args.tindex_file, &args.driver_name) {
            Ok(ds) => ds,
            Err(e) => {
                eprintln!("Error creating tindex dataset: {}", e);
                return 1;
            }
        };

        let (first_srs, entries) = match self.tindex_entries(&args) {
            Ok(entries) => entries,
            Err(()) => return 1,
        };
        let layer = match dataset.open_or_create_layer(&args.layer_name, &first_srs) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error creating layer: {}", e);
                return 1;
            }
        };
        if create_tindex_fields(layer, &args.location_field).is_err() {
            return 1;
        }
        add_tindex_features(layer, &args.location_field, entries)
    }

    fn tindex_entries(&self, args: &TindexCreateArgs) -> Result<(String, Vec<TindexEntry>), ()> {
        let mut first_srs = String::new();
        let mut entries = Vec::new();
        for file in &args.files {
            let mut entry = self.tindex_entry(file, args)?;
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

    fn tindex_entry(&self, file: &str, args: &TindexCreateArgs) -> Result<TindexEntry, ()> {
        let summary = match self.tindex_summary(file) {
            Ok(summary) => summary,
            Err(()) => return Err(()),
        };
        let (minx, miny, maxx, maxy) = tindex_bounds(file, &summary)?;
        let wkt = summary["metadata"]["pipeline"]["stage_0"]["srs"]["wkt"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let location = tindex_location(file, args.write_absolute_path)?;
        Ok(TindexEntry {
            location,
            wkt,
            minx,
            miny,
            maxx,
            maxy,
        })
    }

    fn tindex_summary(&self, file: &str) -> Result<serde_json::Value, ()> {
        let Some(driver) = pdal_core::driver::infer_reader_driver(file) else {
            eprintln!("Error: unable to infer a reader driver for '{file}'");
            return Err(());
        };

        let pipeline_json = serde_json::json!([{ "type": driver, "filename": file }]).to_string();
        let c_json = match CString::new(pipeline_json) {
            Ok(json) => json,
            Err(_) => {
                eprintln!("Error: input path '{file}' contains an interior NUL byte");
                return Err(());
            }
        };

        let pipeline = unsafe { pdal_capi::pdal_pipeline_create_json(c_json.as_ptr()) };
        if pipeline.is_null() {
            self.output_last_error();
            return Err(());
        }

        let json_ptr = unsafe {
            pdal_capi::pdal_pipeline_execute_summary_json(pipeline, std::ptr::null_mut())
        };
        unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };

        if json_ptr.is_null() {
            self.output_last_error();
            return Err(());
        }

        let summary_str = safe_cstr(json_ptr).unwrap_or_default();
        unsafe { pdal_capi::pdal_string_free(json_ptr) };

        serde_json::from_str::<serde_json::Value>(&summary_str).map_err(|err| {
            eprintln!("Error: unable to parse pipeline summary for '{file}': {err}");
        })
    }

    fn run_tindex_merge(&self) -> i32 {
        let mut tindex_file: Option<&str> = None;
        let mut output_file: Option<&str> = None;
        let mut location_field = "location";

        let mut args = self.command_args[1..].iter();
        while let Some(arg) = args.next() {
            if arg == "--tindex" {
                let Some(path) = args.next() else {
                    eprintln!("Error: --tindex requires an index path");
                    return 1;
                };
                tindex_file = Some(path);
            } else if arg == "--filespec" {
                let Some(path) = args.next() else {
                    eprintln!("Error: --filespec requires an output path");
                    return 1;
                };
                output_file = Some(path);
            } else if arg == "--tindex_name" {
                let Some(name) = args.next() else {
                    eprintln!("Error: --tindex_name requires a field name");
                    return 1;
                };
                location_field = name;
            } else if arg == "--lyr_name" || arg == "--ogrdriver" || arg == "-f" {
                let Some(_) = args.next() else {
                    eprintln!("Error: {arg} requires a value");
                    return 1;
                };
            } else if arg.starts_with('-') {
                eprintln!("Error: unknown tindex merge option '{arg}'");
                return 1;
            } else if tindex_file.is_none() {
                tindex_file = Some(arg);
            } else if output_file.is_none() {
                output_file = Some(arg);
            } else {
                eprintln!("Error: tindex merge expects an index path and an output path");
                return 1;
            }
        }

        let Some(tindex_file) = tindex_file else {
            eprintln!("Error: tindex merge requires --tindex <index>");
            return 1;
        };
        let Some(output_file) = output_file else {
            eprintln!("Error: tindex merge requires --filespec <output>");
            return 1;
        };

        let index_json = match pdal_io::source::read_to_string(tindex_file) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("Error: unable to read tindex '{tindex_file}': {err}");
                return 1;
            }
        };
        let index: serde_json::Value = match serde_json::from_str(&index_json) {
            Ok(index) => index,
            Err(err) => {
                eprintln!("Error: unable to parse GeoJSON tindex '{tindex_file}': {err}");
                return 1;
            }
        };
        let Some(features) = index["features"].as_array() else {
            eprintln!("Error: tindex merge expects a GeoJSON FeatureCollection");
            return 1;
        };
        if features.is_empty() {
            eprintln!("Error: tindex contains no features");
            return 1;
        }

        let mut stages = Vec::new();
        let mut tags = Vec::new();
        for (index, feature) in features.iter().enumerate() {
            let Some(location) = feature["properties"][location_field].as_str() else {
                eprintln!("Error: tindex feature is missing '{location_field}'");
                return 1;
            };
            let Some(reader) = pdal_core::driver::infer_reader_driver(location) else {
                eprintln!("Error: unable to infer a reader driver for '{location}'");
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
        let Some(writer) = pdal_core::driver::infer_writer_driver(output_file) else {
            eprintln!("Error: unable to infer a writer driver for '{output_file}'");
            return 1;
        };
        stages.push(serde_json::json!({ "type": writer, "filename": output_file }));

        self.execute_stage_pipeline(stages)
    }

    pub(super) fn run_eval(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!(
                "  pdal eval <predicted> <truth> --labels=<l1,l2,...> \
                 [--prediction_dim=Classification] [--truth_dim=Classification]"
            );
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let mut predicted: Option<&str> = None;
        let mut truth: Option<&str> = None;
        let mut labels = String::new();
        let mut prediction_dim = String::from("Classification");
        let mut truth_dim = String::from("Classification");
        let mut args = self.command_args.iter();
        while let Some(arg) = args.next() {
            if let Some(rest) = arg.strip_prefix("--") {
                let (key, value) = match rest.split_once('=') {
                    Some(pair) => pair,
                    None => {
                        let Some(value) = args.next() else {
                            eprintln!("Error: option '{arg}' requires a value");
                            return 1;
                        };
                        (rest, value.as_str())
                    }
                };
                match key {
                    "predicted" => predicted = Some(value),
                    "truth" => truth = Some(value),
                    "labels" => labels = value.to_string(),
                    "prediction_dim" => prediction_dim = value.to_string(),
                    "truth_dim" => truth_dim = value.to_string(),
                    _ => {
                        eprintln!("Error: unknown eval option '--{key}'");
                        return 1;
                    }
                }
            } else if predicted.is_none() {
                predicted = Some(arg);
            } else if truth.is_none() {
                truth = Some(arg);
            } else {
                eprintln!("Error: eval expects a predicted path and a truth path");
                return 1;
            }
        }
        let (Some(predicted), Some(truth)) = (predicted, truth) else {
            eprintln!("Error: eval expects a predicted path and a truth path");
            return 1;
        };
        if labels.is_empty() {
            eprintln!("Error: eval requires --labels=<comma-separated classification labels>");
            return 1;
        }

        let (c_predicted, c_truth, c_labels, c_prediction_dim, c_truth_dim) = match (
            CString::new(predicted),
            CString::new(truth),
            CString::new(labels.as_str()),
            CString::new(prediction_dim.as_str()),
            CString::new(truth_dim.as_str()),
        ) {
            (Ok(a), Ok(b), Ok(c), Ok(d), Ok(e)) => (a, b, c, d, e),
            _ => {
                eprintln!("Error: an argument contains an interior NUL byte");
                return 1;
            }
        };

        let json_ptr = unsafe {
            pdal_capi::pdal_eval(
                c_predicted.as_ptr(),
                c_truth.as_ptr(),
                c_labels.as_ptr(),
                c_prediction_dim.as_ptr(),
                c_truth_dim.as_ptr(),
            )
        };
        if json_ptr.is_null() {
            self.output_last_error();
            return 1;
        }
        if let Some(json) = safe_cstr(json_ptr) {
            println!("{}", json);
        }
        unsafe { pdal_capi::pdal_string_free(json_ptr) };
        0
    }

    /// Build a pipeline from assembled stage objects and execute it.
    pub(super) fn execute_stage_pipeline(&self, stages: Vec<serde_json::Value>) -> i32 {
        self.execute_stage_pipeline_with_stream(stages, false, false)
    }

    pub(super) fn execute_stage_pipeline_with_stream(
        &self,
        stages: Vec<serde_json::Value>,
        stream_allowed: bool,
        stream_required: bool,
    ) -> i32 {
        let pipeline_json = serde_json::Value::Array(stages).to_string();
        let c_json = match CString::new(pipeline_json) {
            Ok(json) => json,
            Err(_) => {
                eprintln!("Error: a path contains an interior NUL byte");
                return 1;
            }
        };

        let pipeline = unsafe { pdal_capi::pdal_pipeline_create_json(c_json.as_ptr()) };
        if pipeline.is_null() {
            self.output_last_error();
            return 1;
        }
        if stream_allowed {
            let streamed = unsafe { pdal_capi::pdal_pipeline_execute_streaming(pipeline) };
            if streamed >= 0 {
                unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
                return 0;
            }
            if streamed == -2 && stream_required {
                eprintln!("Error: pipeline is not streamable");
                unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
                return 1;
            }
            if streamed == -1 {
                self.output_last_error();
                unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
                return 1;
            }
        }
        let mut result = empty_pipeline_result();
        let status = unsafe {
            pdal_capi::pdal_pipeline_execute_result(pipeline, std::ptr::null_mut(), &mut result)
        };
        unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
        if status < 0 {
            self.output_last_error();
            return 1;
        }
        0
    }
}

#[cfg(test)]
#[path = "analysis_commands/tests.rs"]
mod tests;
