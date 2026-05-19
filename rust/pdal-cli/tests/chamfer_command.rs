use std::path::{Path, PathBuf};
use std::process::Command;

fn data_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn run_chamfer(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("chamfer")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn chamfer_of_a_file_against_itself_is_zero() {
    let file = data_path("test/data/ply/simple_text.ply");
    let result = run_chamfer(&[file.to_str().unwrap(), file.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "pdal-rs chamfer failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["chamfer"], 0.0);
}

#[test]
fn chamfer_of_distinct_files_is_positive_and_finite() {
    let source = data_path("test/data/ply/simple_text.ply");
    let candidate = data_path("test/data/ply/text_extradim.ply");
    let result = run_chamfer(&[source.to_str().unwrap(), candidate.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "pdal-rs chamfer failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let chamfer = json["chamfer"].as_f64().unwrap();
    assert!(chamfer.is_finite() && chamfer > 0.0);
}

#[test]
fn chamfer_without_two_files_fails() {
    let file = data_path("test/data/ply/simple_text.ply");
    let result = run_chamfer(&[file.to_str().unwrap()]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("two filenames"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_chamfer_matches_rust_chamfer() {
    let source = data_path("test/data/ply/simple_text.ply");
    let candidate = data_path("test/data/ply/text_extradim.ply");

    let installed = Command::new("pdal")
        .arg("chamfer")
        .arg(&source)
        .arg(&candidate)
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal chamfer failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );
    let installed_json: serde_json::Value = serde_json::from_slice(&installed.stdout).unwrap();

    let rust = run_chamfer(&[source.to_str().unwrap(), candidate.to_str().unwrap()]);
    assert!(
        rust.status.success(),
        "pdal-rs chamfer failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );
    let rust_json: serde_json::Value = serde_json::from_slice(&rust.stdout).unwrap();

    let installed_value = installed_json["chamfer"].as_f64().unwrap();
    let rust_value = rust_json["chamfer"].as_f64().unwrap();
    assert!(
        (installed_value - rust_value).abs() <= 1e-6 * installed_value.abs().max(1.0),
        "chamfer: installed {installed_value} vs rust {rust_value}"
    );
}
