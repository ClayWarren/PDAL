use crate::error::string_to_c_ptr;
use pdal_core::point::{
    fix_dimension_name, pdal_dimension_interpretation_name as core_dimension_interpretation_name,
    pdal_dimension_type_from_base_and_size as core_dimension_type_from_base_and_size,
    pdal_dimension_type_from_name as core_dimension_type_from_name, resolve_pdal_dimension_type,
    DimId, DimType, PointLayout, PointView,
};
use pdal_core::srs::SpatialReference;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::rc::Rc;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_spatial_result_t {
    pub id: u64,
    pub sqr_dist: f64,
}

#[path = "point_abi/attachments.rs"]
mod attachments;
pub use attachments::*;

#[path = "point_abi/expressions.rs"]
mod expressions;
pub use expressions::*;

#[path = "point_abi/raster.rs"]
mod raster;
pub use raster::*;

#[derive(Clone, Copy, Debug)]
struct QuadPoint {
    id: u64,
    x: f64,
    y: f64,
}

#[derive(Debug)]
pub struct QuadIndexAbi {
    points: Vec<QuadPoint>,
    bounds: pdal_bounds2d_t,
    top_level: u64,
}

/// Map a C string dimension name to the Rust `DimId` enum.
pub(crate) fn dim_id_from_name(name: &str) -> DimId {
    DimId::from_name(name)
}

/// Map an integer type id from the C side to a `DimType`.
pub(crate) fn dim_type_from_id(ty_id: i32) -> DimType {
    match ty_id {
        0 => DimType::U8,
        1 => DimType::U16,
        2 => DimType::U32,
        3 => DimType::U64,
        4 => DimType::I8,
        5 => DimType::I16,
        6 => DimType::I32,
        7 => DimType::I64,
        8 => DimType::F32,
        9 => DimType::F64,
        _ => DimType::F64,
    }
}

pub(crate) fn dim_type_to_id(ty: DimType) -> i32 {
    match ty {
        DimType::U8 => 0,
        DimType::U16 => 1,
        DimType::U32 => 2,
        DimType::U64 => 3,
        DimType::I8 => 4,
        DimType::I16 => 5,
        DimType::I32 => 6,
        DimType::I64 => 7,
        DimType::F32 => 8,
        DimType::F64 => 9,
    }
}

// ---------------------------------------------------------------------------
// PointLayout ABI
// ---------------------------------------------------------------------------

/// Create a new, empty point layout. Returns an owned pointer.
#[no_mangle]
pub extern "C" fn pdal_point_layout_create() -> *mut PointLayout {
    Box::into_raw(Box::new(PointLayout::new()))
}

/// Register a dimension in the layout.
///
/// # Safety
///
/// `layout` must be a valid pointer returned by `pdal_point_layout_create`.
/// `name` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_layout_register_dim(
    layout: *mut PointLayout,
    name: *const c_char,
    ty_id: i32,
) {
    if let (Some(layout), false) = (layout.as_mut(), name.is_null()) {
        let n = CStr::from_ptr(name).to_string_lossy();
        layout.register(dim_id_from_name(&n), dim_type_from_id(ty_id));
    }
}

