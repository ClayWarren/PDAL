use super::*;
use std::fs;
use std::path::PathBuf;

fn temp_tif(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("pdal-native-{name}-{}.tif", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}

#[test]
fn metadata_roundtrip_includes_empty_values_in_memory() {
    register_drivers();
    let path = temp_tif("metadata-empty");
    let mut raster = Raster::create_float64(
        path.to_str().unwrap(),
        "GTiff",
        1,
        1,
        1,
        [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
        "",
    )
    .unwrap();
    raster
        .write_band_f64(1, 1, 1, &[1.0], -9999.0, "Z")
        .unwrap();
    raster.set_metadata_item("AREA_OR_PIXEL", "Pixel").unwrap();
    raster.set_metadata_item("empty", "").unwrap();
    raster
        .set_metadata_item("equals", "some_more_equals===")
        .unwrap();
    assert_eq!(raster.metadata_item("empty").as_deref(), Some(""));

    drop(raster);

    let raster = Raster::open(path.to_str().unwrap()).unwrap();
    assert_eq!(
        raster.metadata_item("AREA_OR_PIXEL").as_deref(),
        Some("Pixel")
    );
    assert_eq!(
        raster.metadata_item("equals").as_deref(),
        Some("some_more_equals===")
    );
    // GTiff does not persist empty metadata values when the dataset is closed.
    assert!(raster.metadata_item("empty").is_none());
}

#[test]
fn raster_create_applies_creation_options() {
    register_drivers();
    let path = temp_tif("creation-options");
    let mut raster = Raster::create_typed_with_options(
        path.to_str().unwrap(),
        "GTiff",
        1,
        1,
        1,
        [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
        "",
        RasterDataType::Float64,
        &["COMPRESS=LZW".to_string()],
    )
    .unwrap();
    raster
        .write_band_f64(1, 1, 1, &[1.0], -9999.0, "Z")
        .unwrap();
    drop(raster);

    let raster = Raster::open(path.to_str().unwrap()).unwrap();
    assert_eq!(
        raster.metadata_item_domain("COMPRESSION", "IMAGE_STRUCTURE"),
        Some("LZW".to_string())
    );
}

#[test]
fn test_raster_create_invalid_driver() {
    let path = temp_tif("invalid-driver");
    let res = Raster::create_float64(
        path.to_str().unwrap(),
        "NonExistentDriverName",
        1,
        1,
        1,
        [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
        "",
    );
    assert!(res.is_err());
    if let Err(e) = res {
        assert!(e.contains("GDAL driver 'NonExistentDriverName' not found"));
    }
}

#[test]
fn raster_paths_and_names_reject_nul_bytes() {
    assert!(Raster::open("bad\0path").is_err());
    assert!(Raster::create_float64(
        "bad\0path",
        "GTiff",
        1,
        1,
        1,
        [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
        "",
    )
    .is_err());
    assert!(Raster::create_float64(
        temp_tif("nul-driver").to_str().unwrap(),
        "bad\0driver",
        1,
        1,
        1,
        [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
        "",
    )
    .is_err());
    assert!(Raster::create_float64(
        temp_tif("nul-srs").to_str().unwrap(),
        "GTiff",
        1,
        1,
        1,
        [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
        "bad\0srs",
    )
    .is_err());
}

#[test]
fn test_raster_read_band_errors() {
    register_drivers();
    let path = temp_tif("read-band-errors");
    let mut raster = Raster::create_float64(
        path.to_str().unwrap(),
        "GTiff",
        2,
        2,
        1,
        [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
        "",
    )
    .unwrap();
    raster
        .write_band_f64(1, 2, 2, &[1.0, 2.0, 3.0, 4.0], -9999.0, "Z")
        .unwrap();

    // 1. Buffer size mismatch
    let mut buf = vec![0.0f64; 3];
    let res = raster.read_band(1, 2, 2, &mut buf);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err(),
        "GDAL band buffer size does not match raster dimensions."
    );

    // 2. Invalid band index
    let mut buf2 = vec![0.0f64; 4];
    let res2 = raster.read_band(2, 2, 2, &mut buf2);
    assert!(res2.is_err());
    assert!(res2.unwrap_err().contains("Failed to get band 2"));
}

#[test]
fn test_raster_read_at_out_of_bounds() {
    register_drivers();
    let path = temp_tif("read-at-bounds");
    let mut raster = Raster::create_float64(
        path.to_str().unwrap(),
        "GTiff",
        2,
        2,
        1,
        [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
        "",
    )
    .unwrap();
    raster
        .write_band_f64(1, 2, 2, &[1.0, 2.0, 3.0, 4.0], -9999.0, "Z")
        .unwrap();

    // Out of bounds coordinates (pixel space is 0..2, 0..2, geotransform starts at 0.0)
    let mut buf = vec![0.0f64; 1];
    let res = raster.read_at(10.0, 10.0, &mut buf);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Out of bounds");
}

#[test]
fn test_raster_create_int32_and_methods() {
    register_drivers();
    let path = temp_tif("int32-raster");
    let mut raster = Raster::create_int32(
        path.to_str().unwrap(),
        "GTiff",
        2,
        3,
        1,
        [10.0, 2.0, 0.0, 20.0, 0.0, -3.0],
        "EPSG:4326",
    )
    .unwrap();

    assert_eq!(raster.width(), 2);
    assert_eq!(raster.height(), 3);
    assert_eq!(raster.band_count(), 1);
    let gt = raster.get_geo_transform().unwrap();
    assert_eq!(gt, [10.0, 2.0, 0.0, 20.0, 0.0, -3.0]);
    let srs = raster.get_wkt_srs();
    assert!(srs.contains("4326") || !srs.is_empty());

    // Test write_band_i32
    let data = [1, 2, 3, 4, 5, 6];
    raster
        .write_band_i32(1, 2, 3, &data, -99, "Int32Band")
        .unwrap();

    // Test write_band_i32 error (buffer mismatch)
    assert!(raster.write_band_i32(1, 2, 3, &[1, 2], -99, "").is_err());
    assert!(raster
        .write_band_i32(2, 2, 3, &data, -99, "InvalidBand")
        .is_err());
    assert!(raster
        .write_band_i32(1, 2, 3, &data, -99, "bad\0description")
        .is_err());

    // Test write_band_f64 error (buffer mismatch)
    assert!(raster.write_band_f64(1, 2, 3, &[1.0], -99.0, "").is_err());
    let float_data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    assert!(raster
        .write_band_f64(2, 2, 3, &float_data, -99.0, "InvalidBand")
        .is_err());
    assert!(raster
        .write_band_f64(1, 2, 3, &float_data, -99.0, "bad\0description")
        .is_err());
    assert!(raster.set_metadata_item("bad\0key", "value").is_err());
    assert!(raster.set_metadata_item("key", "bad\0value").is_err());
    assert!(raster.metadata_item("bad\0key").is_none());

    drop(raster);

    // Reopen and read
    let raster = Raster::open(path.to_str().unwrap()).unwrap();
    let mut read_data = vec![0.0f64; 6];
    raster.read_band(1, 2, 3, &mut read_data).unwrap();
    assert_eq!(read_data, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

    // Test read_at
    let mut pixel_buf = vec![0.0f64; 1];
    // Coordinate at pixel index (1, 0):
    // x = 10.0 + 1 * 2.0 = 12.0
    // y = 20.0 + 0 * -3.0 = 20.0 (using 17.5 results in line 0)
    raster.read_at(12.5, 17.5, &mut pixel_buf).unwrap();
    // pixel index (1, 0) in a 2x3 grid is index 0 * 2 + 1 = 1 (value is 2)
    assert_eq!(pixel_buf[0], 2.0);
}

#[test]
fn raster_read_at_rejects_noninvertible_geotransform() {
    register_drivers();
    let path = temp_tif("read-at-noninvertible");
    let raster = Raster::create_float64(
        path.to_str().unwrap(),
        "GTiff",
        1,
        1,
        1,
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        "",
    )
    .unwrap();

    let mut buf = [0.0];
    assert_eq!(
        raster.read_at(0.0, 0.0, &mut buf).unwrap_err(),
        "Failed to get geo transform"
    );
}

fn temp_shp(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("pdal-native-{name}-{}.shp", std::process::id()));
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(path.with_extension("shx"));
    let _ = fs::remove_file(path.with_extension("dbf"));
    path
}

#[test]
fn test_vector_creation_and_fields() {
    register_drivers();
    let path = temp_shp("vector-test");

    // 1. Vector open failure
    assert!(Vector::open(path.to_str().unwrap()).is_err());
    assert!(Vector::open("bad\0path").is_err());

    // 2. Vector create invalid driver failure
    assert!(Vector::create(path.to_str().unwrap(), "NonExistentVectorDriver").is_err());
    assert!(Vector::create("bad\0path", "ESRI Shapefile").is_err());
    assert!(Vector::create(path.to_str().unwrap(), "bad\0driver").is_err());

    // 3. Vector create success
    let vector = Vector::create(path.to_str().unwrap(), "ESRI Shapefile").unwrap();
    let layer = vector
        .open_or_create_layer("test_layer", "EPSG:4326")
        .unwrap();
    assert!(!layer.is_null());

    // 4. Create fields (unsafe)
    unsafe {
        assert!(Vector::create_string_field(layer, "bad\0name").is_err());
        assert!(Vector::create_datetime_field(layer, "bad\0timestamp").is_err());
        Vector::create_string_field(layer, "name").unwrap();
        Vector::create_datetime_field(layer, "timestamp").unwrap();
        Vector::create_string_field(layer, "name").unwrap();
        Vector::create_datetime_field(layer, "timestamp").unwrap();

        // Add feature
        assert!(Vector::add_feature(layer, "POLYGON EMPTY", &[("bad\0field", "value")]).is_err());
        assert!(Vector::add_feature(layer, "POLYGON EMPTY", &[("name", "bad\0value")]).is_err());
        Vector::add_feature(
            layer,
            "POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))",
            &[("name", "test_geom")],
        )
        .unwrap();
    }

    drop(vector);

    // 5. Open and get features
    let vector = Vector::open(path.to_str().unwrap()).unwrap();
    let features = vector.get_features(0, "name").unwrap();
    assert_eq!(features.len(), 1);
    let (wkt, _val) = &features[0];
    assert!(wkt.contains("POLYGON"));

    // Feature column not found
    assert!(vector.get_features(0, "nonexistent").is_err());
    assert!(vector.get_features(99, "name").is_err());
}

#[test]
fn vector_layer_creation_options_are_forwarded() {
    register_drivers();
    let path = temp_tif("vector-options").with_extension("gpkg");
    let _ = fs::remove_file(&path);

    let vector = Vector::create(path.to_str().unwrap(), "GPKG").unwrap();
    let layer = vector
        .open_or_create_layer_with_options("tiles", "", &["GEOMETRY_NAME=tile_geom".to_string()])
        .unwrap();
    assert!(!layer.is_null());
    drop(vector);

    let vector = Vector::open(path.to_str().unwrap()).unwrap();
    assert_eq!(vector.geometry_column(0).unwrap(), "tile_geom");
}

#[test]
fn vector_attribute_filter_is_cleared_after_read_error() {
    register_drivers();
    let path = temp_tif("vector-filter-reset").with_extension("geojson");
    let _ = fs::remove_file(&path);

    let vector = Vector::create(path.to_str().unwrap(), "GeoJSON").unwrap();
    let layer = vector.open_or_create_layer("tiles", "").unwrap();
    unsafe {
        Vector::create_string_field(layer, "location").unwrap();
        Vector::add_feature(
            layer,
            "POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))",
            &[("location", "keep")],
        )
        .unwrap();
        Vector::add_feature(
            layer,
            "POLYGON ((2 2, 3 2, 3 3, 2 3, 2 2))",
            &[("location", "drop")],
        )
        .unwrap();
    }
    drop(vector);

    let vector = Vector::open(path.to_str().unwrap()).unwrap();
    let err = vector
        .get_string_pair_features_by_layer("", "missing", "", "location = 'keep'")
        .unwrap_err();
    assert!(err.contains("No column name 'missing'"));

    let rows = vector
        .get_string_pair_features_by_layer("", "location", "", "")
        .unwrap();
    assert_eq!(rows.len(), 2);
}

fn temp_geojson(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("pdal-native-{name}-{}.json", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}

#[test]
fn test_polygon_writer_density_and_boundary() {
    register_drivers();

    // Density layer: one Polygon feature per hexagon with ID + COUNT, like
    // the C++ density::OGR::writeDensity path.
    let density = temp_geojson("polygon-density");
    {
        let writer =
            VectorPointWriter::create_polygon(density.to_str().unwrap(), "GeoJSON", "", "hexbins")
                .unwrap();
        writer
            .create_field("ID", VectorFieldType::Integer64)
            .unwrap();
        writer
            .create_field("COUNT", VectorFieldType::Integer)
            .unwrap();
        // A 6-vertex hexagon; write_polygon should close the ring for us.
        let hexagon = [
            (0.0, 1.0),
            (0.87, 0.5),
            (0.87, -0.5),
            (0.0, -1.0),
            (-0.87, -0.5),
            (-0.87, 0.5),
        ];
        writer
            .write_polygon(
                &hexagon,
                &[
                    VectorFieldValue::Integer64(42),
                    VectorFieldValue::Integer(7),
                ],
            )
            .unwrap();
    }
    let vector = Vector::open(density.to_str().unwrap()).unwrap();
    let features = vector.get_features(0, "ID").unwrap();
    assert_eq!(features.len(), 1);
    assert!(features[0].0.contains("POLYGON"));
    assert_eq!(features[0].1, 42);
    drop(vector);
    let _ = fs::remove_file(&density);

    // Boundary layer: a single MultiPolygon feature written from WKT, like
    // the C++ writeBoundary path (the hull comes from the grid as WKT).
    let boundary = temp_geojson("polygon-boundary");
    {
        let writer =
            VectorPointWriter::create_polygon(boundary.to_str().unwrap(), "GeoJSON", "", "hexbins")
                .unwrap();
        writer
            .create_field("ID", VectorFieldType::Integer64)
            .unwrap();
        writer
            .write_geometry_wkt(
                "MULTIPOLYGON (((0 0, 4 0, 4 4, 0 4, 0 0)))",
                &[VectorFieldValue::Integer64(0)],
            )
            .unwrap();
        // Invalid WKT must surface an error, not silently drop the feature.
        assert!(writer.write_geometry_wkt("NOT WKT", &[]).is_err());
    }
    let vector = Vector::open(boundary.to_str().unwrap()).unwrap();
    let features = vector.get_features(0, "ID").unwrap();
    assert_eq!(features.len(), 1);
    assert!(features[0].0.contains("MULTIPOLYGON") || features[0].0.contains("POLYGON"));
    assert_eq!(features[0].1, 0);
    drop(vector);
    let _ = fs::remove_file(&boundary);
}

#[test]
fn version_info_rejects_nul_key() {
    assert_eq!(version_info("bad\0key"), "");
}
