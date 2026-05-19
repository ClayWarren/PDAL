use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    assert_eq!(json["metadata"]["name"], "pipeline");
    assert_eq!(json["metadata"]["children"][0]["name"], "readers.faux");
    assert_eq!(
        json["metadata"]["children"][0]["children"][0]["name"],
        "count"
    );
    assert_eq!(json["metadata"]["children"][0]["children"][0]["value"], 3);
}

#[test]
fn unknown_command_fails_cleanly() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("info")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Unknown Rust command 'info'"));
}

#[test]
fn list_commands_reports_rust_commands() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--list-commands")
        .output()
        .unwrap();

    assert!(result.status.success());
    assert_eq!(String::from_utf8_lossy(&result.stdout), "pipeline\n");
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
    assert_eq!(json[0]["name"], "pipeline");
    assert_eq!(json[0]["full_name"], "kernels.pipeline");
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

fn escape_json_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
