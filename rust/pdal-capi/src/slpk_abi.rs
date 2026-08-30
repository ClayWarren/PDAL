use crate::error::{set_last_error, string_to_c_ptr};
use pdal_io::slpk::summarize_slpk;
use serde_json::json;
use std::ffi::{c_char, CStr};
use std::path::Path;

/// # Safety
/// `filename` must be a valid NUL-terminated string. `dimensions_csv` may be
/// null or a valid NUL-terminated string. The returned string must be freed
/// with `pdal_string_free`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_slpk_summary_json(
    filename: *const c_char,
    dimensions_csv: *const c_char,
) -> *mut c_char {
    let Some(filename) = cstr(filename) else {
        return std::ptr::null_mut();
    };
    let dimensions = if dimensions_csv.is_null() {
        Vec::new()
    } else {
        cstr(dimensions_csv)
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|dim| !dim.is_empty())
            .map(ToString::to_string)
            .collect()
    };
    match summarize_slpk(Path::new(&filename), &dimensions) {
        Ok(summary) => string_to_c_ptr(
            json!({
                "point_count": summary.point_count,
                "dimensions": summary.dimensions,
            })
            .to_string(),
        ),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

unsafe fn cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        set_last_error("null SLPK string");
        return None;
    }
    Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn summarizes_fixture() {
        unsafe {
            let filename = CString::new("../../test/data/i3s/SMALL_AUTZEN_LAS_All.slpk").unwrap();
            let dims = CString::new("intensity, returns").unwrap();
            let ptr = pdal_slpk_summary_json(filename.as_ptr(), dims.as_ptr());
            assert!(!ptr.is_null());
            let text = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            crate::pdal_string_free(ptr);
            assert!(text.contains(r#""point_count":106"#));
            assert!(text.contains("Intensity"));
            assert!(text.contains("NumberOfReturns"));
        }
    }
}
