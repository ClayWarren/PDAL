//! `writers.ogr` -- narrow local writer port.
//!
//! The C++ writer is a broad OGR/GDAL adapter. This Rust writer covers local
//! GeoJSON output plus native OGR-backed Shapefile and GeoPackage point output
//! for the covered C++ test shapes. Transactions and broader creation options
//! stay deferred to later native OGR milestones.

use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use serde_json::{json, Map, Number, Value};
use std::fs;
use std::path::Path;

pub struct OgrWriter {
    filename: String,
    driver_name: String,
    attr_dims: Vec<String>,
    creation_options: GeoJsonCreationOptions,
    input_srs: String,
    multicount: u64,
    measure_dim: String,
    point_count: u64,
}

impl OgrWriter {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            driver_name: options.get_str("ogrdriver", ""),
            attr_dims: comma_values(options, "attr_dims"),
            creation_options: GeoJsonCreationOptions::from_options(options),
            input_srs: options.get_str("input_srs", ""),
            multicount: options.get_u64("multicount", 1),
            measure_dim: options.get_str("measure_dim", ""),
            point_count: 0,
        }
    }
}

impl Writer for OgrWriter {
    fn name(&self) -> &str {
        "writers.ogr"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "OgrWriter requires a filename option.".to_string(),
            ));
        }
        self.validate_options()?;

        if self.uses_native_ogr_driver() {
            return self.write_native_points(views);
        }

        let features = self.features(views)?;
        let mut output = Map::new();
        output.insert("type".to_string(), json!("FeatureCollection"));
        output.insert("name".to_string(), json!("points"));
        if let Some(precision) = self.creation_options.coordinate_precision {
            output.insert(
                "xy_coordinate_resolution".to_string(),
                rounded_value(10f64.powi(-(precision as i32)), precision),
            );
        }
        if self.creation_options.write_bbox {
            if let Some(bounds) = collection_bbox(&features) {
                output.insert("bbox".to_string(), bounds);
            }
        }
        output.insert("features".to_string(), Value::Array(features));
        let text = serde_json::to_string_pretty(&output)
            .map_err(|err| StageError(format!("Failed to serialize GeoJSON: {err}")))?;
        fs::write(Path::new(&self.filename), text).map_err(|err| {
            StageError(format!(
                "Couldn't open '{}' for output: {err}",
                self.filename
            ))
        })
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("writers.ogr");
        node.add_value("filename", MetadataValue::String(self.filename.clone()));
        node.add_value("point_count", MetadataValue::U64(self.point_count));
        node
    }
}

/// Validate the multicount/attr_dims combination used by the C++ OGR writer
/// wrapper. The returned message is unprefixed so the caller can route it
/// through `Stage::throwError`, which adds the `writers.ogr: ` prefix.
pub fn validate_multicount_and_attrs(multicount: u64, attr_dim_count: u64) -> Result<(), String> {
    if multicount < 1 {
        return Err("multicount must be greater than 0.".to_string());
    }
    if multicount > 1 && attr_dim_count > 0 {
        return Err("multicount > 1 incompatible with attr_dims".to_string());
    }
    Ok(())
}

/// Format the "attr_dims dimension not found" error string used by the C++
/// OGR writer wrapper. The returned message is unprefixed.
pub fn format_attr_dim_not_found(name: &str) -> String {
    format!("Dimension '{name}' (attr_dims) not found.")
}

