use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use pdal_core::point::DimId;

mod common;

use common::*;

#[test]
fn root_argument_errors_and_driver_table_run() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--bogus-root-option")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Unexpected argument"));

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--drivers")
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("readers.las"));

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--label")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--label requires"));

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .args(["--options", "all"])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("readers.las"));
}

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
fn pipeline_command_streams_eligible_and_falls_back_otherwise() {
    let temp = make_temp_dir("pdal-rs-pipeline-streaming");

    // faux -> range -> null is a fully streamable linear chain: the executor
    // takes the chunked streaming path.
    let streamable = temp.join("streamable.json");
    fs::write(
        &streamable,
        r#"{"pipeline":[
            {"type":"readers.faux","count":50000,"mode":"ramp","bounds":"([0,1000],[0,1000],[0,1000])"},
            {"type":"filters.range","limits":"X[0:500]"},
            {"type":"writers.null"}
        ]}"#,
    )
    .unwrap();

    // faux -> sort -> null is not streamable (sort needs all points); the
    // executor must fall back to the materializing path and still succeed.
    let fallback = temp.join("fallback.json");
    fs::write(
        &fallback,
        r#"{"pipeline":[
            {"type":"readers.faux","count":50000,"mode":"ramp","bounds":"([0,1000],[0,1000],[0,1000])"},
            {"type":"filters.sort","dimension":"X"},
            {"type":"writers.null"}
        ]}"#,
    )
    .unwrap();

    for path in [&streamable, &fallback] {
        let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
            .arg("pipeline")
            .arg(path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "pdal-rs pipeline failed for {}\nstderr:\n{}",
            path.display(),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg(&streamable)
        .arg("--nostream")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "pdal-rs pipeline --nostream failed\nstderr:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg("--validate")
        .arg(&fallback)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "pdal-rs pipeline --validate failed\nstderr:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let validation: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(validation["valid"], true);
    assert_eq!(validation["streamable"], false);

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg(&fallback)
        .arg("--stream")
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "pdal-rs pipeline --stream should reject a nonstreamable pipeline"
    );

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg(&streamable)
        .arg("--stream")
        .arg("--nostream")
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "pdal-rs pipeline should reject --stream with --nostream"
    );
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
fn pipeline_command_accepts_input_option() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let temp = make_temp_dir("pdal-rs-pipeline-input-option");
    let output = temp.join("out.txt");
    let pipeline = temp.join("pipeline.json");

    write_text_pipeline(&pipeline, &input, &output);

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg("--input")
        .arg(&pipeline)
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "pdal-rs pipeline failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(fs::read_to_string(output).unwrap().starts_with("X,Y,Z\n"));
}

#[test]
fn pipeline_command_accepts_stdin_and_validate() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let temp = make_temp_dir("pdal-rs-pipeline-stdin");
    let output = temp.join("out.txt");
    let pipeline = temp.join("pipeline.json");

    write_text_pipeline(&pipeline, &input, &output);
    let pipeline_json = fs::read_to_string(&pipeline).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg("--stdin")
        .arg("--validate")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(pipeline_json.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();

    assert!(
        result.status.success(),
        "pdal-rs pipeline validate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!output.exists());
}

#[test]
fn pipeline_command_writes_metadata_and_serialization_files() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/text/utm17_1.txt");
    let temp = make_temp_dir("pdal-rs-pipeline-sidecars");
    let output = temp.join("out.txt");
    let pipeline = temp.join("pipeline.json");
    let metadata = temp.join("metadata.json");
    let serialization = temp.join("serialized.json");

    write_text_pipeline(&pipeline, &input, &output);

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg(&pipeline)
        .arg("--metadata")
        .arg(&metadata)
        .arg("--pipeline-serialization")
        .arg(&serialization)
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "pdal-rs pipeline failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let metadata_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&metadata).unwrap()).unwrap();
    assert!(metadata_json["point_count"].is_number());
    assert!(metadata_json["metadata"].is_object());
    let serialized_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&serialization).unwrap()).unwrap();
    assert_eq!(serialized_json["pipeline"][0]["type"], "readers.text");
    assert_eq!(serialized_json["pipeline"][0]["tag"], "readers_text1");
    assert_eq!(serialized_json["pipeline"][1]["type"], "filters.decimation");
    assert_eq!(serialized_json["pipeline"][1]["tag"], "filters_decimation1");
    assert_eq!(serialized_json["pipeline"][2]["type"], "writers.text");
    assert_eq!(serialized_json["pipeline"][2]["tag"], "writers_text1");
    assert!(fs::read_to_string(output).unwrap().starts_with("X,Y,Z\n"));
}

