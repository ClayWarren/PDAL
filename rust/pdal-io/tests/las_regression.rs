use pdal_core::options::Options;
use pdal_core::pipeline::{Pipeline, Writer};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_io::las::LasReader;
use pdal_io::las_writer::LasWriter;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

#[test]
fn las_writer_honors_compression_option_for_las_extension() {
    let temp = make_temp_dir("las-compression-option");
    let output = temp.join("compressed-with-las-extension.las");
    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());
    writer_options.add("compression", true);

    let mut writer = LasWriter::new(&writer_options);
    writer.write(&[single_point_view()]).unwrap();

    let reader = las::Reader::from_path(&output).unwrap();
    assert!(reader.header().point_format().is_compressed);
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_las_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/autzen/autzen-utm.las");
    let temp = make_temp_dir("las-regression");
    let installed_output = temp.join("installed.las");
    let rust_output = temp.join("rust.las");
    let pipeline = temp.join("pipeline.json");

    // Use a small subset of points to keep the test fast
    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.las","filename":"{}"}},
  {{"type":"filters.head","count":100}},
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

    // For LAS, we compare the summary output rather than bit-parity,
    // because header metadata (generating software, date, etc.) and
    // floating point scaling might differ slightly while being behaviorally correct.
    let installed_info = get_pdal_info(&installed_output);
    let rust_info = get_pdal_info(&rust_output);

    assert_eq!(
        installed_info["stats"]["total_points"],
        rust_info["stats"]["total_points"]
    );
    assert_eq!(
        installed_info["stats"]["bbox"]["native"]["bbox"],
        rust_info["stats"]["bbox"]["native"]["bbox"]
    );

    // Verify SRS (if present in fixture)
    if let Some(installed_wkt) = installed_info["metadata"]["srs"]["wkt"].as_str() {
        let rust_wkt = rust_info["metadata"]["srs"]["wkt"]
            .as_str()
            .expect("rust output missing SRS");
        assert_eq!(installed_wkt, rust_wkt);
    }

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

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_laz_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/autzen/autzen-utm.las");
    let temp = make_temp_dir("laz-regression");
    let installed_output = temp.join("installed.laz");
    let rust_output = temp.join("rust.laz");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.las","filename":"{}"}},
  {{"type":"filters.head","count":100}},
  {{"type":"writers.las","filename":"{}","compression":true}}
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

    run_rust_pipeline_laz(&input, &rust_output);

    let installed_info = get_pdal_info(&installed_output);
    let rust_info = get_pdal_info(&rust_output);

    assert_eq!(
        installed_info["stats"]["total_points"],
        rust_info["stats"]["total_points"]
    );
    assert_eq!(
        installed_info["stats"]["bbox"]["native"]["bbox"],
        rust_info["stats"]["bbox"]["native"]["bbox"]
    );

    // Verify SRS (if present in fixture)
    if let Some(installed_wkt) = installed_info["metadata"]["srs"]["wkt"].as_str() {
        let rust_wkt = rust_info["metadata"]["srs"]["wkt"]
            .as_str()
            .expect("rust output missing SRS");
        assert_eq!(installed_wkt, rust_wkt);
    }

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
    // Use filters.head from pdal-filters
    let mut filter_options = Options::new();
    filter_options.add("count", 100);
    let filter = pipeline.add_stage(
        "filters.head",
        Box::new(pdal_core::pipeline::FilterWrapper::new(
            pdal_filters::head::HeadFilter::new(100, false),
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

fn run_rust_pipeline_laz(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());
    writer_options.add("compression", true);

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.las",
        Box::new(LasReader::new(&reader_options)),
        reader_options,
    );
    let mut filter_options = Options::new();
    filter_options.add("count", 100);
    let filter = pipeline.add_stage(
        "filters.head",
        Box::new(pdal_core::pipeline::FilterWrapper::new(
            pdal_filters::head::HeadFilter::new(100, false),
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

fn single_point_view() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    let point = view.add_point();
    view.set_f64(point, &DimId::X, 1.0);
    view.set_f64(point, &DimId::Y, 2.0);
    view.set_f64(point, &DimId::Z, 3.0);
    view
}

fn escape_json_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
