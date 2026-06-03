use super::filters::*;
use super::*;
use pdal_core::point::DimId;

#[test]
fn reads_local_item_asset() {
    let temp = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data/ply/simple_text.ply");
    std::fs::copy(&source, temp.path().join("simple_text.ply")).unwrap();
    let item = temp.path().join("item.json");
    std::fs::write(
        &item,
        r#"{
  "type": "Feature",
  "assets": {
    "data": {"href": "simple_text.ply", "type": "application/octet-stream"}
  }
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", item.display());
    let mut reader = StacReader::new(&options);
    let views = reader.read().unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 3);
    assert_eq!(views[0].get_f64(0, &DimId::X), -1.0);
}

#[test]
fn follows_local_collection_item_links() {
    let temp = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data/ply/simple_text.ply");
    std::fs::copy(&source, temp.path().join("simple_text.ply")).unwrap();
    std::fs::write(
        temp.path().join("item.json"),
        r#"{
  "type": "Feature",
  "assets": {"data": {"href": "simple_text.ply"}}
}"#,
    )
    .unwrap();
    let collection = temp.path().join("collection.json");
    std::fs::write(
        &collection,
        r#"{
  "type": "Collection",
  "links": [{"rel": "item", "href": "item.json"}]
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", collection.display());
    let mut reader = StacReader::new(&options);

    assert_eq!(reader.read().unwrap()[0].len(), 3);
}

#[test]
fn honors_custom_asset_names() {
    let temp = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data/ply/simple_text.ply");
    std::fs::copy(&source, temp.path().join("simple_text.ply")).unwrap();
    let item = temp.path().join("item.json");
    std::fs::write(
        &item,
        r#"{
  "type": "Feature",
  "assets": {"pointcloud": {"href": "simple_text.ply"}}
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", item.display());
    options.add("asset_names", "pointcloud");
    let mut reader = StacReader::new(&options);

    assert_eq!(reader.read().unwrap()[0].len(), 3);
}

#[test]
fn collection_filter_accepts_matching_item() {
    let temp = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data/ply/simple_text.ply");
    std::fs::copy(&source, temp.path().join("simple_text.ply")).unwrap();
    let item = temp.path().join("item.json");
    std::fs::write(
        &item,
        r#"{
  "type": "Feature",
  "collection": "usgs-test",
  "assets": {"data": {"href": "simple_text.ply"}}
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", item.display());
    options.add("collections", "usgs-.*");
    let mut reader = StacReader::new(&options);

    assert_eq!(reader.read().unwrap()[0].len(), 3);
}

#[test]
fn collection_filter_rejects_nonmatching_item() {
    let temp = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data/ply/simple_text.ply");
    std::fs::copy(&source, temp.path().join("simple_text.ply")).unwrap();
    let item = temp.path().join("item.json");
    std::fs::write(
        &item,
        r#"{
  "type": "Feature",
  "collection": "usgs-test",
  "assets": {"data": {"href": "simple_text.ply"}}
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", item.display());
    options.add("collections", "no-match");
    let mut reader = StacReader::new(&options);

    assert!(reader.read().is_err());
}

#[test]
fn collection_filter_rejects_invalid_regex() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        temp.path(),
        br#"{"type":"Feature","collection":"x","assets":{"data":{"href":"x.las"}}}"#,
    )
    .unwrap();
    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    options.add("collections", "[");
    let mut reader = StacReader::new(&options);

    let err = reader.read().err().unwrap();
    assert!(err.0.contains("Invalid collections regular expression"));
}

#[test]
fn preview_applies_collection_and_property_filters_like_read() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        temp.path(),
        br#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "id": "accepted",
      "collection": "collection-a",
      "properties": {"quality": "good", "pc:count": 7},
      "assets": {"data": {"href": "accepted.las"}}
    },
    {
      "type": "Feature",
      "id": "wrong-collection",
      "collection": "collection-b",
      "properties": {"quality": "good", "pc:count": 11},
      "assets": {"data": {"href": "wrong-collection.las"}}
    },
    {
      "type": "Feature",
      "id": "wrong-property",
      "collection": "collection-a",
      "properties": {"quality": "bad", "pc:count": 13},
      "assets": {"data": {"href": "wrong-property.las"}}
    }
  ]
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    options.add("collections", "collection-a");
    options.add("properties", r#"{"quality":"good"}"#);
    let reader = StacReader::new(&options);

    let preview = reader.preview().unwrap();
    assert_eq!(preview.item_ids, vec!["accepted"]);
    assert_eq!(preview.point_count, 7);
}