#[test]
fn pipeline_command_progress_uses_filespec_writer_path() {
    let temp = make_temp_dir("pdal-rs-pipeline-filespec-progress");
    let output = temp.join("out.txt");
    let progress = temp.join("progress.txt");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"{{
  "pipeline": [
    {{"type":"readers.faux","count":1}},
    {{"type":"writers.text","filename":{{"path":"{}"}}}}
  ]
}}"#,
            escape_json_path(&output)
        ),
    )
    .unwrap();
    fs::write(&progress, "").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("pipeline")
        .arg(&pipeline)
        .arg("--progress")
        .arg(&progress)
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "pdal-rs pipeline failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let progress_text = fs::read_to_string(progress).unwrap();
    assert!(progress_text.contains(&format!("READYFILE:{}", output.display())));
    assert!(progress_text.contains(&format!("DONEFILE:{}", output.display())));
    assert!(!progress_text.contains("READYPIPELINE"));
    assert!(!progress_text.contains("DONEPIPELINE"));
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
fn pipeline_command_reads_tindex_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-pipeline-tindex");
    let source_copy = temp.join("simple_text.ply");
    fs::copy(&input, &source_copy).unwrap();
    let index = temp.join("index.geojson");
    let output = temp.join("out.pcd");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &index,
        r#"{
  "type": "FeatureCollection",
  "features": [
    {"type":"Feature","properties":{"location":"simple_text.ply"},"geometry":null},
    {"type":"Feature","properties":{"location":"simple_text.ply"},"geometry":null}
  ]
}"#,
    )
    .unwrap();
    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.tindex","filename":"{}"}},
  {{"type":"writers.pcd","filename":"{}"}}
]"#,
            escape_json_path(&index),
            escape_json_path(&output)
        ),
    )
    .unwrap();

    run_rust_pipeline(&pipeline);

    assert_eq!(read_pcd(&output).len(), 6);
}

#[test]
fn pipeline_command_reads_stac_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("pdal-rs-pipeline-stac");
    fs::copy(&input, temp.join("simple_text.ply")).unwrap();
    let item = temp.join("item.json");
    let output = temp.join("out.pcd");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &item,
        r#"{
  "type": "Feature",
  "assets": {
    "data": {"href": "simple_text.ply", "type": "application/octet-stream"}
  }
}"#,
    )
    .unwrap();
    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.stac","filename":"{}"}},
  {{"type":"writers.pcd","filename":"{}"}}
]"#,
            escape_json_path(&item),
            escape_json_path(&output)
        ),
    )
    .unwrap();

    run_rust_pipeline(&pipeline);

    assert_eq!(read_pcd(&output).len(), 3);
}

#[test]
fn pipeline_command_writes_ogr_geojson_pipeline() {
    let temp = make_temp_dir("pdal-rs-pipeline-ogr");
    let output = temp.join("out.geojson");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.faux","count":2,"mode":"ramp","minx":1,"maxx":2,"miny":3,"maxy":4,"minz":5,"maxz":6}},
  {{"type":"writers.ogr","filename":"{}","ogrdriver":"GeoJSON"}}
]"#,
            escape_json_path(&output)
        ),
    )
    .unwrap();

    run_rust_pipeline(&pipeline);

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert_eq!(json["type"], "FeatureCollection");
    assert_eq!(json["features"].as_array().unwrap().len(), 2);
    assert_eq!(json["features"][0]["geometry"]["coordinates"][0], 1.0);
}

