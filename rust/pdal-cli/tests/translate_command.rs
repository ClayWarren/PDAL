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

fn run_translate(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("translate")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn translate_converts_ply_to_pcd() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-translate-convert");
    let output = temp.join("out.pcd");

    let result = run_translate(&[input.to_str().unwrap(), output.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "pdal-rs translate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let view = read_pcd(&output);
    assert_eq!(view.len(), 3);
    assert_eq!(view.get_f64(0, &DimId::X), -1.0);
    assert_eq!(view.get_f64(2, &DimId::X), 1.0);
}

#[test]
fn translate_applies_a_named_filter_with_stage_options() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-translate-filter");
    let output = temp.join("out.pcd");

    let result = run_translate(&[
        input.to_str().unwrap(),
        output.to_str().unwrap(),
        "decimation",
        "--filters.decimation.step=2",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs translate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    // 3 points, keeping every 2nd, yields 2.
    assert_eq!(read_pcd(&output).len(), 2);
}

#[test]
fn translate_supports_reader_writer_and_filter_options() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-translate-options");
    let extensionless_input = temp.join("input_without_extension");
    let output = temp.join("out_without_extension");
    fs::copy(&input, &extensionless_input).unwrap();

    let result = run_translate(&[
        "--reader",
        "readers.ply",
        "--input",
        extensionless_input.to_str().unwrap(),
        "--writer",
        "writers.pcd",
        "--output",
        output.to_str().unwrap(),
        "--filter",
        "decimation",
        "--filters.decimation.step=2",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs translate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    assert_eq!(read_pcd(&output).len(), 2);
}

#[test]
fn translate_enforces_stream_and_overwrite_options() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-translate-options");
    let stream_output = temp.join("stream.pcd");
    let sort_output = temp.join("sort.pcd");
    let same_io = temp.join("same.ply");
    fs::copy(&input, &same_io).unwrap();

    let result = run_translate(&[
        input.to_str().unwrap(),
        stream_output.to_str().unwrap(),
        "--stream",
    ]);
    assert!(
        result.status.success(),
        "pdal-rs translate --stream failed\nstderr:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let result = run_translate(&[
        input.to_str().unwrap(),
        sort_output.to_str().unwrap(),
        "filters.sort",
        "--stream",
    ]);
    assert!(!result.status.success());

    let result = run_translate(&[
        input.to_str().unwrap(),
        stream_output.to_str().unwrap(),
        "--stream",
        "--nostream",
    ]);
    assert!(!result.status.success());

    let same = same_io.to_str().unwrap();
    assert!(!run_translate(&[same, same]).status.success());
    assert!(run_translate(&[same, same, "--overwrite"]).status.success());
}

#[test]
fn translate_without_paths_prints_usage_and_fails() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("translate")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("pdal translate <input> <output>"));
}

#[test]
fn translate_rejects_an_unknown_input_extension() {
    let temp = make_temp_dir("pdal-rs-translate-unknown");
    let output = temp.join("out.pcd");
    let result = run_translate(&["mystery.xyzzy", output.to_str().unwrap()]);

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("reader driver"));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_translate_matches_rust_translate() {
    let input = data_path("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-translate-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");

    let installed = Command::new("pdal")
        .arg("translate")
        .arg(&input)
        .arg(&installed_output)
        .arg("--writers.pcd.precision=3")
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal translate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = run_translate(&[
        input.to_str().unwrap(),
        rust_output.to_str().unwrap(),
        "--writers.pcd.precision=3",
    ]);
    assert!(
        rust.status.success(),
        "pdal-rs translate failed\nstdout:\n{}\nstderr:\n{}",
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
