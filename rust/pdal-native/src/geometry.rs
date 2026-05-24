//! Geometry support via GEOS.

use geos::{CoordDimensions, CoordSeq, Geom, Geometry as GeosGeometry, WKTWriter};
use serde_json::Value;

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

    /// Parse a GeoJSON geometry. Accepts PDAL's optional top-level `srs` key by
    /// stripping it before handing the JSON to GEOS, which is strict about
    /// GeoJSON shape.
    pub fn from_geojson(json: &str) -> Result<Self, String> {
        let cleaned = strip_non_geojson_keys(json);
        let geos_geom = GeosGeometry::new_from_geojson(&cleaned)
            .map_err(|e| format!("Failed to parse GeoJSON: {}", e))?;
        Ok(Self { geos_geom })
    }

    /// Render this geometry as GeoJSON using GDAL's
    /// `OGR_G_ExportToJsonEx(COORDINATE_PRECISION=precision)` formatting:
    /// single line, spaces around `{`/`}`/`[`/`]`, `", "` separators, and
    /// coordinate values formatted with `precision` decimals after trimming
    /// trailing zeros. Only supports the geometry types currently used by
    /// `pdal::Polygon`.
    pub fn to_gdal_geojson(&self, precision: u32) -> Result<String, String> {
        let geojson = self.geos_geom.to_geojson().map_err(|e| e.to_string())?;
        let value: Value = serde_json::from_str(&geojson).map_err(|e| e.to_string())?;
        format_gdal_geojson_value(&value, precision)
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
        }
        .map_err(|e| e.to_string())?;
        Ok(Self { geos_geom })
    }

    pub fn to_wkt(&self) -> Result<String, String> {
        self.to_wkt_precision(16)
    }

    pub fn to_wkt_precision(&self, precision: u32) -> Result<String, String> {
        let mut writer = WKTWriter::new().map_err(|e| e.to_string())?;
        writer.set_rounding_precision(precision);
        writer.set_trim(true);
        writer.set_output_dimension(CoordDimensions::ThreeD);

        writer
            .write(&self.geos_geom)
            .map(|wkt| normalize_wkt(&wkt))
            .map_err(|e| e.to_string())
    }

    pub fn bounds(&self) -> Result<(f64, f64, f64, f64, f64, f64), String> {
        let geojson = self.geos_geom.to_geojson().map_err(|e| e.to_string())?;
        let value: Value = serde_json::from_str(&geojson).map_err(|e| e.to_string())?;
        let mut coords = Vec::new();
        collect_geojson_coords(&value, &mut coords);

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
            if cx < minx {
                minx = cx;
            }
            if cx > maxx {
                maxx = cx;
            }
            if cy < miny {
                miny = cy;
            }
            if cy > maxy {
                maxy = cy;
            }
            if !cz.is_nan() {
                if cz < minz {
                    minz = cz;
                }
                if cz > maxz {
                    maxz = cz;
                }
            }
        }
        if minz.is_nan() || minz == f64::MAX {
            minz = 0.0;
        }
        if maxz.is_nan() || maxz == f64::MIN {
            maxz = 0.0;
        }
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

fn normalize_wkt(wkt: &str) -> String {
    let wkt = wkt.replace(" Z ", " ");
    let mut output = String::with_capacity(wkt.len());
    let mut token = String::new();

    for ch in wkt.chars() {
        if ch.is_ascii_digit() || matches!(ch, '-' | '+' | '.' | 'e' | 'E') {
            token.push(ch);
        } else {
            push_normalized_wkt_token(&mut output, &mut token);
            if ch == ',' {
                output.push(',');
            } else {
                output.push(ch);
            }
        }
    }
    push_normalized_wkt_token(&mut output, &mut token);
    output.replace(", ", ",")
}

fn push_normalized_wkt_token(output: &mut String, token: &mut String) {
    if token.is_empty() {
        return;
    }
    if let Ok(value) = token.parse::<f64>() {
        output.push_str(&format_significant_decimal(value, 15));
    } else {
        output.push_str(token);
    }
    token.clear();
}

fn format_significant_decimal(value: f64, significant_digits: usize) -> String {
    if !value.is_finite() || value == 0.0 {
        return value.to_string();
    }

    let integer_digits = value.abs().log10().floor().max(0.0) as usize + 1;
    let fractional_digits = significant_digits.saturating_sub(integer_digits);
    let mut formatted = format!("{value:.fractional_digits$}");
    if formatted.contains('.') {
        while formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
    }
    if formatted == "-0" {
        "0".to_string()
    } else {
        formatted
    }
}

pub fn version() -> String {
    geos::version().unwrap_or_default()
}