#[test]
fn pipeline_command_reads_copc_filename_stage() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/copc/1.2-with-color.copc.laz");
    let temp = make_temp_dir("pdal-rs-pipeline-copc");
    let output = temp.join("out.pcd");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  "{}",
  {{"type":"writers.pcd","filename":"{}"}}
]"#,
            escape_json_path(&input),
            escape_json_path(&output)
        ),
    )
    .unwrap();

    run_rust_pipeline(&pipeline);

    assert_eq!(read_pcd(&output).len(), 1065);
}

#[test]
fn pipeline_command_reads_ept_filename_stage() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/ept/1.2-with-color/ept.json");
    let temp = make_temp_dir("pdal-rs-pipeline-ept");
    let output = temp.join("out.pcd");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  "{}",
  {{"type":"writers.pcd","filename":"{}"}}
]"#,
            escape_json_path(&input),
            escape_json_path(&output)
        ),
    )
    .unwrap();

    run_rust_pipeline(&pipeline);

    assert_eq!(read_pcd(&output).len(), 1065);
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
        "chamfer\ndelta\ndensity\neval\nfauxplugin\nground\nhausdorff\ninfo\nlasdump\nnitfwrap\nmerge\npipeline\nrandom\nsort\nsplit\ntile\ntindex\ntranslate\n"
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
    assert!(json
        .as_array()
        .unwrap()
        .iter()
        .any(|kernel| kernel["full_name"] == "kernels.fauxplugin"));
}

#[test]
fn fauxplugin_kernel_matches_existing_plugin_output() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("fauxplugin")
        .arg("7")
        .output()
        .unwrap();

    assert!(result.status.success());
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "FauxPluginKernel running.\n"
    );
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

#[test]
fn stage_options_reports_scoped_ept_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("readers.ept")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"filename"));
    assert!(args.contains(&"bounds"));
    assert!(args.contains(&"resolution"));
    assert!(args.contains(&"origin"));
    assert!(args.contains(&"ignore_unreadable"));
}

#[test]
fn stage_options_reports_scoped_gdal_reader_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("readers.gdal")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"filename"));
    assert!(args.contains(&"header"));
    assert!(args.contains(&"gdalopts"));
}

#[test]
fn stage_options_reports_scoped_gdal_writer_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("writers.gdal")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"filename"));
    assert!(args.contains(&"data_type"));
    assert!(args.contains(&"bounds"));
    assert!(args.contains(&"override_srs"));
    assert!(args.contains(&"default_srs"));
    assert!(args.contains(&"metadata"));
}

#[test]
fn stage_options_reports_scoped_text_reader_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("readers.text")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"filename"));
    assert!(args.contains(&"separator"));
    assert!(args.contains(&"header"));
    assert!(args.contains(&"skip"));
}

#[test]
fn stage_options_reports_scoped_hexbin_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("filters.hexbin")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"sample_size"));
    assert!(args.contains(&"threshold"));
    assert!(args.contains(&"edge_size"));
    assert!(args.contains(&"edge_length"));
    assert!(args.contains(&"density"));
}

#[test]
fn stage_options_reports_scoped_smrf_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("filters.smrf")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"cell"));
    assert!(args.contains(&"slope"));
    assert!(args.contains(&"scalar"));
    assert!(args.contains(&"threshold"));
    assert!(args.contains(&"window"));
    assert!(args.contains(&"returns"));
    assert!(args.contains(&"ground_class"));
    assert!(args.contains(&"other_class"));
    assert!(args.contains(&"only_ground"));
}

#[test]
fn stage_options_reports_scoped_pcd_writer_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("writers.pcd")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"compression"));
    assert!(args.contains(&"keep_unspecified"));
}

#[test]
fn stage_options_do_not_leak_las_writer_options_to_other_writers() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdal-rs"))
        .arg("--showjson")
        .arg("--options")
        .arg("writers.bpf")
        .output()
        .unwrap();

    assert!(result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let args: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["arg"].as_str().unwrap())
        .collect();
    assert!(args.contains(&"compression"));
    assert!(args.contains(&"bundledfile"));
    assert!(!args.contains(&"point_format"));
}
