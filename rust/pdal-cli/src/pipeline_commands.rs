use super::*;
use std::ffi::CString;

impl App {
    pub(super) fn run_pipeline(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal pipeline <pipeline.json>");
            println!("  pdal pipeline --input <pipeline.json>");
            println!("  pdal pipeline --stdin");
            println!("  pdal pipeline <pipeline.json> --metadata <metadata.json>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let mut args = self.command_args.clone();
        if self.show_json {
            args.push("--showjson".to_string());
        }
        self.run_rust_kernel_with_args("pipeline", &args)
    }

    pub(super) fn run_info(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal info [--summary|--metadata] <file>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        enum InfoOutput {
            Summary,
            Metadata,
        }

        let mut filename: Option<&str> = None;
        let mut driver_override: Option<&str> = None;
        let mut output = InfoOutput::Summary;
        let mut args = self.command_args.iter();
        while let Some(arg) = args.next() {
            if arg == "--summary" {
                output = InfoOutput::Summary;
            } else if arg == "--metadata" {
                output = InfoOutput::Metadata;
            } else if arg == "--schema"
                || arg == "--all"
                || arg == "--stac"
                || arg == "--boundary"
                || arg == "--pipeline-serialization"
                || arg == "--dimensions"
                || arg == "--enumerate"
                || arg == "--breakout"
                || arg == "--pc_type"
                || arg == "--stdin"
            {
                eprintln!("Error: unsupported option '{arg}' for info");
                return 1;
            } else if arg == "-p" || arg == "--point" || arg == "--query" {
                let Some(_) = args.next() else {
                    eprintln!("Error: {arg} requires a value");
                    return 1;
                };
                eprintln!("Error: unsupported option '{arg}' for info");
                return 1;
            } else if arg == "--stats" {
                continue;
            } else if arg == "--driver" {
                let Some(driver) = args.next() else {
                    eprintln!("Error: --driver requires a reader driver name");
                    return 1;
                };
                driver_override = Some(driver);
            } else if let Some(driver) = arg.strip_prefix("--driver=") {
                driver_override = Some(driver);
            } else if arg == "--input" || arg == "-i" {
                let Some(input) = args.next() else {
                    eprintln!("Error: {arg} requires an input filename");
                    return 1;
                };
                if filename.replace(input).is_some() {
                    eprintln!("Error: info expects exactly one filename");
                    return 1;
                }
            } else if arg.starts_with("--") {
                eprintln!("Error: unknown option '{arg}' for info");
                return 1;
            } else if filename.replace(arg).is_some() {
                eprintln!("Error: info expects exactly one filename");
                return 1;
            }
        }
        let Some(filename) = filename else {
            eprintln!("Error: info expects exactly one filename");
            return 1;
        };

        // Resolve the reader driver from the filename, as `pdal info` does.
        let driver = match driver_override
            .map(|driver| driver.to_string())
            .or_else(|| pdal_core::driver::infer_reader_driver(filename).map(str::to_string))
        {
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
                let output_value = match output {
                    InfoOutput::Summary => value,
                    InfoOutput::Metadata => serde_json::json!({
                        "metadata": value.get("metadata").cloned().unwrap_or_default(),
                    }),
                };
                serde_json::to_string(&output_value).unwrap_or(summary)
            }
            Err(_) => summary,
        };
        println!("{}", report);
        0
    }

    pub(super) fn run_translate(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
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

        self.run_rust_kernel("translate")
    }

    pub(super) fn run_merge(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal merge <input> [input ...] <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        self.run_rust_kernel("merge")
    }

    pub(super) fn run_sort(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal sort <input> <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        self.run_rust_kernel("sort")
    }

    pub(super) fn run_ground(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal ground <input> <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        self.run_rust_kernel("ground")
    }

    pub(super) fn run_density(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal density <input> <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        self.run_rust_kernel("density")
    }
}
