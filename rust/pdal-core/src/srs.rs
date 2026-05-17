//! Spatial Reference Systems and transformations.

use proj::Proj;

/// A spatial reference system (PDAL's `SpatialReference`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialReference {
    wkt: String,
}

impl SpatialReference {
    pub fn new(wkt: &str) -> Self {
        Self {
            wkt: wkt.to_string(),
        }
    }

    pub fn wkt(&self) -> &str {
        &self.wkt
    }

    pub fn is_empty(&self) -> bool {
        self.wkt.is_empty()
    }
}

/// A coordinate transformation between two SRSs (PDAL's `SrsTransform`).
pub struct SrsTransform {
    proj: Proj,
}

impl SrsTransform {
    pub fn new(src: &SpatialReference, dst: &SpatialReference) -> Result<Self, String> {
        let proj = Proj::new_known_crs(src.wkt(), dst.wkt(), None)
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
