//! Geometry support via GEOS.

use geos::{Geom, Geometry as GeosGeometry, CoordSeq};

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
        let coords = CoordSeq::new_from_vec(&[&[x, y, z]]).map_err(|e| e.to_string())?;
        let point = GeosGeometry::create_point(coords).map_err(|e| e.to_string())?;

        self.geos_geom.distance(&point).map_err(|e| e.to_string())
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        if let Ok(coords) = CoordSeq::new_from_vec(&[&[x, y]]) {
            if let Ok(point) = GeosGeometry::create_point(coords) {
                return self.geos_geom.contains(&point).unwrap_or(false);
            }
        }
        false
    }

    pub fn covers(&self, x: f64, y: f64) -> bool {
        if let Ok(coords) = CoordSeq::new_from_vec(&[&[x, y]]) {
            if let Ok(point) = GeosGeometry::create_point(coords) {
                return self.geos_geom.covers(&point).unwrap_or(false);
            }
        }
        false
    }

    pub fn area(&self) -> Result<f64, String> {
        self.geos_geom.area().map_err(|e| e.to_string())
    }

    pub fn simplify(&self, tolerance: f64, preserve_topology: bool) -> Result<Self, String> {
        let geos_geom = if preserve_topology {
            self.geos_geom.topology_preserve_simplify(tolerance)
        } else {
            self.geos_geom.simplify(tolerance)
        }.map_err(|e| e.to_string())?;
        Ok(Self { geos_geom })
    }

    pub fn to_wkt(&self) -> Result<String, String> {
        self.geos_geom.to_wkt().map_err(|e| e.to_string())
    }

    pub fn bounds(&self) -> Result<(f64, f64, f64, f64, f64, f64), String> {
        fn get_coords(geom: &impl geos::Geom, coords: &mut Vec<(f64, f64, f64)>) -> Result<(), String> {
            use geos::GeometryTypes;
            let g_type = geom.geometry_type().map_err(|e| e.to_string())?;
            match g_type {
                GeometryTypes::Point |
                GeometryTypes::LineString |
                GeometryTypes::LinearRing => {
                    if let Ok(coord_seq) = geom.get_coord_seq() {
                        if let Ok(size) = coord_seq.size() {
                            let has_z = if let Ok(dims) = coord_seq.dimensions() {
                                let d: std::os::raw::c_int = dims.into();
                                d >= 3
                            } else {
                                false
                            };
                            for i in 0..size {
                                let x = coord_seq.get_x(i).unwrap_or(0.0);
                                let y = coord_seq.get_y(i).unwrap_or(0.0);
                                let cz = if has_z {
                                    coord_seq.get_z(i).unwrap_or(f64::NAN)
                                } else {
                                    f64::NAN
                                };
                                coords.push((x, y, cz));
                            }
                        }
                    }
                }
                GeometryTypes::Polygon => {
                    if let Ok(ext) = geom.get_exterior_ring() {
                        get_coords(&ext, coords)?;
                    }
                    if let Ok(num_int) = geom.get_num_interior_rings() {
                        for i in 0..num_int {
                            if let Ok(interior) = geom.get_interior_ring_n(i) {
                                get_coords(&interior, coords)?;
                            }
                        }
                    }
                }
                _ => {
                    if let Ok(num_geoms) = geom.get_num_geometries() {
                        for i in 0..num_geoms {
                            if let Ok(sub_geom) = geom.get_geometry_n(i) {
                                get_coords(&sub_geom, coords)?;
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        let mut coords = Vec::new();
        get_coords(&self.geos_geom, &mut coords)?;

        if coords.is_empty() {
            return Ok((0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
        }
        let mut minx = f64::MAX;
        let mut maxx = f64::MIN;
        let mut miny = f64::MAX;
        let mut maxy = f64::MIN;
        let mut minz = f64::MAX;
        let mut maxz = f64::MIN;
        for coord in coords {
            let cx = coord.0;
            let cy = coord.1;
            let cz = coord.2;
            if cx < minx { minx = cx; }
            if cx > maxx { maxx = cx; }
            if cy < miny { miny = cy; }
            if cy > maxy { maxy = cy; }
            if !cz.is_nan() {
                if cz < minz { minz = cz; }
                if cz > maxz { maxz = cz; }
            }
        }
        if minz.is_nan() || minz == f64::MAX { minz = 0.0; }
        if maxz.is_nan() || maxz == f64::MIN { maxz = 0.0; }
        Ok((minx, maxx, miny, maxy, minz, maxz))
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
