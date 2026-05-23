//! Coordinate transformations through PROJ.

use proj::Proj;
use std::ffi::CStr;

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
