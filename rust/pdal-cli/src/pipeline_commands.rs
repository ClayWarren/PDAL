use super::*;
use std::ffi::CString;
use std::io::Read;

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

        let mut input: Option<&str> = None;
        let mut read_stdin = false;
        let mut validate_only = false;
        let mut metadata_file: Option<&str> = None;
        let mut serialization_file: Option<&str> = None;
        let mut stream_allowed = true;
        let mut stream_required = false;
        let mut args = self.command_args.iter();
        while let Some(arg) = args.next() {
            if arg == "--input" || arg == "-i" {
                let Some(value) = args.next() else {
                    eprintln!("Error: {arg} requires a pipeline filename");
                    return 1;
                };
                input = Some(value);
            } else if arg == "--stdin" || arg == "-s" {
                read_stdin = true;
            } else if arg == "--validate" {
                validate_only = true;
            } else if arg == "--stream" {
                if !stream_allowed {
                    eprintln!("Error: can't execute with --stream and --nostream");
                    return 1;
                }
                stream_allowed = true;
                stream_required = true;
            } else if arg == "--nostream" {
                if stream_required {
                    eprintln!("Error: can't execute with --stream and --nostream");
                    return 1;
                }
                stream_allowed = false;
            } else if arg == "--metadata" {
                let Some(value) = args.next() else {
                    eprintln!("Error: --metadata requires an output filename");
                    return 1;
                };
                metadata_file = Some(value);
            } else if arg == "--pipeline-serialization" {
                let Some(value) = args.next() else {
                    eprintln!("Error: --pipeline-serialization requires an output filename");
                    return 1;
                };
                serialization_file = Some(value);
            } else if arg.starts_with("--") {
                eprintln!("Error: unknown option '{arg}' for pipeline");
                return 1;
            } else if input.replace(arg).is_some() {
                eprintln!("Error: pipeline expects exactly one JSON filename");
                return 1;
            }
        }
        if read_stdin && input.is_some() {
            eprintln!("Error: pipeline accepts either --stdin or an input filename, not both");
            return 1;
        }
        if !read_stdin && input.is_none() {
            eprintln!("Error: pipeline expects exactly one JSON filename");
            return 1;
        }

        let json = if read_stdin {
            let mut json = String::new();
            if let Err(err) = std::io::stdin().read_to_string(&mut json) {
                eprintln!("Error: unable to read pipeline from stdin: {err}");
                return 1;
            }
            json
        } else {
            let input = input.unwrap();
            match pdal_io::source::read_to_string(input) {
                Ok(json) => json,
                Err(err) => {
                    eprintln!("Error: unable to read pipeline '{input}': {err}");
                    return 1;
                }
            }
        };
        if let Some(path) = serialization_file {
            if let Err(err) = std::fs::write(path, &json) {
                eprintln!("Error: unable to write pipeline serialization '{path}': {err}");
                return 1;
            }
        }
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
        if validate_only {
            let validation = serde_json::json!({
                "valid": true,
                "error_detail": "",
                "streamable": unsafe { pdal_capi::pdal_pipeline_streamable(pipeline) },
            });
            println!("{}", serde_json::to_string_pretty(&validation).unwrap());
            unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
            return 0;
        }

        if self.show_json || metadata_file.is_some() {
            let json_ptr = unsafe {
                pdal_capi::pdal_pipeline_execute_summary_json(pipeline, std::ptr::null_mut())
            };
            if json_ptr.is_null() {
                self.output_last_error();
                unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
                return 1;
            }
            let summary = safe_cstr(json_ptr).unwrap_or_default();
            if let Some(path) = metadata_file {
                if let Err(err) = std::fs::write(path, &summary) {
                    eprintln!("Error: unable to write metadata '{path}': {err}");
                    unsafe {
                        pdal_capi::pdal_string_free(json_ptr);
                        pdal_capi::pdal_pipeline_destroy(pipeline);
                    }
                    return 1;
                }
            }
            if self.show_json {
                println!("{}", summary);
            }
            unsafe {
                pdal_capi::pdal_string_free(json_ptr);
                pdal_capi::pdal_pipeline_destroy(pipeline);
            }
            return 0;
        }

        // Try chunked streaming first (bounded memory). -2 means the pipeline is
        // not streaming-eligible, so fall through to the materializing path; the
        // streaming attempt has no side effects in that case.
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
        if status < 0 {
            self.output_last_error();
            unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
            return 1;
        }
        unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
        0
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
