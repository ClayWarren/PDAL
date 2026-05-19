use pdal_core::options::Options;
use pdal_core::pipeline::Pipeline;
use pdal_filters::reprojection::ReprojectionFilter;
use pdal_io::las::LasReader;
use pdal_io::las_writer::LasWriter;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_reprojection_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/autzen/autzen-utm.las");
    let temp = make_temp_dir("reprojection-regression");
    let installed_output = temp.join("installed.las");
    let rust_output = temp.join("rust.las");
    let pipeline = temp.join("pipeline.json");

    // Autzen-utm is EPSG:26910. Let's reproject to WGS84 (EPSG:4326).
    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.las","filename":"{}"}},
  {{"type":"filters.head","count":100}},
  {{"type":"filters.reprojection","out_srs":"EPSG:4326"}},
  {{"type":"writers.las","filename":"{}"}}
]
"#,
            escape_json_path(&input),
            escape_json_path(&installed_output)
        ),
    )
    .unwrap();

    let output = Command::new("pdal")
        .arg("pipeline")
        .arg(&pipeline)
        .output()
        .expect("failed to execute installed pdal");
    assert!(
        output.status.success(),
        "installed pdal failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    run_rust_pipeline(&input, &rust_output);

    let installed_info = get_pdal_info(&installed_output);
    let rust_info = get_pdal_info(&rust_output);

    assert_eq!(
        installed_info["stats"]["total_points"],
        rust_info["stats"]["total_points"]
    );

    // Check SRS WKT (installed PDAL uses 'horizontal' or 'compoundwkt', Rust uses 'wkt')
    let rust_srs = &rust_info["metadata"]["srs"];
    let rust_wkt = rust_srs["compoundwkt"]
        .as_str()
        .or_else(|| rust_srs["horizontal"].as_str())
        .or_else(|| rust_srs["wkt"].as_str())
        .expect("rust output missing SRS");
    assert!(rust_wkt.contains("4326") || rust_wkt.contains("WGS 84"));

    let installed_srs = &installed_info["metadata"]["srs"];
    let installed_wkt = installed_srs["compoundwkt"]
        .as_str()
        .or_else(|| installed_srs["horizontal"].as_str())
        .or_else(|| installed_srs["wkt"].as_str())
        .expect("installed output missing SRS");
    assert!(installed_wkt.contains("4326") || installed_wkt.contains("WGS 84"));

    // Verify some metadata
    assert_eq!(
        installed_info["metadata"]["major_version"],
        rust_info["metadata"]["major_version"]
    );
    assert_eq!(
        installed_info["metadata"]["minor_version"],
        rust_info["metadata"]["minor_version"]
    );
}

fn run_rust_pipeline(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.las",
        Box::new(LasReader::new(&reader_options)),
        reader_options,
    );
    let mut filter_options = Options::new();
    filter_options.add("out_srs", "EPSG:4326");
    let filter = pipeline.add_stage(
        "filters.reprojection",
        Box::new(pdal_core::pipeline::FilterWrapper::new(
            ReprojectionFilter::new("EPSG:4326", None, true),
        )),
        filter_options,
    );
    let writer = pipeline.add_writer(
        "writers.las",
        Box::new(LasWriter::new(&writer_options)),
        writer_options,
    );
    pipeline.add_dependency(filter, reader).unwrap();
    pipeline.add_dependency(writer, filter).unwrap();
    assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
}

fn get_pdal_info(path: &Path) -> serde_json::Value {
    let output = Command::new("pdal")
        .arg("info")
        .arg("--metadata")
        .arg(path)
        .output()
        .expect("failed to execute pdal info");
    serde_json::from_slice(&output.stdout).expect("failed to parse pdal info JSON")
}

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn escape_json_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
