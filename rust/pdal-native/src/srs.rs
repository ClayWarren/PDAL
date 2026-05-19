//! Coordinate transformations through PROJ.

use proj::Proj;

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
    match Proj::new("EPSG:4326") {
        Ok(proj) => proj.lib_info().map(|info| info.version).unwrap_or_default(),
        Err(_) => String::new(),
    }
}
