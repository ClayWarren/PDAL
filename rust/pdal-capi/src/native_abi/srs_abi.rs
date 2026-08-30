use super::c_string_lossy;
use crate::error::{ffi_catch, set_last_error, string_to_c_ptr};
use pdal_native::srs;
use std::os::raw::c_char;

/// Resolve a GDAL `OGRSpatialReference::SetFromUserInput`-style string into
/// canonical WKT1 and WKT2_2018 plus any parsed coordinate epoch. Caller owns
/// the returned WKT strings and must free them with `pdal_string_free`.
///
/// # Safety
///
/// `input` must be a valid NUL-terminated C string. `out_wkt`/`out_wkt2`/
/// `out_epoch` must be null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_user_input_to_wkt(
    input: *const c_char,
    out_wkt: *mut *mut c_char,
    out_wkt2: *mut *mut c_char,
    out_epoch: *mut f64,
) -> bool {
    ffi_catch(false, || {
        match srs::user_input_to_wkt(&c_string_lossy(input)) {
            Ok(result) => {
                if let Some(out_wkt) = out_wkt.as_mut() {
                    *out_wkt = string_to_c_ptr(result.wkt);
                }
                if let Some(out_wkt2) = out_wkt2.as_mut() {
                    *out_wkt2 = string_to_c_ptr(result.wkt2);
                }
                if let Some(out_epoch) = out_epoch.as_mut() {
                    *out_epoch = result.epoch;
                }
                true
            }
            Err(err) => {
                set_last_error(err);
                false
            }
        }
    })
}

/// Translate WKT into the PROJ4 string PDAL's `SpatialReference::getProj4()`
/// produces. Returns an empty string when the WKT cannot be imported. Caller
/// owns the returned string and must free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_proj4` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_wkt_to_proj4(
    wkt: *const c_char,
    out_proj4: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || match srs::wkt_to_proj4(&c_string_lossy(wkt)) {
        Ok(proj4) => {
            if let Some(out_proj4) = out_proj4.as_mut() {
                *out_proj4 = string_to_c_ptr(proj4);
            }
            true
        }
        Err(err) => {
            set_last_error(err);
            false
        }
    })
}

/// Translate WKT into the PROJJSON string PDAL's
/// `SpatialReference::getPROJJSON()` produces. Returns an empty string when
/// the WKT cannot be imported. Caller owns the returned string and must free it
/// with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_projjson` must
/// be null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_wkt_to_projjson(
    wkt: *const c_char,
    epoch: f64,
    out_projjson: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        match srs::wkt_to_projjson(&c_string_lossy(wkt), epoch) {
            Ok(projjson) => {
                if let Some(out_projjson) = out_projjson.as_mut() {
                    *out_projjson = string_to_c_ptr(projjson);
                }
                true
            }
            Err(err) => {
                set_last_error(err);
                false
            }
        }
    })
}

/// Translate WKT into WKT1_GDAL. Caller owns the returned string and must free
/// it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_wkt` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_wkt_to_wkt1(
    wkt: *const c_char,
    epoch: f64,
    out_wkt: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        match srs::wkt_to_wkt1(&c_string_lossy(wkt), epoch) {
            Ok(value) => {
                if let Some(out_wkt) = out_wkt.as_mut() {
                    *out_wkt = string_to_c_ptr(value);
                }
                true
            }
            Err(err) => {
                set_last_error(err);
                false
            }
        }
    })
}

/// Translate WKT into WKT2_2018. Caller owns the returned string and must free
/// it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_wkt` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_wkt_to_wkt2(
    wkt: *const c_char,
    epoch: f64,
    out_wkt: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        match srs::wkt_to_wkt2(&c_string_lossy(wkt), epoch) {
            Ok(value) => {
                if let Some(out_wkt) = out_wkt.as_mut() {
                    *out_wkt = string_to_c_ptr(value);
                }
                true
            }
            Err(err) => {
                set_last_error(err);
                false
            }
        }
    })
}

/// Pretty-format WKT with GDAL's multiline exporter. Caller owns the returned
/// string and must free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_wkt` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_pretty_wkt(
    wkt: *const c_char,
    out_wkt: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || match srs::pretty_wkt(&c_string_lossy(wkt)) {
        Ok(value) => {
            if let Some(out_wkt) = out_wkt.as_mut() {
                *out_wkt = string_to_c_ptr(value);
            }
            true
        }
        Err(err) => {
            set_last_error(err);
            false
        }
    })
}

/// Mirror `SpatialReference::isGeographic()` through Rust GDAL.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_value` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_is_geographic(
    wkt: *const c_char,
    epoch: f64,
    out_value: *mut bool,
) -> bool {
    ffi_catch(false, || {
        let value = srs::is_geographic(&c_string_lossy(wkt), epoch);
        if let Some(out_value) = out_value.as_mut() {
            *out_value = value;
        }
        true
    })
}

