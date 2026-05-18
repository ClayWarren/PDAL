use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::process;

extern "C" {
    fn pdal_version_string() -> *const c_char;
    fn pdal_kernel_list_json() -> *mut c_char;
    fn pdal_stage_list_json() -> *mut c_char;
    fn pdal_stage_options_json(stage_name: *const c_char) -> *mut c_char;
    fn pdal_kernel_run(
        kernel_name: *const c_char,
        argc: c_int,
        argv: *const *const c_char,
    ) -> c_int;
    fn pdal_capi_free(ptr: *mut c_char);
}

fn safe_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
    }
}

fn safe_cstr_free(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let result = unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) };
    unsafe { pdal_capi_free(ptr as *mut _) };
    result
}

struct App {
    command: String,
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
        if let Some(version) = safe_cstr(unsafe { pdal_version_string() }) {
            println!("pdal {}", version);
        } else {
            println!("pdal (version unavailable)");
        }
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
        let json_ptr = unsafe { pdal_stage_list_json() };
        let json_str = match safe_cstr_free(json_ptr) {
            Some(s) => s,
            None => {
                eprintln!("Failed to retrieve stage list");
                return;
            }
        };

        if self.show_json {
            println!("{}", json_str);
            return;
        }

        let stages: Vec<serde_json::Value> = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to parse stage list: {}", e);
                return;
            }
        };

        let name_col = 28;
        let descrip_col = 80 - name_col - 1;
        let tablehead = format!("{} {}", "=".repeat(name_col), "=".repeat(descrip_col));

        println!();
        println!("{}", tablehead);
        println!("{:<name_col$} Description", "Name");
        println!("{}", tablehead);

        for stage in &stages {
            let name = stage["name"].as_str().unwrap_or("");
            let descrip = stage["description"].as_str().unwrap_or("");
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
        let json_ptr = unsafe { pdal_kernel_list_json() };
        let json_str = match safe_cstr_free(json_ptr) {
            Some(s) => s,
            None => {
                eprintln!("Failed to retrieve kernel list");
                return;
            }
        };

        let kernels: Vec<serde_json::Value> = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to parse kernel list: {}", e);
                return;
            }
        };

        for kernel in &kernels {
            let name = kernel["name"].as_str().unwrap_or("");
            println!("{}{}", leader, name);
        }
    }

    fn output_options(&self, stage_name: &str) {
        let c_name = match CString::new(stage_name) {
            Ok(n) => n,
            Err(_) => {
                eprintln!("Invalid stage name: {}", stage_name);
                return;
            }
        };

        let json_ptr = unsafe { pdal_stage_options_json(c_name.as_ptr()) };
        if json_ptr.is_null() {
            eprintln!("Unable to create stage {}", stage_name);
            return;
        }

        let json_str = match safe_cstr_free(json_ptr) {
            Some(s) => s,
            None => return,
        };

        if self.show_json {
            println!("{}", json_str);
            return;
        }

        let link_ptr = unsafe { pdal_stage_list_json() };
        let link = if let Some(json_str) = safe_cstr_free(link_ptr) {
            if let Ok(stages) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                stages
                    .iter()
                    .find(|s| s["name"].as_str() == Some(stage_name))
                    .and_then(|s| s["link"].as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        println!("{} -- {}", stage_name, link);
        println!("{}", "-".repeat(80));

        if let Ok(options) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
            for opt in &options {
                let name = opt["arg"].as_str().unwrap_or("");
                let desc = opt["description"].as_str().unwrap_or("");
                let default = opt.get("default").and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| format!("'{}'", s))
                    } else {
                        v.as_str().map(|s| s.to_string())
                    }
                });

                let default_str = default
                    .map(|d| format!(" (default: {})", d))
                    .unwrap_or_default();
                println!("  {:<20} {}{}", name, desc, default_str);
            }
        }
    }

    fn run(&self) -> i32 {
        let command = self.command.to_lowercase();

        if !command.is_empty() {
            let kernel_name = match CString::new(command.as_str()) {
                Ok(n) => n,
                Err(_) => return 1,
            };

            let mut c_args: Vec<*const c_char> = Vec::new();
            if self.help {
                c_args.push(CString::new("--help").unwrap().into_raw());
            }

            let result = unsafe {
                let argc = c_args.len() as c_int;
                let argv = if c_args.is_empty() {
                    std::ptr::null()
                } else {
                    c_args.as_ptr()
                };
                pdal_kernel_run(kernel_name.as_ptr(), argc, argv)
            };

            for ptr in c_args {
                unsafe {
                    let _ = CString::from_raw(ptr as *mut c_char);
                }
            }

            return result;
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

fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if text.len() <= width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current.clone());
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
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