#[test]
fn preview_reports_empty_after_collection_or_property_filtering() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        temp.path(),
        br#"{
  "type": "Feature",
  "id": "item",
  "collection": "collection-a",
  "properties": {"quality": "good", "pc:count": 5},
  "assets": {"data": {"href": "item.las"}}
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    options.add("collections", "no-match");
    let reader = StacReader::new(&options);
    let err = reader.preview().err().unwrap();
    assert!(err.0.contains("Reader list is empty after filtering"));

    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    options.add("properties", r#"{"quality":"bad"}"#);
    let reader = StacReader::new(&options);
    let err = reader.preview().err().unwrap();
    assert!(err.0.contains("Reader list is empty after filtering"));
}

#[test]
fn preview_validate_schema_rejects_malformed_feature() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        temp.path(),
        br#"{
  "type": "Feature",
  "id": "item",
  "stac_version": "1.0.0",
  "geometry": null,
  "properties": {"pc:count": 5},
  "assets": {"data": {"href": "item.las"}}
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    options.add("validate_schema", "true");
    let reader = StacReader::new(&options);

    let err = reader.preview().err().unwrap();
    assert!(err.0.contains("missing bbox"));
}

#[test]
fn read_validate_schema_rejects_malformed_feature_before_asset_read() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data/ply/simple_text.ply"),
        temp.path().join("simple_text.ply"),
    )
    .unwrap();
    let item = temp.path().join("item.json");
    std::fs::write(
        &item,
        br#"{
  "type": "Feature",
  "id": "item",
  "stac_version": "1.0.0",
  "geometry": null,
  "properties": {},
  "assets": {"data": {"href": "simple_text.ply"}}
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", item.to_string_lossy().into_owned());
    options.add("validate_schema", "true");
    let mut reader = StacReader::new(&options);

    let err = reader.read().err().unwrap();
    assert!(err.0.contains("missing bbox"));
}

#[test]
fn preview_validates_property_filter_input() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        temp.path(),
        br#"{"type":"Feature","id":"item","properties":{},"assets":{"data":{"href":"item.las"}}}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    options.add("properties", "not-json");
    let reader = StacReader::new(&options);

    let err = reader.preview().err().unwrap();
    assert!(err.0.contains("Properties argument must be valid JSON"));
}

#[test]
fn reader_errors_without_filename() {
    let mut reader = StacReader::new(&Options::new());
    let err = reader.read().err().expect("missing filename");
    assert!(err.0.contains("filename"));
}

#[test]
fn reader_errors_on_missing_file() {
    let mut options = Options::new();
    options.add("filename", "/no/such/stac.json");
    let mut reader = StacReader::new(&options);
    assert!(reader.read().is_err());
}

#[test]
fn reader_errors_on_invalid_json() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), b"{not-json").unwrap();
    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    let mut reader = StacReader::new(&options);
    assert!(reader.read().is_err());
}

#[test]
fn reader_metadata_returns_expected_name() {
    let reader = StacReader::new(&Options::new());
    assert_eq!(reader.metadata().name(), "readers.stac");
}

#[test]
fn asset_names_defaults_to_data() {
    let names = asset_names(&Options::new());
    assert_eq!(names, vec!["data".to_string()]);
}

#[test]
fn asset_names_splits_comma_separated_and_trims() {
    let mut options = Options::new();
    options.add("asset_names", "foo, bar,baz,");
    let names = asset_names(&options);
    assert_eq!(names, vec!["foo", "bar", "baz"]);
}

#[test]
fn reader_errors_on_unknown_type() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), br#"{"type":"Weird"}"#).unwrap();
    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    let mut reader = StacReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("Unsupported"));
}