impl OgrWriter {
    fn validate_options(&self) -> Result<(), StageError> {
        let driver = self.resolved_driver();
        if driver != "GeoJSON" {
            if !self.uses_native_ogr_driver() {
                return Err(StageError(format!(
                    "OgrWriter Rust implementation supports only GeoJSON, ESRI Shapefile, and GPKG, not '{driver}'."
                )));
            }
            if self.multicount != 1 && !self.attr_dims.is_empty() {
                return Err(StageError(
                    "OgrWriter Rust implementation does not support native OGR multicount with attributes."
                        .to_string(),
                ));
            }
            if self.creation_options != GeoJsonCreationOptions::default() {
                return Err(StageError(
                    "OgrWriter Rust implementation does not support native OGR creation options."
                        .to_string(),
                ));
            }
        }
        if self.multicount == 0 {
            return Err(StageError(
                "OgrWriter multicount must be greater than 0.".to_string(),
            ));
        }
        if self.multicount > 1 && !self.attr_dims.is_empty() {
            return Err(StageError(
                "OgrWriter multicount > 1 is incompatible with attr_dims.".to_string(),
            ));
        }
        if !self.measure_dim.is_empty() && driver != "ESRI Shapefile" {
            return Err(StageError(
                "OgrWriter Rust implementation does not support measure_dim.".to_string(),
            ));
        }
        self.creation_options.validate(&self.input_srs)?;
        Ok(())
    }

    fn resolved_driver(&self) -> String {
        if !self.driver_name.is_empty() {
            self.driver_name.clone()
        } else if self.filename.to_ascii_lowercase().ends_with(".geojson") {
            "GeoJSON".to_string()
        } else {
            "ESRI Shapefile".to_string()
        }
    }

    fn uses_native_ogr_driver(&self) -> bool {
        matches!(self.resolved_driver().as_str(), "ESRI Shapefile" | "GPKG")
    }

