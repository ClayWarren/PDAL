use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, PointView};
use pdal_io::pcd::PcdReader;
use pdal_io::ply::PlyReader;

pub(crate) fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub(crate) fn write_text_pipeline(pipeline: &Path, input: &Path, output: &Path) {
    fs::write(
        pipeline,
        format!(
            r#"[
  {{"type":"readers.text","filename":"{}"}},
  {{"type":"filters.decimation","step":2}},
  {{"type":"writers.text","filename":"{}","order":"X,Y,Z","quote_header":false,"precision":2}}
]
"#,
            escape_json_path(input),
            escape_json_path(output)
        ),
    )
    .unwrap();
}

pub(crate) fn write_text_pipeline_object(pipeline: &Path, input: &Path, output: &Path) {
    fs::write(
        pipeline,
        format!(
            r#"{{
  "pipeline": [
    {{"type":"readers.text","filename":"{}"}},
    {{"type":"filters.decimation","step":2}},
    {{"type":"writers.text","filename":"{}","order":"X,Y,Z","quote_header":false,"precision":2}}
  ]
}}
"#,
            escape_json_path(input),
            escape_json_path(output)
        ),
    )
    .unwrap();
}

pub(crate) fn write_text_pipeline_strings(pipeline: &Path, input: &Path, output: &Path) {
    fs::write(
        pipeline,
        format!(
            r#"[
  "{}",
  {{"type":"filters.decimation","step":2}},
  "{}"
]
"#,
            escape_json_path(input),
            escape_json_path(output)
        ),
    )
    .unwrap();
}

pub(crate) fn write_pcd_pipeline(pipeline: &Path, input: &Path, output: &Path) {
    fs::write(
        pipeline,
        format!(
            r#"[
  {{"type":"readers.pcd","filename":"{}"}},
  {{"type":"filters.decimation","step":2}},
  {{"type":"writers.pcd","filename":"{}","order":"X,Y,Z","precision":2}}
]
"#,
            escape_json_path(input),
            escape_json_path(output)
        ),
    )
    .unwrap();
}

pub(crate) fn write_ply_pipeline(pipeline: &Path, input: &Path, output: &Path) {
    fs::write(
        pipeline,
        format!(
            r#"[
  {{"type":"readers.ply","filename":"{}"}},
  {{"type":"filters.decimation","step":2}},
  {{"type":"writers.ply","filename":"{}","storage_mode":"ascii","precision":6}}
]
"#,
            escape_json_path(input),
            escape_json_path(output)
        ),
    )
    .unwrap();
}

pub(crate) fn run_installed_pipeline(pipeline: &Path) {
    let installed = Command::new("pdal")
        .arg("pipeline")
        .arg(pipeline)
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal pipeline failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );
}

pub(crate) fn run_rust_pipeline(pipeline: &Path) {
    let rust = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg(pipeline)
        .output()
        .expect("failed to execute pdal-rs");
    assert!(
        rust.status.success(),
        "pdal-rs pipeline failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );
}

pub(crate) fn read_pcd(path: &Path) -> PointView {
    let mut options = Options::new();
    options.add("filename", path.display());
    PcdReader::new(&options).read().unwrap().pop().unwrap()
}

pub(crate) fn read_ply(path: &Path) -> PointView {
    let mut options = Options::new();
    options.add("filename", path.display());
    PlyReader::new(&options).read().unwrap().pop().unwrap()
}

pub(crate) fn assert_views_match_xyz(installed: PointView, rust: PointView) {
    assert_eq!(rust.len(), installed.len());
    for point in 0..rust.len() {
        for dim in [DimId::X, DimId::Y, DimId::Z] {
            assert_eq!(rust.get_f64(point, &dim), installed.get_f64(point, &dim));
        }
    }
}

pub(crate) fn escape_json_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