#[test]
fn reader_errors_on_missing_type() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), br#"{}"#).unwrap();
    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    let mut reader = StacReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("type"));
}

#[test]
fn collects_remote_asset_locations() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
            temp.path(),
            br#"{"type":"Feature","assets":{"data":{"href":"http://example.com/x.copc.laz","type":"application/vnd.laszip+copc"}}}"#,
        )
        .unwrap();
    let mut visited = BTreeSet::new();
    let mut assets = Vec::new();
    let asset_names = [String::from("data")];
    let mut context = StacAssetContext {
        asset_names: &asset_names,
        item_filters: &[],
        date_ranges: &[],
        bounds: None,
        catalogs: &[],
        collections: &[],
        property_filters: &[],
        validate_schema: false,
        visited: &mut visited,
        assets: &mut assets,
        root: true,
    };
    collect_assets(&temp.path().to_string_lossy(), &mut context).unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].driver, "readers.copc");
    assert_eq!(assets[0].location, "http://example.com/x.copc.laz");
}

#[test]
fn copc_asset_content_type_matching_is_case_insensitive() {
    let asset = serde_json::json!({
        "href": "http://example.com/no-extension",
        "type": "Application/VND.LASZIP+COPC"
    });

    assert_eq!(
        driver_for_asset(&asset, "http://example.com/no-extension").unwrap(),
        "readers.copc"
    );
}

#[test]
fn collect_assets_applies_date_ranges() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        temp.path(),
        br#"{
  "type": "Feature",
  "id": "dated",
  "properties": {"datetime": "2022-11-15T00:00:00Z"},
  "assets": {"data": {"href": "x.laz", "type": "application/vnd.laszip"}}
}"#,
    )
    .unwrap();
    let asset_names = [String::from("data")];
    let accepted = parse_date_ranges(&[String::from(
        r#"["2022-11-01T00:00:00Z","2022-11-20T00:00:00Z"]"#,
    )])
    .unwrap();
    let rejected = parse_date_ranges(&[String::from(
        r#"["2022-12-01T00:00:00Z","2022-12-20T00:00:00Z"]"#,
    )])
    .unwrap();

    let mut visited = BTreeSet::new();
    let mut assets = Vec::new();
    let mut context = StacAssetContext {
        asset_names: &asset_names,
        item_filters: &[],
        date_ranges: &accepted,
        bounds: None,
        catalogs: &[],
        collections: &[],
        property_filters: &[],
        validate_schema: false,
        visited: &mut visited,
        assets: &mut assets,
        root: true,
    };
    collect_assets(&temp.path().to_string_lossy(), &mut context).unwrap();
    assert_eq!(assets.len(), 1);

    let mut visited = BTreeSet::new();
    let mut assets = Vec::new();
    let mut context = StacAssetContext {
        asset_names: &asset_names,
        item_filters: &[],
        date_ranges: &rejected,
        bounds: None,
        catalogs: &[],
        collections: &[],
        property_filters: &[],
        validate_schema: false,
        visited: &mut visited,
        assets: &mut assets,
        root: true,
    };
    collect_assets(&temp.path().to_string_lossy(), &mut context).unwrap();
    assert!(assets.is_empty());
}

