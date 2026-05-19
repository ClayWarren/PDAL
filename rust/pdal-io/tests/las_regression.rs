use pdal_core::options::Options;
use pdal_core::pipeline::{Pipeline, Reader, Writer};
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
fn las_writer_honors_format_scale_and_offset_options() {
    let temp = make_temp_dir("las-writer-header-options");
    let output = temp.join("header-options.las");
    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());
    writer_options.add("minor_version", 4);
    writer_options.add("dataformat_id", 7);
    writer_options.add("scale_x", 0.001);
    writer_options.add("scale_y", 0.002);
    writer_options.add("scale_z", 0.003);
    writer_options.add("offset_x", 100.0);
    writer_options.add("offset_y", 200.0);
    writer_options.add("offset_z", -50.0);

    let mut writer = LasWriter::new(&writer_options);
    writer.write(&[single_point_view_with_color()]).unwrap();

    let reader = las::Reader::from_path(&output).unwrap();
    let header = reader.header();
    assert_eq!(header.version().minor, 4);
    assert_eq!(header.point_format().to_u8().unwrap(), 7);
    assert_eq!(header.transforms().x.scale, 0.001);
    assert_eq!(header.transforms().y.scale, 0.002);
    assert_eq!(header.transforms().z.scale, 0.003);
    assert_eq!(header.transforms().x.offset, 100.0);
    assert_eq!(header.transforms().y.offset, 200.0);
    assert_eq!(header.transforms().z.offset, -50.0);
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

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_las_writer_header_options() {
    let temp = make_temp_dir("las-header-options-regression");
    let installed_output = temp.join("installed.las");
    let rust_output = temp.join("rust.las");
    let input = temp.join("input.las");
    write_rust_las_header_option_fixture(&input);

    let pipeline = temp.join("pipeline.json");
    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.las","filename":"{}"}},
  {{
    "type":"writers.las",
    "filename":"{}",
    "minor_version":4,
    "dataformat_id":7,
    "scale_x":0.001,
    "scale_y":0.002,
    "scale_z":0.003,
    "offset_x":100.0,
    "offset_y":200.0,
    "offset_z":-50.0
  }}
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

    write_rust_las_with_header_options(&input, &rust_output);

    let installed = las::Reader::from_path(&installed_output).unwrap();
    let rust = las::Reader::from_path(&rust_output).unwrap();
    assert_las_header_options_match(installed.header(), rust.header());
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

fn write_rust_las_header_option_fixture(output: &Path) {
    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());
    let mut writer = LasWriter::new(&writer_options);
    writer.write(&[single_point_view_with_color()]).unwrap();
}

fn write_rust_las_with_header_options(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut reader = LasReader::new(&reader_options);
    let views = reader.read().unwrap();

    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());
    writer_options.add("minor_version", 4);
    writer_options.add("dataformat_id", 7);
    writer_options.add("scale_x", 0.001);
    writer_options.add("scale_y", 0.002);
    writer_options.add("scale_z", 0.003);
    writer_options.add("offset_x", 100.0);
    writer_options.add("offset_y", 200.0);
    writer_options.add("offset_z", -50.0);
    let mut writer = LasWriter::new(&writer_options);
    writer.write(&views).unwrap();
}

fn assert_las_header_options_match(installed: &las::Header, rust: &las::Header) {
    assert_eq!(installed.version().minor, rust.version().minor);
    assert_eq!(
        installed.point_format().to_u8().unwrap(),
        rust.point_format().to_u8().unwrap()
    );
    assert_eq!(installed.transforms().x.scale, rust.transforms().x.scale);
    assert_eq!(installed.transforms().y.scale, rust.transforms().y.scale);
    assert_eq!(installed.transforms().z.scale, rust.transforms().z.scale);
    assert_eq!(installed.transforms().x.offset, rust.transforms().x.offset);
    assert_eq!(installed.transforms().y.offset, rust.transforms().y.offset);
    assert_eq!(installed.transforms().z.offset, rust.transforms().z.offset);
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

fn single_point_view_with_color() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Red, DimType::U16);
    layout.register(DimId::Green, DimType::U16);
    layout.register(DimId::Blue, DimType::U16);
    let mut view = PointView::new(Rc::new(layout));
    let point = view.add_point();
    view.set_f64(point, &DimId::X, 101.0);
    view.set_f64(point, &DimId::Y, 202.0);
    view.set_f64(point, &DimId::Z, -49.0);
    view.set_f64(point, &DimId::Red, 10.0);
    view.set_f64(point, &DimId::Green, 20.0);
    view.set_f64(point, &DimId::Blue, 30.0);
    view
}

fn escape_json_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
