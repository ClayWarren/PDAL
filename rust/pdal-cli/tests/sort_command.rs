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

fn run_sort(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("sort")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn sort_orders_points_ascending_by_x_by_default() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-sort-default");
    let output = temp.join("out.pcd");

    let result = run_sort(&[input.to_str().unwrap(), output.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "pdal-rs sort failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let view = read_pcd(&output);
    assert_eq!(view.len(), 3);
    assert_eq!(view.get_f64(0, &DimId::X), -1.0);
    assert_eq!(view.get_f64(1, &DimId::X), 0.0);
    assert_eq!(view.get_f64(2, &DimId::X), 1.0);
}

#[test]
fn sort_honors_descending_order() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-sort-desc");
    let output = temp.join("out.pcd");

    let result = run_sort(&[
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "--filters.sort.order=desc",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs sort failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let view = read_pcd(&output);
    assert_eq!(view.get_f64(0, &DimId::X), 1.0);
    assert_eq!(view.get_f64(1, &DimId::X), 0.0);
    assert_eq!(view.get_f64(2, &DimId::X), -1.0);
}

#[test]
fn sort_honors_an_alternate_dimension() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-sort-dim");
    let output = temp.join("out.pcd");

    // simple_text.ply Y values are 0, 1, 0 -> ascending sort gives 0, 0, 1.
    let result = run_sort(&[
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "--filters.sort.dimensions=Y",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs sort failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let view = read_pcd(&output);
    assert_eq!(view.get_f64(0, &DimId::Y), 0.0);
    assert_eq!(view.get_f64(1, &DimId::Y), 0.0);
    assert_eq!(view.get_f64(2, &DimId::Y), 1.0);
}

#[test]
fn sort_supports_driver_and_path_options() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-sort-driver-override");
    let extensionless_input = temp.join("simple_text_without_extension");
    let output = temp.join("out.pcd");
    fs::copy(&input, &extensionless_input).unwrap();

    let result = run_sort(&[
        "--driver",
        "readers.ply",
        "--input",
        extensionless_input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--filters.sort.order=desc",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs sort failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let view = read_pcd(&output);
    assert_eq!(view.get_f64(0, &DimId::X), 1.0);
    assert_eq!(view.get_f64(2, &DimId::X), -1.0);
}

#[test]
fn sort_without_paths_prints_usage_and_fails() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("sort")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("pdal sort <input> <output>"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_sort_matches_rust_sort() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-sort-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");

    // Both default to ascending X; `pdal sort`'s default matches.
    let installed = Command::new("pdal")
        .arg("sort")
        .arg(&input)
        .arg(&installed_output)
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal sort failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = run_sort(&[input.to_str().unwrap(), rust_output.to_str().unwrap()]);
    assert!(
        rust.status.success(),
        "pdal-rs sort failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );

    let installed_view = read_pcd(&installed_output);
    let rust_view = read_pcd(&rust_output);
    assert_eq!(rust_view.len(), installed_view.len());
    for point in 0..rust_view.len() {
        for dim in [DimId::X, DimId::Y, DimId::Z] {
            assert_eq!(
                rust_view.get_f64(point, &dim),
                installed_view.get_f64(point, &dim),
                "dimension {dim:?} differs at point {point}"
            );
        }
    }
}
