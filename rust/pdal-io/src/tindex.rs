use crate::source;
use pdal_core::bounds::{parse_bounds2d, Bounds2D};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use pdal_native::geometry::Geometry;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Tile-index reader for GeoJSON indexes produced by `pdal tindex create`.
pub struct TindexReader {
    filename: String,
    layer_name: String,
    location_field: String,
    attribute_filter: String,
    bounds: String,
}

impl TindexReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            layer_name: options.get_str("lyr_name", ""),
            location_field: options.get_str("tindex_name", "location"),
            attribute_filter: options.get_str("where", ""),
            bounds: options.get_str("bounds", ""),
        }
    }
}

impl Reader for TindexReader {
    fn name(&self) -> &str {
        "readers.tindex"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "TindexReader requires a filename option.".to_string(),
            ));
        }

        let bounds = parse_tindex_bounds(&self.bounds)?;
        let features = read_index_features(
            &self.filename,
            &self.layer_name,
            &self.location_field,
            &self.attribute_filter,
            bounds.as_ref(),
        )?;

        let mut merged: Option<PointView> = None;
        let base = location_base(&self.filename);
        for feature in features {
            let location = resolve_location_text(&base, &feature.location);
            let mut views = read_point_location(&location, None, &Options::new())?;
            for view in views.drain(..) {
                append_view(&mut merged, &view, Path::new(&location))?;
            }
        }

        match merged {
            Some(view) => Ok(vec![view]),
            None => Ok(Vec::new()),
        }
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.tindex")
    }
}

struct IndexFeature {
    location: String,
}

fn read_index_features(
    filename: &str,
    layer_name: &str,
    location_field: &str,
    attribute_filter: &str,
    bounds: Option<&Bounds2D>,
) -> Result<Vec<IndexFeature>, StageError> {
    match source::read_to_string(filename) {
        Ok(text) => match read_geojson_index_features(&text, location_field, bounds) {
            Ok(features) => Ok(features),
            Err(json_err) => read_ogr_index_features(
                filename,
                layer_name,
                location_field,
                attribute_filter,
                bounds,
            )
            .or(Err(json_err)),
        },
        Err(text_err) => read_ogr_index_features(
            filename,
            layer_name,
            location_field,
            attribute_filter,
            bounds,
        )
        .map_err(|ogr_err| StageError(format!("{text_err}; {ogr_err}"))),
    }
}

fn read_geojson_index_features(
    text: &str,
    location_field: &str,
    bounds: Option<&Bounds2D>,
) -> Result<Vec<IndexFeature>, StageError> {
    let json: serde_json::Value = serde_json::from_str(text).map_err(|err| {
        StageError(format!(
            "TindexReader expected a GeoJSON FeatureCollection: {err}"
        ))
    })?;
    let features = json["features"].as_array().ok_or_else(|| {
        StageError("TindexReader expected a GeoJSON FeatureCollection.".to_string())
    })?;
    let mut output = Vec::new();
    for feature in features {
        if !feature_matches_bounds(feature, bounds)? {
            continue;
        }
        let location = feature["properties"][location_field]
            .as_str()
            .ok_or_else(|| {
                StageError(format!(
                    "TindexReader feature is missing '{}'.",
                    location_field
                ))
            })?;
        output.push(IndexFeature {
            location: location.to_string(),
        });
    }
    Ok(output)
}

fn read_ogr_index_features(
    filename: &str,
    layer_name: &str,
    location_field: &str,
    attribute_filter: &str,
    bounds: Option<&Bounds2D>,
) -> Result<Vec<IndexFeature>, StageError> {
    let vector = pdal_native::gdal::Vector::open(filename).map_err(StageError)?;
    let features = vector
        .get_string_features_by_layer(layer_name, location_field, attribute_filter)
        .map_err(StageError)?;
    let mut output = Vec::new();
    for (wkt, location) in features {
        if !wkt_matches_bounds(&wkt, bounds)? {
            continue;
        }
        output.push(IndexFeature { location });
    }
    Ok(output)
}

