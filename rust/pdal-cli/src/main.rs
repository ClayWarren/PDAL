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
        for kernel in kernel_list() {
            println!("{}{}", leader, kernel.name);
        }
    }

    fn output_options(&self, stage_name: &str) {
        if !stage_list().iter().any(|stage| stage.name == stage_name) {
            eprintln!("Unable to create stage {}", stage_name);
            return;
        }
        let json_str = "[]";

        if self.show_json {
            println!("{}", json_str);
            return;
        }

        println!("{}", stage_name);
        println!("{}", "-".repeat(80));
        println!("  No Rust option metadata is available for this stage yet.");
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

        let count =
            unsafe { pdal_capi::pdal_pipeline_execute_count(pipeline, std::ptr::null_mut()) };
        unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
        if count < 0 {
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
            eprintln!("Unknown Rust command '{}'", command);
            return 1;
        }

        if self.show_version {
            self.output_version();
        } else if self.show_drivers {
            self.output_drivers();
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

#[derive(serde::Serialize)]
struct KernelInfo {
    name: &'static str,
    full_name: &'static str,
    description: &'static str,
}

fn kernel_list() -> Vec<KernelInfo> {
    vec![KernelInfo {
        name: "pipeline",
        full_name: "kernels.pipeline",
        description: "execute a PDAL pipeline JSON file through the Rust port",
    }]
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
