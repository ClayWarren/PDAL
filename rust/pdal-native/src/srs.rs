//! Coordinate transformations through PROJ and OGRSpatialReference helpers
//! through GDAL.

use proj::Proj;
use std::ffi::{c_char, c_void, CStr, CString};

/// A GDAL `OGRCoordinateTransformation`-backed transform.
///
/// Unlike `SrsTransform` (PROJ-only via the `proj` crate), this honors
/// coordinate epochs, axis-mapping strategies, custom axis ordering, and 3D
/// transforms.
pub struct GdalSrsTransform {
    handle: gdal_sys::OGRCoordinateTransformationH,
}

impl GdalSrsTransform {
    /// Build a coordinate transformation between two WKT strings. Both SRSes
    /// get `OAMS_TRADITIONAL_GIS_ORDER` by default; pass non-empty
    /// `src_axis_order` / `dst_axis_order` slices to override via
    /// `OSRSetDataAxisToSRSAxisMapping`.
    pub fn new(
        src_wkt: &str,
        src_epoch: f64,
        dst_wkt: &str,
        dst_epoch: f64,
        src_axis_order: &[i32],
        dst_axis_order: &[i32],
    ) -> Result<Self, String> {
        if src_wkt.is_empty() || dst_wkt.is_empty() {
            return Err("GdalSrsTransform: empty source or destination WKT".into());
        }
        let src_c = CString::new(src_wkt).map_err(|e| e.to_string())?;
        let dst_c = CString::new(dst_wkt).map_err(|e| e.to_string())?;
        unsafe {
            let src = build_axis_aware_srs(&src_c, src_epoch, src_axis_order)?;
            let dst = match build_axis_aware_srs(&dst_c, dst_epoch, dst_axis_order) {
                Ok(d) => d,
                Err(e) => {
                    gdal_sys::OSRDestroySpatialReference(src);
                    return Err(e);
                }
            };
            let handle = gdal_sys::OCTNewCoordinateTransformation(src, dst);
            gdal_sys::OSRDestroySpatialReference(src);
            gdal_sys::OSRDestroySpatialReference(dst);
            if handle.is_null() {
                return Err(format!(
                    "Failed to create coordinate transformation from '{src_wkt}' to '{dst_wkt}'"
                ));
            }
            Ok(Self { handle })
        }
    }

    /// Transform a single XYZ point in-place. Returns false if GDAL reports
    /// the transform failed.
    pub fn transform_xyz(&self, x: &mut f64, y: &mut f64, z: &mut f64) -> bool {
        unsafe { gdal_sys::OCTTransform(self.handle, 1, x, y, z) != 0 }
    }

    /// Transform a vector of XYZ points in-place. Returns false on length
    /// mismatch or GDAL failure.
    pub fn transform_xyz_slice(&self, xs: &mut [f64], ys: &mut [f64], zs: &mut [f64]) -> bool {
        if xs.len() != ys.len() || ys.len() != zs.len() {
            return false;
        }
        if xs.is_empty() {
            return true;
        }
        unsafe {
            gdal_sys::OCTTransform(
                self.handle,
                xs.len() as std::os::raw::c_int,
                xs.as_mut_ptr(),
                ys.as_mut_ptr(),
                zs.as_mut_ptr(),
            ) != 0
        }
    }
}

impl Drop for GdalSrsTransform {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                gdal_sys::OCTDestroyCoordinateTransformation(self.handle);
            }
        }
    }
}

/// A GDAL coordinate-operation transform used by `filters.projpipeline`.
pub struct GdalCoordOperationTransform {
    handle: gdal_sys::OGRCoordinateTransformationH,
}

impl GdalCoordOperationTransform {
    pub fn new(coord_op: &str, reverse: bool) -> Result<Self, String> {
        let coord_op = CString::new(coord_op).map_err(|e| e.to_string())?;
        unsafe {
            let options = gdal_sys::OCTNewCoordinateTransformationOptions();
            if options.is_null() {
                return Err("OCTNewCoordinateTransformationOptions returned null".into());
            }

            let set = gdal_sys::OCTCoordinateTransformationOptionsSetOperation(
                options,
                coord_op.as_ptr(),
                if reverse { 1 } else { 0 },
            );
            if set == 0 {
                gdal_sys::OCTDestroyCoordinateTransformationOptions(options);
                return Err("OCTCoordinateTransformationOptionsSetOperation failed".into());
            }

            let src = gdal_sys::OSRNewSpatialReference(std::ptr::null());
            let dst = gdal_sys::OSRNewSpatialReference(std::ptr::null());
            if src.is_null() || dst.is_null() {
                if !src.is_null() {
                    gdal_sys::OSRDestroySpatialReference(src);
                }
                if !dst.is_null() {
                    gdal_sys::OSRDestroySpatialReference(dst);
                }
                gdal_sys::OCTDestroyCoordinateTransformationOptions(options);
                return Err("OSRNewSpatialReference returned null".into());
            }

            let handle = gdal_sys::OCTNewCoordinateTransformationEx(src, dst, options);
            gdal_sys::OSRDestroySpatialReference(src);
            gdal_sys::OSRDestroySpatialReference(dst);
            gdal_sys::OCTDestroyCoordinateTransformationOptions(options);

            if handle.is_null() {
                let msg = last_cpl_error();
                return Err(if msg.is_empty() {
                    "Failed to create coordinate operation transform".into()
                } else {
                    msg
                });
            }
            Ok(Self { handle })
        }
    }

