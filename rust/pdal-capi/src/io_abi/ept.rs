use super::cstr_to_str;
use crate::error::{clear_last_error, set_last_error, string_to_c_ptr};
use crate::point_abi::pdal_bounds3d_t;
use pdal_core::options::Options;
use std::ffi::{c_char, CStr};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_ept_key_t {
    pub d: u32,
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub bounds: pdal_bounds3d_t,
}

#[no_mangle]
pub unsafe extern "C" fn pdal_ept_key_parse(
    value: *const c_char,
    out_key: *mut pdal_ept_key_t,
) -> bool {
    let (Some(value), Some(out_key)) = (cstr_to_str(value), out_key.as_mut()) else {
        return false;
    };
    let Some(key) = parse_ept_key(value) else {
        return false;
    };
    *out_key = key;
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_ept_key_to_string(key: *const pdal_ept_key_t) -> *mut c_char {
    let Some(key) = key.as_ref() else {
        return string_to_c_ptr(String::new());
    };
    string_to_c_ptr(format!("{}-{}-{}-{}", key.d, key.x, key.y, key.z))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_ept_key_bisect(
    key: *const pdal_ept_key_t,
    direction: u64,
    out_key: *mut pdal_ept_key_t,
) -> bool {
    let (Some(key), Some(out_key)) = (key.as_ref(), out_key.as_mut()) else {
        return false;
    };
    *out_key = bisect_ept_key(*key, direction);
    true
}

fn parse_ept_key(value: &str) -> Option<pdal_ept_key_t> {
    let mut parts = value.split('-');
    let d = parts.next()?.parse().ok()?;
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let z = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(pdal_ept_key_t {
        d,
        x,
        y,
        z,
        bounds: empty_abi_bounds3d(),
    })
}

fn bisect_ept_key(mut key: pdal_ept_key_t, direction: u64) -> pdal_ept_key_t {
    key.d += 1;
    step_ept_key_axis(
        &mut key.x,
        &mut key.bounds.minx,
        &mut key.bounds.maxx,
        direction,
        0,
    );
    step_ept_key_axis(
        &mut key.y,
        &mut key.bounds.miny,
        &mut key.bounds.maxy,
        direction,
        1,
    );
    step_ept_key_axis(
        &mut key.z,
        &mut key.bounds.minz,
        &mut key.bounds.maxz,
        direction,
        2,
    );
    key
}

fn step_ept_key_axis(id: &mut u32, min: &mut f64, max: &mut f64, direction: u64, axis: u8) {
    *id *= 2;
    let mid = *min + ((*max - *min) / 2.0);
    if (direction & (1_u64 << axis)) != 0 {
        *min = mid;
        *id += 1;
    } else {
        *max = mid;
    }
}

fn empty_abi_bounds3d() -> pdal_bounds3d_t {
    pdal_bounds3d_t {
        minx: f64::MAX,
        maxx: f64::MIN,
        miny: f64::MAX,
        maxy: f64::MIN,
        minz: f64::MAX,
        maxz: f64::MIN,
    }
}

/// Validate an EPT origin option through the Rust reader implementation.
///
/// # Safety
/// `filename` and `origin` must be valid null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_validate_origin(
    filename: *const c_char,
    origin: *const c_char,
) -> bool {
    if filename.is_null() || origin.is_null() {
        set_last_error("Missing EPT origin validation input.");
        return false;
    }
    let filename = CStr::from_ptr(filename).to_string_lossy().into_owned();
    let origin = CStr::from_ptr(origin).to_string_lossy().into_owned();
    let mut options = Options::new();
    options.add("filename", filename);
    options.add("origin", origin);
    match pdal_io::ept::EptReader::new(&options).validate_origin() {
        Ok(()) => true,
        Err(err) => {
            set_last_error(err.0);
            false
        }
    }
}

/// Validate an EPT bounds option through the Rust reader implementation.
///
/// # Safety
/// `filename` and `bounds` must be valid null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_validate_bounds(
    filename: *const c_char,
    bounds: *const c_char,
) -> bool {
    if filename.is_null() || bounds.is_null() {
        set_last_error("Missing EPT bounds validation input.");
        return false;
    }
    let filename = CStr::from_ptr(filename).to_string_lossy().into_owned();
    let bounds = CStr::from_ptr(bounds).to_string_lossy().into_owned();
    let mut options = Options::new();
    options.add("filename", filename);
    options.add("bounds", bounds);
    match pdal_io::ept::EptReader::new(&options).validate_bounds() {
        Ok(()) => true,
        Err(err) => {
            set_last_error(err.0);
            false
        }
    }
}

