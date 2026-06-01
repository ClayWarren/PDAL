use super::*;
use crate::error::string_to_c_ptr;
use std::ffi::CStr;
use std::os::raw::c_char;

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