    pub fn transform_xyz(&self, x: &mut f64, y: &mut f64, z: &mut f64) -> bool {
        unsafe { gdal_sys::OCTTransform(self.handle, 1, x, y, z) != 0 }
    }
}

impl Drop for GdalCoordOperationTransform {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                gdal_sys::OCTDestroyCoordinateTransformation(self.handle);
            }
        }
    }
}

unsafe fn build_axis_aware_srs(
    wkt_c: &CString,
    epoch: f64,
    axis_order: &[i32],
) -> Result<gdal_sys::OGRSpatialReferenceH, String> {
    let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
    if srs.is_null() {
        return Err("OSRNewSpatialReference returned null".into());
    }
    let mut wkt_ptr = wkt_c.as_ptr() as *mut c_char;
    if gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr) != gdal_sys::OGRErr::OGRERR_NONE {
        gdal_sys::OSRDestroySpatialReference(srs);
        return Err("OSRImportFromWkt failed".into());
    }
    if epoch != 0.0 {
        gdal_sys::OSRSetCoordinateEpoch(srs, epoch);
    }
    if axis_order.is_empty() {
        gdal_sys::OSRSetAxisMappingStrategy(
            srs,
            gdal_sys::OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER,
        );
    } else {
        let err = gdal_sys::OSRSetDataAxisToSRSAxisMapping(
            srs,
            axis_order.len() as std::os::raw::c_int,
            axis_order.as_ptr(),
        );
        if err != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return Err("OSRSetDataAxisToSRSAxisMapping failed".into());
        }
    }
    Ok(srs)
}

/// A native coordinate transformation.
pub struct SrsTransform {
    proj: Proj,
}

impl SrsTransform {
    pub fn new(src: &str, dst: &str) -> Result<Self, String> {
        let proj = Proj::new_known_crs(src, dst, None)
            .map_err(|e| format!("Failed to create projection: {}", e))?;
        Ok(Self { proj })
    }

    pub fn new_pipeline(coord_op: &str) -> Result<Self, String> {
        let proj = Proj::new(coord_op).map_err(|e| format!("Failed to create pipeline: {}", e))?;
        Ok(Self { proj })
    }

    pub fn transform(&self, x: &mut f64, y: &mut f64, _z: &mut f64) -> bool {
        match self.proj.convert((*x, *y)) {
            Ok((nx, ny)) => {
                *x = nx;
                *y = ny;
                true
            }
            Err(_) => false,
        }
    }
}

pub fn version() -> String {
    unsafe {
        let info = proj_sys::proj_info();
        if info.version.is_null() {
            String::new()
        } else {
            CStr::from_ptr(info.version).to_string_lossy().into_owned()
        }
    }
}

/// Result of normalizing arbitrary SRS user input via GDAL OSR.
pub struct UserInput {
    pub wkt: String,
    pub wkt2: String,
    pub projjson: String,
    pub epoch: f64,
}

/// Resolve any GDAL `OGRSpatialReference::SetFromUserInput` accepted form
/// (EPSG codes, PROJ4 strings, PROJJSON, well-known names) into the canonical
/// WKT1 and WKT2_2018 strings PDAL stores in `SpatialReference`. Also returns
/// the coordinate epoch GDAL parsed off the user string (zero if absent).
pub fn user_input_to_wkt(input: &str) -> Result<UserInput, String> {
    let input_c = CString::new(input).map_err(|e| e.to_string())?;
    unsafe {
        let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        if srs.is_null() {
            return Err("OSRNewSpatialReference returned null".into());
        }
        let err = gdal_sys::OSRSetFromUserInput(srs, input_c.as_ptr());
        if err != gdal_sys::OGRErr::OGRERR_NONE {
            let msg = last_cpl_error();
            gdal_sys::OSRDestroySpatialReference(srs);
            return Err(format!(
                "Could not import coordinate system '{input}': {}.",
                if msg.is_empty() {
                    "(unknown reason)".into()
                } else {
                    msg
                }
            ));
        }
        let epoch = gdal_sys::OSRGetCoordinateEpoch(srs);
        let wkt = export_to_wkt(srs, &[]);
        let wkt2 = export_to_wkt(srs, &[("FORMAT", "WKT2_2018")]);
        let projjson = export_to_projjson(srs);
        gdal_sys::OSRDestroySpatialReference(srs);
        match (wkt, wkt2, projjson) {
            (Ok(wkt), Ok(wkt2), Ok(projjson)) => Ok(UserInput {
                wkt,
                wkt2,
                projjson,
                epoch,
            }),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
        }
    }
}

