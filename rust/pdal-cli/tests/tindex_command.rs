use std::path::{Path, PathBuf};
use std::process::Command;

fn data_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn run_tindex(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("tindex")
        .args(args)
        .output()
        .unwrap()
}

fn run_installed_pdal(args: &[&str]) -> Option<std::process::Output> {
    Command::new("pdal").args(args).output().ok()
}

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn tindex_creates_geojson_index() {
    let input1 = data_path("test/data/las/interesting.las");
    let input2 = data_path("test/data/las/1.2-with-color.las");

    let temp = make_temp_dir("tindex_command");
    let output = temp.join("index.geojson");

    let result = run_tindex(&[
        "create",
        "--tindex",
        output.to_str().unwrap(),
        input1.to_str().unwrap(),
        input2.to_str().unwrap(),
        "--ogrdriver",
        "GeoJSON",
        "--fast_boundary",
    ]);

    assert!(
        result.status.success(),
        "tindex failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    assert!(output.exists());
    let geojson = std::fs::read_to_string(&output).unwrap();

    // Ensure both files were indexed and have "location" properties
    assert!(geojson.contains("interesting.las"));
    assert!(geojson.contains("1.2-with-color.las"));
    assert!(geojson.contains("\"location\""));
    assert!(geojson.contains("\"srs\""));
}

#[test]
fn tindex_reads_inputs_from_filelist() {
    let input1 = data_path("test/data/las/interesting.las");
    let input2 = data_path("test/data/las/1.2-with-color.las");

    let temp = make_temp_dir("tindex_filelist");
    let filelist = temp.join("inputs.txt");
    let output = temp.join("index.geojson");
    std::fs::write(
        &filelist,
        format!("{}\n{}\n", input1.display(), input2.display()),
    )
    .unwrap();

    let result = run_tindex(&[
        "create",
        "--tindex",
        output.to_str().unwrap(),
        "--filelist",
        filelist.to_str().unwrap(),
        "--ogrdriver",
        "GeoJSON",
        "--fast_boundary",
    ]);

    assert!(
        result.status.success(),
        "tindex failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let geojson = std::fs::read_to_string(&output).unwrap();
    assert!(geojson.contains("interesting.las"));
    assert!(geojson.contains("1.2-with-color.las"));
}

#[test]
fn tindex_reads_inputs_from_glob() {
    let input = data_path("test/data/las/interesting.las");

    let temp = make_temp_dir("tindex_glob");
    let output = temp.join("index.geojson");
    let link = temp.join("interesting.las");
    std::fs::copy(&input, &link).unwrap();
    let pattern = temp.join("*.las");

    let result = run_tindex(&[
        "create",
        "--tindex",
        output.to_str().unwrap(),
        "--glob",
        pattern.to_str().unwrap(),
        "--ogrdriver",
        "GeoJSON",
        "--fast_boundary",
    ]);

    assert!(
        result.status.success(),
        "tindex failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let geojson = std::fs::read_to_string(&output).unwrap();
    assert!(geojson.contains("interesting.las"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_tindex_matches_rust_tindex_location_index() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("tindex_installed_regression");
    let installed_output = temp.join("installed.geojson");
    let rust_output = temp.join("rust.geojson");

    let installed = run_installed_pdal(&[
        "tindex",
        "create",
        "--tindex",
        installed_output.to_str().unwrap(),
        "--ogrdriver",
        "GeoJSON",
        "--fast_boundary",
        input.to_str().unwrap(),
    ])
    .expect("installed pdal is required for this regression");
    assert!(
        installed.status.success(),
        "installed pdal tindex failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = run_tindex(&[
        "create",
        "--tindex",
        rust_output.to_str().unwrap(),
        "--ogrdriver",
        "GeoJSON",
        "--fast_boundary",
        input.to_str().unwrap(),
    ]);
    assert!(
        rust.status.success(),
        "rust tindex failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );

    let installed_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&installed_output).unwrap()).unwrap();
    let rust_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&rust_output).unwrap()).unwrap();

    assert_eq!(installed_json["type"], "FeatureCollection");
    assert_eq!(rust_json["type"], "FeatureCollection");
    let installed_features = installed_json["features"].as_array().unwrap();
    let rust_features = rust_json["features"].as_array().unwrap();
    assert_eq!(installed_features.len(), 1);
    assert_eq!(rust_features.len(), 1);
    assert_eq!(
        installed_features[0]["properties"]["location"],
        rust_features[0]["properties"]["location"]
    );
}

#[test]
fn tindex_rejects_unknown_options() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("tindex_unknown_option");
    let output = temp.join("index.geojson");

    let result = run_tindex(&[
        "create",
        output.to_str().unwrap(),
        input.to_str().unwrap(),
        "--bogus",
    ]);

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--bogus"));
}

#[test]
fn tindex_rejects_unrecognized_input_files() {
    let temp = make_temp_dir("tindex_bad_input");
    let output = temp.join("index.geojson");
    let unknown = temp.join("points.unknown");
    std::fs::write(&unknown, "not a point cloud").unwrap();

    let result = run_tindex(&[
        "create",
        output.to_str().unwrap(),
        unknown.to_str().unwrap(),
        "-f",
        "GeoJSON",
    ]);

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("unable to infer"));
}