#[no_mangle]
pub extern "C" fn pdal_dimension_resolve_type(type1: i32, type2: i32) -> i32 {
    if type1 < 0 || type2 < 0 {
        return 0;
    }
    resolve_pdal_dimension_type(type1 as u32, type2 as u32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn pdal_dimension_interpretation_name(type_id: i32) -> *mut c_char {
    if type_id < 0 {
        return string_to_c_ptr("unknown".to_string());
    }
    string_to_c_ptr(core_dimension_interpretation_name(type_id as u32).to_string())
}

#[no_mangle]
pub unsafe extern "C" fn pdal_dimension_type_from_name(name: *const c_char) -> i32 {
    if name.is_null() {
        return 0;
    }
    let name = CStr::from_ptr(name).to_string_lossy();
    core_dimension_type_from_name(&name) as i32
}

#[no_mangle]
pub unsafe extern "C" fn pdal_dimension_type_from_base_and_size(
    base: *const c_char,
    size: u64,
) -> i32 {
    if base.is_null() {
        return 0;
    }
    let base = CStr::from_ptr(base).to_string_lossy();
    core_dimension_type_from_base_and_size(&base, size as usize) as i32
}

#[no_mangle]
pub unsafe extern "C" fn pdal_dimension_fix_name(name: *const c_char) -> *mut c_char {
    let name = if name.is_null() {
        String::new()
    } else {
        CStr::from_ptr(name).to_string_lossy().into_owned()
    };
    string_to_c_ptr(fix_dimension_name(&name))
}

/// Destroy a point layout.
///
/// # Safety
///
/// `layout` must be a valid pointer returned by `pdal_point_layout_create`,
/// or null. Must not be called twice on the same pointer. Must not be called
/// after the layout has been consumed by `pdal_point_view_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_layout_destroy(layout: *mut PointLayout) {
    if !layout.is_null() {
        drop(Box::from_raw(layout));
    }
}

// ---------------------------------------------------------------------------
// PointView ABI
// ---------------------------------------------------------------------------

/// Create a new, empty point view from the given layout.
///
/// # Safety
///
/// `layout` must be a valid pointer returned by `pdal_point_layout_create`.
/// Ownership of the layout is transferred — the caller must **not** call
/// `pdal_point_layout_destroy` on it after this call.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_create(layout: *mut PointLayout) -> *mut PointView {
    if layout.is_null() {
        return std::ptr::null_mut();
    }
    let layout_rc = Rc::new(*Box::from_raw(layout));
    Box::into_raw(Box::new(PointView::new(layout_rc)))
}

/// Add a zero-initialised point to the view. Returns its index.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_add_point(view: *mut PointView) -> u64 {
    if let Some(view) = view.as_mut() {
        view.add_point()
    } else {
        0
    }
}

/// Set a dimension value on a point, converting from `f64`.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`.
/// `dim_name` must be a valid, NUL-terminated C string.
/// `idx` must be less than the number of points in the view.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_set_f64(
    view: *mut PointView,
    idx: u64,
    dim_name: *const c_char,
    val: f64,
) {
    if let (Some(view), false) = (view.as_mut(), dim_name.is_null()) {
        let n = CStr::from_ptr(dim_name).to_string_lossy();
        view.set_f64(idx, &dim_id_from_name(&n), val);
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_try_set_f64(
    view: *mut PointView,
    idx: u64,
    dim_name: *const c_char,
    val: f64,
) -> bool {
    if let (Some(view), false) = (view.as_mut(), dim_name.is_null()) {
        let n = CStr::from_ptr(dim_name).to_string_lossy();
        view.try_set_f64(idx, &dim_id_from_name(&n), val)
    } else {
        false
    }
}

/// Write a dimension value to a point from an exact `u64`.
///
/// 64-bit integer dimensions (such as the uint64 `H3` index) store the value
/// without an intermediate `f64` conversion, so values above `2^53` are
/// preserved exactly.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`.
/// `dim_name` must be a valid, NUL-terminated C string.
/// `idx` must be less than the number of points in the view.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_set_u64(
    view: *mut PointView,
    idx: u64,
    dim_name: *const c_char,
    val: u64,
) {
    if let (Some(view), false) = (view.as_mut(), dim_name.is_null()) {
        let n = CStr::from_ptr(dim_name).to_string_lossy();
        view.set_u64(idx, &dim_id_from_name(&n), val);
    }
}

/// Get a dimension value from a point, as an exact `u64`.
///
/// 64-bit integer dimensions are read from their raw storage, so the low bits
/// of large indexes survive (an `f64` getter would round them away). Returns
/// `false` (and leaves `*out` untouched) if `view`/`dim_name`/`out` are null.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`.
/// `dim_name` must be a valid, NUL-terminated C string.
/// `idx` must be less than the number of points in the view.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_get_u64(
    view: *mut PointView,
    idx: u64,
    dim_name: *const c_char,
    out: *mut u64,
) -> bool {
    if out.is_null() {
        return false;
    }
    if let (Some(view), false) = (view.as_ref(), dim_name.is_null()) {
        let n = CStr::from_ptr(dim_name).to_string_lossy();
        *out = view.get_u64(idx, &dim_id_from_name(&n));
        true
    } else {
        false
    }
}

