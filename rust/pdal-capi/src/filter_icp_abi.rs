use pdal_core::point::PointView;
use pdal_filters::icp::{register, IcpParams};

/// Register a moving point view onto a fixed one with Iterative Closest Point.
///
/// Returns a heap-allocated transformed copy of `moving` (free it with
/// `pdal_point_view_destroy`), or null on failure. The recovered transform
/// (16 row-major values), the fixed-cloud centroid (3 values), the
/// convergence flag and the final MSE are written through the out-pointers.
///
/// # Safety
///
/// `fixed` and `moving` must be valid view pointers. `init` must point to 16
/// `f64` values when `has_init` is true. Each non-null out-pointer must be
/// valid and sized as documented (`out_transform`: 16, `out_centroid`: 3).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pdal_icp_register(
    fixed: *const PointView,
    moving: *const PointView,
    max_iters: i32,
    max_similar: i32,
    rotation_threshold: f64,
    translation_threshold: f64,
    mse_abs: f64,
    has_maxdist: bool,
    maxdist: f64,
    has_init: bool,
    init: *const f64,
    out_transform: *mut f64,
    out_centroid: *mut f64,
    out_converged: *mut bool,
    out_mse: *mut f64,
) -> *mut PointView {
    let (fixed_ref, moving_ref) = match (fixed.as_ref(), moving.as_ref()) {
        (Some(f), Some(m)) => (f, m),
        _ => return std::ptr::null_mut(),
    };

    let init_matrix = if has_init && !init.is_null() {
        let mut m = [0.0f64; 16];
        for (i, slot) in m.iter_mut().enumerate() {
            *slot = *init.add(i);
        }
        Some(m)
    } else {
        None
    };

    let params = IcpParams {
        max_iters,
        max_similar,
        rotation_threshold,
        translation_threshold,
        mse_abs,
        maxdist: if has_maxdist { Some(maxdist) } else { None },
        init: init_matrix,
    };

    let result = register(fixed_ref, moving_ref, &params);

    if !out_transform.is_null() {
        for (i, &v) in result.transform.iter().enumerate() {
            *out_transform.add(i) = v;
        }
    }
    if !out_centroid.is_null() {
        for (i, &v) in result.centroid.iter().enumerate() {
            *out_centroid.add(i) = v;
        }
    }
    if !out_converged.is_null() {
        *out_converged = result.converged;
    }
    if !out_mse.is_null() {
        *out_mse = result.mse;
    }

    Box::into_raw(Box::new(result.view))
}