#[test]
fn collect_assets_applies_bounds_filter() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        temp.path(),
        br#"{
  "type": "Feature",
  "id": "bounded",
  "bbox": [-79.0, 38.0, -74.0, 39.0],
  "assets": {"data": {"href": "x.laz", "type": "application/vnd.laszip"}}
}"#,
    )
    .unwrap();
    let asset_names = [String::from("data")];
    let accepted = parse_bounds("([-80,-73],[37,40])").unwrap().unwrap();
    let rejected = parse_bounds("([50,51],[-10,0])").unwrap().unwrap();

    let mut visited = BTreeSet::new();
    let mut assets = Vec::new();
    let mut context = StacAssetContext {
        asset_names: &asset_names,
        item_filters: &[],
        date_ranges: &[],
        bounds: Some(&accepted),
        catalogs: &[],
        collections: &[],
        property_filters: &[],
        validate_schema: false,
        visited: &mut visited,
        assets: &mut assets,
        root: true,
    };
    collect_assets(&temp.path().to_string_lossy(), &mut context).unwrap();
    assert_eq!(assets.len(), 1);

    let mut visited = BTreeSet::new();
    let mut assets = Vec::new();
    let mut context = StacAssetContext {
        asset_names: &asset_names,
        item_filters: &[],
        date_ranges: &[],
        bounds: Some(&rejected),
        catalogs: &[],
        collections: &[],
        property_filters: &[],
        validate_schema: false,
        visited: &mut visited,
        assets: &mut assets,
        root: true,
    };
    collect_assets(&temp.path().to_string_lossy(), &mut context).unwrap();
    assert!(assets.is_empty());
}

#[test]
fn collect_assets_follows_catalog_links() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("catalog.json"),
        br#"{
  "type": "Catalog",
  "id": "root",
  "links": [{"rel": "catalog", "href": "child.json"}]
}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("child.json"),
        br#"{
  "type": "Catalog",
  "id": "child",
  "links": [{"rel": "item", "href": "item.json"}]
}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("item.json"),
        br#"{
  "type": "Feature",
  "assets": {"data": {"href": "x.laz", "type": "application/vnd.laszip"}}
}"#,
    )
    .unwrap();

    let mut visited = BTreeSet::new();
    let mut assets = Vec::new();
    let asset_names = [String::from("data")];
    let mut context = StacAssetContext {
        asset_names: &asset_names,
        item_filters: &[],
        date_ranges: &[],
        bounds: None,
        catalogs: &[],
        collections: &[],
        property_filters: &[],
        validate_schema: false,
        visited: &mut visited,
        assets: &mut assets,
        root: true,
    };
    collect_assets(
        &temp.path().join("catalog.json").to_string_lossy(),
        &mut context,
    )
    .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].driver, "readers.las");
    assert!(assets[0].location.ends_with("x.laz"));
}

#[test]
fn catalog_filter_selects_matching_nested_catalog() {
    let temp = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data/ply/simple_text.ply");
    std::fs::copy(&source, temp.path().join("accepted.ply")).unwrap();
    std::fs::copy(&source, temp.path().join("rejected.ply")).unwrap();
    std::fs::write(
        temp.path().join("catalog.json"),
        br#"{
  "type": "Catalog",
  "id": "root",
  "links": [
    {"rel": "catalog", "href": "accepted.json"},
    {"rel": "catalog", "href": "rejected.json"}
  ]
}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("accepted.json"),
        br#"{
  "type": "Catalog",
  "id": "keep-me",
  "links": [{"rel": "item", "href": "accepted-item.json"}]
}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("rejected.json"),
        br#"{
  "type": "Catalog",
  "id": "drop-me",
  "links": [{"rel": "item", "href": "rejected-item.json"}]
}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("accepted-item.json"),
        br#"{"type":"Feature","assets":{"data":{"href":"accepted.ply"}}}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("rejected-item.json"),
        br#"{"type":"Feature","assets":{"data":{"href":"rejected.ply"}}}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add(
        "filename",
        temp.path()
            .join("catalog.json")
            .to_string_lossy()
            .into_owned(),
    );
    options.add("catalogs", "keep-.*");
    let mut reader = StacReader::new(&options);

    let views = reader.read().unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 3);
}

#[test]
fn reader_errors_on_missing_assets() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), br#"{"type":"Feature"}"#).unwrap();
    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    let mut reader = StacReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("assets"));
}

#[test]
fn reader_errors_on_asset_missing_href() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), br#"{"type":"Feature","assets":{"data":{}}}"#).unwrap();
    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    let mut reader = StacReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("href"));
}

#[test]
fn reader_errors_on_feature_collection_missing_features() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), br#"{"type":"FeatureCollection"}"#).unwrap();
    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    let mut reader = StacReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("features"));
}