/// Strip any non-RFC-7946 top-level keys from a GeoJSON object (currently
/// PDAL's `srs` extension) so the result can be handed to GEOS's strict
/// reader. Returns the input unchanged if it doesn't parse as a JSON object.
fn strip_non_geojson_keys(json: &str) -> String {
    let mut value: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return json.to_string(),
    };
    if let Some(obj) = value.as_object_mut() {
        obj.remove("srs");
    }
    serde_json::to_string(&value).unwrap_or_else(|_| json.to_string())
}

fn format_gdal_geojson_value(value: &Value, precision: u32) -> Result<String, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "to_gdal_geojson: expected GeoJSON object".to_string())?;
    let g_type = obj
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "to_gdal_geojson: missing geometry type".to_string())?;
    if !matches!(g_type, "Point" | "LineString" | "Polygon" | "MultiPolygon") {
        return Err(format!(
            "to_gdal_geojson: unsupported geometry type {g_type}"
        ));
    }
    let coords = obj
        .get("coordinates")
        .ok_or_else(|| "to_gdal_geojson: missing coordinates".to_string())?;

    let mut out = String::new();
    out.push_str("{ \"type\": \"");
    out.push_str(g_type);
    out.push_str("\", \"coordinates\": ");
    format_gdal_coords(coords, precision, &mut out)?;
    out.push_str(" }");
    Ok(out)
}

fn format_gdal_coords(value: &Value, precision: u32, out: &mut String) -> Result<(), String> {
    let values = value
        .as_array()
        .ok_or_else(|| "to_gdal_geojson: coordinates must be arrays".to_string())?;
    out.push_str("[ ");
    for (idx, item) in values.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        if let Some(n) = item.as_f64() {
            out.push_str(&format_coord_num(n, precision));
        } else {
            format_gdal_coords(item, precision, out)?;
        }
    }
    out.push_str(" ]");
    Ok(())
}

fn collect_geojson_coords(value: &Value, coords: &mut Vec<(f64, f64, f64)>) {
    if let Some(obj) = value.as_object() {
        if let Some(geometries) = obj.get("geometries") {
            collect_geojson_coords(geometries, coords);
        }
        if let Some(coordinates) = obj.get("coordinates") {
            collect_geojson_coords(coordinates, coords);
        }
        return;
    }

    let Some(values) = value.as_array() else {
        return;
    };
    if values.len() >= 2 && values.iter().all(Value::is_number) {
        let x = values.first().and_then(Value::as_f64).unwrap_or(0.0);
        let y = values.get(1).and_then(Value::as_f64).unwrap_or(0.0);
        let z = values.get(2).and_then(Value::as_f64).unwrap_or(f64::NAN);
        coords.push((x, y, z));
    } else {
        for item in values {
            collect_geojson_coords(item, coords);
        }
    }
}