/// Mirror `SpatialReference::isGeocentric()` through Rust GDAL.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_value` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_is_geocentric(
    wkt: *const c_char,
    epoch: f64,
    out_value: *mut bool,
) -> bool {
    ffi_catch(false, || {
        let value = srs::is_geocentric(&c_string_lossy(wkt), epoch);
        if let Some(out_value) = out_value.as_mut() {
            *out_value = value;
        }
        true
    })
}

/// Mirror `SpatialReference::isProjected()` through Rust GDAL.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_value` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_is_projected(
    wkt: *const c_char,
    epoch: f64,
    out_value: *mut bool,
) -> bool {
    ffi_catch(false, || {
        let value = srs::is_projected(&c_string_lossy(wkt), epoch);
        if let Some(out_value) = out_value.as_mut() {
            *out_value = value;
        }
        true
    })
}

/// Return GDAL's data-axis to SRS-axis mapping. Caller must release the
/// returned pointer with `pdal_i32_array_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_len` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_axis_ordering(
    wkt: *const c_char,
    epoch: f64,
    out_len: *mut u64,
) -> *mut i32 {
    ffi_catch(std::ptr::null_mut(), || {
        let values = srs::axis_ordering(&c_string_lossy(wkt), epoch);
        leak_i32s(values, out_len)
    })
}

/// Free an `i32` array returned by the C ABI.
///
/// # Safety
///
/// `ptr` must have been returned by this ABI with the same `len`, or null.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_i32_array_free(ptr: *mut i32, len: u64) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len as usize, len as usize));
    }
}

/// Mirror `OGRSpatialReference::IsSame` for two WKT strings at the given
/// coordinate epoch. Pass `0.0` for `epoch` when neither side has a fixed
/// epoch. Returns `false` and clears `out_same` if either side fails to import.
///
/// # Safety
///
/// `wkt_a` and `wkt_b` must be null or valid NUL-terminated C strings.
/// `out_same` must be null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_is_same(
    wkt_a: *const c_char,
    wkt_b: *const c_char,
    epoch: f64,
    out_same: *mut bool,
) -> bool {
    ffi_catch(false, || {
        let same = srs::is_same(&c_string_lossy(wkt_a), &c_string_lossy(wkt_b), epoch);
        if let Some(out_same) = out_same.as_mut() {
            *out_same = same;
        }
        true
    })
}

/// Mirror `SpatialReference::identifyHorizontalEPSG()` via Rust GDAL. Returns
/// the authority code string (e.g. `"32617"`) or an empty string when GDAL
/// cannot auto-identify the EPSG. Caller owns the returned string and must
/// free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_code` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_identify_horizontal_epsg(
    wkt: *const c_char,
    epoch: f64,
    out_code: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        let code = srs::identify_horizontal_epsg(&c_string_lossy(wkt), epoch);
        if let Some(out_code) = out_code.as_mut() {
            *out_code = string_to_c_ptr(code);
        }
        true
    })
}

/// Mirror `SpatialReference::getUTMZone()` via Rust GDAL. Positive for the
/// northern hemisphere, negative for the southern, 0 when not a UTM SRS.
/// Returns `false` only on unimportable WKT; empty WKT returns `0` with `true`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_zone` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_get_utm_zone(wkt: *const c_char, out_zone: *mut i32) -> bool {
    ffi_catch(false, || match srs::get_utm_zone(&c_string_lossy(wkt)) {
        Ok(zone) => {
            if let Some(out_zone) = out_zone.as_mut() {
                *out_zone = zone;
            }
            true
        }
        Err(err) => {
            set_last_error(err);
            false
        }
    })
}

/// Mirror `SpatialReference::getHorizontal()` via Rust GDAL: strip the
/// vertical CS and return the horizontal-only WKT. Caller owns the returned
/// string and must free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_wkt` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_get_horizontal_wkt(
    wkt: *const c_char,
    out_wkt: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        let horiz = srs::get_horizontal_wkt(&c_string_lossy(wkt));
        if let Some(out_wkt) = out_wkt.as_mut() {
            *out_wkt = string_to_c_ptr(horiz);
        }
        true
    })
}

/// Mirror `SpatialReference::getHorizontalUnits()`: linear-units name. Caller
/// owns the returned string and must free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_units` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_get_horizontal_units(
    wkt: *const c_char,
    out_units: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        let units = srs::get_horizontal_units(&c_string_lossy(wkt));
        if let Some(out_units) = out_units.as_mut() {
            *out_units = string_to_c_ptr(units);
        }
        true
    })
}

