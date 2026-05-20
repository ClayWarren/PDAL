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

fn run_merge(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("merge")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn merge_combines_two_inputs_into_one_output() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-merge-combine");
    let output = temp.join("out.pcd");

    // Merge a 3-point file with itself.
    let result = run_merge(&[
        input.to_str().unwrap(),
        input.to_str().unwrap(),
        output.to_str().unwrap(),
    ]);
    assert!(
        result.status.success(),
        "pdal-rs merge failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let view = read_pcd(&output);
    assert_eq!(view.len(), 6);
    // Points 0..3 are the first copy, 3..6 the second.
    assert_eq!(view.get_f64(0, &DimId::X), -1.0);
    assert_eq!(view.get_f64(3, &DimId::X), -1.0);
    assert_eq!(view.get_f64(5, &DimId::X), 1.0);
}

#[test]
fn merge_supports_driver_override_and_files_option() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-merge-driver-override");
    let extensionless_input = temp.join("simple_text_without_extension");
    let output = temp.join("out.pcd");
    fs::copy(&input, &extensionless_input).unwrap();

    let result = run_merge(&[
        "--driver",
        "readers.ply",
        "--files",
        extensionless_input.to_str().unwrap(),
        "--files",
        extensionless_input.to_str().unwrap(),
        output.to_str().unwrap(),
    ]);
    assert!(
        result.status.success(),
        "pdal-rs merge failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    assert_eq!(read_pcd(&output).len(), 6);
}

#[test]
fn merge_without_paths_prints_usage_and_fails() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("merge")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("pdal merge <input>"));
}

#[test]
fn merge_rejects_an_unknown_input_extension() {
    let temp = make_temp_dir("pdal-rs-merge-unknown");
    let output = temp.join("out.pcd");
    let result = run_merge(&["mystery.xyzzy", "other.xyzzy", output.to_str().unwrap()]);

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("reader driver"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_merge_matches_rust_merge() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-merge-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");

    let installed = Command::new("pdal")
        .arg("merge")
        .arg(&input)
        .arg(&input)
        .arg(&installed_output)
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal merge failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = run_merge(&[
        input.to_str().unwrap(),
        input.to_str().unwrap(),
        rust_output.to_str().unwrap(),
    ]);
    assert!(
        rust.status.success(),
        "pdal-rs merge failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );

    let installed_view = read_pcd(&installed_output);
    let rust_view = read_pcd(&rust_output);
    assert_eq!(rust_view.len(), installed_view.len());
    // simple_text.ply has integer coordinates, so X/Y/Z compare exactly.
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