/// Format a coordinate value the way GDAL's
/// `OGR_G_ExportToJsonEx(COORDINATE_PRECISION=precision)` does: fixed-point
/// with `precision` decimals, then trim trailing zeros and a trailing `.`.
fn format_coord_num(value: f64, precision: u32) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let p = precision as usize;
    let mut s = format!("{value:.p$}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s
    }
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

    #[test]
    fn from_geojson_accepts_pdal_srs_extension() {
        let json = r#"{ "srs": "EPSG:2991", "type": "Point", "coordinates": [1, 2] }"#;
        let geom = Geometry::from_geojson(json).unwrap();
        assert!(geom.is_valid().unwrap());
    }

    #[test]
    fn from_geojson_rejects_garbage() {
        assert!(Geometry::from_geojson("not json").is_err());
        assert!(Geometry::from_geojson(r#"{ "type": "Bogus" }"#).is_err());
    }

    #[test]
    fn strip_non_geojson_keys_only_removes_srs_from_objects() {
        assert_eq!(
            strip_non_geojson_keys(
                r#"{ "srs": "EPSG:2991", "type": "Point", "coordinates": [1, 2] }"#
            ),
            r#"{"coordinates":[1,2],"type":"Point"}"#
        );
        assert_eq!(strip_non_geojson_keys("[1, 2]"), "[1,2]");
        assert_eq!(strip_non_geojson_keys("not json"), "not json");
    }

    #[test]
    fn to_gdal_geojson_writes_core_geometry_shapes() {
        let point = Geometry::from_wkt("POINT Z (1.25 2.5 3.75)").unwrap();
        assert_eq!(
            point.to_gdal_geojson(2).unwrap(),
            r#"{ "type": "Point", "coordinates": [ 1.25, 2.5, 3.75 ] }"#
        );

        let line = Geometry::from_wkt("LINESTRING (0 0, 1.25 2.5)").unwrap();
        assert_eq!(
            line.to_gdal_geojson(1).unwrap(),
            r#"{ "type": "LineString", "coordinates": [ [ 0, 0 ], [ 1.2, 2.5 ] ] }"#
        );

        let polygon =
            Geometry::from_wkt("POLYGON ((0 0, 4 0, 4 4, 0 0), (1 1, 2 1, 1 2, 1 1))").unwrap();
        assert_eq!(
            polygon.to_gdal_geojson(0).unwrap(),
            r#"{ "type": "Polygon", "coordinates": [ [ [ 0, 0 ], [ 4, 0 ], [ 4, 4 ], [ 0, 0 ] ], [ [ 1, 1 ], [ 2, 1 ], [ 1, 2 ], [ 1, 1 ] ] ] }"#
        );

        let multipolygon =
            Geometry::from_wkt("MULTIPOLYGON (((0 0, 1 0, 0 1, 0 0)), ((2 2, 3 2, 2 3, 2 2)))")
                .unwrap();
        assert_eq!(
            multipolygon.to_gdal_geojson(0).unwrap(),
            r#"{ "type": "MultiPolygon", "coordinates": [ [ [ [ 0, 0 ], [ 1, 0 ], [ 0, 1 ], [ 0, 0 ] ] ], [ [ [ 2, 2 ], [ 3, 2 ], [ 2, 3 ], [ 2, 2 ] ] ] ] }"#
        );
    }

    #[test]
    fn to_gdal_geojson_rejects_unsupported_geometry_collections() {
        let collection = Geometry::from_wkt("GEOMETRYCOLLECTION (POINT (0 0))").unwrap();
        assert!(collection
            .to_gdal_geojson(3)
            .unwrap_err()
            .contains("unsupported geometry type"));
    }

    #[test]
    fn to_gdal_geojson_polygon_matches_gdal_format() {
        let wkt = "POLYGON ((636889.412951239268295 851528.512293258565478 422.7001953125,\
                   636899.14233423944097 851475.000686757150106 422.4697265625,\
                   636928.33048324030824 851494.459452757611871 422.5400390625,\
                   636976.977398241520859 851513.918218758190051 424.150390625,\
                   637069.406536744092591 851475.000686757150106 438.7099609375,\
                   637132.647526245797053 851445.812537756282836 425.9501953125,\
                   637336.964569251285866 851411.759697255445644 425.8203125,\
                   637473.175931254867464 851158.795739248627797 435.6298828125,\
                   637589.928527257987298 850711.244121236610226 420.509765625,\
                   637244.535430748714134 850511.791769731207751 420.7998046875,\
                   636758.066280735656619 850667.461897735483944 434.609375,\
                   636539.155163229792379 851056.63721774588339 422.6396484375,\
                   636889.412951239268295 851528.512293258565478 422.7001953125))";
        let geom = Geometry::from_wkt(wkt).unwrap();
        let out = geom.to_gdal_geojson(5).unwrap();
        // First and last vertex from the expected GDAL output, plus structural shape.
        assert!(out.starts_with("{ \"type\": \"Polygon\", \"coordinates\": [ [ [ 636889.41295, 851528.51229, 422.7002 ]"));
        assert!(out.ends_with("[ 636889.41295, 851528.51229, 422.7002 ] ] ] }"));
        // 425.9502 (trailing 0 trimmed) and 425.82031 should both appear.
        assert!(out.contains("425.9502"));
        assert!(out.contains("425.82031"));
    }

    #[test]
    fn format_coord_num_strips_trailing_zeros_and_dot() {
        assert_eq!(format_coord_num(422.7001953125, 5), "422.7002");
        assert_eq!(format_coord_num(425.9501953125, 5), "425.9502");
        assert_eq!(format_coord_num(425.8203125, 5), "425.82031");
        assert_eq!(format_coord_num(5.0, 5), "5");
        assert_eq!(format_coord_num(5.5, 0), "6");
        assert_eq!(format_coord_num(0.0, 5), "0");
        assert_eq!(format_coord_num(-0.000001, 5), "0");
        assert_eq!(format_coord_num(f64::NAN, 5), "NaN");
        assert_eq!(format_coord_num(f64::INFINITY, 5), "inf");
    }

    #[test]
    fn covers_reports_covers_and_boundaries() {
        let geometry = Geometry::from_wkt("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();
        // covers includes boundary
        assert!(geometry.covers(5.0, 5.0));
        assert!(geometry.covers(0.0, 0.0));
        assert!(!geometry.covers(15.0, 5.0));
    }

    #[test]
    fn area_computes_area() {
        let geometry = Geometry::from_wkt("POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();
        assert_eq!(geometry.area().unwrap(), 100.0);
        let point = Geometry::from_wkt("POINT(0 0)").unwrap();
        assert_eq!(point.area().unwrap(), 0.0);
    }

    #[test]
    fn simplify_reduces_coordinates() {
        let geometry = Geometry::from_wkt("LINESTRING(0 0, 5 0.01, 10 0)").unwrap();
        let simplified = geometry.simplify(0.1, true).unwrap();
        let wkt = simplified.to_wkt().unwrap();
        assert!(wkt.contains("LINESTRING (0 0,10 0)"));

        let simplified_no_top = geometry.simplify(0.1, false).unwrap();
        assert!(!simplified_no_top.to_wkt().unwrap().is_empty());
    }

    #[test]
    fn to_wkt_converts_back() {
        let geometry = Geometry::from_wkt("POINT (1 2)").unwrap();
        let wkt = geometry.to_wkt().unwrap();
        assert!(wkt.contains("POINT (1") && wkt.contains("2)"));
    }

    #[test]
    fn to_wkt_precision_rounds_coordinates() {
        let geometry = Geometry::from_wkt("POINT (1.23456 2.34567)").unwrap();
        assert_eq!(geometry.to_wkt_precision(2).unwrap(), "POINT (1.23 2.35)");
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn bounds_extracts_coordinates_3d() {
        // Point 2D
        let pt2d = Geometry::from_wkt("POINT(1 2)").unwrap();
        let (minx, maxx, miny, maxy, minz, maxz) = pt2d.bounds().unwrap();
        assert_eq!(minx, 1.0);
        assert_eq!(maxx, 1.0);
        assert_eq!(miny, 2.0);
        assert_eq!(maxy, 2.0);
        assert_eq!(minz, 0.0);
        assert_eq!(maxz, 0.0);

        // Point 3D
        let pt3d = Geometry::from_wkt("POINT(1 2 3)").unwrap();
        let (minx, maxx, miny, maxy, minz, maxz) = pt3d.bounds().unwrap();
        assert_eq!(minx, 1.0);
        assert_eq!(maxx, 1.0);
        assert_eq!(miny, 2.0);
        assert_eq!(maxy, 2.0);
        assert_eq!(minz, 3.0);
        assert_eq!(maxz, 3.0);

        // LineString 3D
        let line = Geometry::from_wkt("LINESTRING(0 0 1, 10 20 30)").unwrap();
        let (minx, maxx, miny, maxy, minz, maxz) = line.bounds().unwrap();
        assert_eq!(minx, 0.0);
        assert_eq!(maxx, 10.0);
        assert_eq!(miny, 0.0);
        assert_eq!(maxy, 20.0);
        assert_eq!(minz, 1.0);
        assert_eq!(maxz, 30.0);

        // Polygon with interior rings
        let poly = Geometry::from_wkt(
            "POLYGON((0 0 0, 10 0 0, 10 10 0, 0 10 0, 0 0 0), (2 2 1, 8 2 1, 8 8 1, 2 8 1, 2 2 1))",
        )
        .unwrap();
        let (minx, maxx, miny, maxy, minz, maxz) = poly.bounds().unwrap();
        assert_eq!(minx, 0.0);
        assert_eq!(maxx, 10.0);
        assert_eq!(miny, 0.0);
        assert_eq!(maxy, 10.0);
        assert_eq!(minz, 0.0);
        assert_eq!(maxz, 1.0);

        // MultiPoint 3D
        let multipoint = Geometry::from_wkt("MULTIPOINT(0 0 5, 10 20 30)").unwrap();
        let (minx, maxx, miny, maxy, minz, maxz) = multipoint.bounds().unwrap();
        assert_eq!(minx, 0.0);
        assert_eq!(maxx, 10.0);
        assert_eq!(miny, 0.0);
        assert_eq!(maxy, 20.0);
        assert_eq!(minz, 5.0);
        assert_eq!(maxz, 30.0);

        // Empty geometry
        let empty = Geometry::from_wkt("GEOMETRYCOLLECTION EMPTY").unwrap();
        let (minx, maxx, miny, maxy, minz, maxz) = empty.bounds().unwrap();
        assert_eq!(minx, 0.0);
        assert_eq!(maxx, 0.0);
        assert_eq!(miny, 0.0);
        assert_eq!(maxy, 0.0);
        assert_eq!(minz, 0.0);
        assert_eq!(maxz, 0.0);

        let collection =
            Geometry::from_wkt("GEOMETRYCOLLECTION (POINT (5 6), LINESTRING (1 2, 3 4))").unwrap();
        assert_eq!(collection.bounds().unwrap(), (1.0, 5.0, 2.0, 6.0, 0.0, 0.0));
    }

    #[test]
    fn significant_decimal_formatter_covers_special_values() {
        assert_eq!(format_significant_decimal(0.0, 15), "0");
        assert_eq!(format_significant_decimal(f64::NAN, 15), "NaN");
        assert_eq!(format_significant_decimal(f64::INFINITY, 15), "inf");
        assert_eq!(format_significant_decimal(-0.00000001, 2), "0");
    }
}
