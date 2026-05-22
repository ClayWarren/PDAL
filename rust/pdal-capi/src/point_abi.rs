use crate::error::string_to_c_ptr;
use pdal_core::point::{
    fix_dimension_name, pdal_dimension_interpretation_name as core_dimension_interpretation_name,
    pdal_dimension_type_from_base_and_size as core_dimension_type_from_base_and_size,
    pdal_dimension_type_from_name as core_dimension_type_from_name, resolve_pdal_dimension_type,
    DimId, DimType, PointLayout, PointView,
};
use pdal_core::raster::RasterLimits;
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_raster_limits_t {
    pub x_origin: f64,
    pub y_origin: f64,
    pub width: u64,
    pub height: u64,
    pub edge_length: f64,
}

impl From<RasterLimits> for pdal_raster_limits_t {
    fn from(value: RasterLimits) -> Self {
        Self {
            x_origin: value.x_origin,
            y_origin: value.y_origin,
            width: value.width as u64,
            height: value.height as u64,
            edge_length: value.edge_length,
        }
    }
}

impl From<pdal_raster_limits_t> for RasterLimits {
    fn from(value: pdal_raster_limits_t) -> Self {
        RasterLimits::new(
            value.x_origin,
            value.y_origin,
            value.width as usize,
            value.height as usize,
            value.edge_length,
        )
    }
}

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

/// Return the number of triangles in this view's mesh, or zero if no mesh
/// exists.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_mesh_triangle_count(view: *const PointView) -> u64 {
    view.as_ref()
        .and_then(PointView::mesh)
        .map_or(0, |mesh| mesh.len() as u64)
}

/// Return the number of triangles in the named mesh, or zero if no mesh
/// exists.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `name` must be a valid, NUL-terminated C
/// string, or null to use the default mesh lookup.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_named_mesh_triangle_count(
    view: *const PointView,
    name: *const c_char,
) -> u64 {
    let Some(view) = view.as_ref() else {
        return 0;
    };
    let name = nullable_cstr(name);
    view.mesh_named(&name).map_or(0, |mesh| mesh.len() as u64)
}

/// Copy one mesh triangle out of a view.
///
/// Returns false when `view` is null, no mesh exists, the index is out of
/// range, or any output pointer is null.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `a`, `b`, and `c` must point to writable
/// memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_mesh_triangle(
    view: *const PointView,
    idx: u64,
    a: *mut u64,
    b: *mut u64,
    c: *mut u64,
) -> bool {
    let (Some(view), Some(a), Some(b), Some(c)) =
        (view.as_ref(), a.as_mut(), b.as_mut(), c.as_mut())
    else {
        return false;
    };
    let Some(mesh) = view.mesh() else {
        return false;
    };
    let Some(triangle) = mesh.triangles().get(idx as usize) else {
        return false;
    };
    *a = triangle.a;
    *b = triangle.b;
    *c = triangle.c;
    true
}

/// Copy one triangle out of a named mesh.
///
/// Returns false when `view` is null, no mesh exists, the index is out of
/// range, or any output pointer is null.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `name` must be a valid, NUL-terminated C
/// string, or null to use the default mesh lookup. `a`, `b`, and `c` must
/// point to writable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_named_mesh_triangle(
    view: *const PointView,
    name: *const c_char,
    idx: u64,
    a: *mut u64,
    b: *mut u64,
    c: *mut u64,
) -> bool {
    let (Some(view), Some(a), Some(b), Some(c)) =
        (view.as_ref(), a.as_mut(), b.as_mut(), c.as_mut())
    else {
        return false;
    };
    let name = nullable_cstr(name);
    let Some(mesh) = view.mesh_named(&name) else {
        return false;
    };
    let Some(triangle) = mesh.triangles().get(idx as usize) else {
        return false;
    };
    *a = triangle.a;
    *b = triangle.b;
    *c = triangle.c;
    true
}

/// Add one triangle to a view's mesh, creating the mesh if needed.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_add_mesh_triangle(
    view: *mut PointView,
    a: u64,
    b: u64,
    c: u64,
) -> bool {
    let Some(view) = view.as_mut() else {
        return false;
    };
    view.create_mesh().add(a, b, c);
    true
}

