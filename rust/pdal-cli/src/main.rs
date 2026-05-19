use std::ffi::{CStr, CString};
use std::process;

use pdal_kernels::word_wrap;

fn safe_cstr(ptr: *const std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
    }
}

struct App {
    command: String,
    command_args: Vec<String>,
    help: bool,
    show_version: bool,
    show_drivers: bool,
    show_commands: bool,
    show_options: Option<String>,
    show_json: bool,
    verbose: u8,
    log: String,
    log_timing: bool,
}

impl App {
    fn new() -> Self {
        Self {
            command: String::new(),
            command_args: Vec::new(),
            help: false,
            show_version: false,
            show_drivers: false,
            show_commands: false,
            show_options: None,
            show_json: false,
            verbose: 0,
            log: String::from("stderr"),
            log_timing: false,
        }
    }

    fn parse_args(&mut self, args: &[String]) -> Result<(), String> {
        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            if !self.command.is_empty() {
                self.command_args.push(arg.clone());
                i += 1;
                continue;
            }
            match arg.as_str() {
                "--help" | "-h" => self.help = true,
                "--version" => self.show_version = true,
                "--drivers" => self.show_drivers = true,
                "--list-commands" => self.show_commands = true,
                "--options" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--options requires a stage name argument".to_string());
                    }
                    self.show_options = Some(args[i].clone());
                }
                "--showjson" => self.show_json = true,
                "--verbose" | "-v" => {
                    self.verbose += 1;
                }
                "--log" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--log requires a filename argument".to_string());
                    }
                    self.log = args[i].clone();
                }
                "--logtiming" => self.log_timing = true,
                _ if !arg.starts_with('-') && self.command.is_empty() => {
                    self.command = arg.clone();
                }
                _ => {
                    return Err(format!("Unexpected argument '{}'", arg));
                }
            }
            i += 1;
        }
        Ok(())
    }

    fn output_version(&self) {
        let headline = "-".repeat(80);
        println!("{}", headline);
        println!("pdal-rs {}", env!("CARGO_PKG_VERSION"));
        println!("{}", headline);
    }

    fn output_help(&self) {
        println!("Usage:");
        println!("  pdal <options>");
        println!("  pdal <command> <command options>");
        println!();
        println!("Options:");
        println!("  -h, --help          Display help text");
        println!("  --version           Show program version");
        println!("  --drivers           List available drivers");
        println!("  --list-commands     List available commands");
        println!("  --options <stage>   Show options for specified stage");
        println!("  -v, --verbose       Set output verbosity");
        println!("  --log <file>        Log filename");
        println!("  --logtiming         Turn on timing for log messages");
        println!("  --showjson          Output as JSON");
        println!();
        println!("The following commands are available:");
        self.output_commands("  - ");
        println!();
        println!("See https://pdal.org/apps/ for more detail");
    }

    fn output_drivers(&self) {
        let stages = stage_list();
        let json_str = serde_json::to_string(&stages).unwrap();

        if self.show_json {
            println!("{}", json_str);
            return;
        }

        let name_col = 28;
        let descrip_col = 80 - name_col - 1;
        let tablehead = format!("{} {}", "=".repeat(name_col), "=".repeat(descrip_col));

        println!();
        println!("{}", tablehead);
        println!("{:<name_col$} Description", "Name");
        println!("{}", tablehead);

        for stage in &stages {
            let name = stage.name;
            let descrip = stage.description;
            let lines = word_wrap(descrip, descrip_col - 1);
            for (i, line) in lines.iter().enumerate() {
                if i == 0 {
                    println!("{:<name_col$} {}", name, line);
                } else {
                    println!("{:<name_col$} {}", "", line);
                }
            }
        }

        println!("{}", tablehead);
        println!();
    }

    fn output_commands(&self, leader: &str) {
        let kernels = kernel_list();
        if self.show_json {
            println!("{}", serde_json::to_string(&kernels).unwrap());
        } else {
            for kernel in kernels {
                println!("{}{}", leader, kernel.name);
            }
        }
    }

    fn output_options(&self, stage_name: &str) {
        if !stage_list().iter().any(|stage| stage.name == stage_name) {
            eprintln!("Unable to create stage {}", stage_name);
            return;
        }
        let options = stage_options(stage_name);
        let json_str = serde_json::to_string(&options).unwrap();

        if self.show_json {
            println!("{}", json_str);
            return;
        }

        println!("{}", stage_name);
        println!("{}", "-".repeat(80));
        if options.is_empty() {
            println!("  No options");
            return;
        }
        for opt in &options {
            let name = opt["arg"].as_str().unwrap_or("");
            let desc = opt["description"].as_str().unwrap_or("");
            let default = opt.get("default").map(|value| {
                if let Some(text) = value.as_str() {
                    format!(" (default: '{}')", text)
                } else {
                    format!(" (default: {})", value)
                }
            });
            println!("  {:<20} {}{}", name, desc, default.unwrap_or_default());
        }
    }

    fn output_last_error(&self) {
        match safe_cstr(pdal_capi::pdal_last_error()) {
            Some(message) if !message.is_empty() => eprintln!("{}", message),
            _ => eprintln!("Rust pipeline execution failed"),
        }
    }

    fn run_pipeline(&self) -> i32 {
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

    fn run_info(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal info <file>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        if self.command_args.len() != 1 {
            eprintln!("Error: info expects exactly one filename");
            return 1;
        }
        let filename = &self.command_args[0];

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

    fn run_translate(&self) -> i32 {
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

    fn run_merge(&self) -> i32 {
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

    fn run_sort(&self) -> i32 {
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

    fn run_random(&self) -> i32 {
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

    fn run_hausdorff(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal hausdorff <source> <candidate>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        if self.command_args.len() != 2 {
            eprintln!("Error: hausdorff expects exactly two filenames");
            return 1;
        }
        let source = &self.command_args[0];
        let candidate = &self.command_args[1];
        let (c_source, c_candidate) = match (
            CString::new(source.as_str()),
            CString::new(candidate.as_str()),
        ) {
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

    fn run_chamfer(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal chamfer <source> <candidate>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        if self.command_args.len() != 2 {
            eprintln!("Error: chamfer expects exactly two filenames");
            return 1;
        }
        let source = &self.command_args[0];
        let candidate = &self.command_args[1];
        let (c_source, c_candidate) = match (
            CString::new(source.as_str()),
            CString::new(candidate.as_str()),
        ) {
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

    fn run_delta(&self) -> i32 {
        if self.help || self.command_args.is_empty() {
            println!("Usage:");
            println!("  pdal delta <source> <candidate>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        if self.command_args.len() != 2 {
            eprintln!("Error: delta expects exactly two filenames");
            return 1;
        }
        let source = &self.command_args[0];
        let candidate = &self.command_args[1];
        let (c_source, c_candidate) = match (
            CString::new(source.as_str()),
            CString::new(candidate.as_str()),
        ) {
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

    /// Build a pipeline from assembled stage objects and execute it.
    fn execute_stage_pipeline(&self, stages: Vec<serde_json::Value>) -> i32 {
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

    fn run(&self) -> i32 {
        let command = self.command.to_lowercase();

        if !command.is_empty() {
            if command == "pipeline" {
                return self.run_pipeline();
            }
            if command == "info" {
                return self.run_info();
            }
            if command == "translate" {
                return self.run_translate();
            }
            if command == "merge" {
                return self.run_merge();
            }
            if command == "sort" {
                return self.run_sort();
            }
            if command == "random" {
                return self.run_random();
            }
            if command == "hausdorff" {
                return self.run_hausdorff();
            }
            if command == "chamfer" {
                return self.run_chamfer();
            }
            if command == "delta" {
                return self.run_delta();
            }
            eprintln!("Unknown Rust command '{}'", command);
            return 1;
        }

        if self.show_version {
            self.output_version();
        } else if self.show_drivers {
            self.output_drivers();
        } else if self.show_commands {
            self.output_commands("");
        } else if let Some(ref stage) = self.show_options {
            if stage == "all" {
                eprintln!("Showing options for all stages is not yet implemented");
            } else {
                self.output_options(stage);
            }
        } else {
            self.output_help();
        }

        0
    }
}

/// A `--<stage>.<key>=<value>` command-line stage option.
struct StageOption {
    stage: String,
    key: String,
    value: String,
}

/// Split command arguments into positional values and
/// `--<stage>.<key>=<value>` stage options.
fn parse_stage_args(args: &[String]) -> Result<(Vec<&str>, Vec<StageOption>), String> {
    let mut positional: Vec<&str> = Vec::new();
    let mut stage_options: Vec<StageOption> = Vec::new();
    for arg in args {
        if let Some(spec) = arg.strip_prefix("--") {
            let parsed = spec
                .split_once('=')
                .and_then(|(lhs, value)| lhs.rsplit_once('.').map(|(s, k)| (s, k, value)));
            match parsed {
                Some((stage, key, value)) => {
                    stage_options.push(StageOption {
                        stage: stage.to_string(),
                        key: key.to_string(),
                        value: value.to_string(),
                    });
                }
                None => {
                    return Err(format!("option '{arg}' must be --<stage>.<key>=<value>"));
                }
            }
        } else {
            positional.push(arg);
        }
    }
    Ok((positional, stage_options))
}

/// Apply `--<stage>.<key>=<value>` options to matching pipeline stages. A bare
/// filter name (`decimation`) also matches `filters.decimation`.
fn apply_stage_options(
    stages: &mut [serde_json::Value],
    options: &[StageOption],
) -> Result<(), String> {
    for option in options {
        let StageOption { stage, key, value } = option;
        let qualified = format!("filters.{stage}");
        let mut applied = false;
        for entry in stages.iter_mut() {
            let entry_type = entry["type"].as_str();
            if entry_type == Some(stage.as_str()) || entry_type == Some(qualified.as_str()) {
                entry[key.as_str()] = serde_json::json!(value);
                applied = true;
            }
        }
        if !applied {
            return Err(format!(
                "no '{stage}' stage in the pipeline for option '--{stage}.{key}'"
            ));
        }
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct KernelInfo {
    name: &'static str,
    full_name: &'static str,
    description: &'static str,
}

fn kernel_list() -> Vec<KernelInfo> {
    vec![
        KernelInfo {
            name: "chamfer",
            full_name: "kernels.chamfer",
            description: "report the Chamfer distance between two point cloud files",
        },
        KernelInfo {
            name: "delta",
            full_name: "kernels.delta",
            description: "report per-dimension deltas between two point cloud files",
        },
        KernelInfo {
            name: "hausdorff",
            full_name: "kernels.hausdorff",
            description: "report the Hausdorff distance between two point cloud files",
        },
        KernelInfo {
            name: "info",
            full_name: "kernels.info",
            description:
                "report metadata, bounds, and per-dimension summary for a point cloud file",
        },
        KernelInfo {
            name: "merge",
            full_name: "kernels.merge",
            description: "merge several point cloud files into one output",
        },
        KernelInfo {
            name: "pipeline",
            full_name: "kernels.pipeline",
            description: "execute a PDAL pipeline JSON file through the Rust port",
        },
        KernelInfo {
            name: "random",
            full_name: "kernels.random",
            description: "generate a file of uniformly random points",
        },
        KernelInfo {
            name: "sort",
            full_name: "kernels.sort",
            description: "sort the points of a file by one or more dimensions",
        },
        KernelInfo {
            name: "translate",
            full_name: "kernels.translate",
            description: "convert a point cloud file, optionally applying filters",
        },
    ]
}

#[derive(serde::Serialize)]
struct StageInfo {
    name: &'static str,
    description: &'static str,
    link: &'static str,
}

fn stage_list() -> Vec<StageInfo> {
    pdal_capi::READER_DRIVERS
        .iter()
        .chain(pdal_capi::FILTER_DRIVERS.iter())
        .chain(pdal_capi::WRITER_DRIVERS.iter())
        .map(|name| StageInfo {
            name,
            description: "Rust-backed stage",
            link: "",
        })
        .collect()
}

fn stage_options(stage_name: &str) -> Vec<serde_json::Value> {
    use serde_json::json;

    fn filename() -> serde_json::Value {
        json!({"arg": "filename", "description": "Input or output filename."})
    }
    fn option(
        arg: &str,
        description: &str,
        default: Option<serde_json::Value>,
    ) -> serde_json::Value {
        let mut value = json!({"arg": arg, "description": description});
        if let Some(default) = default {
            value["default"] = default;
        }
        value
    }

    match stage_name {
        "readers.faux" => vec![
            option(
                "count",
                "Number of synthetic points to create.",
                Some(json!(10)),
            ),
            option(
                "mode",
                "Synthetic point generation mode.",
                Some(json!("constant")),
            ),
        ],
        "readers.bpf" | "readers.fbi" | "readers.obj" | "readers.optech" | "readers.pcd"
        | "readers.ply" | "readers.pts" | "readers.ptx" | "readers.qfit" | "readers.smrmsg"
        | "readers.terrasolid" => vec![filename()],
        "readers.ilvis2" => vec![
            filename(),
            option(
                "mapping",
                "Point mapping to read: low, high, or all.",
                Some(json!("low")),
            ),
            option("metadata", "Optional ILVIS2 XML metadata sidecar.", None),
        ],
        "readers.sbet" => vec![
            filename(),
            option(
                "angles_as_degrees",
                "Convert stored angular values from radians to degrees.",
                Some(json!(true)),
            ),
        ],
        "filters.decimation" => vec![
            option("step", "Keep every Nth point.", Some(json!(1))),
            option("offset", "Starting point offset.", Some(json!(0))),
            option(
                "limit",
                "Maximum number of points to consider.",
                Some(json!(0)),
            ),
        ],
        "filters.head" | "filters.tail" => vec![
            option("count", "Number of points to keep.", Some(json!(10))),
            option(
                "invert",
                "Invert the selected point range.",
                Some(json!(false)),
            ),
        ],
        "filters.groupby" => vec![option("dimension", "Dimension used to group points.", None)],
        "filters.locate" => vec![
            option("dimension", "Dimension to inspect.", None),
            option("minmax", "Select the min or max point.", Some(json!("max"))),
        ],
        "filters.merge" => Vec::new(),
        "filters.mortonorder" => vec![option(
            "reverse",
            "Sort in reverse Morton order.",
            Some(json!(false)),
        )],
        "filters.randomize" => vec![option("seed", "Optional deterministic shuffle seed.", None)],
        "filters.returns" => vec![option(
            "groups",
            "Comma-separated return groups to keep.",
            Some(json!("last")),
        )],
        "filters.sample" => vec![option("radius", "Sample radius.", Some(json!(1.0)))],
        "filters.sort" => vec![
            option("dimensions", "Comma-separated dimensions to sort by.", None),
            option("order", "Sort order: asc or desc.", Some(json!("asc"))),
            option(
                "algorithm",
                "Sort algorithm: normal or stable.",
                Some(json!("normal")),
            ),
        ],
        "filters.stats" => vec![
            option(
                "dimensions",
                "Comma-separated dimensions to summarize.",
                None,
            ),
            option(
                "advanced",
                "Compute advanced statistics.",
                Some(json!(false)),
            ),
        ],
        "filters.voxeldownsize" => vec![option("cell", "Voxel cell size.", Some(json!(1.0)))],
        "writers.null" => Vec::new(),
        "writers.bpf" | "writers.fbi" | "writers.gltf" | "writers.sbet" => vec![filename()],
        "writers.text" => vec![
            filename(),
            option("order", "Comma-separated output dimension order.", None),
            option(
                "precision",
                "Floating-point output precision.",
                Some(json!(6)),
            ),
            option("delimiter", "Output delimiter.", Some(json!(","))),
            option(
                "quote_header",
                "Quote output header fields.",
                Some(json!(true)),
            ),
        ],
        "writers.pcd" => vec![
            filename(),
            option("order", "Comma-separated output dimension order.", None),
            option(
                "precision",
                "Floating-point output precision.",
                Some(json!(2)),
            ),
            option("compression", "PCD storage mode.", Some(json!("ascii"))),
        ],
        "writers.ply" => vec![
            filename(),
            option("storage_mode", "PLY storage mode.", Some(json!("ascii"))),
            option("dims", "Comma-separated dimension list.", None),
            option(
                "sized_types",
                "Use sized PLY type names.",
                Some(json!(true)),
            ),
            option("precision", "Floating-point output precision.", None),
            option("faces", "Write triangular mesh faces.", Some(json!(false))),
        ],
        _ => Vec::new(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut app = App::new();
    if let Err(e) = app.parse_args(&args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    let result = app.run();
    process::exit(result);
}

fn empty_pipeline_result() -> pdal_capi::pdal_pipeline_result_t {
    pdal_capi::pdal_pipeline_result_t {
        point_count: 0,
        view_count: 0,
        has_bounds_2d: false,
        bounds_2d: pdal_capi::pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        },
        has_bounds_3d: false,
        bounds_3d: pdal_capi::pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_preserves_command_arguments() {
        let mut app = App::new();
        app.parse_args(&[
            "pipeline".to_string(),
            "pipeline.json".to_string(),
            "--not-a-root-option".to_string(),
        ])
        .unwrap();

        assert_eq!(app.command, "pipeline");
        assert_eq!(
            app.command_args,
            vec![
                "pipeline.json".to_string(),
                "--not-a-root-option".to_string()
            ]
        );
    }

    #[test]
    fn parse_keeps_root_options_before_command() {
        let mut app = App::new();
        app.parse_args(&[
            "--verbose".to_string(),
            "--showjson".to_string(),
            "pipeline".to_string(),
            "pipeline.json".to_string(),
        ])
        .unwrap();

        assert_eq!(app.verbose, 1);
        assert!(app.show_json);
        assert_eq!(app.command, "pipeline");
        assert_eq!(app.command_args, vec!["pipeline.json".to_string()]);
    }
}
