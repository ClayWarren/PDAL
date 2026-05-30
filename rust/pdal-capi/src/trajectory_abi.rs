//! C ABI for the georeference trajectory loader/interpolator.
//!
//! Ports `filters/private/georeference/Trajectory`: it loads a trajectory via
//! a (Rust-registry) reader and interpolates position/orientation at a query
//! GPS time. The C++ `Trajectory` is now a thin delegator over this handle.

use crate::error::set_last_error;
use crate::registry::{create_reader, options_from_object};
use pdal_core::driver::infer_reader_driver;
use pdal_core::options::Options;
use pdal_core::point::DimId;
use std::ffi::CStr;
use std::os::raw::c_char;

/// One trajectory sample (angles in radians as stored by the reader).
struct TrajRow {
    gpstime: f64,
    roll: f64,
    pitch: f64,
    azimuth: f64,
    wander: f64,
    x: f64,
    y: f64,
    z: f64,
}

/// Opaque handle wrapping the loaded, time-ordered trajectory samples.
#[allow(non_camel_case_types)]
pub struct pdal_trajectory_t {
    rows: Vec<TrajRow>,
}

unsafe fn c_str(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

fn load_trajectory(filename: &str, options_json: &str) -> Result<Vec<TrajRow>, String> {
    // The options JSON mirrors the C++ `trajectory_options` object: an optional
    // "type" driver plus reader options. Build Rust Options the same way the
    // pipeline registry does.
    let parsed: serde_json::Value = if options_json.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(options_json)
            .map_err(|e| format!("invalid trajectory_options JSON: {e}"))?
    };
    let object = parsed
        .as_object()
        .cloned()
        .unwrap_or_else(serde_json::Map::new);

    let driver = object
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| infer_reader_driver(filename).map(|s| s.to_string()))
        .ok_or_else(|| format!("Cannot determine reader for input file: {filename}"))?;

    let mut options: Options = options_from_object(&object).map_err(|e| e.to_string())?;
    options.add("filename", filename);

    let mut reader = create_reader(&driver, &options).map_err(|e| e.to_string())?;
    let views = reader.read().map_err(|e| e.to_string())?;
    let view = views
        .into_iter()
        .next()
        .ok_or_else(|| "trajectory reader produced no view".to_string())?;

    let gpstime = DimId::from_name("GpsTime");
    let roll = DimId::from_name("Roll");
    let pitch = DimId::from_name("Pitch");
    let azimuth = DimId::from_name("Azimuth");
    let wander = DimId::from_name("WanderAngle");

    let mut rows = Vec::with_capacity(view.len() as usize);
    for idx in 0..view.len() {
        rows.push(TrajRow {
            gpstime: view.get_f64(idx, &gpstime),
            roll: view.get_f64(idx, &roll),
            pitch: view.get_f64(idx, &pitch),
            azimuth: view.get_f64(idx, &azimuth),
            wander: view.get_f64(idx, &wander),
            x: view.get_f64(idx, &DimId::X),
            y: view.get_f64(idx, &DimId::Y),
            z: view.get_f64(idx, &DimId::Z),
        });
    }
    Ok(rows)
}

/// Linear blend matching C++ `Utils::getValue(p1, p2, frac) = p1*frac +
/// p2*(1-frac)` (p1 is the earlier sample, p2 the later one).
fn lerp(p1: f64, p2: f64, frac: f64) -> f64 {
    p1 * frac + p2 * (1.0 - frac)
}

/// Angle blend matching C++ `Utils::getAngle(a1, a2, frac)`.
fn angle(a1: f64, a2: f64, frac: f64) -> f64 {
    (frac * a2.sin() + (1.0 - frac) * a1.sin()).atan2(frac * a2.cos() + (1.0 - frac) * a1.cos())
}

impl pdal_trajectory_t {
    /// Replicates `Trajectory::getTrajPoint`: find the first sample with
    /// GpsTime >= `time` (std::lower_bound); when it is strictly interior,
    /// interpolate between it and its predecessor. Returns None otherwise.
    fn interpolate(&self, time: f64) -> Option<[f64; 8]> {
        let rows = &self.rows;
        // lower_bound on gpstime.
        let upper = rows.partition_point(|r| r.gpstime < time);
        if upper == 0 || upper >= rows.len() {
            return None;
        }
        let p1 = &rows[upper - 1];
        let p2 = &rows[upper];
        let frac = (time - p1.gpstime) / (p2.gpstime - p1.gpstime);
        Some([
            angle(p1.roll, p2.roll, frac),
            angle(p1.pitch, p2.pitch, frac),
            angle(p1.azimuth, p2.azimuth, frac),
            angle(p1.wander, p2.wander, frac),
            angle(p1.x, p2.x, frac),
            angle(p1.y, p2.y, frac),
            lerp(p1.z, p2.z, frac),
            lerp(p1.gpstime, p2.gpstime, frac),
        ])
    }
}

/// Load a trajectory. `options_json` is the `trajectory_options` object (or
/// empty). Returns `null` on error and sets the last error message.
///
/// # Safety
///
/// `filename`/`options_json` must be NUL-terminated C strings (or null). The
/// returned pointer is owned by the caller; release with
/// `pdal_trajectory_destroy`.
#[no_mangle]
pub unsafe extern "C" fn pdal_trajectory_create(
    filename: *const c_char,
    options_json: *const c_char,
) -> *mut pdal_trajectory_t {
    let filename = c_str(filename);
    let options_json = c_str(options_json);
    match load_trajectory(&filename, &options_json) {
        Ok(rows) => Box::into_raw(Box::new(pdal_trajectory_t { rows })),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

/// Free a handle from `pdal_trajectory_create`. Safe to pass `null`.
///
/// # Safety
///
/// `handle` must have come from `pdal_trajectory_create` and not been freed.
#[no_mangle]
pub unsafe extern "C" fn pdal_trajectory_destroy(handle: *mut pdal_trajectory_t) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Interpolate the trajectory at `time`. On success writes the eight outputs
/// (roll, pitch, azimuth, wander_angle, x, y, z, time) and returns true.
/// Returns false (writing nothing) when `time` falls outside the trajectory.
///
/// # Safety
///
/// `handle` must come from `pdal_trajectory_create`. All out pointers must be
/// non-null and valid for write.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pdal_trajectory_get_point(
    handle: *const pdal_trajectory_t,
    time: f64,
    roll: *mut f64,
    pitch: *mut f64,
    azimuth: *mut f64,
    wander_angle: *mut f64,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
    out_time: *mut f64,
) -> bool {
    let Some(handle) = handle.as_ref() else {
        return false;
    };
    let Some(values) = handle.interpolate(time) else {
        return false;
    };
    let outs = [roll, pitch, azimuth, wander_angle, x, y, z, out_time];
    for (ptr, value) in outs.into_iter().zip(values) {
        if let Some(slot) = ptr.as_mut() {
            *slot = value;
        }
    }
    true
}