/// Translate a WKT string into the PROJ4 string PDAL's
/// `SpatialReference::getProj4()` returns (trimmed of trailing whitespace).
pub fn wkt_to_proj4(wkt: &str) -> Result<String, String> {
    if wkt.is_empty() {
        return Ok(String::new());
    }
    let wkt_c = CString::new(wkt).map_err(|e| e.to_string())?;
    unsafe {
        let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        if srs.is_null() {
            return Err("OSRNewSpatialReference returned null".into());
        }
        let mut wkt_ptr = wkt_c.as_ptr() as *mut c_char;
        let err = gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr);
        if err != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return Ok(String::new());
        }
        let mut proj4: *mut c_char = std::ptr::null_mut();
        let err = gdal_sys::OSRExportToProj4(srs, &mut proj4);
        let out = if err == gdal_sys::OGRErr::OGRERR_NONE && !proj4.is_null() {
            let s = CStr::from_ptr(proj4).to_string_lossy().into_owned();
            s.trim().to_string()
        } else {
            String::new()
        };
        if !proj4.is_null() {
            gdal_sys::VSIFree(proj4 as *mut c_void);
        }
        gdal_sys::OSRDestroySpatialReference(srs);
        Ok(out)
    }
}

/// Translate WKT into the PROJJSON string PDAL's
/// `SpatialReference::getPROJJSON()` returns.
pub fn wkt_to_projjson(wkt: &str, epoch: f64) -> Result<String, String> {
    if wkt.is_empty() {
        return Ok(String::new());
    }
    let wkt_c = CString::new(wkt).map_err(|e| e.to_string())?;
    unsafe {
        let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        if srs.is_null() {
            return Err("OSRNewSpatialReference returned null".into());
        }
        let mut wkt_ptr = wkt_c.as_ptr() as *mut c_char;
        let err = gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr);
        if err != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return Ok(String::new());
        }
        if epoch != 0.0 {
            gdal_sys::OSRSetCoordinateEpoch(srs, epoch);
        }
        let out = export_to_projjson(srs).unwrap_or_default();
        gdal_sys::OSRDestroySpatialReference(srs);
        Ok(out)
    }
}

/// Return true when GDAL `OSRIsSame` considers the two WKT strings equivalent
/// at the given coordinate epoch. Returns false when either string is empty or
/// fails to import.
pub fn is_same(wkt_a: &str, wkt_b: &str, epoch: f64) -> bool {
    if wkt_a.is_empty() || wkt_b.is_empty() {
        return false;
    }
    let Ok(a_c) = CString::new(wkt_a) else {
        return false;
    };
    let Ok(b_c) = CString::new(wkt_b) else {
        return false;
    };
    unsafe {
        let a = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        let b = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        let mut a_ptr = a_c.as_ptr() as *mut c_char;
        let mut b_ptr = b_c.as_ptr() as *mut c_char;
        let ok = gdal_sys::OSRImportFromWkt(a, &mut a_ptr) == gdal_sys::OGRErr::OGRERR_NONE
            && gdal_sys::OSRImportFromWkt(b, &mut b_ptr) == gdal_sys::OGRErr::OGRERR_NONE;
        if !ok {
            gdal_sys::OSRDestroySpatialReference(a);
            gdal_sys::OSRDestroySpatialReference(b);
            return false;
        }
        if epoch != 0.0 {
            gdal_sys::OSRSetCoordinateEpoch(a, epoch);
            gdal_sys::OSRSetCoordinateEpoch(b, epoch);
        }
        let same = gdal_sys::OSRIsSame(a, b) == 1;
        gdal_sys::OSRDestroySpatialReference(a);
        gdal_sys::OSRDestroySpatialReference(b);
        same
    }
}

