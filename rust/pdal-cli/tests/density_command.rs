use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pdal_native::gdal::Vector;

fn data_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_density(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("density")
        .args(args)
        .output()
        .unwrap()
}

/// Sorted multiset of the `COUNT` property over every density feature.
fn feature_counts(geojson: &serde_json::Value) -> Vec<i64> {
    let mut counts: Vec<i64> = geojson["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|feature| feature["properties"]["COUNT"].as_i64().unwrap())
        .collect();
    counts.sort_unstable();
    counts
}

#[test]
fn density_writes_a_hexagonal_tessellation_as_geojson() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("pdal-rs-density-geojson");
    let output = temp.join("density.geojson");

    // A wide hex edge and low threshold keep several cells "dense" for the
    // 1065-point file.
    let result = run_density(&[
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "--filters.hexbin.edge_length=25",
        "--filters.hexbin.threshold=2",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs density failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let geojson: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(geojson["type"], "FeatureCollection");

    let features = geojson["features"].as_array().unwrap();
    assert!(!features.is_empty(), "expected some dense hexagons");
    for feature in features {
        assert_eq!(feature["type"], "Feature");
        // Each dense cell carries an ID and a COUNT at or above the threshold.
        assert!(feature["properties"]["ID"].is_number());
        assert!(feature["properties"]["COUNT"].as_i64().unwrap() >= 2);
        assert_eq!(feature["geometry"]["type"], "Polygon");
        // A closed hexagon ring: six vertices plus the repeated first point.
        let ring = feature["geometry"]["coordinates"][0].as_array().unwrap();
        assert_eq!(ring.len(), 7);
        assert_eq!(ring.first().unwrap(), ring.last().unwrap());
    }
}

#[test]
fn density_supports_driver_path_and_native_options() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("pdal-rs-density-options");
    let extensionless_input = temp.join("interesting_without_extension");
    let output = temp.join("density.geojson");
    fs::copy(&input, &extensionless_input).unwrap();

    let result = run_density(&[
        "--driver",
        "readers.las",
        "--input",
        extensionless_input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--ogrdriver",
        "GeoJSON",
        "--edge_length",
        "25",
        "--threshold=2",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs density failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let geojson: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(geojson["type"], "FeatureCollection");
    assert!(!feature_counts(&geojson).is_empty());
}

#[test]
fn density_writes_non_geojson_output_through_native_vector_writer() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("pdal-rs-density-gpkg");
    let output = temp.join("density.gpkg");

    let result = run_density(&[
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "--ogrdriver",
        "GPKG",
        "--lyr_name",
        "custom_hexbins",
        "--edge_length=25",
        "--threshold=2",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs density failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let vector = Vector::open(output.to_str().unwrap()).unwrap();
    let features = vector.get_features(0, "COUNT").unwrap();
    assert!(!features.is_empty());
    assert!(features
        .iter()
        .all(|(wkt, count)| wkt.contains("POLYGON") && *count >= 2));
}

#[test]
fn density_without_paths_prints_usage_and_fails() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("density")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("pdal density <input> <output"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_density_matches_rust_density() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("pdal-rs-density-regression");
    let installed_output = temp.join("installed.geojson");
    let rust_output = temp.join("rust.geojson");

    let installed = Command::new("pdal")
        .arg("density")
        .arg(&input)
        .arg(&installed_output)
        .arg("-f")
        .arg("GeoJSON")
        .arg("--edge_length=25")
        .arg("--threshold=2")
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal density failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = run_density(&[
        input.to_str().unwrap(),
        rust_output.to_str().unwrap(),
        "--filters.hexbin.edge_length=25",
        "--filters.hexbin.threshold=2",
    ]);
    assert!(
        rust.status.success(),
        "pdal-rs density failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );

    let installed_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&installed_output).unwrap()).unwrap();
    let rust_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&rust_output).unwrap()).unwrap();

    // The Rust hexbin port reproduces PDAL's tessellation: the same number of
    // dense hexagons with the same per-cell point counts.
    assert_eq!(feature_counts(&rust_json), feature_counts(&installed_json));
}
