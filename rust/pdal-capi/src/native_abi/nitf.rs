use super::c_string_lossy;
use crate::error::{ffi_catch, set_last_error};
use std::os::raw::c_char;

/// Locate the LIDARA data extension segment in a NITF file.
///
/// On success returns true and writes the LIDARA segment offset and length
/// (bytes) to `out_offset` and `out_length`. On failure returns false and
/// sets the last error string.
///
/// # Safety
///
/// `path` must be null or a valid NUL-terminated C string. `out_offset` and
/// `out_length` must be valid for writes (non-null).
#[no_mangle]
pub unsafe extern "C" fn pdal_nitf_lidar_segment(
    path: *const c_char,
    out_offset: *mut u64,
    out_length: *mut u64,
) -> bool {
    ffi_catch(false, || {
        let path = c_string_lossy(path);
        if path.is_empty() {
            set_last_error("pdal_nitf_lidar_segment: null/empty path");
            return false;
        }
        match pdal_native::nitf::lidar_segment(&path) {
            Ok((offset, length)) => {
                if let Some(out_offset) = out_offset.as_mut() {
                    *out_offset = offset;
                }
                if let Some(out_length) = out_length.as_mut() {
                    *out_length = length;
                }
                true
            }
            Err(err) => {
                set_last_error(err);
                false
            }
        }
    })
}

/// Enumerate NITF file/image/DES header fields and TREs, invoking `cb` once
/// per `(key, value)` pair. `key` is a stable dotted path such as `FH.FDT`
/// or `IM:0.IGEOLO`. Returning non-zero from `cb` stops the enumeration.
///
/// # Safety
///
/// `path` must be null or a valid NUL-terminated C string. `cb` must be a
/// callable function pointer with the documented signature.
#[no_mangle]
pub unsafe extern "C" fn pdal_nitf_read_metadata(
    path: *const c_char,
    cb: Option<
        unsafe extern "C" fn(
            key: *const c_char,
            value: *const c_char,
            userdata: *mut std::os::raw::c_void,
        ) -> std::os::raw::c_int,
    >,
    userdata: *mut std::os::raw::c_void,
) -> bool {
    ffi_catch(false, || {
        let Some(cb) = cb else {
            set_last_error("pdal_nitf_read_metadata: null callback");
            return false;
        };
        let path = c_string_lossy(path);
        if path.is_empty() {
            set_last_error("pdal_nitf_read_metadata: null/empty path");
            return false;
        }
        match pdal_native::nitf::read_metadata(&path) {
            Ok(map) => {
                for (key, value) in map {
                    let key_c = match std::ffi::CString::new(key) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    let value_c = match std::ffi::CString::new(value) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    if cb(key_c.as_ptr(), value_c.as_ptr(), userdata) != 0 {
                        break;
                    }
                }
                true
            }
            Err(err) => {
                set_last_error(err);
                false
            }
        }
    })
}

/// C-ABI form of `NitfWriteOptions`. String fields may be null; lists of
/// `name:value` AIMIDB/ACFTB overrides are null-terminated arrays of C strings
/// (or null when unused).
#[repr(C)]
pub struct pdal_nitf_write_options_t {
    pub file_title: *const c_char,
    pub complexity_level: *const c_char,
    pub system_type: *const c_char,
    pub origin_station_id: *const c_char,
    pub file_class: *const c_char,
    pub origin_name: *const c_char,
    pub origin_phone: *const c_char,
    pub fsclsy: *const c_char,
    pub fsctlh: *const c_char,
    pub fscltx: *const c_char,
    pub image_security_class: *const c_char,
    pub image_date_time: *const c_char,
    pub image_id2: *const c_char,
    pub aimidb: *const *const c_char,
    pub acftb: *const *const c_char,
    pub minx: f64,
    pub miny: f64,
    pub maxx: f64,
    pub maxy: f64,
}

/// Wrap an existing LAS/BPF payload at `input_path` as a NITF file at
/// `output_path`, applying the supplied writer options.
///
/// # Safety
///
/// `input_path` and `output_path` must be valid NUL-terminated C strings.
/// `opts` must point to a fully-initialized `pdal_nitf_write_options_t`. Any
/// non-null `aimidb`/`acftb` entries must terminate with a null pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_nitf_write(
    input_path: *const c_char,
    output_path: *const c_char,
    opts: *const pdal_nitf_write_options_t,
) -> bool {
    ffi_catch(false, || {
        let input = c_string_lossy(input_path);
        let output = c_string_lossy(output_path);
        if input.is_empty() || output.is_empty() {
            set_last_error("pdal_nitf_write: null/empty input or output path");
            return false;
        }
        let Some(opts) = opts.as_ref() else {
            set_last_error("pdal_nitf_write: null options");
            return false;
        };

        let owned = pdal_native::nitf::NitfWriteOptions {
            file_title: optional_cstr(opts.file_title),
            complexity_level: optional_cstr(opts.complexity_level),
            system_type: optional_cstr(opts.system_type),
            origin_station_id: optional_cstr(opts.origin_station_id),
            file_class: optional_cstr(opts.file_class),
            origin_name: optional_cstr(opts.origin_name),
            origin_phone: optional_cstr(opts.origin_phone),
            fsclsy: optional_cstr(opts.fsclsy),
            fsctlh: optional_cstr(opts.fsctlh),
            fscltx: optional_cstr(opts.fscltx),
            image_security_class: optional_cstr(opts.image_security_class),
            image_date_time: optional_cstr(opts.image_date_time),
            image_id2: optional_cstr(opts.image_id2),
            aimidb: collect_null_terminated(opts.aimidb),
            acftb: collect_null_terminated(opts.acftb),
            minx: opts.minx,
            miny: opts.miny,
            maxx: opts.maxx,
            maxy: opts.maxy,
        };

        match pdal_native::nitf::write(&input, &output, &owned) {
            Ok(()) => true,
            Err(err) => {
                set_last_error(err);
                false
            }
        }
    })
}

unsafe fn optional_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        let s = c_string_lossy(ptr);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

unsafe fn collect_null_terminated(ptr: *const *const c_char) -> Vec<String> {
    if ptr.is_null() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut cursor = ptr;
    loop {
        let item = *cursor;
        if item.is_null() {
            break;
        }
        out.push(c_string_lossy(item));
        cursor = cursor.add(1);
    }
    out
}