/// Get a dimension value from a point, as `f64`.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`.
/// `dim_name` must be a valid, NUL-terminated C string.
/// `idx` must be less than the number of points in the view.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_get_f64(
    view: *mut PointView,
    idx: u64,
    dim_name: *const c_char,
) -> f64 {
    if let (Some(view), false) = (view.as_ref(), dim_name.is_null()) {
        let n = CStr::from_ptr(dim_name).to_string_lossy();
        view.get_f64(idx, &dim_id_from_name(&n))
    } else {
        0.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_get_point_f64s(
    view: *const PointView,
    idx: u64,
    out_values: *mut f64,
    max_values: u64,
) -> u64 {
    if out_values.is_null() || max_values == 0 {
        return 0;
    }
    let Some(view) = view.as_ref() else {
        return 0;
    };
    if idx >= view.len() {
        return 0;
    }

    let count = view.layout().dim_count().min(max_values as usize);
    let out = std::slice::from_raw_parts_mut(out_values, count);
    for (dim_idx, value) in out.iter_mut().enumerate() {
        *value = view
            .layout()
            .dim_at(dim_idx)
            .map(|(dim, _)| view.get_f64(idx, dim))
            .unwrap_or(0.0);
    }
    count as u64
}

#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_get_u8(
    view: *mut PointView,
    idx: u64,
    dim_name: *const c_char,
    out: *mut u8,
) -> bool {
    if out.is_null() {
        return false;
    }
    let value = pdal_point_view_get_f64(view, idx, dim_name);
    if !value.is_finite() || value < u8::MIN as f64 || value > u8::MAX as f64 {
        return false;
    }
    *out = value as u8;
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_get_i32(
    view: *mut PointView,
    idx: u64,
    dim_name: *const c_char,
    out: *mut i32,
) -> bool {
    if out.is_null() {
        return false;
    }
    let value = pdal_point_view_get_f64(view, idx, dim_name);
    if !value.is_finite() || value < i32::MIN as f64 || value > i32::MAX as f64 {
        return false;
    }
    *out = value as i32;
    true
}

#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_get_f32(
    view: *mut PointView,
    idx: u64,
    dim_name: *const c_char,
    out: *mut f32,
) -> bool {
    if out.is_null() {
        return false;
    }
    let value = pdal_point_view_get_f64(view, idx, dim_name);
    if value.is_finite() && value.abs() > f32::MAX as f64 {
        return false;
    }
    *out = value as f32;
    true
}

/// Return the number of dimensions in the view layout.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_dim_count(view: *const PointView) -> u64 {
    if let Some(view) = view.as_ref() {
        view.layout().dim_count() as u64
    } else {
        0
    }
}

/// Return a newly allocated dimension name at layout index `idx`.
///
/// The caller owns the returned string and must free it with
/// `pdal_string_free`.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_dim_name(view: *const PointView, idx: u64) -> *mut c_char {
    if let Some(view) = view.as_ref() {
        if let Some((id, _)) = view.layout().dim_at(idx as usize) {
            return CString::new(id.name())
                .expect("dimension names do not contain NULs")
                .into_raw();
        }
    }
    std::ptr::null_mut()
}

