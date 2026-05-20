use pdal_core::options::Options;
use pdal_core::pipeline::{Pipeline, Reader, Writer};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_io::gdal_reader::GdalReader;
use pdal_io::gdal_writer::GdalWriter;
use pdal_io::text::TextReader;
use pdal_io::text_writer::TextWriter;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::Mutex;

static GDAL_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn gdal_writer_writes_count_raster() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
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
    assert_eq!(data, vec![0.0, 1.0, 1.0, 0.0]);
}

#[test]
fn gdal_writer_matches_existing_min_grid_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_fixture("min", false);
    assert_band_near(
        &output,
        &[
            5.0, -9999.0, 7.0, 8.0, 8.9, 4.0, -9999.0, 6.0, 7.0, 8.0, 3.0, 4.0, 5.0, 5.4, 6.4, 2.0,
            3.0, 4.0, 4.4, 5.4, 1.0, 2.0, 3.0, 4.0, 5.0,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_count_grid_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_fixture("count", false);
    assert_band_near(
        &output,
        &[
            1.0, 0.0, 1.0, 1.0, 3.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0, 2.0, 1.0, 1.0,
            2.0, 5.0, 4.0, 1.0, 1.0, 1.0, 2.0, 2.0,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_mean_grid_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_fixture("mean", false);
    assert_band_near(
        &output,
        &[
            5.0, -9999.0, 7.0, 8.0, 8.967, 4.0, -9999.0, 6.0, 7.0, 8.0, 3.0, 4.0, 5.0, 5.7, 6.7,
            2.0, 3.0, 4.2, 4.92, 5.8, 1.0, 2.0, 3.0, 4.2, 5.2,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_max_grid_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_fixture("max", false);
    assert_band_near(
        &output,
        &[
            5.0, -9999.0, 7.0, 8.0, 9.1, 4.0, -9999.0, 6.0, 7.0, 8.0, 3.0, 4.0, 5.0, 6.0, 7.0, 2.0,
            3.0, 4.4, 5.4, 6.4, 1.0, 2.0, 3.0, 4.4, 5.4,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_idw_grid_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_fixture("idw", false);
    assert_band_near(
        &output,
        &[
            5.0, -9999.0, 7.0, 8.0, 9.0, 4.0, -9999.0, 6.0, 7.0, 8.0, 3.0, 4.0, 5.0, 6.0, 7.0, 2.0,
            3.0, 4.0, 5.0, 6.0, 1.0, 2.0, 3.0, 4.0, 5.0,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_stdev_grid_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_fixture("stdev", false);
    assert_band_near(
        &output,
        &[
            0.0, -9999.0, 0.0, 0.0, 0.094, 0.0, -9999.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.3, 0.3,
            0.0, 0.0, 0.2, 0.449, 0.424, 0.0, 0.0, 0.0, 0.2, 0.2,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_percentile_grid_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_fixture("p50", true);
    assert_band_near(
        &output,
        &[
            5.0, -9999.0, 7.0, 8.0, 8.9, 4.0, -9999.0, 6.0, 7.0, 8.0, 3.0, 4.0, 5.0, 5.7, 6.7, 2.0,
            3.0, 4.0, 4.4, 5.4, 0.5, 2.0, 3.0, 4.0, 5.0,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_min_window_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_window_fixture("min");
    assert_band_near(
        &output,
        &[
            5.0, 5.457, 7.0, 8.0, 8.9, 4.0, 4.848, 6.0, 7.0, 8.0, 3.0, 4.0, 5.0, 5.4, 6.4, 2.0,
            3.0, 4.0, 4.4, 5.4, 1.0, 2.0, 3.0, 4.0, 5.0,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_max_window_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_window_fixture("max");
    assert_band_near(
        &output,
        &[
            5.0, 5.5, 7.0, 8.0, 9.1, 4.0, 4.942, 6.0, 7.0, 8.0, 3.0, 4.0, 5.0, 6.0, 7.0, 2.0, 3.0,
            4.4, 5.4, 6.4, 1.0, 2.0, 3.0, 4.4, 5.4,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_mean_window_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_window_fixture("mean");
    assert_band_near(
        &output,
        &[
            5.0, 5.478, 7.0, 8.0, 8.967, 4.0, 4.896, 6.0, 7.0, 8.0, 3.0, 4.0, 5.0, 5.7, 6.7, 2.0,
            3.0, 4.2, 4.92, 5.8, 1.0, 2.0, 3.0, 4.2, 5.2,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_idw_window_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_window_fixture("idw");
    assert_band_near(
        &output,
        &[
            5.0, 5.5, 7.0, 8.0, 9.0, 4.0, 4.905, 6.0, 7.0, 8.0, 3.0, 4.0, 5.0, 6.0, 7.0, 2.0, 3.0,
            4.0, 5.0, 6.0, 1.0, 2.0, 3.0, 4.0, 5.0,
        ],
    );
}

#[test]
fn gdal_writer_matches_existing_stdev_window_fixture() {
    let _guard = GDAL_TEST_LOCK.lock().unwrap();
    let output = write_grid_window_fixture("stdev");
    assert_band_near(
        &output,
        &[
            0.0, 0.021, 0.0, 0.0, 0.094, 0.0, 0.045, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.3, 0.3, 0.0,
            0.0, 0.2, 0.449, 0.424, 0.0, 0.0, 0.0, 0.2, 0.2,
        ],
    );
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

fn write_grid_fixture(output_type: &str, binmode: bool) -> PathBuf {
    write_grid_fixture_with_options(output_type, binmode, |_| {})
}

fn write_grid_window_fixture(output_type: &str) -> PathBuf {
    write_grid_fixture_with_options(output_type, false, |options| {
        options.add("window_size", 2);
    })
}

fn write_grid_fixture_with_options(
    output_type: &str,
    binmode: bool,
    configure: impl FnOnce(&mut Options),
) -> PathBuf {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/gdal/grid.txt");
    let temp = make_temp_dir(&format!("gdal-writer-{output_type}"));
    let output = temp.join("grid.tif");

    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut reader = TextReader::new(&reader_options);
    let views = reader.read().unwrap();

    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());
    writer_options.add("output_type", output_type);
    writer_options.add("resolution", 1.0);
    writer_options.add("radius", 7071.0 / 10000.0);
    writer_options.add("binmode", binmode);
    configure(&mut writer_options);
    let mut writer = GdalWriter::new(&writer_options);
    writer.write(&views).unwrap();
    output
}

fn assert_band_near(path: &Path, expected: &[f64]) {
    let raster = pdal_core::gdal::Raster::open(path.to_str().unwrap()).unwrap();
    assert_eq!(raster.width(), 5);
    assert_eq!(raster.height(), 5);
    let mut data = vec![0.0; expected.len()];
    raster
        .read_band(
            1,
            raster.width() as usize,
            raster.height() as usize,
            &mut data,
        )
        .unwrap();
    assert_eq!(data.len(), expected.len());
    for (idx, (actual, expected)) in data.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 0.001,
            "cell {idx}: actual {actual}, expected {expected}"
        );
    }
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
