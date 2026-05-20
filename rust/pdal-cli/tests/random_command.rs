use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, PointView};
use pdal_io::pcd::PcdReader;

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn read_pcd(path: &Path) -> PointView {
    let mut options = Options::new();
    options.add("filename", path.display());
    PcdReader::new(&options).read().unwrap().pop().unwrap()
}

fn run_random(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("random")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn random_writes_the_requested_point_count_inside_the_unit_cube() {
    let temp = make_temp_dir("pdal-rs-random-count");
    let output = temp.join("out.pcd");

    let result = run_random(&[output.to_str().unwrap(), "--count=50"]);
    assert!(
        result.status.success(),
        "pdal-rs random failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let view = read_pcd(&output);
    assert_eq!(view.len(), 50);
    for point in 0..view.len() {
        for dim in [DimId::X, DimId::Y, DimId::Z] {
            let value = view.get_f64(point, &dim);
            assert!(
                (0.0..=1.0).contains(&value),
                "{dim:?} = {value} is outside the unit cube"
            );
        }
    }
}

#[test]
fn random_defaults_to_one_thousand_points() {
    let temp = make_temp_dir("pdal-rs-random-default");
    let output = temp.join("out.pcd");

    let result = run_random(&[output.to_str().unwrap()]);
    assert!(result.status.success());
    assert_eq!(read_pcd(&output).len(), 1000);
}

#[test]
fn random_supports_output_and_separated_count_options() {
    let temp = make_temp_dir("pdal-rs-random-options");
    let output = temp.join("out.pcd");

    let result = run_random(&["--output", output.to_str().unwrap(), "--count", "12"]);
    assert!(
        result.status.success(),
        "pdal-rs random failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    assert_eq!(read_pcd(&output).len(), 12);
}

#[test]
fn random_without_an_output_prints_usage_and_fails() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("random")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("pdal random <output>"));
}

#[test]
fn random_rejects_a_non_numeric_count() {
    let temp = make_temp_dir("pdal-rs-random-badcount");
    let output = temp.join("out.pcd");
    let result = run_random(&[output.to_str().unwrap(), "--count=lots"]);

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--count"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_random_matches_rust_random_count() {
    let temp = make_temp_dir("pdal-rs-random-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");

    let installed = Command::new("pdal")
        .arg("random")
        .arg(&installed_output)
        .arg("--count=50")
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal random failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = run_random(&[rust_output.to_str().unwrap(), "--count=50"]);
    assert!(
        rust.status.success(),
        "pdal-rs random failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );

    // Point values are random, so only the point count is comparable.
    assert_eq!(
        read_pcd(&rust_output).len(),
        read_pcd(&installed_output).len()
    );
}
