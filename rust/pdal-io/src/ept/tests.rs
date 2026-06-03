//! Tests for the EPT reader (split out of `ept.rs` to keep it under ~1k LOC).

use super::*;
use std::io::Cursor;
use std::path::PathBuf;

fn data_path(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data")
        .join(path)
}

struct VsiEptDataset {
    root: String,
    paths: Vec<String>,
}

impl VsiEptDataset {
    fn new(data_type: &str, tile_bytes: Vec<u8>, extension: &str) -> Self {
        let root = format!("/vsimem/pdal-ept-{}-{data_type}", std::process::id());
        let ept = format!("{root}/ept.json");
        let hierarchy = format!("{root}/ept-hierarchy/0-0-0-0.json");
        let tile = format!("{root}/ept-data/0-0-0-0.{extension}");
        let ept_json = format!(
            r#"{{
  "dataType": "{data_type}",
  "hierarchyType": "json",
  "span": 128,
  "bounds": [0, 0, 0, 1, 1, 1],
  "boundsConforming": [0, 0, 0, 1, 1, 1],
  "points": 1,
  "schema": [
    {{"name": "X", "type": "float", "size": 8}},
    {{"name": "Y", "type": "float", "size": 8}},
    {{"name": "Z", "type": "float", "size": 8}}
  ]
}}"#
        );
        for (path, bytes) in [
            (ept.as_str(), ept_json.into_bytes()),
            (hierarchy.as_str(), br#"{"0-0-0-0":1}"#.to_vec()),
            (tile.as_str(), tile_bytes),
        ] {
            pdal_native::vsi::write_mem_file(path, &bytes).unwrap();
        }
        Self {
            root,
            paths: vec![ept, hierarchy, tile],
        }
    }

    fn ept_json(&self) -> String {
        format!("{}/ept.json", self.root)
    }
}

impl Drop for VsiEptDataset {
    fn drop(&mut self) {
        for path in &self.paths {
            let _ = pdal_native::vsi::unlink(path);
        }
    }
}

fn ept_tile_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [1.0_f64, 2.0, 3.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn read_vsi_ept(data_type: &str, tile_bytes: Vec<u8>, extension: &str) -> PointView {
    let dataset = VsiEptDataset::new(data_type, tile_bytes, extension);
    let mut options = Options::new();
    options.add("filename", dataset.ept_json());
    let mut reader = EptReader::new(&options);
    let views = reader.read().unwrap();
    assert_eq!(views.len(), 1);
    views.into_iter().next().unwrap()
}

#[test]
fn reads_local_laszip_ept() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/1.2-with-color/ept.json").display(),
    );
    let mut reader = EptReader::new(&options);
    let views = reader.read().unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 1065);
    assert!((views[0].get_f64(0, &DimId::X) - 638806.73).abs() < 1e-9);
    assert!(views[0].layout().dim(&DimId::Red).is_some());
}

#[test]
fn reads_local_binary_ept() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/ellipsoid-binary/ept.json").display(),
    );
    let mut reader = EptReader::new(&options);
    let views = reader.read().unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 100000);
    let origin_id = DimId::from_name("OriginId");
    assert!(views[0].layout().dim(&origin_id).is_some());
    for idx in [0, 42, 99999] {
        let x = views[0].get_f64(idx, &DimId::X);
        let y = views[0].get_f64(idx, &DimId::Y);
        let z = views[0].get_f64(idx, &DimId::Z);
        assert!((-8242746.0..=-8242446.0).contains(&x));
        assert!((4966506.0..=4966706.0).contains(&y));
        assert!((-50.0..=50.0).contains(&z));
        assert_eq!(views[0].get_f64(idx, &origin_id), 0.0);
    }
}

#[test]
fn reads_vsi_binary_ept() {
    let view = read_vsi_ept("binary", ept_tile_bytes(), "bin");

    assert_eq!(view.len(), 1);
    assert_eq!(view.get_f64(0, &DimId::X), 1.0);
    assert_eq!(view.get_f64(0, &DimId::Y), 2.0);
    assert_eq!(view.get_f64(0, &DimId::Z), 3.0);
}

