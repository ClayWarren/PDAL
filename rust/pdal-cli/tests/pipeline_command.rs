use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, PointView};
use pdal_io::pcd::PcdReader;
use pdal_io::ply::PlyReader;

#[test]
fn pipeline_command_runs_text_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let temp = make_temp_dir("pdal-rs-pipeline-command");
    let output = temp.join("out.txt");
    let pipeline = temp.join("pipeline.json");

    write_text_pipeline(&pipeline, &input, &output);

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg(&pipeline)
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "pdal-rs pipeline failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let text = fs::read_to_string(output).unwrap();
    assert!(text.starts_with("X,Y,Z\n"));
    assert!(text.lines().count() > 1);
}

#[test]
fn pipeline_command_accepts_root_pipeline_object() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let temp = make_temp_dir("pdal-rs-pipeline-command-object");
    let output = temp.join("out.txt");
    let pipeline = temp.join("pipeline.json");

    write_text_pipeline_object(&pipeline, &input, &output);

    run_rust_pipeline(&pipeline);

    let text = fs::read_to_string(output).unwrap();
    assert!(text.starts_with("X,Y,Z\n"));
    assert!(text.lines().count() > 1);
}

#[test]
fn pipeline_command_accepts_filename_string_stages() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let temp = make_temp_dir("pdal-rs-pipeline-command-string");
    let output = temp.join("out.pcd");
    let pipeline = temp.join("pipeline.json");

    write_text_pipeline_strings(&pipeline, &input, &output);

    run_rust_pipeline(&pipeline);

    assert_eq!(read_pcd(&output).len(), 5);
}

#[test]
fn pipeline_command_runs_sort_filter() {
    let temp = make_temp_dir("pdal-rs-pipeline-command-sort");
    let output = temp.join("out.pcd");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.faux","count":4,"mode":"ramp","minx":1,"maxx":4}},
  {{"type":"filters.sort","dimensions":"X","order":"desc"}},
  {{"type":"writers.pcd","filename":"{}","order":"X,Y,Z","precision":2}}
]
"#,
            escape_json_path(&output)
        ),
    )
    .unwrap();

    run_rust_pipeline(&pipeline);

    let view = read_pcd(&output);
    assert_eq!(view.len(), 4);
    assert_eq!(view.get_f64(0, &DimId::X), 4.0);
    assert_eq!(view.get_f64(3, &DimId::X), 1.0);
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_pipeline_command() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let temp = make_temp_dir("pdal-rs-pipeline-command-regression");
    let installed_output = temp.join("installed.txt");
    let rust_output = temp.join("rust.txt");
    let installed_pipeline = temp.join("installed-pipeline.json");
    let rust_pipeline = temp.join("rust-pipeline.json");

    write_text_pipeline(&installed_pipeline, &input, &installed_output);
    write_text_pipeline(&rust_pipeline, &input, &rust_output);

    let installed = Command::new("pdal")
        .arg("pipeline")
        .arg(&installed_pipeline)
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        installed.status.success(),
        "installed pdal pipeline failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );

    let rust = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg(&rust_pipeline)
        .output()
        .expect("failed to execute pdal-rs");
    assert!(
        rust.status.success(),
        "pdal-rs pipeline failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust.stdout),
        String::from_utf8_lossy(&rust.stderr)
    );

    assert_eq!(
        fs::read_to_string(installed_output).unwrap(),
        fs::read_to_string(rust_output).unwrap()
    );
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_root_object_pipeline_command() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let temp = make_temp_dir("pdal-rs-root-object-pipeline-command-regression");
    let installed_output = temp.join("installed.txt");
    let rust_output = temp.join("rust.txt");
    let installed_pipeline = temp.join("installed-pipeline.json");
    let rust_pipeline = temp.join("rust-pipeline.json");

    write_text_pipeline_object(&installed_pipeline, &input, &installed_output);
    write_text_pipeline_object(&rust_pipeline, &input, &rust_output);

    run_installed_pipeline(&installed_pipeline);
    run_rust_pipeline(&rust_pipeline);

    assert_eq!(
        fs::read_to_string(installed_output).unwrap(),
        fs::read_to_string(rust_output).unwrap()
    );
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_filename_string_pipeline_command() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let temp = make_temp_dir("pdal-rs-string-pipeline-command-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");
    let installed_pipeline = temp.join("installed-pipeline.json");
    let rust_pipeline = temp.join("rust-pipeline.json");

    write_text_pipeline_strings(&installed_pipeline, &input, &installed_output);
    write_text_pipeline_strings(&rust_pipeline, &input, &rust_output);

    run_installed_pipeline(&installed_pipeline);
    run_rust_pipeline(&rust_pipeline);

    assert_views_match_xyz(read_pcd(&installed_output), read_pcd(&rust_output));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_pcd_pipeline_command() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/pcd/utm17_space.pcd");
    let temp = make_temp_dir("pdal-rs-pcd-pipeline-command-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");
    let installed_pipeline = temp.join("installed-pipeline.json");
    let rust_pipeline = temp.join("rust-pipeline.json");

    write_pcd_pipeline(&installed_pipeline, &input, &installed_output);
    write_pcd_pipeline(&rust_pipeline, &input, &rust_output);

    run_installed_pipeline(&installed_pipeline);
    run_rust_pipeline(&rust_pipeline);

    assert_views_match_xyz(read_pcd(&installed_output), read_pcd(&rust_output));
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_ply_pipeline_command() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-ply-pipeline-command-regression");
    let installed_output = temp.join("installed.ply");
    let rust_output = temp.join("rust.ply");
    let installed_pipeline = temp.join("installed-pipeline.json");
    let rust_pipeline = temp.join("rust-pipeline.json");

    write_ply_pipeline(&installed_pipeline, &input, &installed_output);
    write_ply_pipeline(&rust_pipeline, &input, &rust_output);

    run_installed_pipeline(&installed_pipeline);
    run_rust_pipeline(&rust_pipeline);

    assert_views_match_xyz(read_ply(&installed_output), read_ply(&rust_output));
}

