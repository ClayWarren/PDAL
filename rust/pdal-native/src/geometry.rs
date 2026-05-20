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

    pub fn is_valid(&self) -> Result<bool, String> {
        self.geos_geom.is_valid().map_err(|e| e.to_string())
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

    /// Return the geometry's boundary (PDAL's `Geometry::getRing`). For a
    /// `Polygon`, the boundary is the closed line of its rings, so distances
    /// measure against the edge rather than the polygon's interior.
    pub fn boundary(&self) -> Result<Self, String> {
        let boundary = self
            .geos_geom
            .boundary()
            .map_err(|err| format!("boundary failed: {err}"))?;
        Ok(Self {
            geos_geom: boundary,
        })
    }
}

pub fn version() -> String {
    geos::version().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_wkt_is_rejected() {
        assert!(Geometry::from_wkt("not wkt").is_err());
    }

    #[test]
    fn validity_reports_geos_result() {
        let valid = Geometry::from_wkt("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();
        let invalid = Geometry::from_wkt("POLYGON((0 0, 10 10, 10 0, 0 10, 0 0))").unwrap();

        assert!(valid.is_valid().unwrap());
        assert!(!invalid.is_valid().unwrap());
    }

    #[test]
    fn polygon_contains_interior_point_but_not_exterior_point() {
        let geometry = Geometry::from_wkt("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();

        assert!(geometry.contains(5.0, 5.0));
        assert!(!geometry.contains(15.0, 5.0));
    }

    #[test]
    fn distance_to_point_uses_geos_distance() {
        let geometry = Geometry::from_wkt("POINT(0 0 0)").unwrap();

        assert_eq!(geometry.distance(3.0, 4.0, 0.0).unwrap(), 5.0);
    }

    #[test]
    fn version_reports_geos() {
        assert!(!version().is_empty());
    }

    #[test]
    fn polygon_boundary_makes_interior_points_have_a_distance() {
        let polygon = Geometry::from_wkt("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();
        // A point at the center has zero distance to the polygon but
        // ~5 units to its boundary line.
        assert_eq!(polygon.distance(5.0, 5.0, 0.0).unwrap(), 0.0);
        let ring = polygon.boundary().unwrap();
        assert_eq!(ring.distance(5.0, 5.0, 0.0).unwrap(), 5.0);
    }
}
