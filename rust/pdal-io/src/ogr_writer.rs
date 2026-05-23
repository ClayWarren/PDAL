//! `writers.ogr` -- GeoJSON-only local writer slice.
//!
//! The C++ writer is a broad OGR/GDAL adapter. This Rust writer covers local
//! GeoJSON output. Shapefile, GeoPackage, transactions, and measure dimensions
//! stay deferred to the native OGR milestone.

use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, PointLayout, PointView};
use pdal_core::stage::StageError;
use serde_json::{json, Map, Number, Value};
use std::fs;
use std::path::Path;

pub struct OgrWriter {
    filename: String,
    driver_name: String,
    attr_dims: Vec<String>,
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

        let features = self.features(views)?;
        let output = json!({
            "type": "FeatureCollection",
            "name": "points",
            "features": features,
        });
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
            return Err(StageError(format!(
                "OgrWriter Rust implementation supports only GeoJSON, not '{driver}'."
            )));
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
        if !self.measure_dim.is_empty() {
            return Err(StageError(
                "OgrWriter Rust implementation does not support measure_dim.".to_string(),
            ));
        }
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
                        "coordinates": point_coordinates(view, point),
                    },
                    "properties": properties(view, point, &attr_dims),
                }));
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
                coordinates.push(Value::Array(point_coordinates(view, point)));
                if coordinates.len() == self.multicount as usize {
                    features.push(multi_point_feature(std::mem::take(&mut coordinates)));
                }
            }
        }
        if !coordinates.is_empty() {
            features.push(multi_point_feature(coordinates));
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
}

fn multi_point_feature(coordinates: Vec<Value>) -> Value {
    json!({
        "type": "Feature",
        "geometry": {
            "type": "MultiPoint",
            "coordinates": coordinates,
        },
        "properties": {},
    })
}

fn point_coordinates(view: &PointView, point: u64) -> Vec<Value> {
    vec![
        json!(view.get_f64(point, &DimId::X)),
        json!(view.get_f64(point, &DimId::Y)),
        json!(view.get_f64(point, &DimId::Z)),
    ]
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
    fn rejects_non_geojson_driver() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let mut options = Options::new();
        options
            .add("filename", temp.path().display())
            .add("ogrdriver", "ESRI Shapefile");
        let mut writer = OgrWriter::new(&options);

        assert!(writer.write(&[test_view()]).is_err());
    }
}