/// Add one triangle to a named mesh, creating that mesh if needed.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `name` must be a valid, NUL-terminated C
/// string, or null to use the default mesh name.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_add_named_mesh_triangle(
    view: *mut PointView,
    name: *const c_char,
    a: u64,
    b: u64,
    c: u64,
) -> bool {
    let Some(view) = view.as_mut() else {
        return false;
    };
    let name = nullable_cstr(name);
    let mesh = if view.mesh_named(&name).is_some() {
        view.mesh_mut_named(&name).expect("mesh exists")
    } else {
        view.create_named_mesh(&name).expect("mesh was absent")
    };
    mesh.add(a, b, c);
    true
}

/// Return the number of raster attachments on a view.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_raster_count(view: *const PointView) -> u64 {
    view.as_ref()
        .map(|view| view.rasters().len() as u64)
        .unwrap_or(0)
}

/// Return a raster name by index. Caller owns the returned string and must
/// free it with `pdal_string_free`.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_raster_name(
    view: *const PointView,
    idx: u64,
) -> *mut c_char {
    view.as_ref()
        .and_then(|view| view.rasters().get(idx as usize))
        .map(|raster| string_to_c_ptr(raster.name().to_string()))
        .unwrap_or(std::ptr::null_mut())
}

/// Create a raster attachment on the view.
///
/// Returns false when arguments are null or a raster with the same name already
/// exists.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `name` must be a valid, NUL-terminated C
/// string. `limits` must point to readable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_create_raster(
    view: *mut PointView,
    name: *const c_char,
    limits: *const pdal_raster_limits_t,
    initializer: f64,
) -> bool {
    let (Some(view), Some(limits)) = (view.as_mut(), limits.as_ref()) else {
        return false;
    };
    if name.is_null() {
        return false;
    }
    let name = CStr::from_ptr(name).to_string_lossy();
    view.create_raster(&name, (*limits).into(), initializer)
        .is_some()
}

/// Return raster limits by name.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `name` must be a valid, NUL-terminated C
/// string, or null to use default raster lookup. `out_limits` must point to
/// writable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_raster_limits(
    view: *const PointView,
    name: *const c_char,
    out_limits: *mut pdal_raster_limits_t,
) -> bool {
    let (Some(view), Some(out_limits)) = (view.as_ref(), out_limits.as_mut()) else {
        return false;
    };
    let name = nullable_cstr(name);
    let Some(raster) = view.raster(&name) else {
        return false;
    };
    *out_limits = raster.limits().clone().into();
    true
}

/// Return the initializer/no-data value for a raster.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `name` must be a valid, NUL-terminated C
/// string, or null to use default raster lookup.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_raster_initializer(
    view: *const PointView,
    name: *const c_char,
) -> f64 {
    let Some(view) = view.as_ref() else {
        return f64::NAN;
    };
    let name = nullable_cstr(name);
    view.raster(&name)
        .map(|raster| raster.initializer())
        .unwrap_or(f64::NAN)
}

/// Read one raster cell using PDAL's bottom-origin cell coordinates.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `name` must be a valid, NUL-terminated C
/// string, or null to use default raster lookup. `out_value` must point to
/// writable memory.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_raster_cell(
    view: *const PointView,
    name: *const c_char,
    x: u64,
    y: u64,
    out_value: *mut f64,
) -> bool {
    let (Some(view), Some(out_value)) = (view.as_ref(), out_value.as_mut()) else {
        return false;
    };
    let name = nullable_cstr(name);
    let Some(raster) = view.raster(&name) else {
        return false;
    };
    let limits = raster.limits();
    if x as usize >= limits.width || y as usize >= limits.height {
        return false;
    }
    *out_value = raster.get_cell(x as usize, y as usize);
    true
}

/// Set one raster cell using PDAL's bottom-origin cell coordinates.
///
/// # Safety
///
/// `view` must be a valid pointer returned by `pdal_point_view_create`, or
/// returned by `pdal_stage_run`. `name` must be a valid, NUL-terminated C
/// string, or null to use default raster lookup.
#[no_mangle]
pub unsafe extern "C" fn pdal_point_view_set_raster_cell(
    view: *mut PointView,
    name: *const c_char,
    x: u64,
    y: u64,
    value: f64,
) -> bool {
    let Some(view) = view.as_mut() else {
        return false;
    };
    let name = nullable_cstr(name);
    let Some(raster) = view.raster_mut(&name) else {
        return false;
    };
    let limits = raster.limits();
    if x as usize >= limits.width || y as usize >= limits.height {
        return false;
    }
    raster.set_cell(x as usize, y as usize, value);
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
