use pdal_core::utils::looks_like_json;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

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

unsafe fn c_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
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