/// Return the integer type id of the dimension at layout index `idx`.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_dim_type(view: *const PointView, idx: u64) -> i32 {
    if let Some(view) = view.as_ref() {
        if let Some((_, ty)) = view.layout().dim_at(idx as usize) {
            return dim_type_to_id(ty);
        }
    }
    -1
}

/// Set a point view's spatial reference.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`.
/// `srs` must be null or a valid pointer returned by
/// `pdal_spatial_reference_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_set_spatial_reference(
    view: *mut PointView,
    srs: *const SpatialReference,
) {
    if let Some(view) = view.as_mut() {
        let spatial_reference = srs.as_ref().cloned().unwrap_or_default();
        view.set_spatial_reference(spatial_reference);
    }
}

/// Return a copy of a point view's spatial reference. Caller owns the result.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_spatial_reference(
    view: *const PointView,
) -> *mut SpatialReference {
    if let Some(view) = view.as_ref() {
        Box::into_raw(Box::new(view.spatial_reference().clone()))
    } else {
        std::ptr::null_mut()
    }
}

/// Return the view's stable identity, or zero for a null view.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_id(view: *const PointView) -> u64 {
    if let Some(view) = view.as_ref() {
        view.id()
    } else {
        0
    }
}

/// Return the number of points in the view.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_length(view: *mut PointView) -> u64 {
    if let Some(view) = view.as_ref() {
        view.len()
    } else {
        0
    }
}

/// Return the original source row for a point in this view.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_source_index(view: *mut PointView, idx: u64) -> u64 {
    if let Some(view) = view.as_ref() {
        view.source_index(idx)
    } else {
        idx
    }
}

/// Swap two point rows in a view.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_swap_points(view: *mut PointView, a: u64, b: u64) -> bool {
    if let Some(view) = view.as_mut() {
        view.swap_points(a, b)
    } else {
        false
    }
}

/// Calculate 2D bounds for X/Y dimensions.
///
/// Returns false when `view` or `out_bounds` is null, the view is empty, or X/Y
/// are not registered in the view's layout.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `out_bounds` must point to writable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_calculate_bounds_2d(
    view: *const PointView,
    out_bounds: *mut pdal_bounds2d_t,
) -> bool {
    let Some(view) = view.as_ref() else {
        return false;
    };
    let Some(bounds) = view.calculate_bounds_2d() else {
        return false;
    };
    let Some(out_bounds) = out_bounds.as_mut() else {
        return false;
    };

    *out_bounds = pdal_bounds2d_t {
        minx: bounds.minx,
        maxx: bounds.maxx,
        miny: bounds.miny,
        maxy: bounds.maxy,
    };
    true
}

/// Calculate 3D bounds for X/Y/Z dimensions.
///
/// Returns false when `view` or `out_bounds` is null, the view is empty, or
/// X/Y/Z are not registered in the view's layout.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `out_bounds` must point to writable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_calculate_bounds_3d(
    view: *const PointView,
    out_bounds: *mut pdal_bounds3d_t,
) -> bool {
    let Some(view) = view.as_ref() else {
        return false;
    };
    let Some(bounds) = view.calculate_bounds_3d() else {
        return false;
    };
    let Some(out_bounds) = out_bounds.as_mut() else {
        return false;
    };

    *out_bounds = pdal_bounds3d_t {
        minx: bounds.minx,
        maxx: bounds.maxx,
        miny: bounds.miny,
        maxy: bounds.maxy,
        minz: bounds.minz,
        maxz: bounds.maxz,
    };
    true
}

unsafe fn nullable_cstr(value: *const c_char) -> String {
    if value.is_null() {
        String::new()
    } else {
        CStr::from_ptr(value).to_string_lossy().into_owned()
    }
}

#[path = "point_abi_bounds.rs"]
mod point_abi_bounds;
pub use point_abi_bounds::*;

#[path = "point_abi_spatial_quad.rs"]
mod point_abi_spatial_quad;
pub use point_abi_spatial_quad::*;
