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
fn reads_cpp_autzen_stac_fixture() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data/stac/autzen_trim.json");

    let mut options = Options::new();
    options.add("filename", source.display());
    options.add("asset_names", "data");
    let mut reader = StacReader::new(&options);
    let views = reader.read().unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 110000);
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
fn read_accepts_cpp_filter_synonyms() {
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
  "id": "keep",
  "links": [{"rel": "item", "href": "accepted-item.json"}]
}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("rejected.json"),
        br#"{
  "type": "Catalog",
  "id": "drop",
  "links": [{"rel": "item", "href": "rejected-item.json"}]
}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("accepted-item.json"),
        br#"{
  "type": "Feature",
  "id": "accepted-item",
  "collection": "accepted-collection",
  "assets": {"data": {"href": "accepted.ply"}}
}"#,
    )
    .unwrap();
    std::fs::write(
        temp.path().join("rejected-item.json"),
        br#"{
  "type": "Feature",
  "id": "rejected-item",
  "collection": "rejected-collection",
  "assets": {"data": {"href": "rejected.ply"}}
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", temp.path().join("catalog.json").display());
    options.add("catalog_ids", "keep");
    options.add("item_ids", "accepted-.*");
    options.add("collection_ids", "accepted-collection");
    let mut reader = StacReader::new(&options);

    let views = reader.read().unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 3);
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
fn preview_accepts_cpp_filter_synonyms() {
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
      "properties": {"pc:count": 7},
      "assets": {"data": {"href": "accepted.las"}}
    },
    {
      "type": "Feature",
      "id": "rejected",
      "collection": "collection-b",
      "properties": {"pc:count": 11},
      "assets": {"data": {"href": "rejected.las"}}
    }
  ]
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", temp.path().to_string_lossy().into_owned());
    options.add("item_ids", "accepted");
    options.add("collection_ids", "collection-a");
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
fn read_validate_schema_rejects_malformed_catalog_link_before_traversal() {
    let temp = tempfile::tempdir().unwrap();
    let catalog = temp.path().join("catalog.json");
    std::fs::write(
        &catalog,
        br#"{
  "type": "Catalog",
  "id": "root",
  "stac_version": "1.0.0",
  "links": [{"rel": "item"}]
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", catalog.to_string_lossy().into_owned());
    options.add("validate_schema", "true");
    let mut reader = StacReader::new(&options);

    let err = reader.read().err().unwrap();
    assert!(err.0.contains("missing string field 'href'"));
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
        headers: &[],
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
        headers: &[],
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
        headers: &[],
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
        headers: &[],
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
        headers: &[],
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
        headers: &[],
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
fn preview_normalizes_visited_local_locations() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("catalog.json");
    let child = temp.path().join("child.json");
    std::fs::write(
        &root,
        br#"{
  "type": "Catalog",
  "id": "root",
  "links": [
    {"rel": "item", "href": "item.json"},
    {"rel": "child", "href": "child.json"}
  ]
}"#,
    )
    .unwrap();
    std::fs::write(
        &child,
        format!(
            r#"{{
  "type": "Catalog",
  "id": "child",
  "links": [{{"rel": "catalog", "href": "{}"}}]
}}"#,
            root.display()
        ),
    )
    .unwrap();
    std::fs::write(
        temp.path().join("item.json"),
        br#"{
  "type": "Feature",
  "id": "one",
  "properties": {"pc:count": 7},
  "assets": {"data": {"href": "x.laz", "type": "application/vnd.laszip"}}
}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", temp.path().join("./catalog.json").display());
    let reader = StacReader::new(&options);
    let preview = reader.preview().unwrap();

    assert_eq!(preview.item_ids, vec!["one"]);
    assert_eq!(preview.point_count, 7);
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

#[path = "ogr_tests.rs"]
mod ogr_tests;