pub(crate) fn resolve_location(base: &Path, location: &str) -> PathBuf {
    let path = Path::new(location);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn location_base(filename: &str) -> String {
    if source::is_vsi_path(filename) {
        return filename
            .rsplit_once('/')
            .map(|(base, _)| base.to_string())
            .unwrap_or_default();
    }

    Path::new(filename)
        .parent()
        .unwrap_or(Path::new(""))
        .display()
        .to_string()
}

fn resolve_location_text(base: &str, location: &str) -> String {
    if source::is_vsi_path(location) || Path::new(location).is_absolute() {
        location.to_string()
    } else if source::is_vsi_path(base) {
        format!("{}/{}", base.trim_end_matches('/'), location)
    } else {
        Path::new(base).join(location).display().to_string()
    }
}

fn wkt_matches_bounds(wkt: &str, bounds: Option<&Bounds2D>) -> Result<bool, StageError> {
    let Some(bounds) = bounds else {
        return Ok(true);
    };
    let geometry = Geometry::from_wkt(wkt).map_err(StageError)?;
    let (minx, maxx, miny, maxy, _, _) = geometry.bounds().map_err(StageError)?;
    Ok(Bounds2D {
        minx,
        maxx,
        miny,
        maxy,
    }
    .overlaps(bounds))
}

fn parse_tindex_bounds(bounds: &str) -> Result<Option<Bounds2D>, StageError> {
    if bounds.trim().is_empty() {
        return Ok(None);
    }
    parse_bounds2d(bounds, 0)
        .map(|parsed| Some(parsed.bounds))
        .map_err(StageError)
}

fn feature_matches_bounds(
    feature: &serde_json::Value,
    bounds: Option<&Bounds2D>,
) -> Result<bool, StageError> {
    let Some(bounds) = bounds else {
        return Ok(true);
    };
    let Some(feature_bounds) = geojson_geometry_bounds(&feature["geometry"])? else {
        return Ok(false);
    };
    Ok(feature_bounds.overlaps(bounds))
}

fn geojson_geometry_bounds(geometry: &serde_json::Value) -> Result<Option<Bounds2D>, StageError> {
    if geometry.is_null() {
        return Ok(None);
    }
    match geometry["type"].as_str().unwrap_or("") {
        "Polygon" => polygon_bounds(geometry),
        "MultiPolygon" => multipolygon_bounds(geometry),
        other => Err(StageError(format!(
            "Unsupported TIndex GeoJSON geometry type '{other}'."
        ))),
    }
}

fn multipolygon_bounds(geometry: &serde_json::Value) -> Result<Option<Bounds2D>, StageError> {
    let Some(polygons) = geometry["coordinates"].as_array() else {
        return Err(StageError(
            "Invalid TIndex MultiPolygon geometry.".to_string(),
        ));
    };
    let mut bounds = None;
    for polygon in polygons {
        let Some(rings) = polygon.as_array() else {
            return Err(StageError(
                "Invalid TIndex MultiPolygon geometry.".to_string(),
            ));
        };
        let Some(polygon_bounds) = rings_bounds(rings)? else {
            continue;
        };
        grow_optional_bounds(&mut bounds, polygon_bounds);
    }
    Ok(bounds)
}

fn polygon_bounds(geometry: &serde_json::Value) -> Result<Option<Bounds2D>, StageError> {
    let Some(rings) = geometry["coordinates"].as_array() else {
        return Err(StageError("Invalid TIndex Polygon geometry.".to_string()));
    };
    rings_bounds(rings)
}

fn rings_bounds(rings: &[serde_json::Value]) -> Result<Option<Bounds2D>, StageError> {
    let Some(outer) = rings.first().and_then(serde_json::Value::as_array) else {
        return Ok(None);
    };
    let mut bounds = None;
    for coord in outer {
        let values = coord
            .as_array()
            .ok_or_else(|| StageError("Invalid TIndex polygon coordinate.".to_string()))?;
        let x = values
            .first()
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| StageError("Invalid TIndex polygon coordinate.".to_string()))?;
        let y = values
            .get(1)
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| StageError("Invalid TIndex polygon coordinate.".to_string()))?;
        grow_optional_bounds(
            &mut bounds,
            Bounds2D {
                minx: x,
                maxx: x,
                miny: y,
                maxy: y,
            },
        );
    }
    Ok(bounds)
}

