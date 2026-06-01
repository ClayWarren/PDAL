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

        self.run_rust_kernel("random")
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
