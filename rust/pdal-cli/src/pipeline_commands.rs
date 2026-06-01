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

        let mut input: Option<&str> = None;
        let mut output: Option<&str> = None;
        let mut reader_override: Option<&str> = None;
        let mut writer_override: Option<&str> = None;
        let mut filters: Vec<&str> = Vec::new();
        let mut stage_options: Vec<StageOption> = Vec::new();

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
                    eprintln!("Error: {arg} requires an output path");
                    return 1;
                };
                output = Some(value);
            } else if arg == "--reader" || arg == "-r" || arg == "--driver" {
                let Some(value) = args.next() else {
                    eprintln!("Error: {arg} requires a reader driver name");
                    return 1;
                };
                reader_override = Some(value);
            } else if arg == "--writer" || arg == "-w" {
                let Some(value) = args.next() else {
                    eprintln!("Error: {arg} requires a writer driver name");
                    return 1;
                };
                writer_override = Some(value);
            } else if arg == "--filter" || arg == "-f" {
                let Some(value) = args.next() else {
                    eprintln!("Error: {arg} requires a filter name");
                    return 1;
                };
                filters.push(value);
            } else if arg.starts_with("--") {
                match parse_stage_option_arg(arg) {
                    Ok(option) => stage_options.push(option),
                    Err(message) => {
                        eprintln!("Error: {message}");
                        return 1;
                    }
                }
            } else if input.is_none() {
                input = Some(arg);
            } else if output.is_none() {
                output = Some(arg);
            } else {
                filters.push(arg);
            }
        }

        let Some(input) = input else {
            eprintln!("Error: translate needs an input path and an output path");
            return 1;
        };
        let Some(output) = output else {
            eprintln!("Error: translate needs an input path and an output path");
            return 1;
        };

        let reader = match reader_override
            .map(str::to_string)
            .or_else(|| pdal_core::driver::infer_reader_driver(input).map(str::to_string))
        {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a reader driver for '{input}'");
                return 1;
            }
        };
        let writer = match writer_override
            .map(str::to_string)
            .or_else(|| pdal_core::driver::infer_writer_driver(output).map(str::to_string))
        {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a writer driver for '{output}'");
                return 1;
            }
        };

        // Assemble reader -> filters -> writer pipeline stages.
        let mut stages: Vec<serde_json::Value> = Vec::new();
        stages.push(serde_json::json!({ "type": reader, "filename": input }));
        for name in &filters {
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
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal merge <input> [input ...] <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let mut positional: Vec<&str> = Vec::new();
        let mut stage_options: Vec<StageOption> = Vec::new();
        let mut driver_override: Option<&str> = None;
        let mut args = self.command_args.iter();
        while let Some(arg) = args.next() {
            if arg == "--driver" {
                let Some(driver) = args.next() else {
                    eprintln!("Error: --driver requires a reader driver name");
                    return 1;
                };
                driver_override = Some(driver);
            } else if let Some(driver) = arg.strip_prefix("--driver=") {
                driver_override = Some(driver);
            } else if arg == "--files" || arg == "-f" {
                let Some(path) = args.next() else {
                    eprintln!("Error: {arg} requires a file path");
                    return 1;
                };
                positional.push(path);
            } else if arg.starts_with("--") {
                match parse_stage_option_arg(arg) {
                    Ok(option) => stage_options.push(option),
                    Err(message) => {
                        eprintln!("Error: {message}");
                        return 1;
                    }
                }
            } else {
                positional.push(arg);
            }
        }
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
            let reader = match driver_override
                .map(str::to_string)
                .or_else(|| pdal_core::driver::infer_reader_driver(input).map(str::to_string))
            {
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
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal sort <input> <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let mut input: Option<&str> = None;
        let mut output: Option<&str> = None;
        let mut driver_override: Option<&str> = None;
        let mut stage_options: Vec<StageOption> = Vec::new();
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
                    eprintln!("Error: {arg} requires an output path");
                    return 1;
                };
                output = Some(value);
            } else if arg == "--driver" {
                let Some(driver) = args.next() else {
                    eprintln!("Error: --driver requires a reader driver name");
                    return 1;
                };
                driver_override = Some(driver);
            } else if let Some(driver) = arg.strip_prefix("--driver=") {
                driver_override = Some(driver);
            } else if arg.starts_with("--") {
                match parse_stage_option_arg(arg) {
                    Ok(option) => stage_options.push(option),
                    Err(message) => {
                        eprintln!("Error: {message}");
                        return 1;
                    }
                }
            } else if input.is_none() {
                input = Some(arg);
            } else if output.is_none() {
                output = Some(arg);
            } else {
                eprintln!("Error: sort expects an input path and an output path");
                return 1;
            }
        }
        let Some(input) = input else {
            eprintln!("Error: sort expects an input path and an output path");
            return 1;
        };
        let Some(output) = output else {
            eprintln!("Error: sort expects an input path and an output path");
            return 1;
        };

        let reader = match driver_override
            .map(str::to_string)
            .or_else(|| pdal_core::driver::infer_reader_driver(input).map(str::to_string))
        {
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
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal ground <input> <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let mut input: Option<&str> = None;
        let mut output: Option<&str> = None;
        let mut driver_override: Option<&str> = None;
        let mut stage_options: Vec<StageOption> = Vec::new();
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
                    eprintln!("Error: {arg} requires an output path");
                    return 1;
                };
                output = Some(value);
            } else if arg == "--driver" {
                let Some(driver) = args.next() else {
                    eprintln!("Error: --driver requires a reader driver name");
                    return 1;
                };
                driver_override = Some(driver);
            } else if let Some(driver) = arg.strip_prefix("--driver=") {
                driver_override = Some(driver);
            } else if arg.starts_with("--") {
                match parse_stage_option_arg(arg) {
                    Ok(option) => stage_options.push(option),
                    Err(message) => {
                        eprintln!("Error: {message}");
                        return 1;
                    }
                }
            } else if input.is_none() {
                input = Some(arg);
            } else if output.is_none() {
                output = Some(arg);
            } else {
                eprintln!("Error: ground expects an input path and an output path");
                return 1;
            }
        }
        let Some(input) = input else {
            eprintln!("Error: ground expects an input path and an output path");
            return 1;
        };
        let Some(output) = output else {
            eprintln!("Error: ground expects an input path and an output path");
            return 1;
        };

        let reader = match driver_override
            .map(str::to_string)
            .or_else(|| pdal_core::driver::infer_reader_driver(input).map(str::to_string))
        {
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
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal density <input> <output> [--<stage>.<key>=<value> ...]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let mut input: Option<&str> = None;
        let mut output: Option<&str> = None;
        let mut driver_override: Option<&str> = None;
        let mut stage_options: Vec<StageOption> = Vec::new();
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
                    eprintln!("Error: {arg} requires an output path");
                    return 1;
                };
                output = Some(value);
            } else if arg == "--driver" {
                let Some(driver) = args.next() else {
                    eprintln!("Error: --driver requires a reader driver name");
                    return 1;
                };
                driver_override = Some(driver);
            } else if let Some(driver) = arg.strip_prefix("--driver=") {
                driver_override = Some(driver);
            } else if arg == "--ogrdriver" || arg == "-f" {
                let Some(value) = args.next() else {
                    eprintln!("Error: {arg} requires an OGR driver name");
                    return 1;
                };
                stage_options.push(StageOption {
                    stage: "filters.hexbin".to_string(),
                    key: "ogrdriver".to_string(),
                    value: value.clone(),
                });
            } else if let Some(value) = arg.strip_prefix("--ogrdriver=") {
                stage_options.push(StageOption {
                    stage: "filters.hexbin".to_string(),
                    key: "ogrdriver".to_string(),
                    value: value.to_string(),
                });
            } else if arg == "--lyr_name" {
                let Some(value) = args.next() else {
                    eprintln!("Error: {arg} requires a layer name");
                    return 1;
                };
                stage_options.push(StageOption {
                    stage: "filters.hexbin".to_string(),
                    key: "lyr_name".to_string(),
                    value: value.clone(),
                });
            } else if let Some(value) = arg.strip_prefix("--lyr_name=") {
                stage_options.push(StageOption {
                    stage: "filters.hexbin".to_string(),
                    key: "lyr_name".to_string(),
                    value: value.to_string(),
                });
            } else if arg == "--edge_length" || arg == "--threshold" {
                let Some(value) = args.next() else {
                    eprintln!("Error: {arg} requires a value");
                    return 1;
                };
                stage_options.push(StageOption {
                    stage: "filters.hexbin".to_string(),
                    key: arg.trim_start_matches("--").to_string(),
                    value: value.clone(),
                });
            } else if let Some(value) = arg.strip_prefix("--edge_length=") {
                stage_options.push(StageOption {
                    stage: "filters.hexbin".to_string(),
                    key: "edge_length".to_string(),
                    value: value.to_string(),
                });
            } else if let Some(value) = arg.strip_prefix("--threshold=") {
                stage_options.push(StageOption {
                    stage: "filters.hexbin".to_string(),
                    key: "threshold".to_string(),
                    value: value.to_string(),
                });
            } else if arg.starts_with("--") {
                match parse_stage_option_arg(arg) {
                    Ok(option) => stage_options.push(option),
                    Err(message) => {
                        eprintln!("Error: {message}");
                        return 1;
                    }
                }
            } else if input.is_none() {
                input = Some(arg);
            } else if output.is_none() {
                output = Some(arg);
            } else {
                eprintln!("Error: density expects an input path and an output path");
                return 1;
            }
        }
        let Some(input) = input else {
            eprintln!("Error: density expects an input path and an output path");
            return 1;
        };
        let Some(output) = output else {
            eprintln!("Error: density expects an input path and an output path");
            return 1;
        };

        let reader = match driver_override
            .map(str::to_string)
            .or_else(|| pdal_core::driver::infer_reader_driver(input).map(str::to_string))
        {
            Some(driver) => driver,
            None => {
                eprintln!("Error: unable to infer a reader driver for '{input}'");
                return 1;
            }
        };

        // reader -> filters.hexbin: the hexbin filter tessellates the X/Y
        // domain and writes the dense-cell density grid as a vector file, so
        // no point-cloud writer is added; `--filters.hexbin.*` options
        // override the hexbin defaults.
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
}
