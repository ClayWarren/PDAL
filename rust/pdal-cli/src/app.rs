use std::ffi::CStr;
use std::path::{Path, PathBuf};

use crate::stage_metadata::{all_stage_options, kernel_list, stage_list, stage_options};
use pdal_kernels::word_wrap;

#[path = "analysis_commands.rs"]
mod analysis_commands;
#[path = "pipeline_commands.rs"]
mod pipeline_commands;

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
                if arg == "--developer-debug" {
                    i += 1;
                    continue;
                }
                if arg == "--label" {
                    i += 2;
                    if i > args.len() {
                        return Err("--label requires a label argument".to_string());
                    }
                    continue;
                }
                if arg.starts_with("--label=") {
                    i += 1;
                    continue;
                }
                self.command_args.push(arg.clone());
                i += 1;
                continue;
            }
            match arg.as_str() {
                "--help" | "-h" => self.help = true,
                "--version" => self.show_version = true,
                "--drivers" => self.show_drivers = true,
                "--list-commands" => self.show_commands = true,
                "--command" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--command requires a command name argument".to_string());
                    }
                    self.command = args[i].clone();
                }
                "--debug" => self.verbose = self.verbose.max(3),
                "--options" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--options requires a stage name argument".to_string());
                    }
                    self.show_options = Some(args[i].clone());
                }
                "--showjson" => self.show_json = true,
                "--developer-debug" => {}
                "--label" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--label requires a label argument".to_string());
                    }
                }
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
        if self.show_json {
            let native_json = pdal_capi::pdal_native_dependencies_json();
            let native = safe_cstr(native_json)
                .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
            unsafe {
                if !native_json.is_null() {
                    pdal_capi::pdal_string_free(native_json);
                }
            }
            println!(
                "{}",
                serde_json::json!({
                    "name": "pdal-rs",
                    "version": env!("CARGO_PKG_VERSION"),
                    "native_dependencies": native,
                })
            );
            return;
        }
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
        println!("  --command <name>    The PDAL command");
        println!("  --debug             Sets the output level to 3");
        println!("  --label <label>     Label the process");
        println!("  --developer-debug   Enable developer debug");
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
        if stage_name == "all" {
            self.output_all_options();
            return;
        }
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

    fn output_all_options(&self) {
        let all = all_stage_options();
        if self.show_json {
            println!("{}", serde_json::Value::Object(all));
            return;
        }
        for stage in stage_list() {
            self.output_options(stage.name);
            println!();
        }
    }

    fn output_last_error(&self) {
        match safe_cstr(pdal_capi::pdal_last_error()) {
            Some(message) if !message.is_empty() => eprintln!("{}", message),
            _ => eprintln!("Rust pipeline execution failed"),
        }
    }

    fn command_help_requested(&self) -> bool {
        self.command_args
            .iter()
            .any(|arg| arg == "--help" || arg == "-h")
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
            if command == "ground" {
                return self.run_ground();
            }
            if command == "density" {
                return self.run_density();
            }
            if command == "split" {
                return self.run_split();
            }
            if command == "tile" {
                return self.run_tile();
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
            if command == "eval" {
                return self.run_eval();
            }
            if command == "tindex" {
                return self.run_tindex();
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
            self.output_options(stage);
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

struct SplitArgs<'a> {
    input: &'a str,
    output: PathBuf,
    reader_driver: Option<&'a str>,
    length: Option<f64>,
    capacity: Option<u64>,
    origin_x: Option<f64>,
    origin_y: Option<f64>,
}

impl<'a> SplitArgs<'a> {
    fn parse(args: &'a [String]) -> Result<Self, String> {
        let mut positional = Vec::new();
        let mut input = None;
        let mut output = None;
        let mut reader_driver = None;
        let mut length = None;
        let mut capacity = None;
        let mut origin_x = None;
        let mut origin_y = None;

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if let Some(value) = arg.strip_prefix("--length=") {
                length = Some(parse_f64_option("length", value)?);
            } else if let Some(value) = arg.strip_prefix("--capacity=") {
                capacity = Some(parse_u64_option("capacity", value)?);
            } else if let Some(value) = arg.strip_prefix("--origin_x=") {
                origin_x = Some(parse_f64_option("origin_x", value)?);
            } else if let Some(value) = arg.strip_prefix("--origin_y=") {
                origin_y = Some(parse_f64_option("origin_y", value)?);
            } else if arg == "--length" {
                let Some(value) = iter.next() else {
                    return Err("--length requires a value".to_string());
                };
                length = Some(parse_f64_option("length", value)?);
            } else if arg == "--capacity" {
                let Some(value) = iter.next() else {
                    return Err("--capacity requires a value".to_string());
                };
                capacity = Some(parse_u64_option("capacity", value)?);
            } else if arg == "--origin_x" {
                let Some(value) = iter.next() else {
                    return Err("--origin_x requires a value".to_string());
                };
                origin_x = Some(parse_f64_option("origin_x", value)?);
            } else if arg == "--origin_y" {
                let Some(value) = iter.next() else {
                    return Err("--origin_y requires a value".to_string());
                };
                origin_y = Some(parse_f64_option("origin_y", value)?);
            } else if arg == "--input" || arg == "-i" {
                let Some(value) = iter.next() else {
                    return Err(format!("{arg} requires an input path"));
                };
                input = Some(value.as_str());
            } else if arg == "--output" || arg == "-o" {
                let Some(value) = iter.next() else {
                    return Err(format!("{arg} requires an output path"));
                };
                output = Some(value.as_str());
            } else if arg == "--driver" {
                let Some(value) = iter.next() else {
                    return Err("--driver requires a reader driver name".to_string());
                };
                reader_driver = Some(value.as_str());
            } else if let Some(value) = arg.strip_prefix("--driver=") {
                reader_driver = Some(value);
            } else if arg.starts_with("--") {
                return Err(format!("unknown option '{arg}' for split"));
            } else {
                positional.push(arg.as_str());
            }
        }

        if input.is_none() && !positional.is_empty() {
            input = Some(positional.remove(0));
        }
        if output.is_none() && !positional.is_empty() {
            output = Some(positional.remove(0));
        }
        if input.is_none() || output.is_none() || !positional.is_empty() {
            return Err("split expects an input path and an output path".to_string());
        }
        if length.is_some() && capacity.is_some() {
            return Err("can't specify both length and capacity".to_string());
        }
        if length.is_none() && (origin_x.is_some() || origin_y.is_some()) {
            return Err("origin_x and origin_y require length mode".to_string());
        }

        let input = input.unwrap();
        let output = output.unwrap();
        Ok(Self {
            input,
            output: split_output_path(input, output),
            reader_driver,
            length,
            capacity,
            origin_x,
            origin_y,
        })
    }
}

fn parse_f64_option(name: &str, value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("--{name} must be numeric"))
}