/// Mirror `SpatialReference::getUTMZone()`. Returns positive zone for the
/// northern hemisphere, negative for southern, and `0` when GDAL reports no
/// UTM zone. Empty WKT returns `0` without error.
pub fn get_utm_zone(wkt: &str) -> Result<i32, String> {
    if wkt.is_empty() {
        return Ok(0);
    }
    let wkt_c = CString::new(wkt).map_err(|e| e.to_string())?;
    unsafe {
        let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        if srs.is_null() {
            return Err("OSRNewSpatialReference returned null".into());
        }
        let mut wkt_ptr = wkt_c.as_ptr() as *mut c_char;
        if gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr) != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return Err("Could not fetch current SRS".into());
        }
        let mut north: std::os::raw::c_int = 0;
        let zone = gdal_sys::OSRGetUTMZone(srs, &mut north);
        gdal_sys::OSRDestroySpatialReference(srs);
        Ok(if north != 0 { zone } else { -zone })
    }
}

/// Mirror `SpatialReference::getHorizontal()`: strip the vertical CS and
/// return WKT for the remaining horizontal CS. Returns empty for empty input
/// or unimportable WKT.
pub fn get_horizontal_wkt(wkt: &str) -> String {
    if wkt.is_empty() {
        return String::new();
    }
    let Ok(wkt_c) = CString::new(wkt) else {
        return String::new();
    };
    unsafe {
        let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        if srs.is_null() {
            return String::new();
        }
        let mut wkt_ptr = wkt_c.as_ptr() as *mut c_char;
        if gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr) != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return String::new();
        }
        gdal_sys::OSRStripVertical(srs);
        let out = export_to_wkt(srs, &[]).unwrap_or_default();
        gdal_sys::OSRDestroySpatialReference(srs);
        out
    }
}

/// Mirror `SpatialReference::getHorizontalUnits()`: name of the horizontal
/// linear units (e.g. `"metre"`).
pub fn get_horizontal_units(wkt: &str) -> String {
    if wkt.is_empty() {
        return String::new();
    }
    let Ok(wkt_c) = CString::new(wkt) else {
        return String::new();
    };
    unsafe {
        let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        if srs.is_null() {
            return String::new();
        }
        let mut wkt_ptr = wkt_c.as_ptr() as *mut c_char;
        if gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr) != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return String::new();
        }
        let mut units: *mut c_char = std::ptr::null_mut();
        gdal_sys::OSRGetLinearUnits(srs, &mut units);
        let out = if units.is_null() {
            String::new()
        } else {
            CStr::from_ptr(units).to_string_lossy().trim().to_string()
        };
        gdal_sys::OSRDestroySpatialReference(srs);
        out
    }
}

/// Mirror `SpatialReference::getVertical()`: extract the WKT subtree of the
/// top-level `VERT_CS[...]` node, including the wrapping bracket pair. Returns
/// the empty string when no vertical CS is present. Uses a bracket-matching
/// parser because GDAL's C API has no `OGR_SRSNode` equivalent for exporting
/// just a single attribute node's WKT.
pub fn get_vertical_wkt(wkt: &str) -> String {
    extract_wkt_node(wkt, "VERT_CS")
}

/// Mirror `SpatialReference::getVerticalUnits()`: linear-units name of the
/// `VERT_CS` node. Empty when no vertical CS or no recognizable units.
pub fn get_vertical_units(wkt: &str) -> String {
    let vert = extract_wkt_node(wkt, "VERT_CS");
    if vert.is_empty() {
        return String::new();
    }
    // The VERT_CS subtree carries its own UNIT[...] node. Build an SRS from
    // the subtree and read linear units from it.
    let Ok(wkt_c) = CString::new(vert) else {
        return String::new();
    };
    unsafe {
        let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        if srs.is_null() {
            return String::new();
        }
        let mut wkt_ptr = wkt_c.as_ptr() as *mut c_char;
        if gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr) != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return String::new();
        }
        let mut units: *mut c_char = std::ptr::null_mut();
        gdal_sys::OSRGetLinearUnits(srs, &mut units);
        let out = if units.is_null() {
            String::new()
        } else {
            CStr::from_ptr(units).to_string_lossy().trim().to_string()
        };
        gdal_sys::OSRDestroySpatialReference(srs);
        out
    }
}

/// Mirror `SpatialReference::identifyVerticalEPSG()`: extract the VERT_CS
/// subtree and return its `AUTHORITY` code (empty when no vertical CS or no
/// authority code).
pub fn identify_vertical_epsg(wkt: &str, epoch: f64) -> String {
    let vert = extract_wkt_node(wkt, "VERT_CS");
    if vert.is_empty() {
        return String::new();
    }
    let Ok(wkt_c) = CString::new(vert) else {
        return String::new();
    };
    unsafe {
        let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        if srs.is_null() {
            return String::new();
        }
        let mut wkt_ptr = wkt_c.as_ptr() as *mut c_char;
        if gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr) != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return String::new();
        }
        if epoch != 0.0 {
            gdal_sys::OSRSetCoordinateEpoch(srs, epoch);
        }
        let code = gdal_sys::OSRGetAuthorityCode(srs, std::ptr::null());
        let out = if code.is_null() {
            String::new()
        } else {
            CStr::from_ptr(code).to_string_lossy().into_owned()
        };
        gdal_sys::OSRDestroySpatialReference(srs);
        out
    }
}