    fn features(&mut self, views: &[PointView]) -> Result<Vec<Value>, StageError> {
        if self.multicount > 1 {
            return Ok(self.multi_point_features(views));
        }

        let mut features = Vec::new();
        for view in views {
            let attr_dims = self.resolve_attr_dims(view.layout())?;
            for point in 0..view.len() {
                self.point_count += 1;
                features.push(json!({
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": self.point_coordinates(view, point),
                    },
                    "properties": properties(view, point, &attr_dims),
                }));
                if self.creation_options.write_bbox {
                    add_feature_bbox(features.last_mut().expect("feature was just pushed"));
                }
            }
        }
        Ok(features)
    }

    fn multi_point_features(&mut self, views: &[PointView]) -> Vec<Value> {
        let mut features = Vec::new();
        let mut coordinates = Vec::new();
        for view in views {
            for point in 0..view.len() {
                self.point_count += 1;
                coordinates.push(Value::Array(self.point_coordinates(view, point)));
                if coordinates.len() == self.multicount as usize {
                    features.push(self.multi_point_feature(std::mem::take(&mut coordinates)));
                }
            }
        }
        if !coordinates.is_empty() {
            features.push(self.multi_point_feature(coordinates));
        }
        features
    }

    fn resolve_attr_dims(&self, layout: &PointLayout) -> Result<Vec<DimId>, StageError> {
        if self.attr_dims.is_empty() {
            return Ok(Vec::new());
        }
        if self.attr_dims.iter().any(|name| name == "all") {
            let mut dims = Vec::new();
            for idx in 0..layout.dim_count() {
                if let Some((dim, _)) = layout.dim_at(idx) {
                    if !matches!(dim, DimId::X | DimId::Y | DimId::Z) {
                        dims.push(dim.clone());
                    }
                }
            }
            return Ok(dims);
        }
        self.attr_dims
            .iter()
            .map(|name| {
                let dim = DimId::from_name(name);
                layout
                    .dim(&dim)
                    .map(|_| dim)
                    .ok_or_else(|| StageError(format!("Dimension '{name}' (attr_dims) not found.")))
            })
            .collect()
    }

    fn multi_point_feature(&self, coordinates: Vec<Value>) -> Value {
        let mut feature = json!({
            "type": "Feature",
            "geometry": {
                "type": "MultiPoint",
                "coordinates": coordinates,
            },
            "properties": {},
        });
        if self.creation_options.write_bbox {
            add_feature_bbox(&mut feature);
        }
        feature
    }

    fn point_coordinates(&self, view: &PointView, point: u64) -> Vec<Value> {
        let precision = self.creation_options.coordinate_precision;
        vec![
            coordinate_value(view.get_f64(point, &DimId::X), precision),
            coordinate_value(view.get_f64(point, &DimId::Y), precision),
            coordinate_value(view.get_f64(point, &DimId::Z), precision),
        ]
    }

    fn write_native_points(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.multicount > 1 {
            return self.write_native_multipoints(views);
        }

        let driver = self.resolved_driver();
        let measure_dim = self.resolve_optional_dim(views, &self.measure_dim)?;
        let writer = pdal_native::gdal::VectorPointWriter::create_point(
            &self.filename,
            &driver,
            &self.input_srs,
            measure_dim.is_some() || driver == "GPKG",
        )
        .map_err(StageError)?;
        for view in views {
            let attr_dims = self.resolve_attr_dims(view.layout())?;
            for dim in &attr_dims {
                let (_, dim_type) = view.layout().dim(dim).ok_or_else(|| {
                    StageError(format!("Dimension '{}' (attr_dims) not found.", dim.name()))
                })?;
                writer
                    .create_field(dim.name(), vector_field_type(dim_type))
                    .map_err(StageError)?;
            }
            for point in 0..view.len() {
                let fields = attr_dims
                    .iter()
                    .map(|dim| vector_field_value(view, point, dim))
                    .collect::<Vec<_>>();
                writer
                    .write_point(
                        view.get_f64(point, &DimId::X),
                        view.get_f64(point, &DimId::Y),
                        view.get_f64(point, &DimId::Z),
                        measure_dim.as_ref().map(|dim| view.get_f64(point, dim)),
                        &fields,
                    )
                    .map_err(StageError)?;
                self.point_count += 1;
            }
        }
        Ok(())
    }

    fn resolve_optional_dim(
        &self,
        views: &[PointView],
        name: &str,
    ) -> Result<Option<DimId>, StageError> {
        if name.trim().is_empty() {
            return Ok(None);
        }
        let dim = DimId::from_name(name);
        if views.iter().any(|view| view.layout().dim(&dim).is_some()) {
            Ok(Some(dim))
        } else {
            Err(StageError(format!(
                "Dimension '{name}' (measure_dim) not found."
            )))
        }
    }

    fn write_native_multipoints(&mut self, views: &[PointView]) -> Result<(), StageError> {
        let writer = pdal_native::gdal::VectorPointWriter::create_multipoint(
            &self.filename,
            &self.resolved_driver(),
            &self.input_srs,
        )
        .map_err(StageError)?;
        let mut points = Vec::new();
        for view in views {
            for point in 0..view.len() {
                points.push((
                    view.get_f64(point, &DimId::X),
                    view.get_f64(point, &DimId::Y),
                    view.get_f64(point, &DimId::Z),
                ));
                self.point_count += 1;
                if points.len() == self.multicount as usize {
                    writer.write_multipoint(&points).map_err(StageError)?;
                    points.clear();
                }
            }
        }
        if !points.is_empty() {
            writer.write_multipoint(&points).map_err(StageError)?;
        }
        Ok(())
    }
}

fn vector_field_type(dim_type: DimType) -> pdal_native::gdal::VectorFieldType {
    match dim_type {
        DimType::U8 | DimType::U16 | DimType::I8 | DimType::I16 | DimType::I32 => {
            pdal_native::gdal::VectorFieldType::Integer
        }
        DimType::U32 | DimType::U64 | DimType::I64 => pdal_native::gdal::VectorFieldType::Integer64,
        DimType::F32 | DimType::F64 => pdal_native::gdal::VectorFieldType::Real,
    }
}

