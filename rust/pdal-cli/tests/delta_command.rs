use std::path::{Path, PathBuf};
use std::process::Command;

fn data_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn run_delta(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("delta")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn delta_of_a_file_against_itself_is_zero() {
    let file = data_path("test/data/ply/simple_text.ply");
    let result = run_delta(&[file.to_str().unwrap(), file.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "pdal-rs delta failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    for dim in ["X", "Y", "Z"] {
        for stat in ["min", "mean", "max"] {
            assert_eq!(json[dim][stat], 0.0, "{dim}.{stat} should be zero");
        }
    }
}

#[test]
fn delta_of_distinct_files_reports_finite_dimension_stats() {
    let source = data_path("test/data/ply/simple_text.ply");
    let candidate = data_path("test/data/ply/text_extradim.ply");
    let result = run_delta(&[source.to_str().unwrap(), candidate.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "pdal-rs delta failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    for dim in ["X", "Y", "Z"] {
        let min = json[dim]["min"].as_f64().unwrap();
        let mean = json[dim]["mean"].as_f64().unwrap();
        let max = json[dim]["max"].as_f64().unwrap();
        assert!(min.is_finite() && mean.is_finite() && max.is_finite());
        assert!(
            min <= mean && mean <= max,
            "{dim}: {min} <= {mean} <= {max}"
        );
    }
}

#[test]
fn delta_supports_named_source_and_candidate() {
    let source = data_path("test/data/ply/simple_text.ply");
    let candidate = data_path("test/data/ply/text_extradim.ply");
    let result = run_delta(&[
        "--source",
        source.to_str().unwrap(),
        "--candidate",
        candidate.to_str().unwrap(),
    ]);
    assert!(
        result.status.success(),
        "pdal-rs delta failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(json["X"]["max"].as_f64().unwrap().is_finite());
}

#[test]
fn delta_supports_detail_output() {
    let file = data_path("test/data/ply/simple_text.ply");
    let result = run_delta(&["--detail", file.to_str().unwrap(), file.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "pdal-rs delta --detail failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let first = json["delta"].as_array().unwrap().first().unwrap();
    assert_eq!(first["i"], 0);
    assert_eq!(first["X"], 0.0);
    assert_eq!(first["Y"], 0.0);
    assert_eq!(first["Z"], 0.0);
}

#[test]
fn delta_supports_all_dimensions() {
    let file = data_path("test/data/ply/text_extradim.ply");
    let result = run_delta(&["--alldims", file.to_str().unwrap(), file.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "pdal-rs delta --alldims failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let object = json.as_object().unwrap();
    let has_extra_dim = object
        .keys()
        .any(|key| !matches!(key.as_str(), "source" | "candidate" | "X" | "Y" | "Z"));
    assert!(
        has_extra_dim,
        "expected --alldims to include an extra dimension"
    );
}

#[test]
fn delta_without_two_files_fails() {
    let file = data_path("test/data/ply/simple_text.ply");
    let result = run_delta(&[file.to_str().unwrap()]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("two filenames"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_delta_matches_rust_delta() {
    let source = data_path("test/data/ply/simple_text.ply");
    let candidate = data_path("test/data/ply/text_extradim.ply");

    let installed = Command::new("pdal")
        .arg("delta")
        .arg(&source)
        .arg(&candidate)
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal delta failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );
    let installed_json: serde_json::Value = serde_json::from_slice(&installed.stdout).unwrap();

    let rust = run_delta(&[source.to_str().unwrap(), candidate.to_str().unwrap()]);
    assert!(
        rust.status.success(),
        "pdal-rs delta failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );
    let rust_json: serde_json::Value = serde_json::from_slice(&rust.stdout).unwrap();

    for dim in ["X", "Y", "Z"] {
        for stat in ["min", "mean", "max"] {
            let installed_value = installed_json[dim][stat].as_f64().unwrap();
            let rust_value = rust_json[dim][stat].as_f64().unwrap();
            assert!(
                (installed_value - rust_value).abs() < 1e-6,
                "{dim}.{stat}: installed {installed_value} vs rust {rust_value}"
            );
        }
    }
}
