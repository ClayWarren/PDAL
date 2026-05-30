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
use pdal_filters::transformation::invert_affine;
use pdal_native::srs::TopocentricTransform;
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

// ---------------------------------------------------------------------------
// Per-point georeferencing (ports GeoreferenceFilter::processOne).
// ---------------------------------------------------------------------------

/// Build the affine transform from a position+orientation, matching the C++
/// `georeference::Utils::getTransformation` (row-major 4x4).
fn get_transformation(x: f64, y: f64, z: f64, roll: f64, pitch: f64, yaw: f64) -> [f64; 16] {
    let a = yaw.cos();
    let b = yaw.sin();
    let c = pitch.cos();
    let d = pitch.sin();
    let e = roll.cos();
    let f = roll.sin();
    let de = d * e;
    let df = d * f;
    [
        a * c,
        a * df - b * e,
        b * f + a * de,
        x, //
        b * c,
        a * e + b * df,
        b * de - a * f,
        y, //
        -d,
        c * f,
        c * e,
        z, //
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

/// Row-major 4x4 matrix product `a * b`.
fn mat4_mul(a: &[f64; 16], b: &[f64; 16]) -> [f64; 16] {
    let mut out = [0.0; 16];
    for i in 0..4 {
        for j in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[i * 4 + k] * b[k * 4 + j];
            }
            out[i * 4 + j] = sum;
        }
    }
    out
}

/// Affine transform of a 3-vector by a row-major 4x4 (implicit w=1).
fn mat4_vec3(m: &[f64; 16], v: [f64; 3]) -> [f64; 3] {
    [
        m[0] * v[0] + m[1] * v[1] + m[2] * v[2] + m[3],
        m[4] * v[0] + m[5] * v[1] + m[6] * v[2] + m[7],
        m[8] * v[0] + m[9] * v[1] + m[10] * v[2] + m[11],
    ]
}

