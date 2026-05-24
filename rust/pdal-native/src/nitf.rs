//! Nitro-backed NITF helpers.

use std::ffi::CString;
use std::os::raw::{c_char, c_double};

const ERR_LEN: usize = 1024;

extern "C" {
    fn pdal_native_nitf_lidar_segment(
        input: *const c_char,
        offset: *mut u64,
        length: *mut u64,
        err: *mut c_char,
        err_len: usize,
    ) -> i32;

    fn pdal_native_nitf_wrap(
        input: *const c_char,
        output: *const c_char,
        title: *const c_char,
        bounds: *const c_double,
        err: *mut c_char,
        err_len: usize,
    ) -> i32;
}

pub fn lidar_segment(path: &str) -> Result<(u64, u64), String> {
    let path = CString::new(path).map_err(|e| e.to_string())?;
    let mut offset = 0;
    let mut length = 0;
    let mut err = [0 as c_char; ERR_LEN];
    let ok = unsafe {
        pdal_native_nitf_lidar_segment(
            path.as_ptr(),
            &mut offset,
            &mut length,
            err.as_mut_ptr(),
            err.len(),
        )
    };
    if ok == 0 {
        return Err(take_error(&err));
    }
    Ok((offset, length))
}

pub fn wrap(input: &str, output: &str, title: &str, bounds: [f64; 4]) -> Result<(), String> {
    let input = CString::new(input).map_err(|e| e.to_string())?;
    let output = CString::new(output).map_err(|e| e.to_string())?;
    let title = CString::new(title).map_err(|e| e.to_string())?;
    let mut err = [0 as c_char; ERR_LEN];
    let ok = unsafe {
        pdal_native_nitf_wrap(
            input.as_ptr(),
            output.as_ptr(),
            title.as_ptr(),
            bounds.as_ptr(),
            err.as_mut_ptr(),
            err.len(),
        )
    };
    if ok == 0 {
        return Err(take_error(&err));
    }
    Ok(())
}

fn take_error(err: &[c_char]) -> String {
    let bytes: Vec<u8> = err
        .iter()
        .take_while(|&&ch| ch != 0)
        .map(|&ch| ch as u8)
        .collect();
    if bytes.is_empty() {
        "NITF operation failed".to_string()
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_paths_with_nul_bytes() {
        let err = lidar_segment("bad\0path").unwrap_err();
        assert!(err.contains("nul byte"));

        let err = wrap("in\0put", "out.ntf", "title", [0.0, 0.0, 1.0, 1.0]).unwrap_err();
        assert!(err.contains("nul byte"));

        let err = wrap("input.las", "out\0.ntf", "title", [0.0, 0.0, 1.0, 1.0]).unwrap_err();
        assert!(err.contains("nul byte"));

        let err = wrap("input.las", "out.ntf", "bad\0title", [0.0, 0.0, 1.0, 1.0]).unwrap_err();
        assert!(err.contains("nul byte"));
    }

    #[test]
    fn empty_native_errors_have_fallback_text() {
        assert_eq!(take_error(&[0 as c_char; ERR_LEN]), "NITF operation failed");
        let err = [b'N' as c_char, b'o' as c_char, 0, b'x' as c_char];
        assert_eq!(take_error(&err), "No");
    }
}
