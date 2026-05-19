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
        output.to_str().unwrap(),
        input1.to_str().unwrap(),
        input2.to_str().unwrap(),
        "-f",
        "GeoJSON",
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
