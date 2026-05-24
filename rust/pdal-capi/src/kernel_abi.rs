use crate::error::string_to_c_ptr;
use pdal_core::kernel::{parse_stage_option, ParseStageResult};
use pdal_kernels::{FauxPluginKernel, Kernel, KernelArgs};
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub unsafe extern "C" fn pdal_kernel_parse_stage_option(
    input: *const c_char,
    allow_stage_prefix: bool,
    stage: *mut *mut c_char,
    option: *mut *mut c_char,
    value: *mut *mut c_char,
) -> i32 {
    let input = if input.is_null() {
        String::new()
    } else {
        CStr::from_ptr(input).to_string_lossy().into_owned()
    };
    let parsed = parse_stage_option(&input, allow_stage_prefix);

    if !stage.is_null() {
        *stage = string_to_c_ptr(parsed.stage);
    }
    if !option.is_null() {
        *option = string_to_c_ptr(parsed.option);
    }
    if !value.is_null() {
        *value = string_to_c_ptr(parsed.value);
    }

    match parsed.result {
        ParseStageResult::Ok => 0,
        ParseStageResult::Invalid => 1,
        ParseStageResult::Unknown => 2,
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_rust_kernel_run(
    kernel_name: *const c_char,
    argc: i32,
    argv: *const *const c_char,
) -> i32 {
    if kernel_name.is_null() || argc < 0 || (argc > 0 && argv.is_null()) {
        return -1;
    }

    let name = CStr::from_ptr(kernel_name).to_string_lossy().to_lowercase();
    let name = name.strip_prefix("kernels.").unwrap_or(&name);
    if name != "fauxplugin" {
        return -1;
    }

    let mut args = Vec::new();
    for i in 0..argc {
        let arg = *argv.add(i as usize);
        if arg.is_null() {
            return 1;
        }
        args.push(CStr::from_ptr(arg).to_string_lossy().into_owned());
    }

    if args.is_empty() {
        eprintln!("PDAL: kernels.fauxplugin: Missing value for positional argument 'fakearg'.");
        return 1;
    }

    let mut kernel = FauxPluginKernel::default();
    match kernel.run(&KernelArgs::new(args)) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn rust_kernel_run_reports_unsupported_kernels() {
        let name = CString::new("kernels.sort").unwrap();

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

        assert_eq!(result, -1);
    }

    #[test]
    fn rust_kernel_run_dispatches_fauxplugin() {
        let name = CString::new("fauxplugin").unwrap();
        let arg = CString::new("7").unwrap();
        let argv = [arg.as_ptr()];

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 1, argv.as_ptr()) };

        assert_eq!(result, 0);
    }

    #[test]
    fn rust_kernel_run_requires_fauxplugin_arg() {
        let name = CString::new("kernels.fauxplugin").unwrap();

        let result = unsafe { pdal_rust_kernel_run(name.as_ptr(), 0, std::ptr::null()) };

        assert_eq!(result, 1);
    }
}