/// Build the SRS WKT/user-input string from an EPT info JSON document,
/// matching the C++ `EptInfo::initialize()` rules.
///
/// On success returns `true`. When the info has a usable `srs`, `*out_wkt` is
/// set to a newly-allocated string (free with `pdal_string_free`); when no
/// `srs` is present `*out_wkt` is set to null. On a parse or validation error,
/// returns `false`, sets the last error, and leaves `*out_wkt` null.
///
/// # Safety
/// `info_json` must be a valid null-terminated C string and `out_wkt` a valid
/// pointer to a `char*`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_srs_wkt_from_info(
    info_json: *const c_char,
    out_wkt: *mut *mut c_char,
) -> bool {
    let Some(out_wkt) = out_wkt.as_mut() else {
        set_last_error("Missing EPT srs output pointer.");
        return false;
    };
    *out_wkt = std::ptr::null_mut();
    if info_json.is_null() {
        set_last_error("Missing EPT info JSON.");
        return false;
    }
    let info_json = CStr::from_ptr(info_json).to_string_lossy().into_owned();
    let info: serde_json::Value = match serde_json::from_str(&info_json) {
        Ok(value) => value,
        Err(_) => {
            set_last_error("Unable to parse EPT info as JSON.");
            return false;
        }
    };
    match pdal_io::ept::ept_srs_wkt(&info) {
        Ok(Some(wkt)) => {
            *out_wkt = string_to_c_ptr(wkt);
            true
        }
        Ok(None) => true,
        Err(err) => {
            set_last_error(err.0);
            false
        }
    }
}

/// Opaque handle holding the result of a Rust EPT reader preview.
pub struct EptReaderPreviewHandle {
    pub(crate) preview: pdal_io::ept::EptPreview,
}

/// Read EPT preview metadata (boundsConforming, point count, srs wkt, dim
/// names) from a local `ept.json` file. Returns null on error; call
/// `pdal_last_error` for the message. Caller frees with
/// `pdal_ept_reader_preview_destroy`.
///
/// # Safety
/// `filename` must be a valid NUL-terminated C string pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_create(
    filename: *const c_char,
) -> *mut EptReaderPreviewHandle {
    pdal_ept_reader_preview_create_with_options(filename, std::ptr::null())
}

/// Read EPT preview metadata with supported preview-only options.
///
/// # Safety
/// `filename` must be a valid NUL-terminated C string pointer. `resolution`
/// may be null or a valid NUL-terminated C string pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_create_with_options(
    filename: *const c_char,
    resolution: *const c_char,
) -> *mut EptReaderPreviewHandle {
    if filename.is_null() {
        set_last_error("null filename");
        return std::ptr::null_mut();
    }
    let Ok(filename) = CStr::from_ptr(filename).to_str() else {
        set_last_error("non-UTF8 filename");
        return std::ptr::null_mut();
    };
    let resolution = if resolution.is_null() {
        ""
    } else {
        match CStr::from_ptr(resolution).to_str() {
            Ok(resolution) => resolution,
            Err(_) => {
                set_last_error("non-UTF8 resolution");
                return std::ptr::null_mut();
            }
        }
    };
    match pdal_io::ept::read_ept_preview_with_options(filename, resolution) {
        Ok(preview) => {
            clear_last_error();
            Box::into_raw(Box::new(EptReaderPreviewHandle { preview }))
        }
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Get the preview's point count.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_point_count(
    handle: *const EptReaderPreviewHandle,
) -> u64 {
    handle.as_ref().map_or(0, |h| h.preview.point_count)
}

/// Get the preview's bounds_conforming. Writes into `out` and returns true
/// on success.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`. `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_bounds(
    handle: *const EptReaderPreviewHandle,
    out_minx: *mut f64,
    out_miny: *mut f64,
    out_minz: *mut f64,
    out_maxx: *mut f64,
    out_maxy: *mut f64,
    out_maxz: *mut f64,
) -> bool {
    let Some(handle) = handle.as_ref() else {
        return false;
    };
    let b = &handle.preview.bounds_conforming;
    *out_minx = b.minx;
    *out_miny = b.miny;
    *out_minz = b.minz;
    *out_maxx = b.maxx;
    *out_maxy = b.maxy;
    *out_maxz = b.maxz;
    true
}

/// Get the preview's SRS WKT string. Returns an owned C string (possibly
/// empty). Caller frees with `pdal_string_free`.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_srs_wkt(
    handle: *const EptReaderPreviewHandle,
) -> *mut c_char {
    let Some(handle) = handle.as_ref() else {
        return std::ptr::null_mut();
    };
    string_to_c_ptr(handle.preview.srs_wkt.clone())
}

/// Get the number of dim names.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_dim_count(
    handle: *const EptReaderPreviewHandle,
) -> u64 {
    handle
        .as_ref()
        .map_or(0, |h| h.preview.dim_names.len() as u64)
}

/// Get a dim name by index. Returns an owned C string or null when the index
/// is out of range. Caller frees with `pdal_string_free`.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_dim_name(
    handle: *const EptReaderPreviewHandle,
    index: u64,
) -> *mut c_char {
    let Some(handle) = handle.as_ref() else {
        return std::ptr::null_mut();
    };
    let Some(name) = handle.preview.dim_names.get(index as usize) else {
        return std::ptr::null_mut();
    };
    string_to_c_ptr(name.clone())
}

/// Destroy an EPT preview handle.
///
/// # Safety
/// `handle` must be a valid pointer returned by
/// `pdal_ept_reader_preview_create`, or null.
#[no_mangle]
pub unsafe extern "C" fn pdal_ept_reader_preview_destroy(handle: *mut EptReaderPreviewHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}