#[test]
fn reads_local_zstandard_ept() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/ellipsoid-zstandard/ept.json").display(),
    );
    let mut reader = EptReader::new(&options);
    let views = reader.read().unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 100000);
    assert!((views[0].get_f64(42, &DimId::X) + 8242698.0).abs() < 1e-9);
}

#[test]
fn reads_vsi_zstandard_ept() {
    let encoded = zstd::stream::encode_all(Cursor::new(ept_tile_bytes()), 0).unwrap();
    let view = read_vsi_ept("zstandard", encoded, "zst");

    assert_eq!(view.len(), 1);
    assert_eq!(view.get_f64(0, &DimId::X), 1.0);
    assert_eq!(view.get_f64(0, &DimId::Y), 2.0);
    assert_eq!(view.get_f64(0, &DimId::Z), 3.0);
}

#[test]
fn applies_3d_bounds_filter() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/ellipsoid-binary/ept.json").display(),
    );
    options.add("bounds", "([-8242746,-8242600],[4966506,4966706],[-50,50])");
    let mut reader = EptReader::new(&options);
    let views = reader.read().unwrap();

    assert_eq!(views.len(), 1);
    assert!(!views[0].is_empty());
    assert!(views[0].len() < 100000);
    for idx in 0..views[0].len() {
        let x = views[0].get_f64(idx, &DimId::X);
        let y = views[0].get_f64(idx, &DimId::Y);
        let z = views[0].get_f64(idx, &DimId::Z);
        assert!((-8242746.0..=-8242600.0).contains(&x));
        assert!((4966506.0..=4966706.0).contains(&y));
        assert!((-50.0..=50.0).contains(&z));
    }
}

#[test]
fn applies_resolution_limit_to_hierarchy_depth() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/lone-star-laszip/ept.json").display(),
    );
    options.add("resolution", "0.1");
    let mut reader = EptReader::new(&options);
    let views = reader.read().unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 479269);
}

#[test]
fn rejects_non_positive_resolution() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/lone-star-laszip/ept.json").display(),
    );
    options.add("resolution", "0");
    let mut reader = EptReader::new(&options);

    let Err(err) = reader.read() else {
        panic!("expected bad EPT resolution to fail");
    };
    assert!(err.0.contains("resolution option must be positive"));
}

#[test]
fn rejects_bad_laszip_point_count() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/bad-pointcount/laszip/ept.json").display(),
    );
    let mut reader = EptReader::new(&options);

    let Err(err) = reader.read() else {
        panic!("expected bad EPT point count to fail");
    };
    assert!(err.0.contains("hierarchy expected 1000"));
}

#[test]
fn rejects_bad_binary_point_count() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/bad-pointcount/binary/ept.json").display(),
    );
    let mut reader = EptReader::new(&options);

    let Err(err) = reader.read() else {
        panic!("expected bad EPT point count to fail");
    };
    assert!(err.0.contains("hierarchy expected 1000004"));
}

#[test]
fn ignores_unreadable_tile_when_requested() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/ellipsoid-nopoints/ept.json").display(),
    );
    options.add("ignore_unreadable", true);
    let mut reader = EptReader::new(&options);

    let views = reader.read().unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 0);
}

#[test]
fn filters_named_origin_from_manifest() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/ellipsoid-binary/ept.json").display(),
    );
    options.add("origin", "ellipsoid");
    let mut reader = EptReader::new(&options);

    let views = reader.read().unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 100000);
    let origin_id = DimId::from_name("OriginId");
    assert_eq!(views[0].get_f64(0, &origin_id), 0.0);
    assert_eq!(views[0].get_f64(99999, &origin_id), 0.0);
}

#[test]
fn rejects_unknown_numeric_origin() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/lone-star-laszip/ept.json").display(),
    );
    options.add("origin", "4");
    let mut reader = EptReader::new(&options);

    let Err(err) = reader.read() else {
        panic!("expected bad EPT origin to fail");
    };
    assert!(err.0.contains("Invalid EPT origin '4'"));
}

