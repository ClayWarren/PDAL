use super::*;

impl App {
    pub(super) fn run_random(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
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
        let mut stage_options: Vec<StageOption> = Vec::new();
        let mut args = self.command_args.iter();
        while let Some(arg) = args.next() {
            if let Some(value) = arg.strip_prefix("--count=") {
                match value.parse::<u64>() {
                    Ok(parsed) => count = parsed,
                    Err(_) => {
                        eprintln!("Error: --count must be a non-negative integer");
                        return 1;
                    }
                }
            } else if arg == "--count" {
                let Some(value) = args.next() else {
                    eprintln!("Error: --count requires a point count");
                    return 1;
                };
                match value.parse::<u64>() {
                    Ok(parsed) => count = parsed,
                    Err(_) => {
                        eprintln!("Error: --count must be a non-negative integer");
                        return 1;
                    }
                }
            } else if arg == "--output" || arg == "-o" {
                let Some(path) = args.next() else {
                    eprintln!("Error: {arg} requires an output path");
                    return 1;
                };
                if output.replace(path).is_some() {
                    eprintln!("Error: random expects a single output path");
                    return 1;
                }
            } else if arg.starts_with("--") {
                match parse_stage_option_arg(arg) {
                    Ok(option) => stage_options.push(option),
                    Err(message) => {
                        eprintln!("Error: {message}");
                        return 1;
                    }
                }
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
        let mut stages: Vec<serde_json::Value> = vec![
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
        if let Err(message) = apply_stage_options(&mut stages, &stage_options) {
            eprintln!("Error: {message}");
            return 1;
        }
        self.execute_stage_pipeline(stages)
    }

    pub(super) fn run_split(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
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

        let reader = match split
            .reader_driver
            .map(str::to_string)
            .or_else(|| pdal_core::driver::infer_reader_driver(split.input).map(str::to_string))
        {
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
