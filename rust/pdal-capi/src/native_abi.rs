use crate::error::{ffi_catch, set_last_error, string_to_c_ptr};
use pdal_core::geometry::Geometry;
use pdal_native::srs;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Return native dependency diagnostics as JSON.
///
/// Caller owns the returned string and must free it with `pdal_string_free`.
#[no_mangle]
pub extern "C" fn pdal_native_dependencies_json() -> *mut c_char {
    ffi_catch(std::ptr::null_mut(), || {
        let dependencies: Vec<_> = pdal_native::built_dependencies()
            .into_iter()
            .map(|dependency| {
                serde_json::json!({
                    "name": dependency.name,
                    "version": dependency.version,
                })
            })
            .collect();
        string_to_c_ptr(serde_json::to_string(&dependencies).unwrap())
    })
}

/// Evaluate whether WKT geometry is valid using the native GEOS adapter.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_value` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_wkt_is_valid(
    wkt: *const c_char,
    out_value: *mut bool,
) -> bool {
    ffi_catch(false, || {
        let Ok(geometry) = Geometry::from_wkt(&c_string_lossy(wkt)) else {
            set_last_error("Failed to parse WKT geometry");
            return false;
        };
        match geometry.is_valid() {
            Ok(valid) => {
                if let Some(out_value) = out_value.as_mut() {
                    *out_value = valid;
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

/// Compute distance from WKT geometry to a point using the native GEOS adapter.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_value` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_wkt_distance_to_point(
    wkt: *const c_char,
    x: f64,
    y: f64,
    z: f64,
    out_value: *mut f64,
) -> bool {
    ffi_catch(false, || {
        let Ok(geometry) = Geometry::from_wkt(&c_string_lossy(wkt)) else {
            set_last_error("Failed to parse WKT geometry");
            return false;
        };
        match geometry.distance(x, y, z) {
            Ok(distance) => {
                if let Some(out_value) = out_value.as_mut() {
                    *out_value = distance;
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

/// Evaluate whether WKT geometry contains a point using the native GEOS adapter.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_value` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_wkt_contains_point(
    wkt: *const c_char,
    x: f64,
    y: f64,
    out_value: *mut bool,
) -> bool {
    ffi_catch(false, || {
        let Ok(geometry) = Geometry::from_wkt(&c_string_lossy(wkt)) else {
            set_last_error("Failed to parse WKT geometry");
            return false;
        };
        if let Some(out_value) = out_value.as_mut() {
            *out_value = geometry.contains(x, y);
        }
        true
    })
}

/// Evaluate whether WKT geometry covers a point using the native GEOS adapter.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_value` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_wkt_covers_point(
    wkt: *const c_char,
    x: f64,
    y: f64,
    out_value: *mut bool,
) -> bool {
    ffi_catch(false, || {
        let Ok(geometry) = Geometry::from_wkt(&c_string_lossy(wkt)) else {
            set_last_error("Failed to parse WKT geometry");
            return false;
        };
        if let Some(out_value) = out_value.as_mut() {
            *out_value = geometry.covers(x, y);
        }
        true
    })
}

/// Compute the area of WKT geometry using the native GEOS adapter.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_value` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_wkt_area(wkt: *const c_char, out_value: *mut f64) -> bool {
    ffi_catch(false, || {
        let Ok(geometry) = Geometry::from_wkt(&c_string_lossy(wkt)) else {
            set_last_error("Failed to parse WKT geometry");
            return false;
        };
        match geometry.area() {
            Ok(area) => {
                if let Some(out_value) = out_value.as_mut() {
                    *out_value = area;
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

/// Simplify WKT geometry using the native GEOS adapter.
///
/// Caller owns the returned WKT string and must free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_wkt` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_wkt_simplify(
    wkt: *const c_char,
    tolerance: f64,
    preserve_topology: bool,
    out_wkt: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        let Ok(geometry) = Geometry::from_wkt(&c_string_lossy(wkt)) else {
            set_last_error("Failed to parse WKT geometry");
            return false;
        };
        match geometry.simplify(tolerance, preserve_topology) {
            Ok(simplified) => match simplified.to_wkt() {
                Ok(wkt_str) => {
                    if let Some(out_wkt) = out_wkt.as_mut() {
                        *out_wkt = string_to_c_ptr(wkt_str);
                    }
                    true
                }
                Err(err) => {
                    set_last_error(err);
                    false
                }
            },
            Err(err) => {
                set_last_error(err);
                false
            }
        }
    })
}

/// Convert WKT geometry to canonical WKT using the native GEOS adapter.
///
/// Caller owns the returned WKT string and must free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_wkt` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_wkt_to_wkt(
    wkt: *const c_char,
    out_wkt: *mut *mut c_char,
) -> bool {
    pdal_geometry_wkt_to_wkt_precision(wkt, 16, out_wkt)
}

/// Convert WKT geometry to canonical WKT using the native GEOS adapter and an
/// explicit rounding precision.
///
/// Caller owns the returned WKT string and must free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_wkt` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_wkt_to_wkt_precision(
    wkt: *const c_char,
    precision: u32,
    out_wkt: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        let Ok(geometry) = Geometry::from_wkt(&c_string_lossy(wkt)) else {
            set_last_error("Failed to parse WKT geometry");
            return false;
        };
        match geometry.to_wkt_precision(precision) {
            Ok(wkt_str) => {
                if let Some(out_wkt) = out_wkt.as_mut() {
                    *out_wkt = string_to_c_ptr(wkt_str);
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

/// Compute the 3D bounding box bounds of WKT geometry using the native GEOS adapter.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_bounds` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_wkt_bounds(
    wkt: *const c_char,
    out_bounds: *mut crate::point_abi::pdal_bounds3d_t,
) -> bool {
    ffi_catch(false, || {
        let Ok(geometry) = Geometry::from_wkt(&c_string_lossy(wkt)) else {
            set_last_error("Failed to parse WKT geometry");
            return false;
        };
        match geometry.bounds() {
            Ok((minx, maxx, miny, maxy, minz, maxz)) => {
                if let Some(out_bounds) = out_bounds.as_mut() {
                    *out_bounds = crate::point_abi::pdal_bounds3d_t {
                        minx,
                        maxx,
                        miny,
                        maxy,
                        minz,
                        maxz,
                    };
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

/// Evaluate whether a GeoJSON string is a parseable geometry using the
/// native GEOS adapter. PDAL's optional top-level `srs` key is stripped
/// before parsing.
///
/// # Safety
///
/// `json` must be null or a valid NUL-terminated C string. `out_value` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_json_is_valid(
    json: *const c_char,
    out_value: *mut bool,
) -> bool {
    ffi_catch(false, || {
        let Ok(geometry) = Geometry::from_geojson(&c_string_lossy(json)) else {
            set_last_error("Failed to parse GeoJSON geometry");
            return false;
        };
        match geometry.is_valid() {
            Ok(valid) => {
                if let Some(out_value) = out_value.as_mut() {
                    *out_value = valid;
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

/// Convert WKT geometry to a GDAL-formatted GeoJSON string
/// (`OGR_G_ExportToJsonEx(COORDINATE_PRECISION=precision)` byte-for-byte
/// shape: single line, spaces around braces/brackets, fixed coordinate
/// precision with trailing zeros trimmed). Only supports geometry types
/// used by `pdal::Polygon`: Point, LineString, Polygon, MultiPolygon.
///
/// Caller owns the returned string and must free it with `pdal_string_free`.
///
/// # Safety
///
/// `wkt` must be null or a valid NUL-terminated C string. `out_json` must be
/// null or valid for writes.
#[no_mangle]
pub unsafe extern "C" fn pdal_geometry_wkt_to_json(
    wkt: *const c_char,
    precision: u32,
    out_json: *mut *mut c_char,
) -> bool {
    ffi_catch(false, || {
        let Ok(geometry) = Geometry::from_wkt(&c_string_lossy(wkt)) else {
            set_last_error("Failed to parse WKT geometry");
            return false;
        };
        match geometry.to_gdal_geojson(precision) {
            Ok(json_str) => {
                if let Some(out_json) = out_json.as_mut() {
                    *out_json = string_to_c_ptr(json_str);
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

/// Resolve a GDAL `OGRSpatialReference::SetFromUserInput`-style string into
/// canonical WKT1 and WKT2_2018 plus any parsed coordinate epoch. Caller owns
/// the returned WKT strings and must free them with `pdal_string_free`.
///
/// # Safety
///
/// `input` must be a valid NUL-terminated C string. `out_wkt`/`out_wkt2`/
/// `out_epoch` must be null or valid for writes.
#[no_mangle]
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
#[no_mangle]
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

/// Mirror `OGRSpatialReference::IsSame` for two WKT strings at the given
/// coordinate epoch. Pass `0.0` for `epoch` when neither side has a fixed
/// epoch. Returns `false` and clears `out_same` if either side fails to import.
///
/// # Safety
///
/// `wkt_a` and `wkt_b` must be null or valid NUL-terminated C strings.
/// `out_same` must be null or valid for writes.
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
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
#[no_mangle]
pub unsafe extern "C" fn pdal_srs_transform_destroy(handle: *mut pdal_srs_transform_t) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Transform a single XYZ point in place. Returns false on null handle or
/// GDAL-reported failure.
///
/// # Safety
///
/// `handle` must come from `pdal_srs_transform_create`. `x`/`y`/`z` must be
/// non-null and valid for read+write.
#[no_mangle]
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
#[no_mangle]
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

unsafe fn c_string_lossy(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

// --- NITF native bridge (readers.nitf / writers.nitf) ---

/// Locate the LIDARA data extension segment in a NITF file.
///
/// On success returns true and writes the LIDARA segment offset and length
/// (bytes) to `out_offset` and `out_length`. On failure returns false and
/// sets the last error string.
///
/// # Safety
///
/// `path` must be null or a valid NUL-terminated C string. `out_offset` and
/// `out_length` must be valid for writes (non-null).
#[no_mangle]
pub unsafe extern "C" fn pdal_nitf_lidar_segment(
    path: *const c_char,
    out_offset: *mut u64,
    out_length: *mut u64,
) -> bool {
    ffi_catch(false, || {
        let path = c_string_lossy(path);
        if path.is_empty() {
            set_last_error("pdal_nitf_lidar_segment: null/empty path");
            return false;
        }
        match pdal_native::nitf::lidar_segment(&path) {
            Ok((offset, length)) => {
                if let Some(out_offset) = out_offset.as_mut() {
                    *out_offset = offset;
                }
                if let Some(out_length) = out_length.as_mut() {
                    *out_length = length;
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

/// Enumerate NITF file/image/DES header fields and TREs, invoking `cb` once
/// per `(key, value)` pair. `key` is a stable dotted path such as `FH.FDT`
/// or `IM:0.IGEOLO`. Returning non-zero from `cb` stops the enumeration.
///
/// # Safety
///
/// `path` must be null or a valid NUL-terminated C string. `cb` must be a
/// callable function pointer with the documented signature.
#[no_mangle]
pub unsafe extern "C" fn pdal_nitf_read_metadata(
    path: *const c_char,
    cb: Option<
        unsafe extern "C" fn(
            key: *const c_char,
            value: *const c_char,
            userdata: *mut std::os::raw::c_void,
        ) -> std::os::raw::c_int,
    >,
    userdata: *mut std::os::raw::c_void,
) -> bool {
    ffi_catch(false, || {
        let Some(cb) = cb else {
            set_last_error("pdal_nitf_read_metadata: null callback");
            return false;
        };
        let path = c_string_lossy(path);
        if path.is_empty() {
            set_last_error("pdal_nitf_read_metadata: null/empty path");
            return false;
        }
        match pdal_native::nitf::read_metadata(&path) {
            Ok(map) => {
                for (key, value) in map {
                    let key_c = match std::ffi::CString::new(key) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    let value_c = match std::ffi::CString::new(value) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    if cb(key_c.as_ptr(), value_c.as_ptr(), userdata) != 0 {
                        break;
                    }
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

/// C-ABI form of `NitfWriteOptions`. String fields may be null; lists of
/// `name:value` AIMIDB/ACFTB overrides are null-terminated arrays of C strings
/// (or null when unused).
#[repr(C)]
pub struct pdal_nitf_write_options_t {
    pub file_title: *const c_char,
    pub complexity_level: *const c_char,
    pub system_type: *const c_char,
    pub origin_station_id: *const c_char,
    pub file_class: *const c_char,
    pub origin_name: *const c_char,
    pub origin_phone: *const c_char,
    pub fsclsy: *const c_char,
    pub fsctlh: *const c_char,
    pub fscltx: *const c_char,
    pub image_security_class: *const c_char,
    pub image_date_time: *const c_char,
    pub image_id2: *const c_char,
    pub aimidb: *const *const c_char,
    pub acftb: *const *const c_char,
    pub minx: f64,
    pub miny: f64,
    pub maxx: f64,
    pub maxy: f64,
}

/// Wrap an existing LAS/BPF payload at `input_path` as a NITF file at
/// `output_path`, applying the supplied writer options.
///
/// # Safety
///
/// `input_path` and `output_path` must be valid NUL-terminated C strings.
/// `opts` must point to a fully-initialized `pdal_nitf_write_options_t`. Any
/// non-null `aimidb`/`acftb` entries must terminate with a null pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_nitf_write(
    input_path: *const c_char,
    output_path: *const c_char,
    opts: *const pdal_nitf_write_options_t,
) -> bool {
    ffi_catch(false, || {
        let input = c_string_lossy(input_path);
        let output = c_string_lossy(output_path);
        if input.is_empty() || output.is_empty() {
            set_last_error("pdal_nitf_write: null/empty input or output path");
            return false;
        }
        let Some(opts) = opts.as_ref() else {
            set_last_error("pdal_nitf_write: null options");
            return false;
        };

        let owned = pdal_native::nitf::NitfWriteOptions {
            file_title: optional_cstr(opts.file_title),
            complexity_level: optional_cstr(opts.complexity_level),
            system_type: optional_cstr(opts.system_type),
            origin_station_id: optional_cstr(opts.origin_station_id),
            file_class: optional_cstr(opts.file_class),
            origin_name: optional_cstr(opts.origin_name),
            origin_phone: optional_cstr(opts.origin_phone),
            fsclsy: optional_cstr(opts.fsclsy),
            fsctlh: optional_cstr(opts.fsctlh),
            fscltx: optional_cstr(opts.fscltx),
            image_security_class: optional_cstr(opts.image_security_class),
            image_date_time: optional_cstr(opts.image_date_time),
            image_id2: optional_cstr(opts.image_id2),
            aimidb: collect_null_terminated(opts.aimidb),
            acftb: collect_null_terminated(opts.acftb),
            minx: opts.minx,
            miny: opts.miny,
            maxx: opts.maxx,
            maxy: opts.maxy,
        };

        match pdal_native::nitf::write(&input, &output, &owned) {
            Ok(()) => true,
            Err(err) => {
                set_last_error(err);
                false
            }
        }
    })
}

unsafe fn optional_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        let s = c_string_lossy(ptr);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

unsafe fn collect_null_terminated(ptr: *const *const c_char) -> Vec<String> {
    if ptr.is_null() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut cursor = ptr;
    loop {
        let item = *cursor;
        if item.is_null() {
            break;
        }
        out.push(c_string_lossy(item));
        cursor = cursor.add(1);
    }
    out
}
