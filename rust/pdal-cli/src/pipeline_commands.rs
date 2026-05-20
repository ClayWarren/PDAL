use super::*;
use std::ffi::CString;

impl App {
    pub(super) fn run_pipeline(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal pipeline <pipeline.json>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        if self.command_args.len() != 1 {
            eprintln!("Error: pipeline expects exactly one JSON filename");
            return 1;
        }

        let json = match std::fs::read_to_string(&self.command_args[0]) {
            Ok(json) => json,
            Err(err) => {
                eprintln!(
                    "Error: unable to read pipeline '{}': {}",
                    self.command_args[0], err
                );
                return 1;
            }
        };
        let c_json = match CString::new(json) {
            Ok(json) => json,
            Err(_) => {
                eprintln!("Error: pipeline JSON contains an interior NUL byte");
                return 1;
            }
        };

        let pipeline = unsafe { pdal_capi::pdal_pipeline_create_json(c_json.as_ptr()) };
        if pipeline.is_null() {
            self.output_last_error();
            return 1;
        }

        if self.show_json {
            let json_ptr = unsafe {
                pdal_capi::pdal_pipeline_execute_summary_json(pipeline, std::ptr::null_mut())
            };
            if json_ptr.is_null() {
                self.output_last_error();
                unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
                return 1;
            }
            if let Some(json) = safe_cstr(json_ptr) {
                println!("{}", json);
            }
            unsafe {
                pdal_capi::pdal_string_free(json_ptr);
                pdal_capi::pdal_pipeline_destroy(pipeline);
            }
            return 0;
        }

        let mut result = empty_pipeline_result();
        let status = unsafe {
            pdal_capi::pdal_pipeline_execute_result(pipeline, std::ptr::null_mut(), &mut result)
        };
        if status < 0 {
            self.output_last_error();
            unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
            return 1;
        }
        unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
        0
    }

    pub(super) fn run_info(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal info [--summary] <file>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        let mut filename: Option<&str> = None;
        for arg in &self.command_args {
            if arg == "--summary" {
                continue;
            }
            if arg.starts_with("--") {
                eprintln!("Error: unknown option '{arg}' for info");
                return 1;
            }
            if filename.replace(arg).is_some() {
                eprintln!("Error: info expects exactly one filename");
                return 1;
            }
        }
        let Some(filename) = filename else {
            eprintln!("Error: info expects exactly one filename");
            return 1;
        };

        // Resolve the reader driver from the filename, as `pdal info` does.
        let driver = match pdal_core::driver::infer_reader_driver(filename) {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a reader driver for '{}'", filename);
                return 1;
            }
        };

        // `info` reads the file through a reader-only pipeline and reports the
        // execution summary: point count, bounds, per-dimension stats, and
        // metadata.
        let pipeline_json =
            serde_json::json!([{ "type": driver, "filename": filename }]).to_string();
        let c_json = match CString::new(pipeline_json) {
            Ok(json) => json,
            Err(_) => {
                eprintln!("Error: filename contains an interior NUL byte");
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
        let summary = safe_cstr(json_ptr);
        unsafe {
            if !json_ptr.is_null() {
                pdal_capi::pdal_string_free(json_ptr);
            }
            pdal_capi::pdal_pipeline_destroy(pipeline);
        }

        let Some(summary) = summary.filter(|text| !text.is_empty()) else {
            self.output_last_error();
            return 1;
        };

        // Tag the execution summary with the filename and resolved driver.
        let report = match serde_json::from_str::<serde_json::Value>(&summary) {
            Ok(mut value) => {
                if let Some(object) = value.as_object_mut() {
                    object.insert("filename".to_string(), serde_json::json!(filename));
                    object.insert("driver".to_string(), serde_json::json!(driver));
                }
                serde_json::to_string(&value).unwrap_or(summary)
            }
            Err(_) => summary,
        };
        println!("{}", report);
        0
    }

    pub(super) fn run_translate(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!(
                "  pdal translate <input> <output> [filter ...] [--<stage>.<key>=<value> ...]"
            );
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let (positional, stage_options) = match parse_stage_args(&self.command_args) {
            Ok(parsed) => parsed,
            Err(message) => {
                eprintln!("Error: {message}");
                return 1;
            }
        };
        if positional.len() < 2 {
            eprintln!("Error: translate needs an input path and an output path");
            return 1;
        }
        // PDAL `translate` positional order is: input, output, then filters.
        let input = positional[0];
        let output = positional[1];
        let filters = &positional[2..];

        let reader = match pdal_core::driver::infer_reader_driver(input) {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a reader driver for '{input}'");
                return 1;
            }
        };
        let writer = match pdal_core::driver::infer_writer_driver(output) {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a writer driver for '{output}'");
                return 1;
            }
        };