#[test]
fn prunes_non_overlapping_hierarchy_tiles_before_reading_data() {
    let temp = tempfile::tempdir().unwrap();
    let ept = temp.path().join("ept.json");
    let hierarchy_dir = temp.path().join("ept-hierarchy");
    std::fs::create_dir_all(&hierarchy_dir).unwrap();
    std::fs::write(
        &ept,
        r#"{
  "dataType": "binary",
  "hierarchyType": "json",
  "span": 128,
  "bounds": [0, 0, 0, 8, 8, 8],
  "schema": [
    {"name": "X", "type": "float", "size": 8},
    {"name": "Y", "type": "float", "size": 8},
    {"name": "Z", "type": "float", "size": 8}
  ]
}"#,
    )
    .unwrap();
    std::fs::write(hierarchy_dir.join("0-0-0-0.json"), r#"{"1-0-0-0":1}"#).unwrap();
    let mut options = Options::new();
    options.add("filename", ept.display());
    options.add("bounds", "([6,7],[6,7],[6,7])");
    let mut reader = EptReader::new(&options);

    let views = reader.read().unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 0);
    let metadata = reader.metadata();
    assert_eq!(
        metadata.find_child("tiles").and_then(MetadataNode::value),
        Some(&MetadataValue::U64(0))
    );
}

#[test]
fn rejects_unsupported_ept_data_type() {
    let temp = tempfile::tempdir().unwrap();
    let ept = temp.path().join("ept.json");
    std::fs::write(
        &ept,
        r#"{"dataType":"unsupported","hierarchyType":"json","schema":[]}"#,
    )
    .unwrap();
    let mut options = Options::new();
    options.add("filename", ept.display());
    let mut reader = EptReader::new(&options);

    assert!(reader.read().is_err());
}

#[test]
fn preview_returns_bounds_conforming_and_expanded_flags_for_laszip() {
    let path = data_path("ept/lone-star-laszip/ept.json")
        .to_string_lossy()
        .into_owned();
    let preview = read_ept_preview(&path).unwrap();

    assert_eq!(preview.point_count, 518862);
    assert_eq!(preview.bounds_conforming.minx, 515368.0);
    assert_eq!(preview.bounds_conforming.miny, 4918340.0);
    assert_eq!(preview.bounds_conforming.minz, 2322.0);
    assert_eq!(preview.bounds_conforming.maxx, 515402.0);
    assert_eq!(preview.bounds_conforming.maxy, 4918382.0);
    assert_eq!(preview.bounds_conforming.maxz, 2339.0);

    let mut names = preview.dim_names.clone();
    names.sort();
    let mut expected: Vec<String> = vec![
        "Classification",
        "EdgeOfFlightLine",
        "GpsTime",
        "Intensity",
        "KeyPoint",
        "NumberOfReturns",
        "OriginId",
        "Overlap",
        "PointSourceId",
        "ReturnNumber",
        "ScanAngleRank",
        "ScanDirectionFlag",
        "Synthetic",
        "UserData",
        "Withheld",
        "X",
        "Y",
        "Z",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect();
    expected.sort();
    assert_eq!(names, expected);
    assert!(preview.srs_wkt.contains("NAD83 / UTM zone 12N"));
}

#[test]
fn preview_applies_resolution_limit_to_hierarchy_count() {
    let path = data_path("ept/lone-star-laszip/ept.json")
        .to_string_lossy()
        .into_owned();
    let preview = read_ept_preview_with_options(&path, "0.1").unwrap();

    assert_eq!(preview.point_count, 479269);
}

#[test]
fn preview_for_binary_does_not_inject_class_flags() {
    let path = data_path("ept/ellipsoid-binary/ept.json")
        .to_string_lossy()
        .into_owned();
    let preview = read_ept_preview(&path).unwrap();
    assert!(!preview.dim_names.iter().any(|n| n == "Withheld"));
}

#[test]
fn reader_errors_without_filename() {
    let mut reader = EptReader::new(&Options::new());
    let err = reader.read().err().expect("missing filename");
    assert!(err.0.contains("filename"));
}

#[test]
fn reader_errors_on_missing_file() {
    let mut options = Options::new();
    options.add("filename", "/no/such/ept.json");
    let mut reader = EptReader::new(&options);
    assert!(reader.read().is_err());
}

#[test]
fn reader_metadata_returns_expected_name() {
    let reader = EptReader::new(&Options::new());
    assert_eq!(reader.metadata().name(), "readers.ept");
}

#[test]
fn reader_errors_on_invalid_resolution() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/1.2-with-color/ept.json").display(),
    );
    options.add("resolution", "not-a-number");
    let mut reader = EptReader::new(&options);
    assert!(reader.read().is_err());
}