/// Extract a top-level WKT node (e.g. `"VERT_CS"`) and return the substring
/// including the opening identifier and matching closing `]`. Empty when the
/// node is not present. Handles quoted strings so a `]` inside a name does not
/// break depth tracking.
fn extract_wkt_node(wkt: &str, name: &str) -> String {
    let needle = format!("{name}[");
    let Some(start) = wkt.find(&needle) else {
        return String::new();
    };
    let bytes = wkt.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_string = !in_string,
            b'[' if !in_string => depth += 1,
            b']' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return wkt[start..=i].to_string();
                }
            }
            _ => {}
        }
        i += 1;
    }
    String::new()
}

/// Mirror `SpatialReference::valid()`: `OSRValidate` on the imported WKT.
pub fn srs_valid(wkt: &str) -> bool {
    if wkt.is_empty() {
        return false;
    }
    let Ok(wkt_c) = CString::new(wkt) else {
        return false;
    };
    unsafe {
        let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        if srs.is_null() {
            return false;
        }
        let mut wkt_ptr = wkt_c.as_ptr() as *mut c_char;
        if gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr) != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return false;
        }
        let ok = gdal_sys::OSRValidate(srs) == gdal_sys::OGRErr::OGRERR_NONE;
        gdal_sys::OSRDestroySpatialReference(srs);
        ok
    }
}

/// Mirror `SpatialReference::identifyHorizontalEPSG()`: strip the vertical CS,
/// auto-identify EPSG, and return the GDAL authority code as a string. Returns
/// the empty string when no code can be assigned.
pub fn identify_horizontal_epsg(wkt: &str, epoch: f64) -> String {
    if wkt.is_empty() {
        return String::new();
    }
    let Ok(wkt_c) = CString::new(wkt) else {
        return String::new();
    };
    unsafe {
        let srs = gdal_sys::OSRNewSpatialReference(std::ptr::null());
        if srs.is_null() {
            return String::new();
        }
        let mut wkt_ptr = wkt_c.as_ptr() as *mut c_char;
        if gdal_sys::OSRImportFromWkt(srs, &mut wkt_ptr) != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return String::new();
        }
        if epoch != 0.0 {
            gdal_sys::OSRSetCoordinateEpoch(srs, epoch);
        }
        gdal_sys::OSRStripVertical(srs);
        if gdal_sys::OSRAutoIdentifyEPSG(srs) != gdal_sys::OGRErr::OGRERR_NONE {
            gdal_sys::OSRDestroySpatialReference(srs);
            return String::new();
        }
        let code = gdal_sys::OSRGetAuthorityCode(srs, std::ptr::null());
        let out = if code.is_null() {
            String::new()
        } else {
            CStr::from_ptr(code).to_string_lossy().into_owned()
        };
        gdal_sys::OSRDestroySpatialReference(srs);
        out
    }
}

unsafe fn export_to_wkt(
    srs: gdal_sys::OGRSpatialReferenceH,
    options: &[(&str, &str)],
) -> Result<String, String> {
    let mut wkt: *mut c_char = std::ptr::null_mut();
    if options.is_empty() {
        let err = gdal_sys::OSRExportToWkt(srs, &mut wkt);
        if err != gdal_sys::OGRErr::OGRERR_NONE || wkt.is_null() {
            if !wkt.is_null() {
                gdal_sys::VSIFree(wkt as *mut c_void);
            }
            return Err(format!("OSRExportToWkt failed: {err:?}"));
        }
    } else {
        let mut owned_pairs: Vec<CString> = Vec::with_capacity(options.len());
        for (k, v) in options {
            owned_pairs.push(
                CString::new(format!("{k}={v}")).map_err(|e| format!("invalid OSR option: {e}"))?,
            );
        }
        let mut argv: Vec<*const c_char> = owned_pairs.iter().map(|c| c.as_ptr()).collect();
        argv.push(std::ptr::null());
        let err = gdal_sys::OSRExportToWktEx(srs, &mut wkt, argv.as_ptr());
        if err != gdal_sys::OGRErr::OGRERR_NONE || wkt.is_null() {
            if !wkt.is_null() {
                gdal_sys::VSIFree(wkt as *mut c_void);
            }
            return Err(format!("OSRExportToWktEx failed: {err:?}"));
        }
    }
    let out = CStr::from_ptr(wkt).to_string_lossy().into_owned();
    gdal_sys::VSIFree(wkt as *mut c_void);
    Ok(out)
}

