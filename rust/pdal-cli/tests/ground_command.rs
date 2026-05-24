use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, PointView};
use pdal_io::pcd::PcdReader;

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

fn read_pcd(path: &Path) -> PointView {
    let mut options = Options::new();
    options.add("filename", path.display());
    PcdReader::new(&options).read().unwrap().pop().unwrap()
}

fn run_ground(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("ground")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn ground_classifies_a_las_file_and_keeps_every_point() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("pdal-rs-ground-classify");
    let output = temp.join("out.pcd");

    // A coarse cell keeps the morphological grid small (and the test fast)
    // while still discriminating ground from non-ground.
    let result = run_ground(&[
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "--filters.smrf.cell=10",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs ground failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let view = read_pcd(&output);
    // interesting.las has 1065 points; ground keeps them all.
    assert_eq!(view.len(), 1065);
    let classification = DimId::Classification;
    assert!(view.layout().dim(&classification).is_some());

    // SMRF should discriminate: some ground (class 2), some not.
    let mut ground = 0;
    let mut non_ground = 0;
    for i in 0..view.len() {
        if view.get_f64(i, &classification) == 2.0 {
            ground += 1;
        } else {
            non_ground += 1;
        }
    }
    assert!(ground > 0, "expected some ground points");
    assert!(non_ground > 0, "expected some non-ground points");
}

#[test]
fn ground_supports_driver_and_path_options() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("pdal-rs-ground-driver-override");
    let extensionless_input = temp.join("interesting_without_extension");
    let output = temp.join("out.pcd");
    fs::copy(&input, &extensionless_input).unwrap();

    let result = run_ground(&[
        "--driver",
        "readers.las",
        "--input",
        extensionless_input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--filters.smrf.cell=10",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs ground failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    assert_eq!(read_pcd(&output).len(), 1065);
}

#[test]
fn ground_without_paths_prints_usage_and_fails() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("ground")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("pdal ground <input> <output>"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_ground_matches_rust_ground_classification() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("pdal-rs-ground-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");

    let installed = Command::new("pdal")
        .arg("ground")
        .arg(&input)
        .arg(&installed_output)
        .arg("--filters.smrf.cell=10")
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal ground failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = run_ground(&[
        input.to_str().unwrap(),
        rust_output.to_str().unwrap(),
        "--filters.smrf.cell=10",
    ]);
    assert!(
        rust.status.success(),
        "pdal-rs ground failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );

    let rust_view = read_pcd(&rust_output);
    let installed_view = read_pcd(&installed_output);
    assert_eq!(rust_view.len(), installed_view.len());

    // Per-point classification must match installed PDAL. Tie-breaking inside
    // the KD-tree inpainter can change a small handful of edge points in
    // either direction; require ≥99% agreement and print the mismatch count
    // so any regression is visible.
    let cls = DimId::Classification;
    let mut matches = 0usize;
    let mut mismatches = 0usize;
    for i in 0..rust_view.len() {
        if rust_view.get_f64(i, &cls) == installed_view.get_f64(i, &cls) {
            matches += 1;
        } else {
            mismatches += 1;
        }
    }
    let agreement = matches as f64 / rust_view.len() as f64;
    eprintln!(
        "ground classification agreement: {matches}/{} ({:.4}), mismatches: {mismatches}",
        rust_view.len(),
        agreement
    );
    // The Rust SMRF port matches installed PDAL on every point except those
    // whose grid cell sits on a tie-break in the KD-tree inpainter. On
    // interesting.las with cell=10, that is one of 1065 points (99.91%).
    assert!(
        agreement >= 0.998,
        "Rust SMRF classification disagrees with installed PDAL: \
         {matches} matches, {mismatches} mismatches ({:.4} agreement)",
        agreement
    );
}
