use crate::error::{set_last_error, string_to_c_ptr};
use pdal_core::program_args::{parse_program_args, ArgSpec};
use std::ffi::{c_char, CStr};

/// # Safety
/// `specs_json` and `args_json` must be valid NUL-terminated strings.
/// The returned string must be freed with `pdal_string_free`.
#[no_mangle]
pub unsafe extern "C" fn pdal_program_args_parse_json(
    specs_json: *const c_char,
    args_json: *const c_char,
    simple: bool,
) -> *mut c_char {
    let Some(specs_json) = cstr(specs_json) else {
        return std::ptr::null_mut();
    };
    let Some(args_json) = cstr(args_json) else {
        return std::ptr::null_mut();
    };
    let specs: Vec<ArgSpec> = match serde_json::from_str(&specs_json) {
        Ok(specs) => specs,
        Err(err) => {
            set_last_error(err.to_string());
            return std::ptr::null_mut();
        }
    };
    let args: Vec<String> = match serde_json::from_str(&args_json) {
        Ok(args) => args,
        Err(err) => {
            set_last_error(err.to_string());
            return std::ptr::null_mut();
        }
    };
    match parse_program_args(&specs, &args, simple) {
        Ok(result) => match serde_json::to_string(&result) {
            Ok(text) => string_to_c_ptr(text),
            Err(err) => {
                set_last_error(err.to_string());
                std::ptr::null_mut()
            }
        },
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

unsafe fn cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        set_last_error("null program args string");
        return None;
    }
    Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn parses_specs_and_args_from_json() {
        unsafe {
            let specs =
                CString::new(r#"[{"name":"foo","short":"f","kind":"string","default":"foo"}]"#)
                    .unwrap();
            let args = CString::new(r#"["--foo","bar"]"#).unwrap();
            let ptr = pdal_program_args_parse_json(specs.as_ptr(), args.as_ptr(), false);
            assert!(!ptr.is_null());
            let text = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            crate::pdal_string_free(ptr);
            assert!(text.contains(r#""foo":"bar""#));
        }
    }
}