#[test]
fn pipeline_command_supports_json_summary() {
    let temp = make_temp_dir("pdal-rs-pipeline-summary");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        r#"[
  {"type":"readers.faux","count":3,"mode":"ramp","minx":-10,"maxx":20,"miny":-15,"maxy":7,"minz":-50,"maxz":100}
]
"#,
    )
    .unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("pipeline")
        .arg(&pipeline)
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "pdal-rs pipeline failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["point_count"], 3);
    assert_eq!(json["view_count"], 1);
    assert_eq!(json["bounds_2d"]["minx"], -10.0);
    assert_eq!(json["bounds_2d"]["maxx"], 20.0);
    assert_eq!(json["bounds_3d"]["minz"], -50.0);
    assert_eq!(json["bounds_3d"]["maxz"], 100.0);
    assert_eq!(json["dimension_summaries"][0]["name"], "X");
    assert_eq!(json["dimension_summaries"][0]["count"], 3);
    assert_eq!(json["dimension_summaries"][0]["minimum"], -10.0);
    assert_eq!(json["dimension_summaries"][0]["maximum"], 20.0);
    assert_eq!(json["dimension_summaries"][0]["mean"], 5.0);
    // Flat metadata summary
    assert_eq!(json["metadata"]["stage_0"]["count"], 3);
}

#[test]
fn unknown_command_fails_cleanly() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("bogus-command")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("Unknown Rust command 'bogus-command'")
    );
}

#[test]
fn command_local_help_succeeds() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("tindex")
        .arg("--help")
        .output()
        .unwrap();

    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("pdal tindex create"));
    assert!(stdout.contains("--filelist"));
}

#[test]
fn list_commands_reports_rust_commands() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--list-commands")
        .output()
        .unwrap();

    assert!(result.status.success());
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "chamfer\ndelta\ndensity\neval\nground\nhausdorff\ninfo\nmerge\npipeline\nrandom\nsort\nsplit\ntile\ntindex\ntranslate\n"
    );
}

#[test]
fn version_supports_json_native_dependency_report() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--version")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["name"], "pdal-rs");
    assert!(json["native_dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dependency| dependency["name"] == "PROJ"));
}

#[test]
fn list_commands_supports_json() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--list-commands")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json[0]["name"], "chamfer");
    assert_eq!(json[0]["full_name"], "kernels.chamfer");
    assert_eq!(json[1]["name"], "delta");
    assert_eq!(json[1]["full_name"], "kernels.delta");
}

#[test]
fn stage_options_reports_rust_metadata() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--options")
        .arg("filters.decimation")
        .output()
        .unwrap();

    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("filters.decimation"));
    assert!(stdout.contains("step"));
    assert!(stdout.contains("Keep every Nth point."));
}

#[test]
fn stage_options_supports_json() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("writers.ply")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(json
        .as_array()
        .unwrap()
        .iter()
        .any(|option| option["arg"] == "faces"));
}

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_text_pipeline(pipeline: &Path, input: &Path, output: &Path) {
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

fn write_text_pipeline_object(pipeline: &Path, input: &Path, output: &Path) {
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

fn write_text_pipeline_strings(pipeline: &Path, input: &Path, output: &Path) {
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

fn write_pcd_pipeline(pipeline: &Path, input: &Path, output: &Path) {
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

fn write_ply_pipeline(pipeline: &Path, input: &Path, output: &Path) {
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

fn run_installed_pipeline(pipeline: &Path) {
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

fn run_rust_pipeline(pipeline: &Path) {
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

fn read_pcd(path: &Path) -> PointView {
    let mut options = Options::new();
    options.add("filename", path.display());
    PcdReader::new(&options).read().unwrap().pop().unwrap()
}

fn read_ply(path: &Path) -> PointView {
    let mut options = Options::new();
    options.add("filename", path.display());
    PlyReader::new(&options).read().unwrap().pop().unwrap()
}

fn assert_views_match_xyz(installed: PointView, rust: PointView) {
    assert_eq!(rust.len(), installed.len());
    for point in 0..rust.len() {
        for dim in [DimId::X, DimId::Y, DimId::Z] {
            assert_eq!(rust.get_f64(point, &dim), installed.get_f64(point, &dim));
        }
    }
}

fn escape_json_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