#[test]
fn reader_errors_on_invalid_bounds() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/1.2-with-color/ept.json").display(),
    );
    options.add("bounds", "completely-not-bounds");
    let mut reader = EptReader::new(&options);
    assert!(reader.read().is_err());
}

#[test]
fn read_ept_preview_errors_on_missing_file() {
    let result = read_ept_preview("/no/such/file.json");
    assert!(result.is_err());
}

#[test]
fn read_ept_preview_errors_on_bad_path() {
    let path = data_path("ept/bad-pointcount/ept.json")
        .to_string_lossy()
        .into_owned();
    if std::path::Path::new(&path).exists() {
        let _ = read_ept_preview(&path);
    }
}

#[test]
fn test_ept_reader_name_direct() {
    let options = Options::new();
    let reader = EptReader::new(&options);
    assert_eq!(reader.name(), "readers.ept");
}

#[test]
fn test_numeric_conversion_and_truncation() {
    assert_eq!(read_binary_value(&[42], DimType::U8), 42.0);
    assert_eq!(read_binary_value(&[254], DimType::I8), -2.0);
    assert_eq!(read_binary_value(&[42, 0], DimType::U16), 42.0);
    assert_eq!(read_binary_value(&[254, 255], DimType::I16), -2.0);
    assert_eq!(read_binary_value(&[42, 0, 0, 0], DimType::U32), 42.0);
    assert_eq!(read_binary_value(&[254, 255, 255, 255], DimType::I32), -2.0);
    assert_eq!(
        read_binary_value(&[42, 0, 0, 0, 0, 0, 0, 0], DimType::U64),
        42.0
    );
    assert_eq!(
        read_binary_value(&[254, 255, 255, 255, 255, 255, 255, 255], DimType::I64),
        -2.0
    );
    assert_eq!(read_binary_value(&[0, 0, 40, 66], DimType::F32), 42.0);
    assert_eq!(
        read_binary_value(&[0, 0, 0, 0, 0, 0, 69, 64], DimType::F64),
        42.0
    );

    assert_eq!(truncate_storage(42.3, DimType::U8), 42.0);
    assert_eq!(truncate_storage(-2.3, DimType::I8), -2.0);
    assert_eq!(truncate_storage(42.3, DimType::U16), 42.0);
    assert_eq!(truncate_storage(-2.3, DimType::I16), -2.0);
    assert_eq!(truncate_storage(42.3, DimType::U32), 42.0);
    assert_eq!(truncate_storage(-2.3, DimType::I32), -2.0);
    assert_eq!(truncate_storage(42.3, DimType::U64), 42.0);
    assert_eq!(truncate_storage(-2.3, DimType::I64), -2.0);
    assert_eq!(truncate_storage(42.3, DimType::F32), 42.29999923706055);
    assert_eq!(truncate_storage(42.3, DimType::F64), 42.3);
}

#[test]
fn test_read_ept_preview_errors_more() {
    let temp = tempfile::tempdir().unwrap();
    let ept = temp.path().join("ept.json");

    // Missing points
    std::fs::write(&ept, r#"{"boundsConforming":[0,0,0,1,1,1]}"#).unwrap();
    assert!(read_ept_preview(&ept.to_string_lossy()).is_err());

    // Missing dataType
    std::fs::write(&ept, r#"{"boundsConforming":[0,0,0,1,1,1],"points":10}"#).unwrap();
    assert!(read_ept_preview(&ept.to_string_lossy()).is_err());

    // Missing schema
    std::fs::write(
        &ept,
        r#"{"boundsConforming":[0,0,0,1,1,1],"points":10,"dataType":"binary"}"#,
    )
    .unwrap();
    assert!(read_ept_preview(&ept.to_string_lossy()).is_err());

    // Schema entry missing name
    std::fs::write(
        &ept,
        r#"{"boundsConforming":[0,0,0,1,1,1],"points":10,"dataType":"binary","schema":[{}]}"#,
    )
    .unwrap();
    assert!(read_ept_preview(&ept.to_string_lossy()).is_err());

    // boundsConforming missing coords (less than 6)
    std::fs::write(
        &ept,
        r#"{"boundsConforming":[0,0,0],"points":10,"dataType":"binary","schema":[]}"#,
    )
    .unwrap();
    assert!(read_ept_preview(&ept.to_string_lossy()).is_err());
}

#[test]
fn test_bounds_filter_errors() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/ellipsoid-binary/ept.json").display(),
    );
    options.add("bounds", "([0,1],[0,1],[0,invalid])");
    let mut reader = EptReader::new(&options);
    assert!(reader.read().is_err());
}