fn parse_u64_option(name: &str, value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("--{name} must be a non-negative integer"))
}

fn split_output_path(input: &str, output: &str) -> PathBuf {
    let output_path = Path::new(output);
    if output.ends_with(std::path::MAIN_SEPARATOR) || output_path.is_dir() {
        let filename = Path::new(input)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new(input));
        output_path.join(filename)
    } else {
        output_path.to_path_buf()
    }
}

fn numbered_output(path: &Path, index: usize) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    let suffix = format!("{stem}_{index}");
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if !extension.is_empty() => {
            path.with_file_name(format!("{suffix}.{extension}"))
        }
        _ => path.with_file_name(suffix),
    }
}

fn parse_stage_option_arg(arg: &str) -> Result<StageOption, String> {
    let Some(spec) = arg.strip_prefix("--") else {
        return Err(format!("option '{arg}' must be --<stage>.<key>=<value>"));
    };
    let parsed = spec
        .split_once('=')
        .and_then(|(lhs, value)| lhs.rsplit_once('.').map(|(s, k)| (s, k, value)));
    match parsed {
        Some((stage, key, value)) => Ok(StageOption {
            stage: stage.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        }),
        None => Err(format!("option '{arg}' must be --<stage>.<key>=<value>")),
    }
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

pub fn run(args: Vec<String>) -> i32 {
    let mut app = App::new();
    if let Err(e) = app.parse_args(&args) {
        eprintln!("Error: {}", e);
        return 1;
    }

    app.run()
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

    #[test]
    fn parse_supports_command_option() {
        let mut app = App::new();
        app.parse_args(&[
            "--command".to_string(),
            "pipeline".to_string(),
            "pipeline.json".to_string(),
        ])
        .unwrap();

        assert_eq!(app.command, "pipeline");
        assert_eq!(app.command_args, vec!["pipeline.json".to_string()]);
    }

    #[test]
    fn parse_supports_debug_option() {
        let mut app = App::new();
        app.parse_args(&["--debug".to_string(), "--verbose".to_string()])
            .unwrap();

        assert_eq!(app.verbose, 4);
    }

    #[test]
    fn parse_ignores_standard_label_and_developer_debug_options() {
        let mut app = App::new();
        app.parse_args(&[
            "info".to_string(),
            "--label".to_string(),
            "smoke".to_string(),
            "--developer-debug".to_string(),
            "--summary".to_string(),
            "input.las".to_string(),
        ])
        .unwrap();

        assert_eq!(app.command, "info");
        assert_eq!(
            app.command_args,
            vec!["--summary".to_string(), "input.las".to_string()]
        );
    }

    #[test]
    fn command_help_requested_detects_command_local_help() {
        let mut app = App::new();
        app.parse_args(&["tindex".to_string(), "--help".to_string()])
            .unwrap();

        assert!(app.command_help_requested());
    }
}
