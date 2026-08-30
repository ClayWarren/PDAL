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
#[pdal_capi_macros::ffi_export]
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
#[pdal_capi_macros::ffi_export]
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
#[pdal_capi_macros::ffi_export]
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
                if let Err(e) = Filter::run(&mut filter, std::slice::from_ref(view)) {
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
#[pdal_capi_macros::ffi_export]
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

#[derive(Clone, Copy)]
#[repr(C)]
pub struct pdal_summary_merge_entry_t {
    pub value: f64,
    pub count: u64,
}

#[repr(C)]
pub struct pdal_summary_merge_state_t {
    pub name: *const c_char,
    pub enumerate: u32,
    pub advanced: bool,
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub m1: f64,
    pub m2: f64,
    pub m3: f64,
    pub m4: f64,
    pub median: f64,
    pub mad: f64,
    pub values: *mut pdal_summary_merge_entry_t,
    pub values_len: u64,
    pub values_capacity: u64,
    pub data: *mut f64,
    pub data_len: u64,
    pub data_capacity: u64,
}

fn summary_from_merge_state(state: &pdal_summary_merge_state_t) -> Option<stats::Summary> {
    if state.name.is_null() {
        return None;
    }
    let name = unsafe { CStr::from_ptr(state.name) }
        .to_string_lossy()
        .into_owned();
    let mut summary = stats::Summary::new(name, state.enumerate, state.advanced);
    summary.cnt = state.count;
    summary.min = state.min;
    summary.max = state.max;
    summary.m1 = state.m1;
    summary.m2 = state.m2;
    summary.m3 = state.m3;
    summary.m4 = state.m4;
    summary.median = state.median;
    summary.mad = state.mad;

    if !state.values.is_null() {
        for idx in 0..state.values_len {
            let entry = unsafe { *state.values.add(idx as usize) };
            summary.values.insert(entry.value.to_bits(), entry.count);
        }
    }

    if !state.data.is_null() && state.enumerate == 3 {
        summary.data =
            unsafe { std::slice::from_raw_parts(state.data, state.data_len as usize).to_vec() };
    }

    Some(summary)
}

fn write_merge_state(summary: &stats::Summary, state: &mut pdal_summary_merge_state_t) {
    state.count = summary.cnt;
    state.min = summary.min;
    state.max = summary.max;
    state.m1 = summary.m1;
    state.m2 = summary.m2;
    state.m3 = summary.m3;
    state.m4 = summary.m4;
    state.median = summary.median;
    state.mad = summary.mad;

    if !state.values.is_null() {
        let mut idx = 0;
        let capacity = if state.values_capacity > 0 {
            state.values_capacity
        } else {
            state.values_len
        };
        for (value, count) in &summary.values {
            if idx >= capacity {
                break;
            }
            unsafe {
                *state.values.add(idx as usize) = pdal_summary_merge_entry_t {
                    value: f64::from_bits(*value),
                    count: *count,
                };
            }
            idx += 1;
        }
        state.values_len = idx;
    }

    if !state.data.is_null() && summary.enumerate == 3 {
        let capacity = if state.data_capacity > 0 {
            state.data_capacity as usize
        } else {
            state.data_len as usize
        };
        let len = summary.data.len().min(capacity);
        unsafe {
            std::ptr::copy_nonoverlapping(summary.data.as_ptr(), state.data, len);
        }
        state.data_len = len as u64;
    }
}

/// Merge one stats summary into another.
///
/// # Safety
///
/// Both state pointers must be valid. Value and data buffers must be large
/// enough to hold the merged result.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_stats_summary_merge(
    target: *mut pdal_summary_merge_state_t,
    other: *const pdal_summary_merge_state_t,
) -> bool {
    if target.is_null() || other.is_null() {
        return false;
    }

    let Some(mut left) = summary_from_merge_state(&*target) else {
        return false;
    };
    let Some(right) = summary_from_merge_state(&*other) else {
        return false;
    };

    if !left.merge(&right) {
        return false;
    }

    write_merge_state(&left, &mut *target);
    true
}