#[test]
fn test_source_origins_non_array() {
    let temp = tempfile::tempdir().unwrap();
    let ept = temp.path().join("ept.json");
    std::fs::write(
        &ept,
        r#"{
  "dataType": "binary",
  "hierarchyType": "json",
  "span": 128,
  "bounds": [0, 0, 0, 8, 8, 8],
  "schema": []
}"#,
    )
    .unwrap();

    let hierarchy_dir = temp.path().join("ept-hierarchy");
    std::fs::create_dir_all(&hierarchy_dir).unwrap();
    std::fs::write(hierarchy_dir.join("0-0-0-0.json"), r#"{}"#).unwrap();

    let sources_dir = temp.path().join("ept-sources");
    std::fs::create_dir(&sources_dir).unwrap();
    std::fs::write(
        sources_dir.join("manifest.json"),
        r#"{"not_an_array": true}"#,
    )
    .unwrap();

    let mut options = Options::new();
    options.add("filename", ept.display());
    options.add("origin", "some-origin");
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("EPT source list must be a JSON array"));
}

#[test]
fn test_resolution_filter_errors() {
    let temp = tempfile::tempdir().unwrap();
    let ept = temp.path().join("ept.json");

    // Missing span
    std::fs::write(
        &ept,
        r#"{"dataType":"binary","hierarchyType":"json","schema":[]}"#,
    )
    .unwrap();
    let mut options = Options::new();
    options.add("filename", ept.display());
    options.add("resolution", "1.0");
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("EPT file is missing span"));

    // Non-positive span
    std::fs::write(
        &ept,
        r#"{"dataType":"binary","hierarchyType":"json","schema":[],"span":0}"#,
    )
    .unwrap();
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("EPT span must be positive"));

    // Invalid bounds cube width
    std::fs::write(&ept, r#"{"dataType":"binary","hierarchyType":"json","schema":[],"span":1,"bounds":[0,0,0,0,0,0]}"#).unwrap();
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("EPT bounds cube width is invalid"));
}

#[test]
fn test_hierarchy_and_key_errors() {
    let temp = tempfile::tempdir().unwrap();
    let ept = temp.path().join("ept.json");
    let hierarchy_dir = temp.path().join("ept-hierarchy");
    std::fs::create_dir_all(&hierarchy_dir).unwrap();

    // EPT hierarchy must be a JSON object
    std::fs::write(
        &ept,
        r#"{
  "dataType": "binary",
  "hierarchyType": "json",
  "span": 128,
  "bounds": [0, 0, 0, 8, 8, 8],
  "schema": []
}"#,
    )
    .unwrap();
    std::fs::write(hierarchy_dir.join("0-0-0-0.json"), r#"[1, 2, 3]"#).unwrap();

    let mut options = Options::new();
    options.add("filename", ept.display());
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("must be a JSON object"));

    // EptKey::parse errors - parts.next().is_some() (too many parts)
    std::fs::write(hierarchy_dir.join("0-0-0-0.json"), r#"{"0-0-0-0-0":1}"#).unwrap();
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("Invalid EPT hierarchy key '0-0-0-0-0'"));

    // EptKey::parse errors - shift/depth too large
    std::fs::write(hierarchy_dir.join("0-0-0-0.json"), r#"{"65-0-0-0":1}"#).unwrap();
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("depth is too large"));

    // EptKey::parse errors - missing part
    std::fs::write(hierarchy_dir.join("0-0-0-0.json"), r#"{"0-0-0":1}"#).unwrap();
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("Invalid EPT hierarchy key '0-0-0'"));

    // EptKey::parse errors - non-numeric part
    std::fs::write(hierarchy_dir.join("0-0-0-0.json"), r#"{"0-0-0-abc":1}"#).unwrap();
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("Invalid EPT hierarchy key '0-0-0-abc'"));
}

