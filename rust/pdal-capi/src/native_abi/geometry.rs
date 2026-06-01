use super::c_string_lossy;
use crate::error::{ffi_catch, set_last_error, string_to_c_ptr};
use pdal_core::geometry::Geometry;
use std::os::raw::c_char;

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