fn normalize(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

/// Point dims operated on in place: [x, y, z, beamOrigin xyz, beamDirection xyz].
type Pt = [f64; 9];

#[allow(clippy::too_many_arguments)]
fn process_point(
    traj: &pdal_trajectory_t,
    scan2imu: &[f64; 16],
    reverse: bool,
    ned: bool,
    transform_beam: bool,
    time_offset: f64,
    gpstime: f64,
    p: &mut Pt,
) -> bool {
    let Some(b) = traj.interpolate(gpstime + time_offset) else {
        return false;
    };
    // b = [roll, pitch, azimuth, wander, tx, ty, tz, time]
    let (roll, pitch, azimuth, wander, tx, ty, tz) = (b[0], b[1], b[2], b[3], b[4], b[5], b[6]);
    let transform = mat4_mul(
        &get_transformation(0.0, 0.0, 0.0, roll, pitch, azimuth - wander),
        scan2imu,
    );
    let Ok(lc) = TopocentricTransform::new(ty.to_degrees(), tx.to_degrees(), tz) else {
        return false;
    };

    if reverse {
        let Ok(inv) = invert_affine(&transform) else {
            return false;
        };
        let (fx, fy, fz) = lc.forward(p[0], p[1], p[2]);
        let v = if ned { [fy, fx, -fz] } else { [fx, fy, fz] };
        let scan = mat4_vec3(&inv, v);
        p[0] = scan[0];
        p[1] = scan[1];
        p[2] = scan[2];

        if transform_beam {
            let (ox0, oy0, oz0) = (p[3], p[4], p[5]);
            let (dx0, dy0, dz0) = (ox0 + p[6], oy0 + p[7], oz0 + p[8]);
            let (ox, oy, oz) = lc.forward(ox0, oy0, oz0);
            let (dx, dy, dz) = lc.forward(dx0, dy0, dz0);
            let scan_orig = mat4_vec3(&inv, if ned { [oy, ox, -oz] } else { [ox, oy, oz] });
            let scan_dir = mat4_vec3(&inv, if ned { [dy, dx, -dz] } else { [dx, dy, dz] });
            p[3] = scan_orig[0];
            p[4] = scan_orig[1];
            p[5] = scan_orig[2];
            let dir = normalize([
                scan_dir[0] - scan_orig[0],
                scan_dir[1] - scan_orig[1],
                scan_dir[2] - scan_orig[2],
            ]);
            p[6] = dir[0];
            p[7] = dir[1];
            p[8] = dir[2];
        }
    } else {
        let ned_v = mat4_vec3(&transform, [p[0], p[1], p[2]]);
        if ned {
            p[0] = ned_v[1];
            p[1] = ned_v[0];
            p[2] = -ned_v[2];
        } else {
            p[0] = ned_v[0];
            p[1] = ned_v[1];
            p[2] = ned_v[2];
        }
        let (rx, ry, rz) = lc.reverse(p[0], p[1], p[2]);
        p[0] = rx;
        p[1] = ry;
        p[2] = rz;

        if transform_beam {
            let ned_orig = mat4_vec3(&transform, [p[3], p[4], p[5]]);
            let ned_dir = mat4_vec3(&transform, [p[3] + p[6], p[4] + p[7], p[5] + p[8]]);
            let (mut ox, mut oy, mut oz) = (ned_orig[0], ned_orig[1], ned_orig[2]);
            let (mut dx, mut dy, mut dz) = (ned_dir[0], ned_dir[1], ned_dir[2]);
            if ned {
                dx = ned_dir[1];
                dy = ned_dir[0];
                dz = -ned_dir[2];
                ox = ned_orig[1];
                oy = ned_orig[0];
                oz = -ned_orig[2];
            }
            let (rox, roy, roz) = lc.reverse(ox, oy, oz);
            let (rdx, rdy, rdz) = lc.reverse(dx, dy, dz);
            p[3] = rox;
            p[4] = roy;
            p[5] = roz;
            let dir = normalize([rdx - rox, rdy - roy, rdz - roz]);
            p[6] = dir[0];
            p[7] = dir[1];
            p[8] = dir[2];
        }
    }
    true
}

/// Georeference one point in place, matching `GeoreferenceFilter::processOne`.
/// `scan2imu` is the row-major 4x4 scanner->IMU matrix. Returns false (leaving
/// all outputs untouched) when the trajectory has no sample bracketing the
/// point's adjusted GPS time. Beam pointers are only read/written when
/// `transform_beam` is true.
///
/// # Safety
///
/// `traj` must come from `pdal_trajectory_create`. `scan2imu` must point to 16
/// f64s. The mutable pointers must be non-null and valid for read+write.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pdal_georeference_process_point(
    traj: *const pdal_trajectory_t,
    scan2imu: *const f64,
    reverse: bool,
    ned: bool,
    transform_beam: bool,
    time_offset: f64,
    gpstime: f64,
    x: *mut f64,
    y: *mut f64,
    z: *mut f64,
    beam_origin_x: *mut f64,
    beam_origin_y: *mut f64,
    beam_origin_z: *mut f64,
    beam_direction_x: *mut f64,
    beam_direction_y: *mut f64,
    beam_direction_z: *mut f64,
) -> bool {
    let Some(traj) = traj.as_ref() else {
        return false;
    };
    if scan2imu.is_null() {
        return false;
    }
    let mut matrix = [0.0f64; 16];
    matrix.copy_from_slice(std::slice::from_raw_parts(scan2imu, 16));

    let outs = [
        x,
        y,
        z,
        beam_origin_x,
        beam_origin_y,
        beam_origin_z,
        beam_direction_x,
        beam_direction_y,
        beam_direction_z,
    ];
    let mut p: Pt = [0.0; 9];
    for (slot, ptr) in p.iter_mut().zip(outs.iter()) {
        if let Some(v) = ptr.as_ref() {
            *slot = *v;
        }
    }

    if !process_point(
        traj,
        &matrix,
        reverse,
        ned,
        transform_beam,
        time_offset,
        gpstime,
        &mut p,
    ) {
        return false;
    }

    for (value, ptr) in p.iter().zip(outs.iter()) {
        if let Some(slot) = ptr.as_mut() {
            *slot = *value;
        }
    }
    true
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