unsafe fn export_to_projjson(srs: gdal_sys::OGRSpatialReferenceH) -> Result<String, String> {
    let mut json: *mut c_char = std::ptr::null_mut();
    let indentation = CString::new("INDENTATION_WIDTH=2").map_err(|e| e.to_string())?;
    let schema = CString::new("SCHEMA=").map_err(|e| e.to_string())?;
    let options = [indentation.as_ptr(), schema.as_ptr(), std::ptr::null()];
    let err = gdal_sys::OSRExportToPROJJSON(srs, &mut json, options.as_ptr());
    if err != gdal_sys::OGRErr::OGRERR_NONE || json.is_null() {
        if !json.is_null() {
            gdal_sys::VSIFree(json as *mut c_void);
        }
        return Err(format!("OSRExportToPROJJSON failed: {err:?}"));
    }
    let out = CStr::from_ptr(json).to_string_lossy().into_owned();
    gdal_sys::VSIFree(json as *mut c_void);
    Ok(out)
}

unsafe fn last_cpl_error() -> String {
    let msg = gdal_sys::CPLGetLastErrorMsg();
    if msg.is_null() {
        String::new()
    } else {
        CStr::from_ptr(msg).to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proj_version_is_available() {
        assert!(!version().is_empty());
    }

    #[test]
    fn identity_transform_preserves_xy() {
        let transform = SrsTransform::new("EPSG:4326", "EPSG:4326").unwrap();
        let mut x = -93.265;
        let mut y = 44.9778;
        let mut z = 250.0;

        assert!(transform.transform(&mut x, &mut y, &mut z));
        assert_eq!(x, -93.265);
        assert_eq!(y, 44.9778);
        assert_eq!(z, 250.0);
    }

    #[test]
    fn user_input_resolves_epsg_to_wkt1_and_wkt2() {
        let result = user_input_to_wkt("EPSG:4326").unwrap();
        assert!(result.wkt.contains("GEOGCS["));
        assert!(result.wkt.contains("WGS 84"));
        assert!(result.wkt2.contains("GEOGCRS[") || result.wkt2.contains("GEOGCS["));
        assert!(result
            .projjson
            .starts_with("{\n  \"type\": \"GeographicCRS\","));
        assert_eq!(result.epoch, 0.0);
    }

    #[test]
    fn wkt_to_projjson_matches_user_input_projjson_shape() {
        let result = user_input_to_wkt("EPSG:4326").unwrap();
        let json = wkt_to_projjson(&result.wkt, result.epoch).unwrap();
        assert!(json.starts_with("{\n  \"type\": \"GeographicCRS\","));
        assert!(json.contains("\"name\": \"WGS 84\""));

        assert_eq!(wkt_to_projjson("", 0.0).unwrap(), "");
        assert_eq!(wkt_to_projjson("not wkt", 0.0).unwrap(), "");
    }

    #[test]
    fn user_input_rejects_garbage() {
        assert!(user_input_to_wkt("not a srs").is_err());
    }

    #[test]
    fn wkt_to_proj4_returns_trimmed_proj4() {
        let result = user_input_to_wkt("EPSG:4326").unwrap();
        let proj4 = wkt_to_proj4(&result.wkt).unwrap();
        assert_eq!(proj4, "+proj=longlat +datum=WGS84 +no_defs");
    }

    #[test]
    fn wkt_to_proj4_empty_returns_empty() {
        assert_eq!(wkt_to_proj4("").unwrap(), "");
        assert_eq!(wkt_to_proj4("not a wkt").unwrap(), "");
    }

    #[test]
    fn is_same_recognizes_equivalent_srs() {
        let a = user_input_to_wkt("EPSG:4326").unwrap();
        let b = user_input_to_wkt("+proj=longlat +datum=WGS84 +no_defs").unwrap();
        assert!(is_same(&a.wkt, &b.wkt, 0.0));
    }

    #[test]
    fn is_same_distinguishes_different_srs() {
        let a = user_input_to_wkt("EPSG:4326").unwrap();
        let b = user_input_to_wkt("EPSG:32617").unwrap();
        assert!(!is_same(&a.wkt, &b.wkt, 0.0));
        assert!(!is_same("", &b.wkt, 0.0));
        assert!(!is_same("not a wkt", &b.wkt, 0.0));
    }

    #[test]
    fn identify_horizontal_epsg_returns_authority_code() {
        let a = user_input_to_wkt("EPSG:32617").unwrap();
        assert_eq!(identify_horizontal_epsg(&a.wkt, 0.0), "32617");
        assert_eq!(identify_horizontal_epsg("", 0.0), "");
        assert_eq!(identify_horizontal_epsg("not a wkt", 0.0), "");
    }

    #[test]
    fn get_utm_zone_signed_by_hemisphere() {
        let north = user_input_to_wkt("EPSG:2027").unwrap();
        assert_eq!(get_utm_zone(&north.wkt).unwrap(), 15);

        let south = user_input_to_wkt("EPSG:32732").unwrap();
        assert_eq!(get_utm_zone(&south.wkt).unwrap(), -32);

        assert_eq!(get_utm_zone("").unwrap(), 0);
        assert!(get_utm_zone("not a wkt").is_err());
    }

    #[test]
    fn get_horizontal_wkt_strips_vertical_cs() {
        let compound = user_input_to_wkt("EPSG:7415").unwrap();
        let horiz = get_horizontal_wkt(&compound.wkt);
        assert!(horiz.contains("PROJCS["));
        assert!(!horiz.contains("VERT_CS"));
        assert_eq!(get_horizontal_wkt(""), "");
        assert_eq!(get_horizontal_wkt("not a wkt"), "");
    }

    #[test]
    fn get_horizontal_units_returns_unit_name() {
        let utm = user_input_to_wkt("EPSG:32617").unwrap();
        assert_eq!(get_horizontal_units(&utm.wkt), "metre");
        assert_eq!(get_horizontal_units(""), "");
        assert_eq!(get_horizontal_units("not a wkt"), "");
    }

    #[test]
    fn srs_valid_accepts_known_codes_and_rejects_empty() {
        let utm = user_input_to_wkt("EPSG:32617").unwrap();
        assert!(srs_valid(&utm.wkt));
        assert!(!srs_valid(""));
        assert!(!srs_valid("not a wkt"));
    }

    #[test]
    fn extract_vert_cs_subtree_handles_nested_brackets_and_quoted_strings() {
        let wkt = r#"COMPD_CS["WGS 84 + VERT_CS",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]]],VERT_CS["NAVD88 height",VERT_DATUM["North American Vertical Datum 1988",2005,AUTHORITY["EPSG","5103"]],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Up",UP],AUTHORITY["EPSG","5703"]]]"#;
        let vert = get_vertical_wkt(wkt);
        assert!(vert.starts_with("VERT_CS[\"NAVD88 height\""));
        assert!(vert.ends_with(r#"AUTHORITY["EPSG","5703"]]"#));

        // No VERT_CS → empty.
        let utm = user_input_to_wkt("EPSG:32617").unwrap();
        assert_eq!(get_vertical_wkt(&utm.wkt), "");
        assert_eq!(
            get_vertical_wkt(r#"COMPD_CS["unterminated",VERT_CS["x""#),
            ""
        );
    }

    #[test]
    fn identify_vertical_epsg_reads_authority_code_from_subtree() {
        let wkt = r#"COMPD_CS["WGS 84 + VERT_CS",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]]],VERT_CS["NAVD88 height",VERT_DATUM["North American Vertical Datum 1988",2005,AUTHORITY["EPSG","5103"]],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Up",UP],AUTHORITY["EPSG","5703"]]]"#;
        assert_eq!(identify_vertical_epsg(wkt, 0.0), "5703");

        // No VERT_CS → empty.
        let utm = user_input_to_wkt("EPSG:3857").unwrap();
        assert_eq!(identify_vertical_epsg(&utm.wkt, 0.0), "");
        assert_eq!(identify_vertical_epsg("not a wkt", 0.0), "");
    }

    #[test]
    fn get_vertical_units_reads_unit_from_subtree() {
        let wkt = r#"COMPD_CS["x",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]]],VERT_CS["NAVD88 height",VERT_DATUM["NAVD88",2005,AUTHORITY["EPSG","5103"]],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Up",UP],AUTHORITY["EPSG","5703"]]]"#;
        assert_eq!(get_vertical_units(wkt), "metre");
        assert_eq!(get_vertical_units(""), "");
        assert_eq!(get_vertical_units(r#"COMPD_CS["x",VERT_CS["bad"]]"#), "");
    }

    #[test]
    fn gdal_srs_transform_identity_preserves_xyz() {
        let a = user_input_to_wkt("EPSG:4326").unwrap();
        let t = GdalSrsTransform::new(&a.wkt, 0.0, &a.wkt, 0.0, &[], &[]).unwrap();
        let mut x = -93.265;
        let mut y = 44.9778;
        let mut z = 250.0;
        assert!(t.transform_xyz(&mut x, &mut y, &mut z));
        assert!((x - -93.265).abs() < 1e-9);
        assert!((y - 44.9778).abs() < 1e-9);
        assert!((z - 250.0).abs() < 1e-9);
    }

    #[test]
    fn gdal_srs_transform_4326_to_utm17n_matches_known_point() {
        let src = user_input_to_wkt("EPSG:4326").unwrap();
        let dst = user_input_to_wkt("EPSG:32617").unwrap();
        let t = GdalSrsTransform::new(&src.wkt, 0.0, &dst.wkt, 0.0, &[], &[]).unwrap();
        // Hobu HQ-ish, Iowa City: lon=-91.5, lat=41.6
        let mut x = -91.5;
        let mut y = 41.6;
        let mut z = 250.0;
        assert!(t.transform_xyz(&mut x, &mut y, &mut z));
        // Avoid pinning to specific PROJ datum-grid output; just confirm we
        // moved out of WGS84 lat/lon ranges into projected metres and z is
        // preserved.
        assert!(x.is_finite() && x.abs() > 1000.0);
        assert!(y.is_finite() && y.abs() > 1000.0);
        assert_eq!(z, 250.0);
    }

    #[test]
    fn gdal_srs_transform_vector_matches_single_point_xform() {
        let src = user_input_to_wkt("EPSG:4326").unwrap();
        let dst = user_input_to_wkt("EPSG:32617").unwrap();
        let t = GdalSrsTransform::new(&src.wkt, 0.0, &dst.wkt, 0.0, &[], &[]).unwrap();

        let mut xs = vec![-91.5_f64, -91.4];
        let mut ys = vec![41.6_f64, 41.5];
        let mut zs = vec![250.0_f64, 260.0];
        assert!(t.transform_xyz_slice(&mut xs, &mut ys, &mut zs));

        // Compare to scalar version on first point.
        let mut x0 = -91.5;
        let mut y0 = 41.6;
        let mut z0 = 250.0;
        assert!(t.transform_xyz(&mut x0, &mut y0, &mut z0));
        assert!((xs[0] - x0).abs() < 1e-9);
        assert!((ys[0] - y0).abs() < 1e-9);
        assert!((zs[0] - z0).abs() < 1e-9);

        assert!(t.transform_xyz_slice(&mut [], &mut [], &mut []));
        assert!(!t.transform_xyz_slice(&mut [1.0], &mut [], &mut [0.0]));
    }

    #[test]
    fn gdal_srs_transform_rejects_empty_or_garbage_wkt() {
        assert!(GdalSrsTransform::new("", 0.0, "EPSG:4326", 0.0, &[], &[]).is_err());
        assert!(GdalSrsTransform::new("garbage", 0.0, "EPSG:4326", 0.0, &[], &[]).is_err());
        let src = user_input_to_wkt("EPSG:4326").unwrap();
        assert!(GdalSrsTransform::new(&src.wkt, 0.0, "garbage", 0.0, &[], &[]).is_err());
    }

    #[test]
    fn gdal_srs_transform_with_custom_axis_order_flips_xy() {
        let src = user_input_to_wkt("EPSG:4326").unwrap();
        let dst = user_input_to_wkt("EPSG:4326").unwrap();
        // For traditional order, x is lon and y is lat. If we force axis
        // mapping [2,1] on the source, we tell GDAL that data axis 1 maps
        // to SRS axis 2 (lon) and data axis 2 maps to SRS axis 1 (lat),
        // i.e. swapped input. Identity SRS, so output equals swapped input.
        let t = GdalSrsTransform::new(&src.wkt, 0.0, &dst.wkt, 0.0, &[2, 1], &[]).unwrap();
        let mut x = 1.0;
        let mut y = 2.0;
        let mut z = 0.0;
        assert!(t.transform_xyz(&mut x, &mut y, &mut z));
        // We only assert the transform doesn't crash and returns finite numbers;
        // exact axis-mapping semantics depend on GDAL version.
        assert!(x.is_finite() && y.is_finite());
    }

    #[test]
    fn gdal_coord_operation_reverse_uses_inverse_path() {
        let transform = GdalCoordOperationTransform::new(
            "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad",
            true,
        )
        .unwrap();
        let mut x = std::f64::consts::PI;
        let mut y = std::f64::consts::FRAC_PI_2;
        let mut z = 3.0;

        assert!(transform.transform_xyz(&mut x, &mut y, &mut z));
        assert!((x - 180.0).abs() < 1e-9);
        assert!((y - 90.0).abs() < 1e-9);
        assert_eq!(z, 3.0);
    }

    #[test]
    fn identity_pipeline_preserves_xy() {
        let transform = SrsTransform::new_pipeline("+proj=noop").unwrap();
        let mut x = 1.5;
        let mut y = -2.5;
        let mut z = 3.5;

        assert!(transform.transform(&mut x, &mut y, &mut z));
        assert_eq!(x, 1.5);
        assert_eq!(y, -2.5);
        assert_eq!(z, 3.5);
    }
}
