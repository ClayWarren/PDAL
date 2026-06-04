use pdal_core::options::Options;
use pdal_core::pipeline::Pipeline;
use pdal_filters::head::HeadFilter;
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

    // Check decoded SRS metadata or the LAS projection VLR path.
    assert!(has_wgs84_srs(&rust_info), "rust output missing SRS");
    assert!(
        has_wgs84_srs(&installed_info),
        "installed output missing SRS"
    );

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
    let mut head_options = Options::new();
    head_options.add("count", 100);
    let head = pipeline.add_stage(
        "filters.head",
        Box::new(pdal_core::pipeline::FilterWrapper::new(HeadFilter::new(
            100, false,
        ))),
        head_options,
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
    pipeline.add_dependency(head, reader).unwrap();
    pipeline.add_dependency(filter, head).unwrap();
    pipeline.add_dependency(writer, filter).unwrap();
    assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
}

fn get_pdal_info(path: &Path) -> serde_json::Value {
    let output = Command::new("pdal")
        .env_remove("DYLD_LIBRARY_PATH")
        .env_remove("DYLD_FALLBACK_LIBRARY_PATH")
        .env_remove("LD_LIBRARY_PATH")
        .arg("info")
        .arg("--metadata")
        .arg(path)
        .output()
        .expect("failed to execute pdal info");
    serde_json::from_slice(&output.stdout).expect("failed to parse pdal info JSON")
}

fn has_wgs84_srs(info: &serde_json::Value) -> bool {
    let metadata = &info["metadata"];
    let srs = &metadata["srs"];
    for key in ["compoundwkt", "horizontal", "wkt"] {
        if srs[key]
            .as_str()
            .is_some_and(|wkt| wkt.contains("4326") || wkt.contains("WGS 84"))
        {
            return true;
        }
    }

    metadata["spatialreference"]
        .as_str()
        .is_some_and(|wkt| wkt.contains("4326") || wkt.contains("WGS 84"))
        || has_projection_vlr(&metadata["stage_0"])
}

fn has_projection_vlr(stage: &serde_json::Value) -> bool {
    stage.as_object().is_some_and(|metadata| {
        metadata.iter().any(|(key, value)| {
            key.starts_with("vlr_")
                && value["record_id"].as_u64() == Some(2112)
                && value["user_id"]
                    .as_str()
                    .is_some_and(|user_id| matches!(user_id, "LASF_Projection" | "liblas"))
        })
    })
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
