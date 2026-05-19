use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::PointView;
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

fn run_split(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("split")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn split_chips_input_by_capacity() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-split-capacity");
    let output = temp.join("out.pcd");

    let result = run_split(&[
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "--capacity=2",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs split failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let first = temp.join("out_1.pcd");
    let second = temp.join("out_2.pcd");
    assert!(first.exists());
    assert!(second.exists());
    assert_eq!(read_pcd(&first).len() + read_pcd(&second).len(), 3);
}

#[test]
fn split_by_length_writes_numbered_outputs() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-split-length");
    let output = temp.join("out.pcd");

    let result = run_split(&[
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "--length=1",
        "--origin_x=0",
        "--origin_y=0",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs split failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let outputs: Vec<_> = fs::read_dir(&temp)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "pcd"))
        .collect();
    assert!(outputs.len() >= 2);
}

#[test]
fn split_rejects_length_and_capacity_together() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-split-invalid");
    let output = temp.join("out.pcd");

    let result = run_split(&[
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "--length=1",
        "--capacity=2",
    ]);

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("length and capacity"));
}

#[test]
fn split_without_paths_prints_usage_and_fails() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("split")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("pdal split <input>"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_split_matches_rust_split_capacity() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-split-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");

    let installed = Command::new("pdal")
        .arg("split")
        .arg(&input)
        .arg(&installed_output)
        .arg("--capacity=2")
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal split failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = run_split(&[
        input.to_str().unwrap(),
        rust_output.to_str().unwrap(),
        "--capacity=2",
    ]);
    assert!(
        rust.status.success(),
        "pdal-rs split failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );

    let installed_total = read_pcd(&temp.join("installed_1.pcd")).len()
        + read_pcd(&temp.join("installed_2.pcd")).len();
    let rust_total =
        read_pcd(&temp.join("rust_1.pcd")).len() + read_pcd(&temp.join("rust_2.pcd")).len();
    assert_eq!(rust_total, installed_total);
}
