use pdal_core::utils::{
    base64_decode, base64_encode, charbuf_seekoff, charbuf_seekpos, compare_approx,
    escape_json, escape_nonprinting_bytes, format_f64, format_i32, get_env, iequals,
    looks_like_json, normalize_longitude, random, random_seed, replace_all, set_env,
    simple_wordexp, split2_char, split_char, starts_with, to_lower, to_upper, trim_leading,
    trim_trailing, unset_env, word_wrap, word_wrap2,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr;

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_is_json(value: *const c_char) -> bool {
    if value.is_null() {
        return false;
    }
    looks_like_json(&CStr::from_ptr(value).to_string_lossy())
}

fn string_to_c(value: String) -> *mut c_char {
    CString::new(value).unwrap_or_default().into_raw()
}

fn bytes_to_c(value: Vec<u8>) -> *mut c_char {
    CString::new(value).unwrap_or_default().into_raw()
}

fn string_list_to_c(values: Vec<String>) -> *mut c_char {
    string_to_c(serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_string()))
}

unsafe fn c_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

unsafe fn c_bytes(ptr: *const c_char) -> Vec<u8> {
    if ptr.is_null() {
        Vec::new()
    } else {
        CStr::from_ptr(ptr).to_bytes().to_vec()
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_trim_leading(value: *const c_char) -> *mut c_char {
    string_to_c(trim_leading(&c_string(value)))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_trim_trailing(value: *const c_char) -> *mut c_char {
    string_to_c(trim_trailing(&c_string(value)))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_replace_all(
    value: *const c_char,
    replace_what: *const c_char,
    replace_with: *const c_char,
) -> *mut c_char {
    string_to_c(replace_all(
        &c_string(value),
        &c_string(replace_what),
        &c_string(replace_with),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_to_lower(value: *const c_char) -> *mut c_char {
    string_to_c(to_lower(&c_string(value)))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_to_upper(value: *const c_char) -> *mut c_char {
    string_to_c(to_upper(&c_string(value)))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_iequals(left: *const c_char, right: *const c_char) -> bool {
    iequals(&c_string(left), &c_string(right))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_starts_with(
    value: *const c_char,
    prefix: *const c_char,
) -> bool {
    starts_with(&c_string(value), &c_string(prefix))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_split_char(value: *const c_char, split: c_char) -> *mut c_char {
    let split = split as u8 as char;
    string_list_to_c(split_char(&c_string(value), split))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_split2_char(
    value: *const c_char,
    split: c_char,
) -> *mut c_char {
    let split = split as u8 as char;
    string_list_to_c(split2_char(&c_string(value), split))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_escape_json(value: *const c_char) -> *mut c_char {
    string_to_c(escape_json(&c_string(value)))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_escape_nonprinting(value: *const c_char) -> *mut c_char {
    bytes_to_c(escape_nonprinting_bytes(&c_bytes(value)))
}

#[no_mangle]
pub extern "C" fn pdal_utils_normalize_longitude(longitude: f64) -> f64 {
    normalize_longitude(longitude)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_word_wrap(
    value: *const c_char,
    line_length: u64,
    first_length: u64,
) -> *mut c_char {
    string_list_to_c(word_wrap(
        &c_string(value),
        line_length as usize,
        first_length as usize,
    ))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_word_wrap2(
    value: *const c_char,
    line_length: u64,
    first_length: u64,
) -> *mut c_char {
    string_list_to_c(word_wrap2(
        &c_string(value),
        line_length as usize,
        first_length as usize,
    ))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_simple_wordexp(value: *const c_char) -> *mut c_char {
    string_list_to_c(simple_wordexp(&c_string(value)))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_base64_encode(bytes: *const u8, len: u64) -> *mut c_char {
    if len == 0 {
        return string_to_c(String::new());
    }
    if bytes.is_null() && len != 0 {
        return string_to_c(String::new());
    }
    let bytes = std::slice::from_raw_parts(bytes, len as usize);
    string_to_c(base64_encode(bytes))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_base64_decode(
    value: *const c_char,
    out_len: *mut u64,
) -> *mut u8 {
    let decoded = base64_decode(&c_string(value));
    if !out_len.is_null() {
        *out_len = decoded.len() as u64;
    }
    if decoded.is_empty() {
        return ptr::null_mut();
    }
    let mut decoded = decoded.into_boxed_slice();
    let ptr = decoded.as_mut_ptr();
    std::mem::forget(decoded);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn pdal_u8_array_free(ptr: *mut u8, len: u64) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len as usize, len as usize));
    }
}

#[no_mangle]
pub extern "C" fn pdal_charbuf_seekpos(pos: i64, offset: i64, len: i64, for_output: bool) -> i64 {
    charbuf_seekpos(pos, offset, len, for_output).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn pdal_charbuf_seekoff(
    off: i64,
    dir: u8,
    offset: i64,
    len: i64,
    current: i64,
) -> i64 {
    charbuf_seekoff(off, dir, offset, len, current).unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_getenv(name: *const c_char) -> *mut c_char {
    if name.is_null() {
        return ptr::null_mut();
    }
    match get_env(&c_string(name)) {
        Some(value) => string_to_c(value),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_setenv(name: *const c_char, value: *const c_char) -> i32 {
    if name.is_null() || value.is_null() {
        return -1;
    }
    set_env(&c_string(name), &c_string(value))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_unsetenv(name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }
    unset_env(&c_string(name))
}

#[no_mangle]
pub extern "C" fn pdal_utils_random_seed(seed: u32) {
    random_seed(seed);
}

#[no_mangle]
pub extern "C" fn pdal_utils_random(minimum: f64, maximum: f64) -> f64 {
    random(minimum, maximum)
}

#[no_mangle]
pub extern "C" fn pdal_utils_compare_approx(v1: f64, v2: f64, tolerance: f64) -> bool {
    compare_approx(v1, v2, tolerance)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_utils_to_string_f64(value: f64, precision: u32) -> *mut c_char {
    string_to_c(format_f64(value, precision))
}

#[no_mangle]
pub extern "C" fn pdal_utils_to_string_i32(value: i32) -> *mut c_char {
    string_to_c(format_i32(value))
}

fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn current_dir_with_slash() -> String {
    let mut cwd = path_string(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if !cwd.ends_with('/') && !cwd.ends_with('\\') {
        cwd.push('/');
    }
    cwd
}

fn absolute_path(path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn add_trailing_slash(mut path: String) -> String {
    if !path.ends_with('/') && !path.ends_with('\\') {
        path.push('/');
    }
    path
}

fn filename_string(path: &str) -> String {
    if path.is_empty() || path.ends_with('/') || path.ends_with('\\') {
        return String::new();
    }
    if path == "." || path == ".." {
        return path.to_string();
    }
    Path::new(path)
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn pdal_file_utils_getcwd() -> *mut c_char {
    string_to_c(current_dir_with_slash())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_to_absolute_path(filename: *const c_char) -> *mut c_char {
    string_to_c(path_string(absolute_path(&c_string(filename))))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_to_absolute_path_with_base(
    filename: *const c_char,
    base: *const c_char,
) -> *mut c_char {
    let filename = c_string(filename);
    let base = path_string(absolute_path(&c_string(base)));
    string_to_c(path_string(PathBuf::from(base).join(filename)))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_get_filename(path: *const c_char) -> *mut c_char {
    string_to_c(filename_string(&c_string(path)))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_get_directory(path: *const c_char) -> *mut c_char {
    let path = c_string(path);
    let directory = Path::new(&path)
        .parent()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    string_to_c(add_trailing_slash(directory))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_stem(path: *const c_char) -> *mut c_char {
    let filename = filename_string(&c_string(path));
    if filename == "." || filename == ".." {
        return string_to_c(filename);
    }
    let stem = filename
        .rfind('.')
        .map(|idx| filename[..idx].to_string())
        .unwrap_or(filename);
    string_to_c(stem)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_extension(path: *const c_char) -> *mut c_char {
    let path = c_string(path);
    let extension = path
        .rfind('.')
        .map(|idx| path[idx..].to_string())
        .unwrap_or_default();
    string_to_c(extension)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_is_absolute_path(path: *const c_char) -> bool {
    let path = c_string(path);
    path.contains("://") || Path::new(&path).is_absolute()
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_directory_exists(dirname: *const c_char) -> bool {
    let path_str = c_string(dirname);
    let path = Path::new(&path_str);
    path.is_dir()
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_create_directory(dirname: *const c_char) -> i32 {
    let path_str = c_string(dirname);
    let path = Path::new(&path_str);
    match std::fs::create_dir(path) {
        Ok(_) => 1,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                0
            } else {
                crate::error::set_last_error(e.to_string());
                -1
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_create_directories(path: *const c_char) -> i32 {
    let path_str = c_string(path);
    let path = Path::new(&path_str);
    if path.is_dir() {
        return 0;
    }
    match std::fs::create_dir_all(path) {
        Ok(_) => 1,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                0
            } else {
                crate::error::set_last_error(e.to_string());
                -1
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_delete_directory(dirname: *const c_char) {
    let path_str = c_string(dirname);
    let path = Path::new(&path_str);
    let _ = std::fs::remove_dir_all(path);
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_delete_file(filename: *const c_char) -> bool {
    let path_str = c_string(filename);
    let path = Path::new(&path_str);
    std::fs::remove_file(path).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn pdal_file_utils_rename_file(dest: *const c_char, src: *const c_char) {
    let dest_str = c_string(dest);
    let src_str = c_string(src);
    let _ = std::fs::rename(Path::new(&src_str), Path::new(&dest_str));
}


#[repr(C)]
pub struct pdal_xyz_t {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[repr(C)]
pub struct pdal_rotation_matrix_t {
    pub m00: f64,
    pub m01: f64,
    pub m02: f64,
    pub m10: f64,
    pub m11: f64,
    pub m12: f64,
    pub m20: f64,
    pub m21: f64,
    pub m22: f64,
}

fn rotate(point: pdal_xyz_t, matrix: &pdal_rotation_matrix_t) -> pdal_xyz_t {
    pdal_xyz_t {
        x: matrix.m00 * point.x + matrix.m01 * point.y + matrix.m02 * point.z,
        y: matrix.m10 * point.x + matrix.m11 * point.y + matrix.m12 * point.z,
        z: matrix.m20 * point.x + matrix.m21 * point.y + matrix.m22 * point.z,
    }
}

#[no_mangle]
pub extern "C" fn pdal_georeference_wgs84(
    range: f64,
    scan_angle: f64,
    boresight: pdal_rotation_matrix_t,
    imu: pdal_rotation_matrix_t,
    gps: pdal_xyz_t,
) -> pdal_xyz_t {
    const A: f64 = 6_378_137.0;
    const F: f64 = 1.0 / 298.257_223_563;
    let e2 = 2.0 * F - F * F;

    let p_socs = pdal_xyz_t {
        x: range * scan_angle.sin(),
        y: 0.0,
        z: -range * scan_angle.cos(),
    };
    let p_socs_aligned = rotate(p_socs, &boresight);
    let p_local_level = rotate(p_socs_aligned, &imu);
    let w = (1.0 - e2 * gps.y.sin() * gps.y.sin()).sqrt();
    let n = A / w;
    let m = A * (1.0 - e2) / (w * w * w);
    let p_curvilinear = pdal_xyz_t {
        x: p_local_level.x / (n * gps.y.cos()),
        y: p_local_level.y / m,
        z: p_local_level.z,
    };

    pdal_xyz_t {
        x: gps.x + p_curvilinear.x,
        y: gps.y + p_curvilinear.y,
        z: gps.z + p_curvilinear.z,
    }
}

fn mag2(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    (x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1)
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn pdal_barycentric_interpolation(
    x1: f64,
    y1: f64,
    z1: f64,
    x2: f64,
    y2: f64,
    z2: f64,
    x3: f64,
    y3: f64,
    z3: f64,
    x: f64,
    y: f64,
) -> f64 {
    let area_total = ((x2 - x1) * (y3 - y2)) - ((y2 - y1) * (x3 - x2));
    if area_total == 0.0 {
        return f64::INFINITY;
    }

    let sign_total = area_total.is_sign_negative();
    let almost_zero = 1e-14;

    let mut area12 = (x2 - x1) * (y - y1) - (y2 - y1) * (x - x1);
    if area12 != 0.0 && area12.is_sign_negative() != sign_total {
        let magsum = mag2(x1, y1, x2, y2) + mag2(x1, y1, x, y);
        if (area12 / magsum).abs() > almost_zero {
            return f64::INFINITY;
        }
        area12 = 0.0;
    }

    let mut area23 = (x3 - x2) * (y - y2) - (y3 - y2) * (x - x2);
    if area23 != 0.0 && area23.is_sign_negative() != sign_total {
        let magsum = mag2(x3, y3, x2, y2) + mag2(x3, y3, x, y);
        if (area23 / magsum).abs() > almost_zero {
            return f64::INFINITY;
        }
        area23 = 0.0;
    }

    let mut area31 = (x1 - x3) * (y - y3) - (y1 - y3) * (x - x3);
    if area31 != 0.0 && area31.is_sign_negative() != sign_total {
        let magsum = mag2(x3, y3, x1, y1) + mag2(x3, y3, x, y);
        if (area31 / magsum).abs() > almost_zero {
            return f64::INFINITY;
        }
        area31 = 0.0;
    }

    (area12 * z3 + area23 * z1 + area31 * z2) / area_total
}
