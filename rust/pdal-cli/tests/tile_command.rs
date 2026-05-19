use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_io::las::LasReader;
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

fn run_tile(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("tile")
        .args(args)
        .output()
        .unwrap()
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

fn las_len(path: &Path) -> usize {
    let mut options = Options::new();
    options.add("filename", path.display());
    LasReader::new(&options)
        .read()
        .unwrap()
        .pop()
        .unwrap()
        .len() as usize
}

/// File name -> point count for every tile written into `dir`.
fn tile_counts(dir: &Path, ext: &str, len: impl Fn(&Path) -> usize) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            counts.insert(name, len(&path));
        }
    }
    counts
}

#[test]
fn tile_splits_a_file_into_a_grid() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("pdal-rs-tile-grid");
    let template = temp.join("tile#.pcd");

    let result = run_tile(&[
        input.to_str().unwrap(),
        template.to_str().unwrap(),
        "--length=1000",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs tile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let counts = tile_counts(&temp, "pcd", pcd_len);
    // interesting.las spans more than one 1000-unit cell.
    assert!(
        counts.len() > 1,
        "expected several tiles, got {}",
        counts.len()
    );
    // The '#' placeholder is replaced by the cell's `<x>_<y>` coordinates.
    for name in counts.keys() {
        assert!(
            name.starts_with("tile") && name.ends_with(".pcd"),
            "unexpected tile name '{name}'"
        );
    }
    // With no buffer the tiles partition the cloud: every point lands once.
    assert_eq!(counts.values().sum::<usize>(), 1065);
}

#[test]
fn tile_requires_a_hash_template() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("pdal-rs-tile-no-hash");
    let output = temp.join("tile.pcd");

    let result = run_tile(&[input.to_str().unwrap(), output.to_str().unwrap()]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains('#'));
}

#[test]
fn tile_without_paths_prints_usage_and_fails() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("tile")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("pdal tile <input> <output"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_tile_matches_rust_tile() {
    let input = data_path("test/data/las/interesting.las");
    let temp = make_temp_dir("pdal-rs-tile-regression");
    let installed_dir = temp.join("installed");
    let rust_dir = temp.join("rust");
    fs::create_dir_all(&installed_dir).unwrap();
    fs::create_dir_all(&rust_dir).unwrap();

    let installed = Command::new("pdal")
        .arg("tile")
        .arg(&input)
        .arg(installed_dir.join("t#.las"))
        .arg("--length=1000")
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal tile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = run_tile(&[
        input.to_str().unwrap(),
        rust_dir.join("t#.las").to_str().unwrap(),
        "--length=1000",
    ]);
    assert!(rust.status.success());

    // The Rust tile kernel reproduces PDAL's grid: the same set of occupied
    // cells with the same per-tile point counts.
    assert_eq!(
        tile_counts(&rust_dir, "las", las_len),
        tile_counts(&installed_dir, "las", las_len)
    );
}
