use crate::error::string_to_c_ptr;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub unsafe extern "C" fn pdal_infer_reader_driver(filename: *const c_char) -> *mut c_char {
    if filename.is_null() {
        return string_to_c_ptr(String::new());
    }
    let filename = CStr::from_ptr(filename).to_string_lossy();
    string_to_c_ptr(infer_reader_driver(&filename).unwrap_or("").to_string())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_infer_writer_driver(filename: *const c_char) -> *mut c_char {
    if filename.is_null() {
        return string_to_c_ptr(String::new());
    }
    let filename = CStr::from_ptr(filename).to_string_lossy();
    string_to_c_ptr(infer_writer_driver(&filename).unwrap_or("").to_string())
}
