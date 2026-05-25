use crate::error::{set_last_error, string_to_c_ptr};
use pdal_io::vsi::local_io_scenario;
use std::ffi::{c_char, CStr};
use std::path::Path;

/// # Safety
/// `filename` and `scenario` must be valid NUL-terminated strings. The
/// returned string must be freed with `pdal_string_free`.
#[no_mangle]
pub unsafe extern "C" fn pdal_vsi_local_io_scenario_json(
    filename: *const c_char,
    scenario: *const c_char,
    buffer_size: u64,
) -> *mut c_char {
    let Some(filename) = cstr(filename) else {
        return std::ptr::null_mut();
    };
    let Some(scenario) = cstr(scenario) else {
        return std::ptr::null_mut();
    };
    match local_io_scenario(Path::new(&filename), &scenario, buffer_size as usize) {
        Ok(summary) => string_to_c_ptr(summary.to_string()),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

unsafe fn cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        set_last_error("null VSI string");
        return None;
    }
    Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn runs_local_io_scenario() {
        unsafe {
            let temp = tempfile::NamedTempFile::new().unwrap();
            let filename = CString::new(temp.path().display().to_string()).unwrap();
            let scenario = CString::new("tells").unwrap();
            let ptr = pdal_vsi_local_io_scenario_json(filename.as_ptr(), scenario.as_ptr(), 2);
            assert!(!ptr.is_null());
            let text = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            crate::pdal_string_free(ptr);
            assert!(text.contains(r#""file_size":9"#));
        }
    }
}
