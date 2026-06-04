use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_io::pcd::PcdReader;

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

fn run_tindex_with_stdin(args: &[&str], stdin: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("tindex")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn run_installed_pdal(args: &[&str]) -> Option<std::process::Output> {
    Command::new("pdal").args(args).output().ok()
}

fn pcd_len(path: &Path) -> usize {
    let mut options = Options::new();
    options.add("filename", path.display());
    PcdReader::new(&options)
        .read()
        .unwrap()
        .pop()
        .unwrap()
        .len() as usize
}

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
#[ignore = "requires PDAL_BIN env var pointing at a locally-built pdal binary"]
fn locally_built_pdal_tindex_rich_boundary_produces_multipolygon() {
    // The rich-boundary path lives in the C ABI kernel_abi/tindex.rs and is
    // exercised through the C++ `pdal` binary. PDAL_BIN should point at a
    // binary built with this Rust port (the brew/installed pdal does NOT
    // share our Rust code).
    let Ok(pdal_bin) = std::env::var("PDAL_BIN") else {
        eprintln!("set PDAL_BIN to a locally-built pdal binary");
        return;
    };
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("tindex_rich_boundary");
    let output = temp.join("rich.geojson");

    let result = Command::new(&pdal_bin)
        .args([
            "tindex",
            "create",
            "--tindex",
            output.to_str().unwrap(),
            input.to_str().unwrap(),
            "--ogrdriver",
            "GeoJSON",
            "--threshold=1",
            "--resolution=10",
        ])
        .output()
        .expect("failed to execute pdal");
    assert!(
        result.status.success(),
        "pdal tindex rich boundary failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let geojson = std::fs::read_to_string(&output).unwrap();
    // The bbox path emits a 5-vertex Polygon ring; the exact boundary
    // produced by hexer should have many more vertices.
    let coord_pairs = geojson.matches('[').count();
    assert!(
        coord_pairs > 10,
        "expected a rich boundary with many vertices, got {coord_pairs} '[' tokens in {geojson}"
    );
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
fn tindex_reads_inputs_from_stdin() {
    let input = data_path("test/data/las/interesting.las");

    let temp = make_temp_dir("tindex_stdin");
    let output = temp.join("index.geojson");

    let result = run_tindex_with_stdin(
        &[
            "create",
            "--tindex",
            output.to_str().unwrap(),
            "--stdin",
            "--ogrdriver",
            "GeoJSON",
            "--fast_boundary",
        ],
        &format!("{}\n", input.display()),
    );

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
fn tindex_applies_location_path_options() {
    let input = data_path("test/data/las/interesting.las");

    let temp = make_temp_dir("tindex_path_options");
    let output = temp.join("index.geojson");

    // `--write_absolute_path` and `--path_prefix` are mutually exclusive in
    // C++ (`TIndexKernel::validateSwitches`). The input here is already an
    // absolute path, so `--path_prefix` alone yields an absolute location.
    let result = run_tindex(&[
        "create",
        "--tindex",
        output.to_str().unwrap(),
        "--path_prefix",
        "prefix:",
        input.to_str().unwrap(),
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

    let geojson: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let location = geojson["features"][0]["properties"]["location"]
        .as_str()
        .unwrap();
    assert!(location.starts_with("prefix:/"));
    assert!(location.ends_with("interesting.las"));
}

#[test]
fn tindex_uses_custom_location_field_name() {
    let input = data_path("test/data/las/interesting.las");

    let temp = make_temp_dir("tindex_location_field");
    let output = temp.join("index.geojson");

    let result = run_tindex(&[
        "create",
        "--tindex",
        output.to_str().unwrap(),
        "--tindex_name",
        "source_file",
        "--lyr_name",
        "custom_layer",
        input.to_str().unwrap(),
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

    let geojson: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        geojson["features"][0]["properties"]["source_file"],
        input.to_str().unwrap()
    );
    assert!(geojson["features"][0]["properties"]
        .get("location")
        .is_none());
}

#[test]
fn tindex_merge_combines_geojson_index_sources() {
    let input = data_path("test/data/ply/simple_text.ply");

    let temp = make_temp_dir("tindex_merge");
    let index = temp.join("index.geojson");
    let output = temp.join("merged.pcd");

    let create = run_tindex(&[
        "create",
        "--tindex",
        index.to_str().unwrap(),
        input.to_str().unwrap(),
        input.to_str().unwrap(),
        "--ogrdriver",
        "GeoJSON",
        "--fast_boundary",
    ]);
    assert!(
        create.status.success(),
        "tindex create failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&create.stdout),
        String::from_utf8_lossy(&create.stderr)
    );

    let merge = run_tindex(&[
        "merge",
        "--tindex",
        index.to_str().unwrap(),
        "--filespec",
        output.to_str().unwrap(),
    ]);
    assert!(
        merge.status.success(),
        "tindex merge failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&merge.stdout),
        String::from_utf8_lossy(&merge.stderr)
    );

    assert_eq!(pcd_len(&output), 6);
}

#[test]
fn tindex_merge_combines_gpkg_index_sources() {
    let input = data_path("test/data/ply/simple_text.ply");

    let temp = make_temp_dir("tindex_merge_gpkg");
    let index = temp.join("index.gpkg");
    let output = temp.join("merged.pcd");

    let create = run_tindex(&[
        "create",
        "--tindex",
        index.to_str().unwrap(),
        input.to_str().unwrap(),
        input.to_str().unwrap(),
        "--ogrdriver",
        "GPKG",
        "--lyr_name",
        "tiles",
        "--fast_boundary",
    ]);
    assert!(
        create.status.success(),
        "tindex create failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&create.stdout),
        String::from_utf8_lossy(&create.stderr)
    );

    let merge = run_tindex(&[
        "merge",
        "--tindex",
        index.to_str().unwrap(),
        "--filespec",
        output.to_str().unwrap(),
        "--lyr_name",
        "tiles",
    ]);
    assert!(
        merge.status.success(),
        "tindex merge failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&merge.stdout),
        String::from_utf8_lossy(&merge.stderr)
    );

    assert_eq!(pcd_len(&output), 6);
}

#[test]
fn tindex_create_appends_to_existing_gpkg_index() {
    let input = data_path("test/data/ply/simple_text.ply");

    let temp = make_temp_dir("tindex_create_append_gpkg");
    let input2 = temp.join("simple_text_copy.ply");
    std::fs::copy(&input, &input2).unwrap();
    let index = temp.join("index.gpkg");
    let output = temp.join("merged.pcd");

    for path in [input.as_path(), input2.as_path()] {
        let create = run_tindex(&[
            "create",
            "--tindex",
            index.to_str().unwrap(),
            path.to_str().unwrap(),
            "--ogrdriver",
            "GPKG",
            "--lyr_name",
            "tiles",
            "--fast_boundary",
        ]);
        assert!(
            create.status.success(),
            "tindex create failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&create.stdout),
            String::from_utf8_lossy(&create.stderr)
        );
    }

    let merge = run_tindex(&[
        "merge",
        "--tindex",
        index.to_str().unwrap(),
        "--filespec",
        output.to_str().unwrap(),
        "--lyr_name",
        "tiles",
    ]);
    assert!(
        merge.status.success(),
        "tindex merge failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&merge.stdout),
        String::from_utf8_lossy(&merge.stderr)
    );

    assert_eq!(pcd_len(&output), 6);
}

#[test]
fn tindex_create_rejects_duplicate_gpkg_entries() {
    let input = data_path("test/data/ply/simple_text.ply");

    let temp = make_temp_dir("tindex_create_duplicate_gpkg");
    let index = temp.join("index.gpkg");
    let output = temp.join("merged.pcd");

    let args = [
        "create",
        "--tindex",
        index.to_str().unwrap(),
        input.to_str().unwrap(),
        "--ogrdriver",
        "GPKG",
        "--lyr_name",
        "tiles",
        "--fast_boundary",
    ];
    let first = run_tindex(&args);
    assert!(
        first.status.success(),
        "tindex create failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );

    let second = run_tindex(&args);
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("Couldn't index any files"));

    let merge = run_tindex(&[
        "merge",
        "--tindex",
        index.to_str().unwrap(),
        "--filespec",
        output.to_str().unwrap(),
        "--lyr_name",
        "tiles",
    ]);
    assert!(
        merge.status.success(),
        "tindex merge failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&merge.stdout),
        String::from_utf8_lossy(&merge.stderr)
    );

    assert_eq!(pcd_len(&output), 3);
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
