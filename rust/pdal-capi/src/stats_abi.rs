use crate::error::{clear_last_error, ffi_catch, set_last_error};
use crate::stage_abi::StageWrapper;
use pdal_core::metadata::MetadataNode;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::Filter;
use pdal_filters::expressionstats::ExpressionStatsFilter as ExpressionStatsMetadataFilter;
use pdal_filters::stats;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Dim Stats representation for FFI.
#[repr(C)]
pub struct pdal_dim_stats_t {
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub m1: f64,
    pub m2: f64,
    pub m3: f64,
    pub m4: f64,
    pub median: f64,
    pub mad: f64,
    pub unique_values: *mut f64,
    pub unique_counts: *mut u64,
    pub unique_len: u64,
}

/// Compute statistics on a PointView.
///
/// # Safety
///
/// Pre-allocated arrays and bounds must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdal_stats_compute(
    view: *mut PointView,
    dims: *const *const c_char,
    dims_count: u64,
    advanced: bool,
    enums: *const *const c_char,
    enums_count: u64,
    counts: *const *const c_char,
    counts_count: u64,
    globals: *const *const c_char,
    globals_count: u64,
    out_stats: *mut pdal_dim_stats_t,
) {
    if view.is_null() || dims.is_null() || out_stats.is_null() {
        return;
    }
    let pt_view = &mut *view;

    let mut enum_names = std::collections::HashSet::new();
    for i in 0..enums_count {
        let ptr = *enums.offset(i as isize);
        if !ptr.is_null() {
            enum_names.insert(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }
    }

    let mut count_names = std::collections::HashSet::new();
    for i in 0..counts_count {
        let ptr = *counts.offset(i as isize);
        if !ptr.is_null() {
            count_names.insert(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }
    }

    let mut global_names = std::collections::HashSet::new();
    for i in 0..globals_count {
        let ptr = *globals.offset(i as isize);
        if !ptr.is_null() {
            global_names.insert(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }
    }

    for i in 0..dims_count {
        let ptr = *dims.offset(i as isize);
        if ptr.is_null() {
            continue;
        }
        let dim_name = CStr::from_ptr(ptr).to_string_lossy().into_owned();

        let enum_type = if global_names.contains(&dim_name) {
            3
        } else if count_names.contains(&dim_name) {
            2
        } else if enum_names.contains(&dim_name) {
            1
        } else {
            0
        };

        let mut summary = stats::Summary::new(dim_name.clone(), enum_type, advanced);
        let dim_id = DimId::from_name(&dim_name);
        for pt_idx in 0..pt_view.len() {
            let val = pt_view.get_f64(pt_idx, &dim_id);
            summary.insert(val);
        }
        if enum_type == 3 {
            summary.compute_global_stats();
        }

        let mut unique_values_ptr = std::ptr::null_mut();
        let mut unique_counts_ptr = std::ptr::null_mut();
        let mut unique_len = 0;

        if enum_type == 1 || enum_type == 2 {
            let mut keys: Vec<f64> = summary
                .values
                .keys()
                .map(|&bits| f64::from_bits(bits))
                .collect();
            keys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            unique_len = keys.len() as u64;

            let mut vals = Vec::new();
            for &k in &keys {
                let bits = k.to_bits();
                vals.push(*summary.values.get(&bits).unwrap_or(&0));
            }

            let mut boxed_keys = keys.into_boxed_slice();
            unique_values_ptr = boxed_keys.as_mut_ptr();
            std::mem::forget(boxed_keys);

            let mut boxed_vals = vals.into_boxed_slice();
            unique_counts_ptr = boxed_vals.as_mut_ptr();
            std::mem::forget(boxed_vals);
        }

        *out_stats.offset(i as isize) = pdal_dim_stats_t {
            count: summary.cnt,
            min: summary.min,
            max: summary.max,
            m1: summary.m1,
            m2: summary.m2,
            m3: summary.m3,
            m4: summary.m4,
            median: summary.median,
            mad: summary.mad,
            unique_values: unique_values_ptr,
            unique_counts: unique_counts_ptr,
            unique_len,
        };
    }
}

/// Free the allocated arrays within `pdal_dim_stats_t`.
///
/// # Safety
///
/// Always safe if pointers match allocated memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_free_stats_arrays(ptr: *mut pdal_dim_stats_t, dims_count: u64) {
    if ptr.is_null() {
        return;
    }
    for i in 0..dims_count {
        let stats = &*ptr.offset(i as isize);
        if !stats.unique_values.is_null() {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                stats.unique_values,
                stats.unique_len as usize,
            ));
        }
        if !stats.unique_counts.is_null() {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                stats.unique_counts,
                stats.unique_len as usize,
            ));
        }
    }
}

/// Compute expression statistics metadata.
///
/// # Safety
///
/// `view` must be a valid point view. `dim_name` must be a valid
/// NUL-terminated C string. `expressions` must point to `count`
/// NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn pdal_expressionstats_metadata(
    view: *mut PointView,
    dim_name: *const c_char,
    expressions: *const *const c_char,
    count: u64,
) -> *mut MetadataNode {
    ffi_catch(std::ptr::null_mut(), || {
        clear_last_error();
        if view.is_null() || dim_name.is_null() || (count > 0 && expressions.is_null()) {
            set_last_error("null expressionstats input");
            return std::ptr::null_mut();
        }

        let Some(view) = view.as_ref() else {
            set_last_error("null point view");
            return std::ptr::null_mut();
        };
        let dim_name = CStr::from_ptr(dim_name).to_string_lossy().into_owned();
        let mut sources = Vec::new();
        for i in 0..count {
            let ptr = *expressions.offset(i as isize);
            if ptr.is_null() {
                set_last_error("null expression string");
                return std::ptr::null_mut();
            }
            sources.push(CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }

        match ExpressionStatsMetadataFilter::new(&dim_name, &sources) {
            Ok(mut filter) => {
                if let Err(e) = Filter::run(&mut filter, view) {
                    set_last_error(e.to_string());
                    std::ptr::null_mut()
                } else {
                    Box::into_raw(Box::new(Filter::metadata(&filter)))
                }
            }
            Err(e) => {
                set_last_error(e.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

/// Create a `filters.reprojection` stage.
///
/// # Safety
///
/// `out_srs`, `in_srs` must be valid null-terminated C strings (in_srs can be null).
#[no_mangle]
pub unsafe extern "C" fn pdal_stage_create_reprojection(
    out_srs: *const c_char,
    in_srs: *const c_char,
    error_on_failure: bool,
) -> *mut StageWrapper {
    clear_last_error();
    if out_srs.is_null() {
        set_last_error("null output srs ");
        return std::ptr::null_mut();
    }
    let out_srs_text = CStr::from_ptr(out_srs).to_string_lossy();
    let in_srs_text = if in_srs.is_null() {
        None
    } else {
        Some(CStr::from_ptr(in_srs).to_string_lossy().into_owned())
    };

    let filter = Box::new(pdal_filters::reprojection::ReprojectionFilter::new(
        &out_srs_text,
        in_srs_text,
        error_on_failure,
    ));
    Box::into_raw(Box::new(StageWrapper { filter }))
}
