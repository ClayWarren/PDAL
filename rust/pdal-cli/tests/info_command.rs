use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn data_path(rel: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
        .display()
        .to_string()
}

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn info_reports_a_summary_for_a_ply_file() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("info")
        .arg(data_path("test/data/ply/simple_text.ply"))
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "pdal-rs info failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["driver"], "readers.ply");
    assert!(json["filename"]
        .as_str()
        .unwrap()
        .ends_with("simple_text.ply"));
    assert_eq!(json["point_count"], 3);
    assert_eq!(json["bounds_3d"]["minx"], -1.0);
    assert_eq!(json["bounds_3d"]["maxx"], 1.0);
    assert_eq!(json["bounds_3d"]["maxy"], 1.0);
    assert_eq!(json["bounds_3d"]["minz"], 0.0);
    assert!(json["dimension_summaries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dim| dim["name"] == "X"));
}

#[test]
fn info_supports_driver_override_and_input_option() {
    let temp = make_temp_dir("info-driver-override");
    let input = temp.join("simple_text_without_extension");
    fs::copy(data_path("test/data/ply/simple_text.ply"), &input).unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("info")
        .arg("--driver")
        .arg("readers.ply")
        .arg("--input")
        .arg(&input)
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "pdal-rs info failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["driver"], "readers.ply");
    assert_eq!(json["point_count"], 3);
}

#[test]
fn info_without_a_file_prints_usage_and_fails() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("info")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("pdal info [--summary] <file>"));
}

#[test]
fn info_rejects_a_filename_with_no_known_driver() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("info")
        .arg("mystery.xyzzy")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("reader driver"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_info_matches_rust_info() {
    let file = data_path("test/data/ply/simple_text.ply");

    let installed = Command::new("pdal")
        .arg("info")
        .arg("--summary")
        .arg(&file)
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal info failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );
    let installed_json: serde_json::Value = serde_json::from_slice(&installed.stdout).unwrap();

    let rust = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("info")
        .arg("--summary")
        .arg(&file)
        .output()
        .expect("failed to execute pdal-rs");
    assert!(
        rust.status.success(),
        "pdal-rs info failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );
    let rust_json: serde_json::Value = serde_json::from_slice(&rust.stdout).unwrap();

    // Point count matches PDAL's summary.
    assert_eq!(
        rust_json["point_count"].as_u64(),
        installed_json["summary"]["num_points"].as_u64()
    );

    // 3D bounds match (compared as f64; PDAL emits whole numbers as integers).
    let installed_bounds = &installed_json["summary"]["bounds"];
    let rust_bounds = &rust_json["bounds_3d"];
    for key in ["minx", "maxx", "miny", "maxy", "minz", "maxz"] {
        assert_eq!(
            rust_bounds[key].as_f64(),
            installed_bounds[key].as_f64(),
            "bound '{key}' differs"
        );
    }
}