/// Mirror `SpatialReference::getVertical()`: WKT subtree of the top-level
/// `VERT_CS[...]` node. Empty when no vertical CS is present. Caller owns the
/// returned string and must free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_wkt` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_get_vertical_wkt(
    wkt: *const c_char,
    out_wkt: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        let vert = srs::get_vertical_wkt(&c_string_lossy(wkt));
        if let Some(out_wkt) = out_wkt.as_mut() {
            *out_wkt = string_to_c_ptr(vert);
        }
        true
    })
}

/// Mirror `SpatialReference::getVerticalUnits()`: linear-units name of the
/// VERT_CS node. Caller owns the returned string and must free it with
/// `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_units` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_get_vertical_units(
    wkt: *const c_char,
    out_units: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        let units = srs::get_vertical_units(&c_string_lossy(wkt));
        if let Some(out_units) = out_units.as_mut() {
            *out_units = string_to_c_ptr(units);
        }
        true
    })
}

/// Mirror `SpatialReference::identifyVerticalEPSG()`: VERT_CS authority code.
/// Caller owns the returned string and must free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_code` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_identify_vertical_epsg(
    wkt: *const c_char,
    epoch: f64,
    out_code: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        let code = srs::identify_vertical_epsg(&c_string_lossy(wkt), epoch);
        if let Some(out_code) = out_code.as_mut() {
            *out_code = string_to_c_ptr(code);
        }
        true
    })
}

/// Mirror `SpatialReference::valid()`: `OSRValidate` on the imported WKT.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_valid` must be
/// null or valid for writes.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_valid(wkt: *const c_char, out_valid: *mut bool) -> bool {
    ffi_catch(false, || {
        let v = srs::srs_valid(&c_string_lossy(wkt));
        if let Some(out_valid) = out_valid.as_mut() {
            *out_valid = v;
        }
        true
    })
}

/// Opaque handle owned by Rust wrapping a GDAL OGRCoordinateTransformation.
/// Construct with `pdal_srs_transform_create`; release with
/// `pdal_srs_transform_destroy`.
#[allow(non_camel_case_types)]
pub struct pdal_srs_transform_t {
    inner: srs::GdalSrsTransform,
}

/// Create an OGR-backed coordinate transformation. Pass `0.0` for either
/// epoch when no coordinate epoch applies. Pass `null` / `0` for the axis
/// order pointers to use the default `OAMS_TRADITIONAL_GIS_ORDER`. Returns
/// `null` on error and sets the last error message.
///
/// # Safety
///
/// `src_wkt` and `dst_wkt` must be NUL-terminated C strings.
/// `src_axis_order`/`dst_axis_order` must be valid for `_len` `int32_t`
/// elements when `_len > 0`. The returned pointer is owned by the caller and
/// must be released with `pdal_srs_transform_destroy`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_transform_create(
    src_wkt: *const c_char,
    src_epoch: f64,
    dst_wkt: *const c_char,
    dst_epoch: f64,
    src_axis_order: *const i32,
    src_axis_order_len: usize,
    dst_axis_order: *const i32,
    dst_axis_order_len: usize,
) -> *mut pdal_srs_transform_t {
    ffi_catch(std::ptr::null_mut(), || {
        let src_slice = if src_axis_order.is_null() || src_axis_order_len == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(src_axis_order, src_axis_order_len)
        };
        let dst_slice = if dst_axis_order.is_null() || dst_axis_order_len == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(dst_axis_order, dst_axis_order_len)
        };
        match srs::GdalSrsTransform::new(
            &c_string_lossy(src_wkt),
            src_epoch,
            &c_string_lossy(dst_wkt),
            dst_epoch,
            src_slice,
            dst_slice,
        ) {
            Ok(inner) => Box::into_raw(Box::new(pdal_srs_transform_t { inner })),
            Err(err) => {
                set_last_error(err);
                std::ptr::null_mut()
            }
        }
    })
}

/// Free a transform handle returned by `pdal_srs_transform_create`. Safe to
/// pass `null`.
///
/// # Safety
///
/// `handle` must have come from `pdal_srs_transform_create` and not been
/// destroyed already.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_transform_destroy(handle: *mut pdal_srs_transform_t) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Opaque handle wrapping a local-cartesian (topocentric ENU) transform around
/// an anchor lat/lon/h on WGS84. Backs the C++ `georeference::LocalCartesian`.
/// Construct with `pdal_topocentric_create`; release with
/// `pdal_topocentric_destroy`.
#[allow(non_camel_case_types)]
pub struct pdal_topocentric_transform_t {
    inner: srs::TopocentricTransform,
}

