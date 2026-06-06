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
        for idx in 0..xs.len() {
            if !self.transform_xyz(&mut xs[idx], &mut ys[idx], &mut zs[idx]) {
                return false;
            }
        }
        true
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

/// A local-cartesian (topocentric ENU) transform around an anchor lat/lon/h on
/// the WGS84 ellipsoid, ported from the C++ `filters/private/georeference`
/// `LocalCartesian`. Forward maps geocentric (ECEF) XYZ to local ENU; reverse
/// maps local ENU back to ECEF. Implemented directly over PROJ's
/// `+proj=topocentric` pipeline (3D), since the `proj` crate's high-level
/// `convert` is 2D only.
pub struct TopocentricTransform {
    ctx: *mut proj_sys::pj_ctx,
    pj: *mut proj_sys::PJconsts,
}

impl TopocentricTransform {
    pub fn new(lat0: f64, lon0: f64, h0: f64) -> Result<Self, String> {
        // Match the C++ definition string byte-for-byte (fixed, 12 digits).
        let def = format!(
            "+proj=topocentric +ellps=WGS84 +lon_0={lon0:.12} +lat_0={lat0:.12} +h_0={h0:.12}"
        );
        let c_def = CString::new(def).map_err(|_| "topocentric def has NUL".to_string())?;
        unsafe {
            let ctx = proj_sys::proj_context_create();
            if ctx.is_null() {
                return Err("proj_context_create failed".into());
            }
            let pj = proj_sys::proj_create(ctx, c_def.as_ptr());
            if pj.is_null() {
                proj_sys::proj_context_destroy(ctx);
                return Err("proj_create failed for topocentric pipeline".into());
            }
            Ok(Self { ctx, pj })
        }
    }

    fn trans(&self, dir: proj_sys::PJ_DIRECTION, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        unsafe {
            let mut coord = proj_sys::PJ_COORD {
                xyzt: proj_sys::PJ_XYZT {
                    x,
                    y,
                    z,
                    t: f64::INFINITY,
                },
            };
            coord = proj_sys::proj_trans(self.pj, dir, coord);
            let xyzt = coord.xyzt;
            (xyzt.x, xyzt.y, xyzt.z)
        }
    }

    /// ECEF -> local ENU.
    pub fn forward(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        self.trans(proj_sys::PJ_DIRECTION_PJ_FWD, x, y, z)
    }

    /// Local ENU -> ECEF.
    pub fn reverse(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        self.trans(proj_sys::PJ_DIRECTION_PJ_INV, x, y, z)
    }
}

impl Drop for TopocentricTransform {
    fn drop(&mut self) {
        unsafe {
            if !self.pj.is_null() {
                proj_sys::proj_destroy(self.pj);
            }
            if !self.ctx.is_null() {
                proj_sys::proj_context_destroy(self.ctx);
            }
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
        let _ = gdal_sys::OSRAutoIdentifyEPSG(srs);
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

/// Translate WKT into GDAL WKT1 using PDAL's LAS writer-compatible options.
pub fn wkt_to_wkt1(wkt: &str, epoch: f64) -> Result<String, String> {
    export_imported_wkt(
        wkt,
        epoch,
        &[
            ("FORMAT", "WKT1_GDAL"),
            ("OUTPUT_AXIS", "NO"),
            ("ALLOW_ELLIPSOIDAL_HEIGHT_AS_VERTICAL_CRS", "YES"),
        ],
    )
}

/// Translate WKT into WKT2_2018.
pub fn wkt_to_wkt2(wkt: &str, epoch: f64) -> Result<String, String> {
    export_imported_wkt(wkt, epoch, &[("FORMAT", "WKT2_2018")])
}

/// Format WKT across multiple lines, matching `OGRSpatialReference`'s
/// `MULTILINE=YES` export option.
pub fn pretty_wkt(wkt: &str) -> Result<String, String> {
    export_imported_wkt(wkt, 0.0, &[("MULTILINE", "YES")])
}

/// Classify a WKT string with GDAL's `OSRIsGeographic`.
pub fn is_geographic(wkt: &str, epoch: f64) -> bool {
    with_imported_srs(wkt, epoch, |srs| unsafe {
        gdal_sys::OSRIsGeographic(srs) == 1
    })
    .unwrap_or(false)
}

/// Classify a WKT string with GDAL's `OSRIsGeocentric`.
pub fn is_geocentric(wkt: &str, epoch: f64) -> bool {
    with_imported_srs(wkt, epoch, |srs| unsafe {
        gdal_sys::OSRIsGeocentric(srs) == 1
    })
    .unwrap_or(false)
}

/// Classify a WKT string with GDAL's `OSRIsProjected`.
pub fn is_projected(wkt: &str, epoch: f64) -> bool {
    with_imported_srs(wkt, epoch, |srs| unsafe {
        gdal_sys::OSRIsProjected(srs) == 1
    })
    .unwrap_or(false)
}

/// Return GDAL's data-axis to SRS-axis mapping. Empty input or unimportable WKT
/// returns an empty vector, matching the C++ wrapper behavior.
pub fn axis_ordering(wkt: &str, epoch: f64) -> Vec<i32> {
    with_imported_srs(wkt, epoch, |srs| unsafe {
        let mut count = 0;
        let mapping = gdal_sys::OSRGetDataAxisToSRSAxisMapping(srs, &mut count);
        if mapping.is_null() || count <= 0 {
            return Vec::new();
        }
        std::slice::from_raw_parts(mapping, count as usize).to_vec()
    })
    .unwrap_or_default()
}

fn export_imported_wkt(wkt: &str, epoch: f64, options: &[(&str, &str)]) -> Result<String, String> {
    with_imported_srs(wkt, epoch, |srs| unsafe {
        let _ = gdal_sys::OSRAutoIdentifyEPSG(srs);
        canonicalize_identified_epsg_srs(srs);
        export_to_wkt(srs, options)
    })?
}

unsafe fn canonicalize_identified_epsg_srs(srs: gdal_sys::OGRSpatialReferenceH) {
    let authority = gdal_sys::OSRGetAuthorityName(srs, std::ptr::null());
    if authority.is_null() {
        return;
    }
    let authority = CStr::from_ptr(authority).to_string_lossy();
    if authority != "EPSG" {
        return;
    }

    let code = gdal_sys::OSRGetAuthorityCode(srs, std::ptr::null());
    if code.is_null() {
        return;
    }
    let code = CStr::from_ptr(code).to_string_lossy();
    let Ok(code) = code.parse::<std::os::raw::c_int>() else {
        return;
    };
    let _ = gdal_sys::OSRImportFromEPSG(srs, code);
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

fn with_imported_srs<T, F>(wkt: &str, epoch: f64, f: F) -> Result<T, String>
where
    F: FnOnce(gdal_sys::OGRSpatialReferenceH) -> T,
{
    if wkt.is_empty() {
        return Err("empty WKT".into());
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
            return Err("OSRImportFromWkt failed".into());
        }
        if epoch != 0.0 {
            gdal_sys::OSRSetCoordinateEpoch(srs, epoch);
        }
        let out = f(srs);
        gdal_sys::OSRDestroySpatialReference(srs);
        Ok(out)
    }
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
mod tests;
