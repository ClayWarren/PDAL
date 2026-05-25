use crate::error::set_last_error;
use pdal_io::ept_addon::validate_ept_addon_input;
use std::ffi::{c_char, CStr};

/// # Safety
/// `reader_name` must be a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_addon_validate_input(reader_name: *const c_char) -> bool {
    let Some(reader_name) = cstr(reader_name) else {
        return false;
    };
    match validate_ept_addon_input(&reader_name) {
        Ok(()) => true,
        Err(err) => {
            set_last_error(err);
            false
        }
    }
}

unsafe fn cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        set_last_error("null EPT addon reader name");
        return None;
    }
    Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn validates_reader_name() {
        unsafe {
            let ept = CString::new("readers.ept").unwrap();
            let las = CString::new("readers.las").unwrap();
            assert!(pdal_ept_addon_validate_input(ept.as_ptr()));
            assert!(!pdal_ept_addon_validate_input(las.as_ptr()));
        }
    }
}
