use super::*;
use std::ffi::CString;
use std::io::Read;
use std::path::Path;

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
            println!("  pdal delta <source> <candidate>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        let (source, candidate) = match parse_source_candidate_args("delta", &self.command_args) {
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

        let json_ptr = unsafe { pdal_capi::pdal_delta(c_source.as_ptr(), c_candidate.as_ptr()) };
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
            eprintln!("Error: merge is not yet supported in the Rust tindex kernel");
            return 1;
        } else if subcommand != "create" {
            eprintln!("Error: expected 'create' or 'merge' subcommand");
            return 1;
        }

        if self.command_args.len() < 2 {
            eprintln!("Error: tindex {subcommand} needs more arguments");
            return 1;
        }

        let mut tindex_file: Option<String> = None;
        let mut files = Vec::new();
        let mut driver_name = "ESRI Shapefile".to_string();
        let mut path_prefix: Option<String> = None;
        let mut write_absolute_path = false;
        let mut layer_name = "pdal".to_string();
        let mut location_field = "location".to_string();

        let mut args_iter = self.command_args[1..].iter();
        while let Some(arg) = args_iter.next() {
            if arg == "--tindex" {
                let Some(path) = args_iter.next() else {
                    eprintln!("Error: --tindex requires an output path");
                    return 1;
                };
                tindex_file = Some(path.clone());
            } else if arg == "--filelist" {
                let Some(path) = args_iter.next() else {
                    eprintln!("Error: --filelist requires a path");
                    return 1;
                };
                let contents = match std::fs::read_to_string(path) {
                    Ok(contents) => contents,
                    Err(err) => {
                        eprintln!("Error: unable to read file list '{path}': {err}");
                        return 1;
                    }
                };
                files.extend(
                    contents
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .map(str::to_string),
                );
            } else if arg == "--glob" {
                let Some(pattern) = args_iter.next() else {
                    eprintln!("Error: --glob requires a pattern");
                    return 1;
                };
                let entries = match glob::glob(pattern) {
                    Ok(entries) => entries,
                    Err(err) => {
                        eprintln!("Error: invalid glob pattern '{pattern}': {err}");
                        return 1;
                    }
                };
                let mut matched = false;
                for entry in entries {
                    match entry {
                        Ok(path) => {
                            matched = true;
                            files.push(path.to_string_lossy().into_owned());
                        }
                        Err(err) => {
                            eprintln!("Error reading glob match for '{pattern}': {err}");
                            return 1;
                        }
                    }
                }
                if !matched {
                    eprintln!("Error: glob pattern '{pattern}' did not match any files");
                    return 1;
                }
            } else if arg == "--stdin" || arg == "-s" {
                let mut contents = String::new();
                if let Err(err) = std::io::stdin().read_to_string(&mut contents) {
                    eprintln!("Error: unable to read tindex input list from stdin: {err}");
                    return 1;
                }
                files.extend(
                    contents
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .map(str::to_string),
                );
            } else if arg == "--path_prefix" {
                let Some(prefix) = args_iter.next() else {
                    eprintln!("Error: --path_prefix requires a prefix");
                    return 1;
                };
                path_prefix = Some(prefix.clone());
            } else if arg == "--write_absolute_path" {
                write_absolute_path = true;
            } else if arg == "--lyr_name" {
                let Some(name) = args_iter.next() else {
                    eprintln!("Error: --lyr_name requires a layer name");
                    return 1;
                };
                layer_name = name.clone();
            } else if arg == "--tindex_name" {
                let Some(name) = args_iter.next() else {
                    eprintln!("Error: --tindex_name requires a field name");
                    return 1;
                };
                location_field = name.clone();
            } else if arg == "--fast_boundary" {
                // The Rust tindex implementation currently writes extent
                // polygons, matching PDAL's fast-boundary mode.
            } else if arg == "-f" || arg == "--ogrdriver" {
                let Some(d) = args_iter.next() else {
                    eprintln!("Error: {arg} requires an OGR driver name");
                    return 1;
                };
                driver_name = d.clone();
            } else if arg.starts_with('-') {
                eprintln!("Error: unknown tindex option '{arg}'");
                return 1;
            } else if tindex_file.is_none() {
                tindex_file = Some(arg.clone());
            } else {
                files.push(arg.clone());
            }
        }
        let Some(tindex_file) = tindex_file else {
            eprintln!("Error: tindex create requires --tindex <output>");
            return 1;
        };
        if files.is_empty() {
            eprintln!("Error: tindex create needs at least one input file");
            return 1;
        }

        // Register drivers
        pdal_core::gdal::register_drivers();

        // Create OGR dataset
        let dataset = match pdal_core::gdal::Vector::create(&tindex_file, &driver_name) {
            Ok(ds) => ds,
            Err(e) => {
                eprintln!("Error creating tindex dataset: {}", e);
                return 1;
            }
        };

        // For first file, we get its SRS to define the layer
        let mut first_srs = String::new();
        let mut valid_files = Vec::new();

        for file in files {
            let driver = match pdal_core::driver::infer_reader_driver(&file) {
                Some(driver) => driver,
                None => {
                    eprintln!("Error: unable to infer a reader driver for '{file}'");
                    return 1;
                }
            };

            let pipeline_json =
                serde_json::json!([{ "type": driver, "filename": file }]).to_string();
            let c_json = match CString::new(pipeline_json) {
                Ok(json) => json,
                Err(_) => {
                    eprintln!("Error: input path '{file}' contains an interior NUL byte");
                    return 1;
                }
            };

            let pipeline = unsafe { pdal_capi::pdal_pipeline_create_json(c_json.as_ptr()) };
            if pipeline.is_null() {
                self.output_last_error();
                return 1;
            }

            let json_ptr = unsafe {
                pdal_capi::pdal_pipeline_execute_summary_json(pipeline, std::ptr::null_mut())
            };
            unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };

            if json_ptr.is_null() {
                self.output_last_error();
                return 1;
            }

            let summary_str = safe_cstr(json_ptr).unwrap_or_default();
            unsafe { pdal_capi::pdal_string_free(json_ptr) };

            let summary = match serde_json::from_str::<serde_json::Value>(&summary_str) {
                Ok(summary) => summary,
                Err(err) => {
                    eprintln!("Error: unable to parse pipeline summary for '{file}': {err}");
                    return 1;
                }
            };
            let Some(bounds) = summary.get("bounds_2d") else {
                eprintln!("Error: '{file}' produced no 2D bounds");
                return 1;
            };
            let Some(minx) = bounds["minx"].as_f64() else {
                eprintln!("Error: '{file}' produced invalid minx bounds");
                return 1;
            };
            let Some(maxx) = bounds["maxx"].as_f64() else {
                eprintln!("Error: '{file}' produced invalid maxx bounds");
                return 1;
            };
            let Some(miny) = bounds["miny"].as_f64() else {
                eprintln!("Error: '{file}' produced invalid miny bounds");
                return 1;
            };
            let Some(maxy) = bounds["maxy"].as_f64() else {
                eprintln!("Error: '{file}' produced invalid maxy bounds");
                return 1;
            };
            let wkt = summary["metadata"]["pipeline"]["stage_0"]["srs"]["wkt"]
                .as_str()
                .unwrap_or("")
                .to_string();

            if first_srs.is_empty() && !wkt.is_empty() {
                first_srs = wkt.clone();
            }
            let mut location = if write_absolute_path {
                match Path::new(&file).canonicalize() {
                    Ok(path) => path.to_string_lossy().into_owned(),
                    Err(err) => {
                        eprintln!("Error: unable to resolve absolute path for '{file}': {err}");
                        return 1;
                    }
                }
            } else {
                file.clone()
            };
            if let Some(prefix) = &path_prefix {
                location = format!("{prefix}{location}");
            }
            valid_files.push((location, wkt, minx, miny, maxx, maxy));
        }

        if valid_files.is_empty() {
            eprintln!("Error: no valid files to index");
            return 1;
        }

        let layer = match dataset.open_or_create_layer(&layer_name, &first_srs) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error creating layer: {}", e);
                return 1;
            }
        };

        unsafe {
            for result in [
                pdal_core::gdal::Vector::create_string_field(layer, &location_field),
                pdal_core::gdal::Vector::create_string_field(layer, "srs"),
                pdal_core::gdal::Vector::create_datetime_field(layer, "created"),
                pdal_core::gdal::Vector::create_datetime_field(layer, "modified"),
            ] {
                if let Err(err) = result {
                    eprintln!("Error creating tindex field: {err}");
                    return 1;
                }
            }
        }

        for (file, wkt, minx, miny, maxx, maxy) in valid_files {
            // WKT for POLYGON
            let poly_wkt = format!(
                "POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))",
                minx, miny, maxx, miny, maxx, maxy, minx, maxy, minx, miny
            );

            let fields = vec![
                (location_field.as_str(), file.as_str()),
                ("srs", wkt.as_str()),
            ];

            unsafe {
                if let Err(e) = pdal_core::gdal::Vector::add_feature(layer, &poly_wkt, &fields) {
                    eprintln!("Error adding feature for {}: {}", file, e);
                    return 1;
                } else {
                    println!("Indexed file {}", file);
                }
            }
        }

        0
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
