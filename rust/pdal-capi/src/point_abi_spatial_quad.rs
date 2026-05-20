use super::*;
use pdal_core::point::DimensionSummary;
use serde_json::json;

#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_knn(
    view: *const PointView,
    dim_names: *const *const c_char,
    query: *const f64,
    dim_count: u64,
    k: u64,
    stride: u64,
    out_results: *mut pdal_spatial_result_t,
    max_results: u64,
) -> u64 {
    if view.is_null()
        || dim_names.is_null()
        || query.is_null()
        || out_results.is_null()
        || dim_count == 0
        || k == 0
        || max_results == 0
    {
        return 0;
    }
    let Some(view) = view.as_ref() else {
        return 0;
    };
    let dims = spatial_dims(dim_names, dim_count);
    let query = std::slice::from_raw_parts(query, dim_count as usize);
    let stride = stride.max(1) as usize;
    let want = (k as usize).min(max_results as usize);
    let mut results = spatial_results(view, &dims, query, f64::INFINITY);
    let search_count = want.saturating_mul(stride).min(results.len());
    results.truncate(search_count);

    let mut written = 0;
    for idx in (0..results.len()).step_by(stride).take(want) {
        *out_results.add(written) = results[idx];
        written += 1;
    }
    written as u64
}

#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_radius(
    view: *const PointView,
    dim_names: *const *const c_char,
    query: *const f64,
    dim_count: u64,
    radius: f64,
    out_len: *mut u64,
) -> *mut pdal_spatial_result_t {
    if !out_len.is_null() {
        *out_len = 0;
    }
    if view.is_null() || dim_names.is_null() || query.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let Some(view) = view.as_ref() else {
        return std::ptr::null_mut();
    };
    let dims = spatial_dims(dim_names, dim_count);
    let query = std::slice::from_raw_parts(query, dim_count as usize);
    let mut results = spatial_results(view, &dims, query, radius * radius);
    *out_len = results.len() as u64;
    let ptr = results.as_mut_ptr();
    std::mem::forget(results);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn pdal_spatial_results_free(ptr: *mut pdal_spatial_result_t, len: u64) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len as usize, len as usize));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_create(
    xs: *const f64,
    ys: *const f64,
    ids: *const u64,
    count: u64,
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
    top_level: u64,
) -> *mut QuadIndexAbi {
    if xs.is_null() || ys.is_null() || ids.is_null() {
        return std::ptr::null_mut();
    }

    let xs = std::slice::from_raw_parts(xs, count as usize);
    let ys = std::slice::from_raw_parts(ys, count as usize);
    let ids = std::slice::from_raw_parts(ids, count as usize);
    let points = (0..count as usize)
        .map(|idx| QuadPoint {
            id: ids[idx],
            x: xs[idx],
            y: ys[idx],
        })
        .collect();

    Box::into_raw(Box::new(QuadIndexAbi {
        points,
        bounds: pdal_bounds2d_t {
            minx: x_min,
            maxx: x_max,
            miny: y_min,
            maxy: y_max,
        },
        top_level,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_bounds(
    index: *const QuadIndexAbi,
    out_bounds: *mut pdal_bounds2d_t,
) {
    if let (Some(index), Some(out_bounds)) = (index.as_ref(), out_bounds.as_mut()) {
        *out_bounds = index.bounds;
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_depth(index: *const QuadIndexAbi) -> u64 {
    if index.as_ref().is_none_or(|index| index.points.is_empty()) {
        0
    } else {
        index.as_ref().unwrap().top_level
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_fills(
    index: *const QuadIndexAbi,
    out_len: *mut u64,
) -> *mut u64 {
    let Some(index) = index.as_ref() else {
        return leak_u64s(Vec::new(), out_len);
    };
    let mut fills = vec![0; pdal_quad_index_depth(index) as usize + 1];
    if let Some(last) = fills.last_mut() {
        *last = index.points.len() as u64;
    }
    leak_u64s(fills, out_len)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_points_by_depth(
    index: *const QuadIndexAbi,
    depth_begin: u64,
    depth_end: u64,
    out_len: *mut u64,
) -> *mut u64 {
    let Some(index) = index.as_ref() else {
        return leak_u64s(Vec::new(), out_len);
    };
    if depth_end != 0 && depth_begin >= depth_end {
        return leak_u64s(Vec::new(), out_len);
    }
    leak_u64s(index.points.iter().map(|point| point.id).collect(), out_len)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_points_in_bounds(
    index: *const QuadIndexAbi,
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
    depth_begin: u64,
    depth_end: u64,
    out_len: *mut u64,
) -> *mut u64 {
    let Some(index) = index.as_ref() else {
        return leak_u64s(Vec::new(), out_len);
    };
    if depth_end != 0 && depth_begin >= depth_end {
        return leak_u64s(Vec::new(), out_len);
    }

    let minx = x_min.min(x_max);
    let maxx = x_min.max(x_max);
    let miny = y_min.min(y_max);
    let maxy = y_min.max(y_max);
    let ids = index
        .points
        .iter()
        .filter(|point| point.x >= minx && point.x < maxx && point.y >= miny && point.y < maxy)
        .map(|point| point.id)
        .collect();
    leak_u64s(ids, out_len)
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_points_raster_level(
    index: *const QuadIndexAbi,
    rasterize: u64,
    x_begin: *mut f64,
    x_end: *mut f64,
    x_step: *mut f64,
    y_begin: *mut f64,
    y_end: *mut f64,
    y_step: *mut f64,
    out_len: *mut u64,
) -> *mut u64 {
    let Some(index) = index.as_ref() else {
        return leak_u64s(Vec::new(), out_len);
    };
    let exp = 1usize.checked_shl(rasterize as u32).unwrap_or(0);
    if exp == 0 {
        return leak_u64s(Vec::new(), out_len);
    }

    let step_x = (index.bounds.maxx - index.bounds.minx) / exp as f64;
    let step_y = (index.bounds.maxy - index.bounds.miny) / exp as f64;
    let begin_x = index.bounds.minx + step_x / 2.0;
    let begin_y = index.bounds.miny + step_y / 2.0;
    if let Some(out) = x_begin.as_mut() {
        *out = begin_x;
    }
    if let Some(out) = x_end.as_mut() {
        *out = index.bounds.maxx + step_x / 2.0;
    }
    if let Some(out) = x_step.as_mut() {
        *out = step_x;
    }
    if let Some(out) = y_begin.as_mut() {
        *out = begin_y;
    }
    if let Some(out) = y_end.as_mut() {
        *out = index.bounds.maxy + step_y / 2.0;
    }
    if let Some(out) = y_step.as_mut() {
        *out = step_y;
    }

    rasterize_points(
        index,
        begin_x,
        index.bounds.maxx + step_x / 2.0,
        step_x,
        begin_y,
        index.bounds.maxy + step_y / 2.0,
        step_y,
        out_len,
    )
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_points_raster_bounds(
    index: *const QuadIndexAbi,
    x_begin: f64,
    x_end: f64,
    x_step: f64,
    y_begin: f64,
    y_end: f64,
    y_step: f64,
    out_len: *mut u64,
) -> *mut u64 {
    let Some(index) = index.as_ref() else {
        return leak_u64s(Vec::new(), out_len);
    };
    rasterize_points(
        index, x_begin, x_end, x_step, y_begin, y_end, y_step, out_len,
    )
}

#[no_mangle]
pub unsafe extern "C" fn pdal_u64_array_free(ptr: *mut u64, len: u64) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len as usize, len as usize));
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_quad_index_destroy(index: *mut QuadIndexAbi) {
    if !index.is_null() {
        drop(Box::from_raw(index));
    }
}

/// Return per-dimension summaries for a view as JSON. Caller must free with
/// `pdal_string_free`.
///
/// # Safety
///
/// `view` must be null or a valid pointer returned by
/// `pdal_point_view_create`, or returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_dimension_summaries_json(
    view: *const PointView,
) -> *mut c_char {
    let summaries = view
        .as_ref()
        .map(|view| {
            serde_json::Value::Array(
                view.summarize_dimensions()
                    .iter()
                    .map(dimension_summary_json)
                    .collect(),
            )
        })
        .unwrap_or_else(|| json!([]));
    string_to_c_ptr(serde_json::to_string(&summaries).unwrap_or_else(|_| "[]".to_string()))
}

/// Destroy a point view.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create` or
/// `pdal_stage_run`, or null. Must not be called twice on the same pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_destroy(view: *mut PointView) {
    if !view.is_null() {
        drop(Box::from_raw(view));
    }
}

fn dimension_summary_json(summary: &DimensionSummary) -> serde_json::Value {
    json!({
        "name": summary.name,
        "count": summary.count,
        "minimum": summary.minimum,
        "maximum": summary.maximum,
        "mean": summary.mean,
    })
}

unsafe fn spatial_dims(dim_names: *const *const c_char, dim_count: u64) -> Vec<DimId> {
    let mut dims = Vec::with_capacity(dim_count as usize);
    for idx in 0..dim_count {
        let ptr = *dim_names.add(idx as usize);
        if !ptr.is_null() {
            let name = CStr::from_ptr(ptr).to_string_lossy();
            dims.push(dim_id_from_name(&name));
        }
    }
    dims
}

fn spatial_results(
    view: &PointView,
    dims: &[DimId],
    query: &[f64],
    max_sqr_dist: f64,
) -> Vec<pdal_spatial_result_t> {
    let mut results = Vec::new();
    for idx in 0..view.len() {
        let mut sqr_dist = 0.0;
        for (dim_idx, dim) in dims.iter().enumerate() {
            let delta = view.get_f64(idx, dim) - query[dim_idx];
            sqr_dist += delta * delta;
        }
        if sqr_dist <= max_sqr_dist {
            results.push(pdal_spatial_result_t { id: idx, sqr_dist });
        }
    }
    results.sort_by(|a, b| {
        a.sqr_dist
            .total_cmp(&b.sqr_dist)
            .then_with(|| a.id.cmp(&b.id))
    });
    results
}

fn leak_u64s(mut values: Vec<u64>, out_len: *mut u64) -> *mut u64 {
    unsafe {
        if !out_len.is_null() {
            *out_len = values.len() as u64;
        }
    }
    let ptr = values.as_mut_ptr();
    std::mem::forget(values);
    ptr
}

#[allow(clippy::too_many_arguments)]
fn rasterize_points(
    index: &QuadIndexAbi,
    x_begin: f64,
    x_end: f64,
    x_step: f64,
    y_begin: f64,
    y_end: f64,
    y_step: f64,
    out_len: *mut u64,
) -> *mut u64 {
    if x_step == 0.0 || y_step == 0.0 {
        return leak_u64s(Vec::new(), out_len);
    }
    let width = ((x_end - x_begin) / x_step).round().max(0.0) as usize;
    let height = ((y_end - y_begin) / y_step).round().max(0.0) as usize;
    let mut ids = vec![u64::MAX; width.saturating_mul(height)];

    for point in &index.points {
        if point.x < x_begin
            || point.y < y_begin
            || point.x >= x_end - x_step
            || point.y >= y_end - y_step
        {
            continue;
        }

        let x_offset = ((point.x - x_begin) / x_step).round();
        let y_offset = ((point.y - y_begin) / y_step).round();
        let idx = (y_offset * ((x_end - x_begin) / x_step) + x_offset).round();
        if idx >= 0.0 {
            let idx = idx as usize;
            if let Some(slot) = ids.get_mut(idx) {
                *slot = point.id;
            }
        }
    }

    leak_u64s(ids, out_len)
}