        // Assemble reader -> filters -> writer pipeline stages.
        let mut stages: Vec<serde_json::Value> = Vec::new();
        stages.push(serde_json::json!({ "type": reader, "filename": input }));
        for name in filters {
            let stage_type = if name.contains('.') {
                (*name).to_string()
            } else {
                format!("filters.{name}")
            };
            stages.push(serde_json::json!({ "type": stage_type }));
        }
        stages.push(serde_json::json!({ "type": writer, "filename": output }));

        if let Err(message) = apply_stage_options(&mut stages, &stage_options) {
            eprintln!("Error: {message}");
            return 1;
        }
        self.execute_stage_pipeline(stages)
    }

    pub(super) fn run_merge(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal merge <input> [input ...] <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let (positional, stage_options) = match parse_stage_args(&self.command_args) {
            Ok(parsed) => parsed,
            Err(message) => {
                eprintln!("Error: {message}");
                return 1;
            }
        };
        if positional.len() < 2 {
            eprintln!("Error: merge needs at least one input path and an output path");
            return 1;
        }
        // PDAL `merge` positional order is: inputs..., then the output.
        let output = positional[positional.len() - 1];
        let inputs = &positional[..positional.len() - 1];

        // Each input is a tagged reader; a single `filters.merge` joins them.
        let mut stages: Vec<serde_json::Value> = Vec::new();
        let mut tags: Vec<String> = Vec::new();
        for (index, input) in inputs.iter().enumerate() {
            let reader = match pdal_core::driver::infer_reader_driver(input) {
                Some(driver) => driver,
                None => {
                    eprintln!("Error: unable to infer a reader driver for '{input}'");
                    return 1;
                }
            };
            let tag = format!("merge_input_{index}");
            stages.push(serde_json::json!({
                "type": reader,
                "filename": input,
                "tag": tag,
            }));
            tags.push(tag);
        }
        stages.push(serde_json::json!({ "type": "filters.merge", "inputs": tags }));
        let writer = match pdal_core::driver::infer_writer_driver(output) {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a writer driver for '{output}'");
                return 1;
            }
        };
        stages.push(serde_json::json!({ "type": writer, "filename": output }));

        if let Err(message) = apply_stage_options(&mut stages, &stage_options) {
            eprintln!("Error: {message}");
            return 1;
        }
        self.execute_stage_pipeline(stages)
    }

    pub(super) fn run_sort(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal sort <input> <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let (positional, stage_options) = match parse_stage_args(&self.command_args) {
            Ok(parsed) => parsed,
            Err(message) => {
                eprintln!("Error: {message}");
                return 1;
            }
        };
        if positional.len() != 2 {
            eprintln!("Error: sort expects an input path and an output path");
            return 1;
        }
        let input = positional[0];
        let output = positional[1];

        let reader = match pdal_core::driver::infer_reader_driver(input) {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a reader driver for '{input}'");
                return 1;
            }
        };
        let writer = match pdal_core::driver::infer_writer_driver(output) {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a writer driver for '{output}'");
                return 1;
            }
        };

        // reader -> filters.sort -> writer, sorting by X unless overridden by
        // a `--filters.sort.*` option.
        let mut stages: Vec<serde_json::Value> = vec![
            serde_json::json!({ "type": reader, "filename": input }),
            serde_json::json!({ "type": "filters.sort", "dimensions": "X" }),
            serde_json::json!({ "type": writer, "filename": output }),
        ];

        if let Err(message) = apply_stage_options(&mut stages, &stage_options) {
            eprintln!("Error: {message}");
            return 1;
        }
        self.execute_stage_pipeline(stages)
    }

    pub(super) fn run_ground(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal ground <input> <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let (positional, stage_options) = match parse_stage_args(&self.command_args) {
            Ok(parsed) => parsed,
            Err(message) => {
                eprintln!("Error: {message}");
                return 1;
            }
        };
        if positional.len() != 2 {
            eprintln!("Error: ground expects an input path and an output path");
            return 1;
        }
        let input = positional[0];
        let output = positional[1];

        let reader = match pdal_core::driver::infer_reader_driver(input) {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a reader driver for '{input}'");
                return 1;
            }
        };
        let writer = match pdal_core::driver::infer_writer_driver(output) {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a writer driver for '{output}'");
                return 1;
            }
        };

        // reader -> filters.smrf -> writer, classifying ground points;
        // `--filters.smrf.*` options override the SMRF defaults.
        let mut stages: Vec<serde_json::Value> = vec![
            serde_json::json!({ "type": reader, "filename": input }),
            serde_json::json!({ "type": "filters.smrf" }),
            serde_json::json!({ "type": writer, "filename": output }),
        ];

        if let Err(message) = apply_stage_options(&mut stages, &stage_options) {
            eprintln!("Error: {message}");
            return 1;
        }
        self.execute_stage_pipeline(stages)
    }

    pub(super) fn run_density(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal density <input> <output.geojson> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let (positional, stage_options) = match parse_stage_args(&self.command_args) {
            Ok(parsed) => parsed,
            Err(message) => {
                eprintln!("Error: {message}");
                return 1;
            }
        };
        if positional.len() != 2 {
            eprintln!("Error: density expects an input path and an output path");
            return 1;
        }
        let input = positional[0];
        let output = positional[1];

        let reader = match pdal_core::driver::infer_reader_driver(input) {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a reader driver for '{input}'");
                return 1;
            }
        };

        // reader -> filters.hexbin: the hexbin filter tessellates the X/Y
        // domain and writes the dense-cell density grid as GeoJSON. The
        // density output is a vector file, so no point-cloud writer is added;
        // `--filters.hexbin.*` options override the hexbin defaults.
        let mut stages: Vec<serde_json::Value> = vec![
            serde_json::json!({ "type": reader, "filename": input }),
            serde_json::json!({ "type": "filters.hexbin", "density": output }),
        ];

        if let Err(message) = apply_stage_options(&mut stages, &stage_options) {
            eprintln!("Error: {message}");
            return 1;
        }
        self.execute_stage_pipeline(stages)
    }

    pub(super) fn run_random(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal random <output> [--count=N]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let mut output: Option<&str> = None;
        let mut count: u64 = 1000;
        for arg in &self.command_args {
            if let Some(value) = arg.strip_prefix("--count=") {
                match value.parse::<u64>() {
                    Ok(parsed) => count = parsed,
                    Err(_) => {
                        eprintln!("Error: --count must be a non-negative integer");
                        return 1;
                    }
                }
            } else if arg.starts_with("--") {
                eprintln!("Error: unknown option '{arg}' for random");
                return 1;
            } else if output.is_none() {
                output = Some(arg.as_str());
            } else {
                eprintln!("Error: random expects a single output path");
                return 1;
            }
        }
        let Some(output) = output else {
            eprintln!("Error: random needs an output path");
            return 1;
        };

        let writer = match pdal_core::driver::infer_writer_driver(output) {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a writer driver for '{output}'");
                return 1;
            }
        };

        // Uniformly random points in the unit cube, via the faux reader.
        let stages: Vec<serde_json::Value> = vec![
            serde_json::json!({
                "type": "readers.faux",
                "count": count,
                "mode": "uniform",
                "minx": 0.0,
                "maxx": 1.0,
                "miny": 0.0,
                "maxy": 1.0,
                "minz": 0.0,
                "maxz": 1.0,
            }),
            serde_json::json!({ "type": writer, "filename": output }),
        ];
        self.execute_stage_pipeline(stages)
    }

    pub(super) fn run_split(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal split <input> <output> [--length=N | --capacity=N] [--origin_x=X] [--origin_y=Y]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let split = match SplitArgs::parse(&self.command_args) {
            Ok(split) => split,
            Err(message) => {
                eprintln!("Error: {message}");
                return 1;
            }
        };

        let reader = match pdal_core::driver::infer_reader_driver(split.input) {
            Some(driver) => driver,
            None => {
                eprintln!(
                    "Error: unable to infer a reader driver for '{}'",
                    split.input
                );
                return 1;
            }
        };
        let output_name = split.output.to_string_lossy();
        let writer = match pdal_core::driver::infer_writer_driver(&output_name) {
            Some(driver) => driver,
            None => {
                eprintln!(
                    "Error: unable to infer a writer driver for '{}'",
                    split.output.display()
                );
                return 1;
            }
        };

        let filter = if let Some(length) = split.length {
            let mut filter = serde_json::json!({
                "type": "filters.splitter",
                "length": length,
            });
            if let Some(origin_x) = split.origin_x {
                filter["origin_x"] = serde_json::json!(origin_x);
            }
            if let Some(origin_y) = split.origin_y {
                filter["origin_y"] = serde_json::json!(origin_y);
            }
            filter
        } else {
            serde_json::json!({
                "type": "filters.chipper",
                "capacity": split.capacity.unwrap_or(100000),
            })
        };

        let stages = serde_json::json!([
            { "type": reader, "filename": split.input },
            filter
        ]);
        let mut pipeline = match pdal_capi::pipeline_from_json(&stages.to_string()) {
            Ok(pipeline) => pipeline,
            Err(err) => {
                eprintln!("Error: {err}");
                return 1;
            }
        };
        let views = match pipeline.execute(Vec::new()) {
            Ok(views) => views,
            Err(err) => {
                eprintln!("Error: {err}");
                return 1;
            }
        };

        for (index, view) in views.iter().enumerate() {
            let filename = numbered_output(&split.output, index + 1);
            let mut options = pdal_core::options::Options::new();
            options.add("filename", filename.display());
            let mut output_writer = match pdal_capi::create_writer(writer, &options) {
                Ok(writer) => writer,
                Err(err) => {
                    eprintln!("Error: {err}");
                    return 1;
                }
            };
            if let Err(err) = output_writer.write(std::slice::from_ref(view)) {
                eprintln!("Error: {err}");
                return 1;
            }
        }

        0
    }
}
