//! C ABI for point-cloud comparison metrics.

use crate::error::{clear_last_error, set_last_error, string_to_c_ptr};
use crate::registry::create_reader;
use pdal_core::options::Options;
use pdal_core::point::PointView;
use pdal_core::stage::StageError;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Read a file's point views into one cloud, inferring the reader driver from
/// the path.
fn read_cloud(path: &str) -> Result<PointView, StageError> {
    let driver = pdal_core::driver::infer_reader_driver(path)
        .ok_or_else(|| StageError(format!("unable to infer a reader driver for '{path}'")))?;
    let mut options = Options::new();
    options.add("filename", path);
    let mut reader = create_reader(driver, &options)?;
    let views = reader.read()?;
    merge_views(views, path)
}

fn merge_views(mut views: Vec<PointView>, path: &str) -> Result<PointView, StageError> {
    if views.is_empty() {
        return Err(StageError(format!("'{path}' produced no point data")));
    }
    if views.len() == 1 {
        return Ok(views.remove(0));
    }

    let mut merged = views[0].make_new();
    for view in &views {
        ensure_same_layout(&views[0], view, path)?;
        for idx in 0..view.len() {
            merged.append_point(view, idx);
        }
    }
    Ok(merged)
}

fn ensure_same_layout(
    reference: &PointView,
    view: &PointView,
    path: &str,
) -> Result<(), StageError> {
    if reference.layout().dim_count() != view.layout().dim_count()
        || reference.layout().point_size() != view.layout().point_size()
    {
        return Err(StageError(format!(
            "'{path}' produced point views with incompatible layouts"
        )));
    }
    for idx in 0..reference.layout().dim_count() {
        if reference.layout().dim_at(idx) != view.layout().dim_at(idx) {
            return Err(StageError(format!(
                "'{path}' produced point views with incompatible layouts"
            )));
        }
    }
    Ok(())
}

/// Compute the Hausdorff and modified-Hausdorff distances between two files.
///
/// On success returns 0 and writes both distances through the out-pointers.
/// On failure returns -1 with the message available via `pdal_last_error`.
///
/// # Safety
///
/// `path_a` and `path_b` must be valid NUL-terminated C strings.
/// `hausdorff` and `modified_hausdorff` must be valid, writable `double`s.
#[no_mangle]
pub unsafe extern "C" fn pdal_hausdorff(
    path_a: *const c_char,
    path_b: *const c_char,
    hausdorff: *mut f64,
    modified_hausdorff: *mut f64,
) -> i32 {
    clear_last_error();
    if path_a.is_null() || path_b.is_null() || hausdorff.is_null() || modified_hausdorff.is_null() {
        set_last_error("null argument to pdal_hausdorff");
        return -1;
    }
    let path_a = CStr::from_ptr(path_a).to_string_lossy().into_owned();
    let path_b = CStr::from_ptr(path_b).to_string_lossy().into_owned();

    let view_a = match read_cloud(&path_a) {
        Ok(view) => view,
        Err(err) => {
            set_last_error(err.to_string());
            return -1;
        }
    };
    let view_b = match read_cloud(&path_b) {
        Ok(view) => view,
        Err(err) => {
            set_last_error(err.to_string());
            return -1;
        }
    };
    if view_a.is_empty() || view_b.is_empty() {
        set_last_error("hausdorff requires non-empty point clouds");
        return -1;
    }

    let (original, modified) = pdal_core::metrics::hausdorff_pair(&view_a, &view_b);
    *hausdorff = original;
    *modified_hausdorff = modified;
    0
}

/// Compute the Chamfer distance between two point cloud files.
///
/// On success returns 0 and writes the distance through `chamfer`. On failure
/// returns -1 with the message available via `pdal_last_error`.
///
/// # Safety
///
/// `path_a` and `path_b` must be valid NUL-terminated C strings.
/// `chamfer` must be a valid, writable `double`.
#[no_mangle]
pub unsafe extern "C" fn pdal_chamfer(
    path_a: *const c_char,
    path_b: *const c_char,
    chamfer: *mut f64,
) -> i32 {
    clear_last_error();
    if path_a.is_null() || path_b.is_null() || chamfer.is_null() {
        set_last_error("null argument to pdal_chamfer");
        return -1;
    }
    let path_a = CStr::from_ptr(path_a).to_string_lossy().into_owned();
    let path_b = CStr::from_ptr(path_b).to_string_lossy().into_owned();

    let view_a = match read_cloud(&path_a) {
        Ok(view) => view,
        Err(err) => {
            set_last_error(err.to_string());
            return -1;
        }
    };
    let view_b = match read_cloud(&path_b) {
        Ok(view) => view,
        Err(err) => {
            set_last_error(err.to_string());
            return -1;
        }
    };
    if view_a.is_empty() || view_b.is_empty() {
        set_last_error("chamfer requires non-empty point clouds");
        return -1;
    }

    *chamfer = pdal_core::metrics::chamfer_distance(&view_a, &view_b);
    0
}

/// Compute per-dimension `X`/`Y`/`Z` delta statistics between two files.
///
/// Returns a newly allocated JSON string (free with `pdal_string_free`), or
/// null on error with the message available via `pdal_last_error`.
///
/// # Safety
///
/// `path_a` and `path_b` must be valid NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_delta(path_a: *const c_char, path_b: *const c_char) -> *mut c_char {
    clear_last_error();
    if path_a.is_null() || path_b.is_null() {
        set_last_error("null argument to pdal_delta");
        return std::ptr::null_mut();
    }
    let path_a = CStr::from_ptr(path_a).to_string_lossy().into_owned();
    let path_b = CStr::from_ptr(path_b).to_string_lossy().into_owned();

    let source = match read_cloud(&path_a) {
        Ok(view) => view,
        Err(err) => {
            set_last_error(err.to_string());
            return std::ptr::null_mut();
        }
    };
    let candidate = match read_cloud(&path_b) {
        Ok(view) => view,
        Err(err) => {
            set_last_error(err.to_string());
            return std::ptr::null_mut();
        }
    };
    if source.is_empty() || candidate.is_empty() {
        set_last_error("delta requires non-empty point clouds");
        return std::ptr::null_mut();
    }

    let stats = pdal_core::metrics::delta_summary(&source, &candidate);
    let dim_json = |stat: &pdal_core::metrics::DeltaStat| serde_json::json!({ "min": stat.min, "mean": stat.mean, "max": stat.max });
    let report = serde_json::json!({
        "source": path_a,
        "candidate": path_b,
        "X": dim_json(&stats[0]),
        "Y": dim_json(&stats[1]),
        "Z": dim_json(&stats[2]),
    });
    string_to_c_ptr(report.to_string())
}
