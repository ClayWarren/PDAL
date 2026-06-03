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
            pdal_native::gdal::Vector::create(index.to_str().unwrap(), "ESRI Shapefile").unwrap();
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
fn reads_ogr_index_with_sql() {
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
    options.add("sql", "SELECT * FROM tiles WHERE bucket = 'keep'");
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
fn polygon_filter_skips_non_intersecting_features() {
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
    options.add("polygon", "POLYGON((0 0,2 0,2 2,0 2,0 0))");
    let mut reader = TindexReader::new(&options);
    let views = reader.read().unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 3);
}

#[test]
fn ogr_polygon_filter_skips_non_intersecting_features() {
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
    options.add("lyr_name", "tiles");
    options.add("polygon", "POLYGON((0 0,2 0,2 2,0 2,0 0))");
    let mut reader = TindexReader::new(&options);

    assert_eq!(reader.read().unwrap()[0].len(), 3);
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
fn target_srs_reprojects_indexed_views() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    view.set_spatial_reference(SpatialReference::new("EPSG:4326"));
    let point = view.add_point();
    view.set_f64(point, &DimId::X, 1.0);
    view.set_f64(point, &DimId::Y, 1.0);
    view.set_f64(point, &DimId::Z, 0.0);

    reproject_to_target_srs(&mut view, "EPSG:3857").unwrap();

    assert!((view.get_f64(point, &DimId::X) - 111_319.49).abs() < 1.0);
    assert!((view.get_f64(point, &DimId::Y) - 111_325.14).abs() < 1.0);
    assert_eq!(view.spatial_reference().wkt(), "EPSG:3857");
}

#[test]
fn polygon_filter_reprojects_from_filter_srs() {
    let polygon = parse_tindex_polygon("POLYGON((0 0,2 0,2 2,0 2,0 0))", "EPSG:4326", "EPSG:3857")
        .unwrap()
        .unwrap();
    let feature = Geometry::from_wkt(
        "POLYGON((100000 100000,120000 100000,120000 120000,100000 120000,100000 100000))",
    )
    .unwrap();

    assert!(feature.intersects(&polygon).unwrap());
}

#[test]
fn geojson_features_capture_srs_column() {
    let text = r#"{
  "type": "FeatureCollection",
  "features": [
    {"type":"Feature","properties":{"location":"simple_text.ply","srs":"EPSG:4326"},"geometry":null}
  ]
}"#;

    let features = read_geojson_index_features(text, "location", "srs", None, None).unwrap();

    assert_eq!(features.len(), 1);
    assert_eq!(features[0].location, "simple_text.ply");
    assert_eq!(features[0].srs.as_deref(), Some("EPSG:4326"));
}

#[test]
fn reader_args_select_options_by_driver() {
    let args = parse_reader_args(
        r#"[
            // reader-specific options forwarded from readers.tindex
            {"type":"readers.las","count":2},
            {"type":"readers.ply","precision":3}
        ]"#,
    )
    .unwrap();

    assert_eq!(
        reader_options_for(Some("readers.las"), &args).get_u64("count", 0),
        2
    );
    assert_eq!(
        reader_options_for(Some("readers.ply"), &args).get_u64("precision", 0),
        3
    );
    assert_eq!(reader_options_for(Some("readers.text"), &args).len(), 0);
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