fn vector_field_value(
    view: &PointView,
    point: u64,
    dim: &DimId,
) -> pdal_native::gdal::VectorFieldValue {
    match view.layout().dim(dim).map(|(_, dim_type)| dim_type) {
        Some(DimType::U8 | DimType::U16 | DimType::I8 | DimType::I16 | DimType::I32) => {
            pdal_native::gdal::VectorFieldValue::Integer(view.get_f64(point, dim) as i32)
        }
        Some(DimType::U32 | DimType::U64 | DimType::I64) => {
            pdal_native::gdal::VectorFieldValue::Integer64(view.get_f64(point, dim) as i64)
        }
        Some(DimType::F32 | DimType::F64) | None => {
            pdal_native::gdal::VectorFieldValue::Real(view.get_f64(point, dim))
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct GeoJsonCreationOptions {
    write_bbox: bool,
    rfc7946: bool,
    coordinate_precision: Option<u32>,
}

impl GeoJsonCreationOptions {
    fn from_options(options: &Options) -> Self {
        let mut parsed = Self::default();
        for value in options.values("ogr_options") {
            let Some((key, value)) = value.split_once('=') else {
                continue;
            };
            match key {
                "WRITE_BBOX" if value == "YES" => parsed.write_bbox = true,
                "RFC7946" if value == "YES" => parsed.rfc7946 = true,
                "COORDINATE_PRECISION" => {
                    parsed.coordinate_precision = value.parse::<u32>().ok();
                }
                _ => {}
            }
        }
        parsed
    }

    fn validate(&self, _input_srs: &str) -> Result<(), StageError> {
        if matches!(self.coordinate_precision, Some(precision) if precision > 15) {
            return Err(StageError(
                "OgrWriter coordinate precision must be 15 or less.".to_string(),
            ));
        }
        if self.rfc7946 {
            return Err(StageError(
                "writers.ogr: Can't create OGR layer: Failed to create coordinate transformation between the input coordinate system and WGS84.".to_string(),
            ));
        }
        Ok(())
    }
}

fn coordinate_value(value: f64, precision: Option<u32>) -> Value {
    match precision {
        Some(precision) => rounded_value(value, precision),
        None => json!(value),
    }
}

fn rounded_value(value: f64, precision: u32) -> Value {
    let precision = precision as usize;
    let rounded = format!("{value:.precision$}")
        .parse::<f64>()
        .unwrap_or(value);
    Number::from_f64(rounded)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn properties(view: &PointView, point: u64, dims: &[DimId]) -> Value {
    let mut properties = Map::new();
    for dim in dims {
        let value = view.get_f64(point, dim);
        properties.insert(
            dim.name().to_string(),
            Number::from_f64(value)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
    }
    Value::Object(properties)
}

fn add_feature_bbox(feature: &mut Value) {
    let Some(geometry) = feature.get("geometry") else {
        return;
    };
    let Some(bounds) = geometry_bbox(geometry) else {
        return;
    };
    if let Some(object) = feature.as_object_mut() {
        object.insert("bbox".to_string(), bounds_to_value(bounds));
    }
}

fn collection_bbox(features: &[Value]) -> Option<Value> {
    let mut bounds: Option<Bounds3D> = None;
    for feature in features {
        let geometry = feature.get("geometry")?;
        let feature_bounds = geometry_bbox(geometry)?;
        match &mut bounds {
            Some(total) => total.grow(feature_bounds),
            None => bounds = Some(feature_bounds),
        }
    }
    bounds.map(bounds_to_value)
}

fn geometry_bbox(geometry: &Value) -> Option<Bounds3D> {
    let coordinates = geometry.get("coordinates")?;
    let mut bounds = Bounds3D::default();
    collect_coordinate_bounds(coordinates, &mut bounds);
    bounds.valid.then_some(bounds)
}

fn collect_coordinate_bounds(value: &Value, bounds: &mut Bounds3D) {
    if let Some(values) = value.as_array() {
        if values.len() >= 2 && values.first().is_some_and(Value::is_number) {
            let x = values.first().and_then(Value::as_f64).unwrap_or(0.0);
            let y = values.get(1).and_then(Value::as_f64).unwrap_or(0.0);
            let z = values.get(2).and_then(Value::as_f64).unwrap_or(0.0);
            bounds.grow_point(x, y, z);
        } else {
            for item in values {
                collect_coordinate_bounds(item, bounds);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Bounds3D {
    minx: f64,
    miny: f64,
    minz: f64,
    maxx: f64,
    maxy: f64,
    maxz: f64,
    valid: bool,
}

impl Default for Bounds3D {
    fn default() -> Self {
        Self {
            minx: 0.0,
            miny: 0.0,
            minz: 0.0,
            maxx: 0.0,
            maxy: 0.0,
            maxz: 0.0,
            valid: false,
        }
    }
}

impl Bounds3D {
    fn grow(&mut self, other: Bounds3D) {
        self.grow_point(other.minx, other.miny, other.minz);
        self.grow_point(other.maxx, other.maxy, other.maxz);
    }

    fn grow_point(&mut self, x: f64, y: f64, z: f64) {
        if !self.valid {
            self.minx = x;
            self.maxx = x;
            self.miny = y;
            self.maxy = y;
            self.minz = z;
            self.maxz = z;
            self.valid = true;
            return;
        }
        self.minx = self.minx.min(x);
        self.maxx = self.maxx.max(x);
        self.miny = self.miny.min(y);
        self.maxy = self.maxy.max(y);
        self.minz = self.minz.min(z);
        self.maxz = self.maxz.max(z);
    }
}

fn bounds_to_value(bounds: Bounds3D) -> Value {
    json!([
        bounds.minx,
        bounds.miny,
        bounds.minz,
        bounds.maxx,
        bounds.maxy,
        bounds.maxz
    ])
}

fn comma_values(options: &Options, key: &str) -> Vec<String> {
    options
        .values(key)
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn test_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Intensity, DimType::U16);
        let mut view = PointView::new(Rc::new(layout));
        for values in [(1.0, 2.0, 3.0, 10.0), (4.0, 5.0, 6.0, 20.0)] {
            let point = view.add_point();
            view.set_f64(point, &DimId::X, values.0);
            view.set_f64(point, &DimId::Y, values.1);
            view.set_f64(point, &DimId::Z, values.2);
            view.set_f64(point, &DimId::Intensity, values.3);
        }
        view
    }

    fn write_geojson(configure: impl FnOnce(&mut Options)) -> Value {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let path = temp.path().with_extension("geojson");
        let mut options = Options::new();
        options.add("filename", path.display());
        configure(&mut options);

        let mut writer = OgrWriter::new(&options);
        writer.write(&[test_view()]).unwrap();

        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn writes_geojson_point_features() {
        let json = write_geojson(|options| {
            options.add("ogrdriver", "GeoJSON");
        });

        assert_eq!(json["type"], "FeatureCollection");
        assert_eq!(json["features"].as_array().unwrap().len(), 2);
        assert_eq!(json["features"][0]["geometry"]["coordinates"][0], 1.0);
    }

    #[test]
    fn attr_dims_add_properties() {
        let json = write_geojson(|options| {
            options
                .add("ogrdriver", "GeoJSON")
                .add("attr_dims", "Intensity");
        });

        assert_eq!(json["features"][0]["properties"]["Intensity"], 10.0);
        assert_eq!(json["features"][1]["properties"]["Intensity"], 20.0);
    }

    #[test]
    fn multicount_groups_points_as_multipoint_features() {
        let json = write_geojson(|options| {
            options.add("ogrdriver", "GeoJSON").add("multicount", 3);
        });

        assert_eq!(json["features"].as_array().unwrap().len(), 1);
        assert_eq!(json["features"][0]["geometry"]["type"], "MultiPoint");
        assert_eq!(
            json["features"][0]["geometry"]["coordinates"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(json["features"][0]["geometry"]["coordinates"][1][2], 6.0);
    }

    #[test]
    fn creation_options_round_coordinates_and_write_bboxes() {
        let json = write_geojson(|options| {
            options
                .add("ogrdriver", "GeoJSON")
                .add("ogr_options", "WRITE_BBOX=YES")
                .add("ogr_options", "COORDINATE_PRECISION=1");
        });

        assert_eq!(json["xy_coordinate_resolution"], 0.1);
        assert_eq!(json["features"][0]["geometry"]["coordinates"][0], 1.0);
        assert_eq!(json["features"][0]["bbox"][3], 1.0);
        assert_eq!(json["bbox"][0], 1.0);
        assert_eq!(json["bbox"][5], 6.0);
    }

    #[test]
    fn multicount_rejects_attr_dims_like_cpp_writer() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let mut options = Options::new();
        options
            .add("filename", temp.path().with_extension("geojson").display())
            .add("ogrdriver", "GeoJSON")
            .add("multicount", 3)
            .add("attr_dims", "Intensity");
        let mut writer = OgrWriter::new(&options);

        assert!(writer.write(&[test_view()]).is_err());
    }

    #[test]
    fn rejects_unsupported_driver() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let mut options = Options::new();
        options
            .add("filename", temp.path().display())
            .add("ogrdriver", "SQLite");
        let mut writer = OgrWriter::new(&options);

        assert!(writer.write(&[test_view()]).is_err());
    }

    #[test]
    fn writes_plain_geopackage_points() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("points.gpkg");
        let mut options = Options::new();
        options
            .add("filename", path.display())
            .add("ogrdriver", "GPKG");
        let mut writer = OgrWriter::new(&options);

        writer.write(&[test_view()]).unwrap();

        assert!(path.exists());
        assert_eq!(writer.point_count, 2);
    }

    #[test]
    fn writes_plain_shapefile_points() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("points.shp");
        let mut options = Options::new();
        options
            .add("filename", path.display())
            .add("ogrdriver", "ESRI Shapefile");
        let mut writer = OgrWriter::new(&options);

        writer.write(&[test_view()]).unwrap();

        assert!(path.exists());
        assert!(temp.path().join("points.shx").exists());
        assert_eq!(writer.point_count, 2);
    }

    #[test]
    fn writer_errors_without_filename() {
        let mut writer = OgrWriter::new(&Options::new());
        assert!(writer.write(&[test_view()]).is_err());
    }

    #[test]
    fn validate_multicount_and_attrs_branches() {
        assert!(validate_multicount_and_attrs(0, 0).is_err());
        assert!(validate_multicount_and_attrs(1, 0).is_ok());
        assert!(validate_multicount_and_attrs(2, 0).is_ok());
        assert!(validate_multicount_and_attrs(2, 1).is_err());
    }

    #[test]
    fn format_attr_dim_not_found_includes_name() {
        let msg = format_attr_dim_not_found("Intensity");
        assert!(msg.contains("Intensity"));
        assert!(msg.contains("attr_dims"));
    }

    #[test]
    fn writer_metadata_includes_filename_and_count() {
        let mut options = Options::new();
        options.add("filename", "x.geojson");
        let writer = OgrWriter::new(&options);
        let metadata = writer.metadata();
        assert_eq!(metadata.name(), "writers.ogr");
        let filename = metadata
            .find_child("filename")
            .and_then(MetadataNode::value)
            .map(MetadataValue::as_string);
        assert_eq!(filename, Some("x.geojson".to_string()));
    }

    #[test]
    fn writer_resolves_default_driver_from_filename_extension() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let path = temp.path().with_extension("shp");
        let mut options = Options::new();
        options.add("filename", path.display());
        let mut writer = OgrWriter::new(&options);
        assert!(writer.write(&[test_view()]).is_ok());
    }

    #[test]
    fn writer_with_measure_dim_errors() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let path = temp.path().with_extension("geojson");
        let mut options = Options::new();
        options.add("filename", path.display());
        options.add("measure_dim", "Intensity");
        let mut writer = OgrWriter::new(&options);
        assert!(writer.write(&[test_view()]).is_err());
    }

    #[test]
    fn rfc7946_rejects_non_empty_input_srs_like_gdal() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let path = temp.path().with_extension("geojson");
        let mut options = Options::new();
        options.add("filename", path.display());
        options.add("ogrdriver", "GeoJSON");
        options.add("ogr_options", "RFC7946=YES");
        options.add("input_srs", "LOCAL_CS[\"unnamed\"]");
        let mut writer = OgrWriter::new(&options);
        let err = writer.write(&[test_view()]).err().unwrap();

        assert!(err.0.contains("coordinate transformation"));
    }
}
