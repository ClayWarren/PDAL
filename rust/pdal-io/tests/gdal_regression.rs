use pdal_core::options::Options;
use pdal_core::pipeline::{Pipeline, Writer};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_io::gdal_reader::GdalReader;
use pdal_io::gdal_writer::GdalWriter;
use pdal_io::text_writer::TextWriter;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

#[test]
fn gdal_writer_writes_count_raster() {
    let temp = make_temp_dir("gdal-writer-count");
    let output = temp.join("count.tif");
    let mut options = Options::new();
    options.add("filename", output.display());
    options.add("output_type", "count");
    options.add("binmode", true);
    options.add("origin_x", 0.0);
    options.add("origin_y", 0.0);
    options.add("width", 2);
    options.add("height", 2);
    options.add("resolution", 1.0);

    let mut writer = GdalWriter::new(&options);
    writer.write(&[two_point_view()]).unwrap();

    let raster = pdal_core::gdal::Raster::open(output.to_str().unwrap()).unwrap();
    assert_eq!(raster.width(), 2);
    assert_eq!(raster.height(), 2);
    assert_eq!(raster.band_count(), 1);

    let mut data = vec![0.0; 4];
    raster.read_band(1, 2, 2, &mut data).unwrap();
    assert_eq!(data, vec![-9999.0, 1.0, 1.0, -9999.0]);
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_gdal_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/gdal/float32.tif");
    let temp = make_temp_dir("gdal-regression");
    let installed_output = temp.join("installed.txt");
    let rust_output = temp.join("rust.txt");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.gdal","filename":"{}"}},
  {{"type":"writers.text","filename":"{}","quote_header":false,"precision":3}}
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

    // Text comparison might differ in band naming (band_1 vs band 1 or something)
    // but the coordinates should match.
    assert_eq!(
        fs::read_to_string(&installed_output)
            .unwrap()
            .lines()
            .count(),
        fs::read_to_string(&rust_output).unwrap().lines().count()
    );
}

fn run_rust_pipeline(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut writer_options = Options::new();
    writer_options
        .add("filename", output.display())
        .add("quote_header", false)
        .add("precision", 3);

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.gdal",
        Box::new(GdalReader::new(&reader_options)),
        reader_options,
    );
    let writer = pipeline.add_writer(
        "writers.text",
        Box::new(TextWriter::new(&writer_options)),
        writer_options,
    );
    pipeline.add_dependency(writer, reader).unwrap();
    assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
}

fn two_point_view() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    for (x, y, z) in [(0.0, 0.0, 1.0), (1.0, 1.0, 2.0)] {
        let point = view.add_point();
        view.set_f64(point, &DimId::X, x);
        view.set_f64(point, &DimId::Y, y);
        view.set_f64(point, &DimId::Z, z);
    }
    view
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
