use crate::error::ffi_catch;
use pdal_io::{lasdump, nitfwrap};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::path::Path;

unsafe fn argv_to_vec(argc: c_int, argv: *const *const c_char) -> Result<Vec<String>, String> {
    if argc < 0 {
        return Err("negative argc".to_string());
    }
    if argc > 0 && argv.is_null() {
        return Err("null argv".to_string());
    }

    let args = std::slice::from_raw_parts(argv, argc as usize);
    args.iter()
        .map(|arg| {
            if arg.is_null() {
                return Err("null argument".to_string());
            }
            CStr::from_ptr(*arg)
                .to_str()
                .map(str::to_string)
                .map_err(|_| "argument is not valid UTF-8".to_string())
        })
        .collect()
}

fn run_lasdump_args(args: &[String]) -> c_int {
    let mut input = None;
    let mut output = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "-o" {
            let Some(value) = iter.next() else {
                eprintln!("Usage: lasdump [-o <output filename>] <las/las file>");
                return 1;
            };
            output = Some(value.as_str());
        } else if arg.starts_with('-') {
            eprintln!("Usage: lasdump [-o <output filename>] <las/las file>");
            return 1;
        } else if input.is_none() {
            input = Some(arg.as_str());
        } else {
            eprintln!("Usage: lasdump [-o <output filename>] <las/las file>");
            return 1;
        }
    }

    let Some(input) = input else {
        eprintln!("Usage: lasdump [-o <output filename>] <las/las file>");
        return 1;
    };

    let text = match lasdump::dump_las(Path::new(input)) {
        Ok(text) => text,
        Err(message) => {
            eprintln!("Error: {message}");
            return 1;
        }
    };

    if let Some(output) = output {
        if std::fs::write(output, text).is_err() {
            eprintln!("Error: Couldn't open output file.");
            return 1;
        }
    } else {
        print!("{text}");
    }
    0
}

fn run_nitfwrap_args(args: &[String]) -> c_int {
    let mut input = None;
    let mut output = None;
    let mut unwrap = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-u" | "--unwrap" => unwrap = true,
            "-o" | "--output" => {
                let Some(value) = iter.next() else {
                    eprintln!("nitfwrap: output option requires a filename");
                    eprintln!("usage: nitfwrap [options] <input> [output]");
                    return 1;
                };
                output = Some(value.as_str());
            }
            _ if arg.starts_with("--output=") => output = arg.strip_prefix("--output="),
            _ if arg.starts_with('-') => {
                eprintln!("nitfwrap: Unexpected argument '{arg}'");
                eprintln!("usage: nitfwrap [options] <input> [output]");
                return 1;
            }
            _ if input.is_none() => input = Some(arg.as_str()),
            _ if output.is_none() => output = Some(arg.as_str()),
            _ => {
                eprintln!("nitfwrap: Unexpected argument '{arg}'");
                eprintln!("usage: nitfwrap [options] <input> [output]");
                return 1;
            }
        }
    }

    let Some(input) = input else {
        eprintln!("usage: nitfwrap [options] <input> [output]");
        return 1;
    };

    let input = Path::new(input);
    let output = output.map(Path::new);
    let result = if unwrap {
        nitfwrap::unwrap(input, output)
    } else {
        nitfwrap::wrap(input, output)
    };
    match result {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("nitfwrap: {err}");
            1
        }
    }
}

#[pdal_capi_macros::ffi_export(fallback = 1)]
pub unsafe extern "C" fn pdal_tool_lasdump_run(argc: c_int, argv: *const *const c_char) -> c_int {
    ffi_catch(1, || match argv_to_vec(argc, argv) {
        Ok(args) => run_lasdump_args(&args),
        Err(err) => {
            eprintln!("Error: {err}");
            1
        }
    })
}

#[pdal_capi_macros::ffi_export(fallback = 1)]
pub unsafe extern "C" fn pdal_tool_nitfwrap_run(argc: c_int, argv: *const *const c_char) -> c_int {
    ffi_catch(1, || match argv_to_vec(argc, argv) {
        Ok(args) => run_nitfwrap_args(&args),
        Err(err) => {
            eprintln!("nitfwrap: {err}");
            1
        }
    })
}
