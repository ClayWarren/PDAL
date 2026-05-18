//! Geometry support via GEOS.

use geos::{Geom, Geometry as GeosGeometry};

/// A geometry (PDAL's `Geometry`).
pub struct Geometry {
    geos_geom: GeosGeometry,
}

impl Geometry {
    pub fn from_wkt(wkt: &str) -> Result<Self, String> {
        let geos_geom =
            GeosGeometry::new_from_wkt(wkt).map_err(|e| format!("Failed to parse WKT: {}", e))?;
        Ok(Self { geos_geom })
    }

    pub fn distance(&self, x: f64, y: f64, z: f64) -> Result<f64, String> {
        let point = GeosGeometry::new_from_wkt(&format!("POINT({} {} {})", x, y, z))
            .map_err(|e| e.to_string())?;

        self.geos_geom.distance(&point).map_err(|e| e.to_string())
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        if let Ok(point) = GeosGeometry::new_from_wkt(&format!("POINT({} {})", x, y)) {
            self.geos_geom.contains(&point).unwrap_or(false)
        } else {
            false
        }
    }
}