fn grow_optional_bounds(bounds: &mut Option<Bounds2D>, other: Bounds2D) {
    match bounds {
        Some(bounds) => bounds.grow_bounds(&other),
        None => *bounds = Some(other),
    }
}

pub(crate) fn read_point_location(
    location: &str,
    driver_hint: Option<&str>,
    extra_options: &Options,
) -> Result<Vec<PointView>, StageError> {
    let driver = driver_hint
        .or_else(|| pdal_core::driver::infer_reader_driver(location))
        .ok_or_else(|| {
            StageError(format!(
                "TindexReader cannot infer a reader driver for '{}'.",
                location
            ))
        })?;
    let mut options = extra_options.clone();
    options.add("filename", location);
    match driver {
        "readers.bpf" => crate::bpf::BpfReader::new(&options).read(),
        "readers.fbi" => crate::fbi::FbiReader::new(&options).read(),
        "readers.gdal" => crate::gdal_reader::GdalReader::new(&options).read(),
        "readers.text" => crate::text::TextReader::new(&options).read(),
        "readers.pcd" => crate::pcd::PcdReader::new(&options).read(),
        "readers.pts" => crate::pts::PtsReader::new(&options).read(),
        "readers.ptx" => crate::ptx::PtxReader::new(&options).read(),
        "readers.ilvis2" => crate::ilvis2::Ilvis2Reader::new(&options).read(),
        "readers.obj" => crate::obj::ObjReader::new(&options).read(),
        "readers.optech" => crate::optech::OptechReader::new(&options).read(),
        "readers.qfit" => crate::qfit::QfitReader::new(&options).read(),
        "readers.sbet" => crate::sbet::SbetReader::new(&options).read(),
        "readers.smrmsg" => crate::smrmsg::SmrmsgReader::new(&options).read(),
        "readers.terrasolid" => crate::terrasolid::TerrasolidReader::new(&options).read(),
        "readers.copc" => crate::copc::CopcReader::new(&options).read(),
        "readers.las" | "readers.laz" => crate::las::LasReader::new(&options).read(),
        "readers.ept" => crate::ept::EptReader::new(&options).read(),
        "readers.ply" => crate::ply::PlyReader::new(&options).read(),
        _ => Err(StageError(format!(
            "TindexReader driver '{driver}' is not available in the Rust port."
        ))),
    }
}

pub(crate) fn append_view(
    merged: &mut Option<PointView>,
    view: &PointView,
    path: &Path,
) -> Result<(), StageError> {
    if merged.is_none() {
        *merged = Some(empty_view_like(view, &view_dims(view)));
    }

    let current = merged.take().unwrap();
    let dims = union_dims(&current, view, path)?;
    let mut target = if same_dims(&current, &dims) {
        current
    } else {
        normalize_view(&current, &dims)
    };
    let source = if same_dims(view, &dims) {
        view.clone()
    } else {
        normalize_view(view, &dims)
    };
    for idx in 0..source.len() {
        target.append_point(&source, idx);
    }
    *merged = Some(target);
    Ok(())
}

fn view_dims(view: &PointView) -> Vec<(DimId, DimType)> {
    (0..view.layout().dim_count())
        .filter_map(|idx| view.layout().dim_at(idx))
        .map(|(dim, ty)| (dim.clone(), ty))
        .collect()
}

fn union_dims(
    target: &PointView,
    source: &PointView,
    path: &Path,
) -> Result<Vec<(DimId, DimType)>, StageError> {
    let mut dims = view_dims(target);
    for (dim, ty) in view_dims(source) {
        match dims.iter().find(|(existing, _)| existing == &dim) {
            Some((_, existing_ty)) if *existing_ty != ty => {
                return Err(StageError(format!(
                    "'{}' has dimension '{}' with incompatible types.",
                    path.display(),
                    dim.name()
                )));
            }
            Some(_) => {}
            None => dims.push((dim, ty)),
        }
    }
    Ok(dims)
}

fn same_dims(view: &PointView, dims: &[(DimId, DimType)]) -> bool {
    view.layout().dim_count() == dims.len()
        && dims.iter().enumerate().all(|(idx, (dim, ty))| {
            view.layout()
                .dim_at(idx)
                .is_some_and(|(view_dim, view_ty)| view_dim == dim && view_ty == *ty)
        })
}