#[test]
fn reader_handles_empty_feature_collection_with_links() {
    let temp = tempfile::tempdir().unwrap();
    let coll = temp.path().join("fc.json");
    std::fs::write(&coll, br#"{"type":"FeatureCollection","features":[]}"#).unwrap();
    let mut options = Options::new();
    options.add("filename", coll.display());
    let mut reader = StacReader::new(&options);
    // FeatureCollection with empty features -> Ok with no views
    let views = reader.read().unwrap();
    assert!(views.is_empty());
}

#[test]
fn ogr_bounds_filter_reads_geojson_feature_by_sql_id() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
            temp.path(),
            br#"{
  "type": "FeatureCollection",
  "features": [
    {"type":"Feature","properties":{"id":1},"geometry":{"type":"Polygon","coordinates":[[[0,0],[0,1],[1,1],[1,0],[0,0]]]}},
    {"type":"Feature","properties":{"id":2},"geometry":{"type":"Polygon","coordinates":[[[50,-10],[50,0],[51,0],[51,-10],[50,-10]]]}}
  ]
}"#,
        )
        .unwrap();
    let ogr = format!(
        r#"{{"type":"ogr","datasource":"{}","sql":"select * from ogr_boundary WHERE id = 2"}}"#,
        temp.path().display()
    );
    let bounds = parse_ogr_bounds(&ogr).unwrap().unwrap();

    assert_eq!(bounds.minx, 50.0);
    assert_eq!(bounds.maxx, 51.0);
    assert_eq!(bounds.miny, -10.0);
    assert_eq!(bounds.maxy, 0.0);

    let ogr = format!(
        r#"{{"type":"ogr","datasource":"{}","sql":"select * from ogr_boundary"}}"#,
        temp.path().display()
    );
    let bounds = parse_ogr_bounds(&ogr).unwrap().unwrap();
    assert_eq!(bounds.minx, 0.0);
    assert_eq!(bounds.maxx, 51.0);
    assert_eq!(bounds.miny, -10.0);
    assert_eq!(bounds.maxy, 1.0);
}

#[test]
fn ogr_bounds_filter_supports_multipolygon_geojson() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        temp.path(),
        br#"{
  "type": "FeatureCollection",
  "features": [
    {"type":"Feature","properties":{"id":1},"geometry":{"type":"MultiPolygon","coordinates":[
      [[[10,10],[11,10],[11,11],[10,11],[10,10]]],
      [[[-2,-3],[-1,-3],[-1,-2],[-2,-2],[-2,-3]]]
    ]}}
  ]
}"#,
    )
    .unwrap();
    let ogr = format!(
        r#"{{"type":"ogr","datasource":"{}","sql":"select * from ogr_boundary"}}"#,
        temp.path().display()
    );
    let bounds = parse_ogr_bounds(&ogr).unwrap().unwrap();

    assert_eq!(bounds.minx, -2.0);
    assert_eq!(bounds.maxx, 11.0);
    assert_eq!(bounds.miny, -3.0);
    assert_eq!(bounds.maxy, 11.0);
}

#[test]
fn ogr_bounds_filter_reads_native_datasource_by_sql_id() {
    let temp = tempfile::tempdir().unwrap();
    let datasource = temp.path().join("boundary.shp");
    {
        let vector =
            pdal_native::gdal::Vector::create(datasource.to_str().unwrap(), "ESRI Shapefile")
                .unwrap();
        let layer = vector.open_or_create_layer("boundary", "").unwrap();
        unsafe {
            pdal_native::gdal::Vector::create_string_field(layer, "id").unwrap();
            pdal_native::gdal::Vector::add_feature(
                layer,
                "POLYGON((0 0,1 0,1 1,0 1,0 0))",
                &[("id", "1")],
            )
            .unwrap();
            pdal_native::gdal::Vector::add_feature(
                layer,
                "POLYGON((50 -10,51 -10,51 0,50 0,50 -10))",
                &[("id", "2")],
            )
            .unwrap();
        }
    }

    let ogr = format!(
        r#"{{"type":"ogr","datasource":"{}","sql":"select * from boundary WHERE id = 2"}}"#,
        datasource.display()
    );
    let bounds = parse_ogr_bounds(&ogr).unwrap().unwrap();
    assert_eq!(bounds.minx, 50.0);
    assert_eq!(bounds.maxx, 51.0);
    assert_eq!(bounds.miny, -10.0);
    assert_eq!(bounds.maxy, 0.0);

    let ogr = format!(
        r#"{{"type":"ogr","datasource":"{}","sql":"select * from boundary"}}"#,
        datasource.display()
    );
    let bounds = parse_ogr_bounds(&ogr).unwrap().unwrap();
    assert_eq!(bounds.minx, 0.0);
    assert_eq!(bounds.maxx, 51.0);
    assert_eq!(bounds.miny, -10.0);
    assert_eq!(bounds.maxy, 1.0);
}