/// Create a topocentric transform anchored at `lat0`/`lon0` (degrees) and `h0`
/// (metres). Returns `null` on error and sets the last error message.
///
/// # Safety
///
/// The returned pointer is owned by the caller and must be released with
/// `pdal_topocentric_destroy`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_topocentric_create(
    lat0: f64,
    lon0: f64,
    h0: f64,
) -> *mut pdal_topocentric_transform_t {
    ffi_catch(
        std::ptr::null_mut(),
        || match srs::TopocentricTransform::new(lat0, lon0, h0) {
            Ok(inner) => Box::into_raw(Box::new(pdal_topocentric_transform_t { inner })),
            Err(err) => {
                set_last_error(err);
                std::ptr::null_mut()
            }
        },
    )
}

/// Free a handle returned by `pdal_topocentric_create`. Safe to pass `null`.
///
/// # Safety
///
/// `handle` must have come from `pdal_topocentric_create` and not been
/// destroyed already.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_topocentric_destroy(handle: *mut pdal_topocentric_transform_t) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Forward transform a single XYZ point in place: geocentric (ECEF) -> local
/// ENU. Returns false on null handle/pointers.
///
/// # Safety
///
/// `handle` must come from `pdal_topocentric_create`. `x`/`y`/`z` must be
/// non-null and valid for read+write.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_topocentric_forward(
    handle: *const pdal_topocentric_transform_t,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
) -> bool {
    ffi_catch(false, || {
        let Some(handle) = handle.as_ref() else {
            return false;
        };
        let (Some(x), Some(y), Some(z)) = (x.as_mut(), y.as_mut(), z.as_mut()) else {
            return false;
        };
        let (nx, ny, nz) = handle.inner.forward(*x, *y, *z);
        *x = nx;
        *y = ny;
        *z = nz;
        true
    })
}

/// Reverse transform a single XYZ point in place: local ENU -> geocentric
/// (ECEF). Returns false on null handle/pointers.
///
/// # Safety
///
/// `handle` must come from `pdal_topocentric_create`. `x`/`y`/`z` must be
/// non-null and valid for read+write.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_topocentric_reverse(
    handle: *const pdal_topocentric_transform_t,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
) -> bool {
    ffi_catch(false, || {
        let Some(handle) = handle.as_ref() else {
            return false;
        };
        let (Some(x), Some(y), Some(z)) = (x.as_mut(), y.as_mut(), z.as_mut()) else {
            return false;
        };
        let (nx, ny, nz) = handle.inner.reverse(*x, *y, *z);
        *x = nx;
        *y = ny;
        *z = nz;
        true
    })
}

/// Transform a single XYZ point in place. Returns false on null handle or
/// GDAL-reported failure.
///
/// # Safety
///
/// `handle` must come from `pdal_srs_transform_create`. `x`/`y`/`z` must be
/// non-null and valid for read+write.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_transform_xyz(
    handle: *const pdal_srs_transform_t,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
) -> bool {
    ffi_catch(false, || {
        let Some(handle) = handle.as_ref() else {
            return false;
        };
        let (Some(x), Some(y), Some(z)) = (x.as_mut(), y.as_mut(), z.as_mut()) else {
            return false;
        };
        handle.inner.transform_xyz(x, y, z)
    })
}

/// Transform a packed-array XYZ batch in place. Returns false on null handle,
/// length mismatch (all three arrays must be `len` long), or GDAL-reported
/// failure. Empty batches return true.
///
/// # Safety
///
/// `handle` must come from `pdal_srs_transform_create`. `xs`/`ys`/`zs` must
/// be non-null and valid for `len` `f64` reads+writes when `len > 0`.
#[pdal_capi_macros::ffi_export]
pub unsafe extern "C" fn pdal_srs_transform_xyz_array(
    handle: *const pdal_srs_transform_t,
    xs: *mut f64,
    ys: *mut f64,
    zs: *mut f64,
    len: usize,
) -> bool {
    ffi_catch(false, || {
        let Some(handle) = handle.as_ref() else {
            return false;
        };
        if len == 0 {
            return true;
        }
        if xs.is_null() || ys.is_null() || zs.is_null() {
            return false;
        }
        let xs = std::slice::from_raw_parts_mut(xs, len);
        let ys = std::slice::from_raw_parts_mut(ys, len);
        let zs = std::slice::from_raw_parts_mut(zs, len);
        handle.inner.transform_xyz_slice(xs, ys, zs)
    })
}

unsafe fn leak_i32s(values: Vec<i32>, out_len: *mut u64) -> *mut i32 {
    if !out_len.is_null() {
        *out_len = values.len() as u64;
    }
    if values.is_empty() {
        return std::ptr::null_mut();
    }
    let mut values = values.into_boxed_slice();
    let ptr = values.as_mut_ptr();
    std::mem::forget(values);
    ptr
}