fn normalize_view(view: &PointView, dims: &[(DimId, DimType)]) -> PointView {
    let mut output = empty_view_like(view, dims);
    for idx in 0..view.len() {
        let out_idx = output.add_point();
        output.set_source_index(out_idx, view.source_index(idx));
        for (dim, _) in dims {
            output.set_f64(out_idx, dim, view.get_f64(idx, dim));
        }
    }
    output
}

fn empty_view_like(view: &PointView, dims: &[(DimId, DimType)]) -> PointView {
    let mut layout = PointLayout::new();
    for (dim, ty) in dims {
        layout.register(dim.clone(), *ty);
    }
    let mut output = PointView::new(Rc::new(layout));
    output.set_spatial_reference(view.spatial_reference().clone());
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::DimId;

    #[test]
    fn reads_geojson_index_and_merges_referenced_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        let source_copy = temp.path().join("simple_text.ply");
        std::fs::copy(&source, &source_copy).unwrap();
        let index = temp.path().join("index.geojson");
        std::fs::write(
            &index,
            r#"{
  "type": "FeatureCollection",
  "features": [
    {"type":"Feature","properties":{"location":"simple_text.ply"},"geometry":null},
    {"type":"Feature","properties":{"location":"simple_text.ply"},"geometry":null}
  ]
}"#,
        )
        .unwrap();

        let mut options = Options::new();
        options.add("filename", index.display());
        let mut reader = TindexReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 6);
        assert_eq!(views[0].get_f64(0, &DimId::X), -1.0);
        assert_eq!(views[0].get_f64(3, &DimId::X), -1.0);
    }

    #[test]
    fn honors_custom_location_field() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        let source_copy = temp.path().join("simple_text.ply");
        std::fs::copy(&source, &source_copy).unwrap();
        let index = temp.path().join("index.geojson");
        std::fs::write(
            &index,
            r#"{
  "type": "FeatureCollection",
  "features": [
    {"type":"Feature","properties":{"source_file":"simple_text.ply"},"geometry":null}
  ]
}"#,
        )
        .unwrap();

        let mut options = Options::new();
        options.add("filename", index.display());
        options.add("tindex_name", "source_file");
        let mut reader = TindexReader::new(&options);

        assert_eq!(reader.read().unwrap()[0].len(), 3);
    }

    #[test]
    fn reads_ogr_index_and_merges_referenced_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        let source_copy = temp.path().join("simple_text.ply");
        std::fs::copy(&source, &source_copy).unwrap();
        let index = temp.path().join("index.shp");
        {
            let vector =
                pdal_native::gdal::Vector::create(index.to_str().unwrap(), "ESRI Shapefile")
                    .unwrap();
            let layer = vector.open_or_create_layer("index", "").unwrap();
            unsafe {
                pdal_native::gdal::Vector::create_string_field(layer, "location").unwrap();
                pdal_native::gdal::Vector::add_feature(
                    layer,
                    "POLYGON((0 0,1 0,1 1,0 1,0 0))",
                    &[("location", "simple_text.ply")],
                )
                .unwrap();
                pdal_native::gdal::Vector::add_feature(
                    layer,
                    "POLYGON((50 50,51 50,51 51,50 51,50 50))",
                    &[("location", "simple_text.ply")],
                )
                .unwrap();
            }
        }

        let mut options = Options::new();
        options.add("filename", index.display());
        options.add("bounds", "([0, 2],[0, 2])");
        let mut reader = TindexReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 3);
    }

    #[test]
    fn reads_named_ogr_layer() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        let source_copy = temp.path().join("simple_text.ply");
        std::fs::copy(&source, &source_copy).unwrap();
        let index = temp.path().join("index.gpkg");
        {
            let vector = pdal_native::gdal::Vector::create(index.to_str().unwrap(), "GPKG")
                .expect("GPKG driver is available");
            let layer = vector.open_or_create_layer("tiles", "").unwrap();
            unsafe {
                pdal_native::gdal::Vector::create_string_field(layer, "location").unwrap();
                pdal_native::gdal::Vector::add_feature(
                    layer,
                    "POLYGON((0 0,1 0,1 1,0 1,0 0))",
                    &[("location", "simple_text.ply")],
                )
                .unwrap();
            }
        }

        let mut options = Options::new();
        options.add("filename", index.display());
        options.add("lyr_name", "tiles");
        let mut reader = TindexReader::new(&options);

        assert_eq!(reader.read().unwrap()[0].len(), 3);
    }

    #[test]
    fn reads_ogr_index_with_attribute_filter() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        let source_copy = temp.path().join("simple_text.ply");
        std::fs::copy(&source, &source_copy).unwrap();
        let index = temp.path().join("index.gpkg");
        {
            let vector = pdal_native::gdal::Vector::create(index.to_str().unwrap(), "GPKG")
                .expect("GPKG driver is available");
            let layer = vector.open_or_create_layer("tiles", "").unwrap();
            unsafe {
                pdal_native::gdal::Vector::create_string_field(layer, "location").unwrap();
                pdal_native::gdal::Vector::create_string_field(layer, "bucket").unwrap();
                pdal_native::gdal::Vector::add_feature(
                    layer,
                    "POLYGON((0 0,1 0,1 1,0 1,0 0))",
                    &[("location", "simple_text.ply"), ("bucket", "keep")],
                )
                .unwrap();
                pdal_native::gdal::Vector::add_feature(
                    layer,
                    "POLYGON((0 0,1 0,1 1,0 1,0 0))",
                    &[("location", "simple_text.ply"), ("bucket", "skip")],
                )
                .unwrap();
            }
        }

        let mut options = Options::new();
        options.add("filename", index.display());
        options.add("lyr_name", "tiles");
        options.add("where", "bucket = 'keep'");
        let mut reader = TindexReader::new(&options);

        assert_eq!(reader.read().unwrap()[0].len(), 3);
    }

    #[test]
    fn bounds_filter_skips_non_overlapping_features() {
        let temp = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply");
        let source_copy = temp.path().join("simple_text.ply");
        std::fs::copy(&source, &source_copy).unwrap();
        let index = temp.path().join("index.geojson");
        std::fs::write(
            &index,
            r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type":"Feature",
      "properties":{"location":"simple_text.ply"},
      "geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,1],[0,0]]]}
    },
    {
      "type":"Feature",
      "properties":{"location":"simple_text.ply"},
      "geometry":{"type":"Polygon","coordinates":[[[50,50],[51,50],[51,51],[50,51],[50,50]]]}
    }
  ]
}"#,
        )
        .unwrap();

        let mut options = Options::new();
        options.add("filename", index.display());
        options.add("bounds", "([0, 2],[0, 2])");
        let mut reader = TindexReader::new(&options);
        let views = reader.read().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].len(), 3);
    }

    #[test]
    fn bounds_filter_supports_multipolygon_features() {
        let geometry = serde_json::json!({
            "type": "MultiPolygon",
            "coordinates": [
                [[[10,10],[11,10],[11,11],[10,11],[10,10]]],
                [[[0,0],[1,0],[1,1],[0,1],[0,0]]]
            ]
        });
        let bounds = geojson_geometry_bounds(&geometry).unwrap().unwrap();

        assert_eq!(bounds.minx, 0.0);
        assert_eq!(bounds.maxx, 11.0);
        assert_eq!(bounds.miny, 0.0);
        assert_eq!(bounds.maxy, 11.0);
    }

    #[test]
    fn bounds_filter_rejects_unsupported_geometry() {
        let geometry = serde_json::json!({"type": "LineString", "coordinates": [[0, 0], [1, 1]]});
        assert!(geojson_geometry_bounds(&geometry).is_err());
    }

    #[test]
    fn reader_errors_without_filename() {
        let mut reader = TindexReader::new(&Options::new());
        let err = reader.read().err().expect("missing filename");
        assert!(err.0.contains("filename"));
    }

    #[test]
    fn reader_errors_on_missing_file() {
        let mut options = Options::new();
        options.add("filename", "/no/such/tindex.json");
        let mut reader = TindexReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_invalid_json() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), b"{not json").unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = TindexReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_errors_on_non_feature_collection() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), b"{\"type\":\"Other\"}").unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = TindexReader::new(&options);
        assert!(reader.read().is_err());
    }

    #[test]
    fn reader_metadata_returns_expected_name() {
        let reader = TindexReader::new(&Options::new());
        assert_eq!(reader.metadata().name(), "readers.tindex");
    }

    #[test]
    fn resolve_location_absolute_path_preserved() {
        let base = Path::new("/base");
        let resolved = resolve_location(base, "/absolute/file.las");
        assert_eq!(resolved, Path::new("/absolute/file.las"));
    }

    #[test]
    fn resolve_location_relative_path_uses_base() {
        let base = Path::new("/base");
        let resolved = resolve_location(base, "child.las");
        assert_eq!(resolved, Path::new("/base/child.las"));
    }

    #[test]
    fn remote_location_helpers_preserve_http_and_vsi_sources() {
        assert_eq!(
            location_base("https://example.com/indexes/a.geojson"),
            "https://example.com/indexes"
        );
        assert_eq!(
            location_base("/vsicurl/https://example.com/indexes/a.geojson"),
            "/vsicurl/https://example.com/indexes"
        );
        assert_eq!(
            resolve_location_text("https://example.com/indexes", "tile.las"),
            "https://example.com/indexes/tile.las"
        );
        assert_eq!(
            resolve_location_text("/vsicurl/https://example.com/indexes", "tile.las"),
            "/vsicurl/https://example.com/indexes/tile.las"
        );
        assert_eq!(
            resolve_location_text("/tmp/indexes", "https://example.com/tile.las"),
            "https://example.com/tile.las"
        );
    }

    #[test]
    fn reader_errors_on_feature_missing_location() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp.path(),
            br#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{}}]}"#,
        )
        .unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = TindexReader::new(&options);
        let err = reader.read().err().unwrap();
        assert!(err.0.contains("location"));
    }

    #[test]
    fn reader_returns_empty_for_empty_feature_collection() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp.path(),
            br#"{"type":"FeatureCollection","features":[]}"#,
        )
        .unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().into_owned());
        let mut reader = TindexReader::new(&options);
        let views = reader.read().unwrap();
        assert!(views.is_empty());
    }

    #[test]
    fn read_point_file_errors_on_unknown_extension() {
        assert!(read_point_location("/no/such/file.unknownext", None, &Options::new()).is_err());
    }

    #[test]
    fn append_view_expands_to_union_layout() {
        let mut first_layout = PointLayout::new();
        first_layout.register(DimId::X, DimType::F64);
        let mut first = PointView::new(Rc::new(first_layout));
        let p0 = first.add_point();
        first.set_f64(p0, &DimId::X, 1.0);
        first.set_source_index(p0, 42);

        let mut second_layout = PointLayout::new();
        second_layout.register(DimId::X, DimType::F64);
        second_layout.register(DimId::Intensity, DimType::U16);
        let mut second = PointView::new(Rc::new(second_layout));
        let p1 = second.add_point();
        second.set_f64(p1, &DimId::X, 2.0);
        second.set_f64(p1, &DimId::Intensity, 9.0);

        let mut merged = None;
        append_view(&mut merged, &first, Path::new("first.las")).unwrap();
        append_view(&mut merged, &second, Path::new("second.las")).unwrap();
        let merged = merged.unwrap();

        assert_eq!(merged.len(), 2);
        assert_eq!(merged.layout().dim_count(), 2);
        assert_eq!(merged.get_f64(0, &DimId::X), 1.0);
        assert_eq!(merged.get_f64(0, &DimId::Intensity), 0.0);
        assert_eq!(merged.get_f64(1, &DimId::X), 2.0);
        assert_eq!(merged.get_f64(1, &DimId::Intensity), 9.0);
        assert_eq!(merged.source_index(0), 42);
    }

    #[test]
    fn reader_errors_on_unknown_reader_extension() {
        let temp = tempfile::tempdir().unwrap();
        let index = temp.path().join("idx.geojson");
        std::fs::write(
            &index,
            br#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"location":"file.unknownext"}}]}"#,
        )
        .unwrap();
        let mut options = Options::new();
        options.add("filename", index.display());
        let mut reader = TindexReader::new(&options);
        assert!(reader.read().is_err());
    }
}