#[test]
fn ogr_bounds_filter_rejects_invalid_polygon() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
            temp.path(),
            br#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"id":3},"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,1]]]}}]}"#,
        )
        .unwrap();
    let ogr = format!(
        r#"{{"type":"ogr","datasource":"{}","sql":"select * from ogr_boundary WHERE id = 3"}}"#,
        temp.path().display()
    );

    assert!(parse_ogr_bounds(&ogr).is_err());
}

#[test]
fn stac_filter_helpers_cover_validation_and_matching_edges() {
    let feature = serde_json::json!({
        "type": "Feature",
        "id": "item-01",
        "stac_version": "1.0.0",
        "geometry": null,
        "bbox": [0.0, 0.0, 1.0, 1.0],
        "collection": "collection-a",
        "properties": {
            "datetime": "2024-01-02T03:04:05Z",
            "quality": "good",
            "count": 2
        },
        "assets": {
            "data": {"href": "points.las"},
            "thumbnail": {"href": "thumb.png"}
        }
    });
    assert!(validate_stac_object(&feature, "item.json").is_ok());
    assert!(item_has_requested_asset(
        &feature,
        &[String::from("thumbnail")]
    ));
    assert!(!item_has_requested_asset(
        &feature,
        &[String::from("missing")]
    ));

    let id_filters = compile_regexes(&[String::from("item-\\d+")], "items").unwrap();
    assert!(item_matches_id_filters(&feature, &id_filters));
    let collection_filters =
        compile_regexes(&[String::from("collection-[ab]")], "collections").unwrap();
    assert!(collection_matches(&feature, &collection_filters));
    assert!(!collection_matches(
        &serde_json::json!({}),
        &collection_filters
    ));
    assert!(catalog_matches(
        &serde_json::json!({"id": "root-catalog"}),
        &compile_regexes(&[String::from("root-.*")], "catalogs").unwrap()
    ));
    assert!(!catalog_matches(&serde_json::json!({}), &id_filters));

    let property_filters =
        parse_property_filters(r#"{"quality":["bad","good"],"count":2}"#).unwrap();
    assert!(item_matches_property_filters(&feature, &property_filters).unwrap());
    let rejected = parse_property_filters(r#"{"quality":"bad"}"#).unwrap();
    assert!(!item_matches_property_filters(&feature, &rejected).unwrap());
    assert!(item_matches_property_filters(&serde_json::json!({}), &property_filters).is_err());
    assert!(parse_property_filters("").unwrap().is_empty());
    assert!(parse_property_filters("[]").is_err());
}

#[test]
fn stac_reader_args_dates_bounds_and_paths_cover_edge_cases() {
    let args = parse_reader_args(
        r#"[
            // reader-specific options forwarded from readers.stac
            {"type":"readers.las","nosrs":true,"count":2,"bounds":{"x":[0,1]}}
        ]"#,
    )
    .unwrap();
    assert_eq!(args.len(), 1);
    assert_eq!(args[0].driver, "readers.las");
    assert_eq!(args[0].options.get_str("nosrs", ""), "true");
    assert_eq!(args[0].options.get_str("count", ""), "2");
    assert!(args[0].options.get_str("bounds", "").contains("\"x\""));
    assert!(parse_reader_args("{}").is_err());
    assert!(parse_reader_args(r#"[{"count":2}]"#).is_err());

    assert_eq!(normalize_stac_time("2024-1-2T3:4:5Z"), "2024-1-2T03:04:05Z");
    assert_eq!(normalize_stac_time("2024-01-02"), "2024-01-02");
    let ranges = parse_date_ranges(&[String::from(
        r#"["2024-01-01T00:00:00Z","2024-01-31T00:00:00Z"]"#,
    )])
    .unwrap();
    assert!(item_matches_dates(
        &serde_json::json!({"properties":{"start_datetime":"2024-01-15T00:00:00Z","end_datetime":"2024-01-16T00:00:00Z"}}),
        &ranges
    ));
    assert!(!item_matches_dates(
        &serde_json::json!({"properties":{}}),
        &ranges
    ));
    assert!(parse_date_ranges(&[String::from(r#"["2024-01-01T00:00:00Z"]"#)]).is_err());
    assert!(parse_stac_time("not-a-date").is_err());

    let bounds = parse_bounds("([5,1],[9,3])/EPSG:4326").unwrap().unwrap();
    assert_eq!(
        (bounds.minx, bounds.maxx, bounds.miny, bounds.maxy),
        (1.0, 5.0, 3.0, 9.0)
    );
    assert!(parse_bounds("").unwrap().is_none());
    assert!(parse_bounds("([1],[2])").is_err());
    assert!(item_matches_bounds(
        &serde_json::json!({"bbox":[0.0, 0.0, 2.0, 2.0]}),
        &Bounds2D {
            minx: 1.0,
            maxx: 3.0,
            miny: 1.0,
            maxy: 3.0
        }
    ));
    assert!(item_matches_bounds(
        &serde_json::json!({"bbox":[0.0, 0.0, -100.0, 2.0, 2.0, 100.0]}),
        &Bounds2D {
            minx: 1.0,
            maxx: 3.0,
            miny: 1.0,
            maxy: 3.0
        }
    ));
    assert!(!item_matches_bounds(
        &serde_json::json!({"bbox":[0.0]}),
        &bounds
    ));

    assert_eq!(
        remote_base("https://example.com/a/b/item.json"),
        "https://example.com/a/b"
    );
    assert_eq!(remote_base("item.json"), "");
    assert_eq!(
        resolve_stac_link("https://example.com/root", "./item.json"),
        "https://example.com/root/item.json"
    );
    assert_eq!(
        resolve_stac_link("/tmp/base", "/absolute/item.json"),
        "/absolute/item.json"
    );
    assert_eq!(
        normalize_local_location("/vsicurl/https://example.com/item.json"),
        "/vsicurl/https://example.com/item.json"
    );
}

#[test]
fn geojson_geometry_bounds_reports_null_and_invalid_shapes() {
    assert!(geojson_geometry_bounds(&Value::Null).unwrap().is_none());
    let point_err = geojson_geometry_bounds(&serde_json::json!({"type":"Point"}));
    assert!(point_err.is_err());
    assert!(point_err.err().unwrap().0.contains("Unsupported"));
    assert!(geojson_geometry_bounds(&serde_json::json!({"type":"Polygon"})).is_err());
    assert!(geojson_geometry_bounds(
        &serde_json::json!({"type":"Polygon","coordinates":[[[0,0],[0,1],[1,1],[1,"bad"],[0,0]]]})
    )
    .is_err());

    let bounds = geojson_geometry_bounds(
        &serde_json::json!({"type":"Polygon","coordinates":[[[0,0],[2,0],[2,3],[0,3],[0,0]]]}),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        (bounds.minx, bounds.maxx, bounds.miny, bounds.maxy),
        (0.0, 2.0, 0.0, 3.0)
    );
    assert_eq!(
        ogr_sql_id_filter("select * from x WHERE id = -42"),
        Some(-42)
    );
    assert_eq!(ogr_sql_id_filter("select * from x"), None);
}

#[test]
fn is_remote_detects_url_schemes() {
    assert!(is_remote("http://example.com/x"));
    assert!(is_remote("https://example.com/x"));
    assert!(!is_remote("/local/path.las"));
}
