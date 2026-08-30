//! C ABI for point-cloud comparison metrics.

use crate::error::{clear_last_error, set_last_error, string_to_c_ptr};
use crate::registry::create_reader;
use pdal_core::options::Options;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::StageError;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Read a file's point views into one cloud, inferring the reader driver from
/// the path.
pub(crate) fn read_cloud(path: &str) -> Result<PointView, StageError> {
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
#[pdal_capi_macros::ffi_export(fallback = -1)]
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
#[pdal_capi_macros::ffi_export(fallback = -1)]
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
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_delta(path_a: *const c_char, path_b: *const c_char) -> *mut c_char {
    pdal_delta_ex(path_a, path_b, false, false)
}

/// Extended delta report with C++ `DeltaKernel` option parity.
///
/// `detail` selects per-point deltas instead of min/mean/max summaries.
/// `all_dims` includes every dimension common to both layouts; otherwise only
/// `X`, `Y`, and `Z` are reported.
///
/// # Safety
///
/// `path_a` and `path_b` must be valid NUL-terminated C strings.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_delta_ex(
    path_a: *const c_char,
    path_b: *const c_char,
    detail: bool,
    all_dims: bool,
) -> *mut c_char {
    clear_last_error();
    if path_a.is_null() || path_b.is_null() {
        set_last_error("null argument to pdal_delta_ex");
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

    let dims = delta_dimensions(&source, &candidate, all_dims);
    let report = if detail {
        let mut root = serde_json::Map::new();
        for detail in pdal_core::metrics::delta_details_for_dims(&source, &candidate, &dims) {
            let mut item = serde_json::Map::new();
            item.insert("i".to_string(), serde_json::json!(detail.index));
            for (name, value) in detail.values {
                item.insert(name, serde_json::json!(value));
            }
            root.entry("delta".to_string())
                .or_insert_with(|| serde_json::json!([]))
                .as_array_mut()
                .expect("delta array")
                .push(serde_json::Value::Object(item));
        }
        serde_json::Value::Object(root)
    } else {
        let stats = pdal_core::metrics::delta_summary_for_dims(&source, &candidate, &dims);
        let mut root = serde_json::Map::new();
        root.insert("source".to_string(), serde_json::json!(path_a));
        root.insert("candidate".to_string(), serde_json::json!(path_b));
        for stat in stats {
            root.insert(
                stat.dimension,
                serde_json::json!({ "min": stat.min, "mean": stat.mean, "max": stat.max }),
            );
        }
        serde_json::Value::Object(root)
    };
    string_to_c_ptr(report.to_string())
}

fn delta_dimensions(
    source: &PointView,
    candidate: &PointView,
    all_dims: bool,
) -> Vec<(String, DimId)> {
    if !all_dims {
        return vec![
            ("X".to_string(), DimId::X),
            ("Y".to_string(), DimId::Y),
            ("Z".to_string(), DimId::Z),
        ];
    }

    let mut dims = Vec::new();
    for idx in 0..source.layout().dim_count() {
        let Some((dim, _)) = source.layout().dim_at(idx) else {
            continue;
        };
        if candidate.layout().dim(dim).is_some() {
            dims.push((dim.name().to_string(), dim.clone()));
        }
    }
    dims
}

/// Evaluate predicted classification labels against truth labels.
///
/// For each point of the predicted file, the nearest point of the truth file
/// is found, and the two labels are tallied into a confusion matrix over the
/// comma-separated `labels`. Returns a newly allocated JSON string (free with
/// `pdal_string_free`) reporting per-label and aggregate metrics, or null on
/// error with the message available via `pdal_last_error`.
///
/// # Safety
///
/// All pointer arguments must be valid NUL-terminated C strings.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_eval(
    predicted_path: *const c_char,
    truth_path: *const c_char,
    labels: *const c_char,
    predicted_dim: *const c_char,
    truth_dim: *const c_char,
) -> *mut c_char {
    clear_last_error();
    if predicted_path.is_null()
        || truth_path.is_null()
        || labels.is_null()
        || predicted_dim.is_null()
        || truth_dim.is_null()
    {
        set_last_error("null argument to pdal_eval");
        return std::ptr::null_mut();
    }
    let predicted_path = CStr::from_ptr(predicted_path)
        .to_string_lossy()
        .into_owned();
    let truth_path = CStr::from_ptr(truth_path).to_string_lossy().into_owned();
    let labels_str = CStr::from_ptr(labels).to_string_lossy().into_owned();
    let predicted_dim_name = CStr::from_ptr(predicted_dim).to_string_lossy().into_owned();
    let truth_dim_name = CStr::from_ptr(truth_dim).to_string_lossy().into_owned();

    let mut label_list = Vec::new();
    for token in labels_str
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        match token.parse::<i64>() {
            Ok(value) => label_list.push(value),
            Err(_) => {
                set_last_error(format!("eval: '{token}' is not a valid integer label"));
                return std::ptr::null_mut();
            }
        }
    }
    if label_list.is_empty() {
        set_last_error("eval: must specify a comma-separated list of labels to evaluate");
        return std::ptr::null_mut();
    }

    let predicted = match read_cloud(&predicted_path) {
        Ok(view) => view,
        Err(err) => {
            set_last_error(err.to_string());
            return std::ptr::null_mut();
        }
    };
    let truth = match read_cloud(&truth_path) {
        Ok(view) => view,
        Err(err) => {
            set_last_error(err.to_string());
            return std::ptr::null_mut();
        }
    };
    if predicted.is_empty() || truth.is_empty() {
        set_last_error("eval requires non-empty point clouds");
        return std::ptr::null_mut();
    }

    let predicted_dim_id = DimId::from_name(&predicted_dim_name);
    if predicted.layout().dim(&predicted_dim_id).is_none() {
        set_last_error(format!(
            "eval: predicted dimension '{predicted_dim_name}' does not exist"
        ));
        return std::ptr::null_mut();
    }
    let truth_dim_id = DimId::from_name(&truth_dim_name);
    if truth.layout().dim(&truth_dim_id).is_none() {
        set_last_error(format!(
            "eval: truth dimension '{truth_dim_name}' does not exist"
        ));
        return std::ptr::null_mut();
    }

    let report = pdal_core::metrics::evaluate(
        &predicted,
        &truth,
        &predicted_dim_id,
        &truth_dim_id,
        &label_list,
    );
    let labels_json: Vec<serde_json::Value> = report
        .labels
        .iter()
        .map(|metrics| {
            serde_json::json!({
                "label": metrics.label,
                "support": metrics.support,
                "intersection_over_union": metrics.intersection_over_union,
                "f1_score": metrics.f1_score,
                "sensitivity": metrics.sensitivity,
                "specificity": metrics.specificity,
                "precision": metrics.precision,
                "accuracy": metrics.accuracy,
            })
        })
        .collect();
    let report_json = serde_json::json!({
        "predicted_file": predicted_path,
        "truth_file": truth_path,
        "labels": labels_json,
        "mean_intersection_over_union": report.mean_intersection_over_union,
        "overall_accuracy": report.overall_accuracy,
        "f1_score": report.f1_score,
        "confusion_matrix": report.confusion_matrix,
    });
    string_to_c_ptr(report_json.to_string())
}