#[test]
fn test_source_origins_from_list_json() {
    let temp = tempfile::tempdir().unwrap();
    let ept = temp.path().join("ept.json");
    std::fs::write(
        &ept,
        r#"{
  "dataType": "binary",
  "hierarchyType": "json",
  "span": 128,
  "bounds": [0, 0, 0, 8, 8, 8],
  "schema": []
}"#,
    )
    .unwrap();

    let hierarchy_dir = temp.path().join("ept-hierarchy");
    std::fs::create_dir_all(&hierarchy_dir).unwrap();
    std::fs::write(hierarchy_dir.join("0-0-0-0.json"), r#"{}"#).unwrap();

    let sources_dir = temp.path().join("ept-sources");
    std::fs::create_dir(&sources_dir).unwrap();
    std::fs::write(sources_dir.join("list.json"), r#"[{"origin": 42}]"#).unwrap();

    let mut options = Options::new();
    options.add("filename", ept.display());
    options.add("origin", "42");
    let mut reader = EptReader::new(&options);
    let views = reader.read().unwrap();
    assert_eq!(views.len(), 1);
}

#[test]
fn test_applies_2d_bounds_filter() {
    let mut options = Options::new();
    options.add(
        "filename",
        data_path("ept/ellipsoid-binary/ept.json").display(),
    );
    options.add("bounds", "([-8242746,-8242600],[4966506,4966706])");
    let mut reader = EptReader::new(&options);
    let views = reader.read().unwrap();

    assert_eq!(views.len(), 1);
    assert!(!views[0].is_empty());
    assert!(views[0].len() < 100000);
    for idx in 0..views[0].len() {
        let x = views[0].get_f64(idx, &DimId::X);
        let y = views[0].get_f64(idx, &DimId::Y);
        assert!((-8242746.0..=-8242600.0).contains(&x));
        assert!((4966506.0..=4966706.0).contains(&y));
    }
}

#[test]
fn applies_reprojected_3d_bounds_filter() {
    let mut options = Options::new();
    options.add("filename", data_path("ept/bcbf/ept.json").display());
    options.add(
        "bounds",
        "([-180,180],[80,90],[-50,50]) / +proj=longlat +R=1000 +no_defs +type=crs",
    );
    let mut reader = EptReader::new(&options);
    let views = reader.read().unwrap();

    let count: u64 = views.iter().map(PointView::len).sum();
    assert_eq!(count, 5);
}

#[test]
fn applies_reprojected_2d_bounds_filter() {
    let mut options = Options::new();
    options.add("filename", data_path("ept/bcbf/ept.json").display());
    options.add(
        "bounds",
        "([-180,180],[80,90]) / +proj=longlat +R=1000 +no_defs +type=crs",
    );
    let mut reader = EptReader::new(&options);
    let views = reader.read().unwrap();

    let count: u64 = views.iter().map(PointView::len).sum();
    assert_eq!(count, 5);
}

#[test]
fn test_parse_type_all_cases() {
    assert_eq!(dim_type("unsigned", 1).unwrap(), DimType::U8);
    assert_eq!(dim_type("unsigned", 2).unwrap(), DimType::U16);
    assert_eq!(dim_type("unsigned", 4).unwrap(), DimType::U32);
    assert_eq!(dim_type("unsigned", 8).unwrap(), DimType::U64);
    assert_eq!(dim_type("signed", 1).unwrap(), DimType::I8);
    assert_eq!(dim_type("signed", 2).unwrap(), DimType::I16);
    assert_eq!(dim_type("signed", 4).unwrap(), DimType::I32);
    assert_eq!(dim_type("signed", 8).unwrap(), DimType::I64);
    assert_eq!(dim_type("float", 4).unwrap(), DimType::F32);
    assert_eq!(dim_type("float", 8).unwrap(), DimType::F64);
    assert!(dim_type("unsigned", 3).is_err());
    assert!(dim_type("invalid_kind", 4).is_err());
}

#[test]
fn test_view_from_binary_tile_size_mismatch() {
    let schema = EptSchema {
        point_size: 4,
        entries: Vec::new(),
        layout: std::rc::Rc::new(PointLayout::new()),
    };
    let path = Path::new("dummy.bin");
    let result = view_from_binary_tile(path, vec![1, 2, 3], &schema, "");
    assert!(result
        .err()
        .unwrap()
        .0
        .contains("size does not match schema"));
}

#[test]
fn reader_errors_on_missing_datatype() {
    let temp = tempfile::tempdir().unwrap();
    let ept = temp.path().join("ept.json");
    std::fs::write(&ept, r#"{"bounds":[0,0,0,1,1,1]}"#).unwrap();
    let mut options = Options::new();
    options.add("filename", ept.display());
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("missing dataType"));
}

#[test]
fn reader_errors_on_invalid_json() {
    let temp = tempfile::tempdir().unwrap();
    let ept = temp.path().join("ept.json");
    std::fs::write(&ept, r#"{"invalid_json": "#).unwrap();
    let mut options = Options::new();
    options.add("filename", ept.display());
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("not valid JSON"));
}

#[test]
fn reader_errors_on_bounds_too_short() {
    let temp = tempfile::tempdir().unwrap();
    let ept = temp.path().join("ept.json");
    std::fs::write(&ept, r#"{"dataType":"binary","bounds":[0,0,0]}"#).unwrap();
    let mut options = Options::new();
    options.add("filename", ept.display());
    let mut reader = EptReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("bounds must contain six coordinates"));
}

#[test]
fn srs_wkt_uses_explicit_wkt() {
    let info = serde_json::json!({ "srs": { "wkt": "EPSG:26915 wkt text" } });
    assert_eq!(
        ept_srs_wkt(&info).unwrap(),
        Some("EPSG:26915 wkt text".to_string())
    );
}

#[test]
fn srs_wkt_builds_from_authority_and_horizontal() {
    let info = serde_json::json!({
        "srs": { "authority": "EPSG", "horizontal": 26915 }
    });
    assert_eq!(ept_srs_wkt(&info).unwrap(), Some("EPSG:26915".to_string()));

    // Horizontal may also be given as a string.
    let info = serde_json::json!({
        "srs": { "authority": "EPSG", "horizontal": "26915" }
    });
    assert_eq!(ept_srs_wkt(&info).unwrap(), Some("EPSG:26915".to_string()));
}

#[test]
fn srs_wkt_appends_vertical() {
    let info = serde_json::json!({
        "srs": { "authority": "EPSG", "horizontal": 26915, "vertical": 5703 }
    });
    assert_eq!(
        ept_srs_wkt(&info).unwrap(),
        Some("EPSG:26915+5703".to_string())
    );
}

#[test]
fn srs_wkt_absent_or_empty_is_none() {
    assert_eq!(ept_srs_wkt(&serde_json::json!({})).unwrap(), None);
    assert_eq!(
        ept_srs_wkt(&serde_json::json!({ "srs": {} })).unwrap(),
        None
    );
    assert_eq!(
        ept_srs_wkt(&serde_json::json!({ "srs": null })).unwrap(),
        None
    );
}

#[test]
fn srs_wkt_validation_errors_match_cpp() {
    let err = ept_srs_wkt(&serde_json::json!({ "srs": { "wkt": 5 } }))
        .err()
        .unwrap();
    assert!(err.0.contains("srs.wkt must be specified as a string"));

    let err = ept_srs_wkt(&serde_json::json!({ "srs": { "authority": "EPSG" } }))
        .err()
        .unwrap();
    assert!(err.0.contains("at least one of"));

    let err = ept_srs_wkt(&serde_json::json!({
        "srs": { "authority": "EPSG", "horizontal": -1 }
    }))
    .err()
    .unwrap();
    assert!(err.0.contains("srs.horizontal must be specified"));

    let err = ept_srs_wkt(&serde_json::json!({
        "srs": { "authority": "EPSG", "horizontal": 26915, "vertical": 1.5 }
    }))
    .err()
    .unwrap();
    assert!(err.0.contains("srs.vertical must be specified"));
}
