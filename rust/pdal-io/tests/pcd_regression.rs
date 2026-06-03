use pdal_core::options::Options;
use pdal_core::pipeline::{FilterWrapper, Pipeline, Reader, Writer};
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_filters::decimation::DecimationFilter;
use pdal_io::pcd::{PcdReader, PcdWriter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_pcd_decimation_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/pcd/utm17_space.pcd");
    let temp = make_temp_dir("pcd-decimation-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.pcd","filename":"{}"}},
  {{"type":"filters.decimation","step":2}},
  {{"type":"writers.pcd","filename":"{}","order":"X,Y,Z","precision":2}}
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

    let installed = read_pcd(&installed_output);
    let rust = read_pcd(&rust_output);
    assert_eq!(rust.len(), installed.len());
    for point in 0..rust.len() {
        for dim in [DimId::X, DimId::Y, DimId::Z] {
            assert_eq!(rust.get_f64(point, &dim), installed.get_f64(point, &dim));
        }
    }
}

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_compressed_pcd_writer() {
    let temp = make_temp_dir("pcd-compressed-writer-regression");
    let input = temp.join("input.txt");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &input,
        "X,Y,Z,Intensity\n1,2,3,42\n4.5,5.5,6.5,43\n7.25,8.25,9.25,44\n",
    )
    .unwrap();

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.text","filename":"{}"}},
  {{"type":"writers.pcd","filename":"{}","order":"X=Float,Y=Float,Z=Float,Intensity=Unsigned16","compression":"compressed"}}
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

    let view = compressed_writer_view();
    let mut writer = PcdWriter::new(&compressed_writer_options(&rust_output));
    writer.write(std::slice::from_ref(&view)).unwrap();

    assert_contains_binary_compressed_marker(&installed_output);
    assert_contains_binary_compressed_marker(&rust_output);
    assert_compressed_writer_points_match(&read_pcd(&installed_output));
    assert_compressed_writer_points_match(&read_pcd(&rust_output));
}

fn run_rust_pipeline(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut filter_options = Options::new();
    filter_options.add("step", 2);
    let mut writer_options = Options::new();
    writer_options
        .add("filename", output.display())
        .add("order", "X,Y,Z")
        .add("precision", 2);

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.pcd",
        Box::new(PcdReader::new(&reader_options)),
        reader_options,
    );
    let filter = pipeline.add_stage(
        "filters.decimation",
        Box::new(FilterWrapper::new(DecimationFilter::new(&filter_options))),
        filter_options,
    );
    let writer = pipeline.add_writer(
        "writers.pcd",
        Box::new(PcdWriter::new(&writer_options)),
        writer_options,
    );
    pipeline.add_dependency(filter, reader).unwrap();
    pipeline.add_dependency(writer, filter).unwrap();
    assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
}

fn compressed_writer_options(output: &Path) -> Options {
    let mut options = Options::new();
    options
        .add("filename", output.display())
        .add("order", "X=Float,Y=Float,Z=Float,Intensity=Unsigned16")
        .add("compression", "compressed");
    options
}

fn compressed_writer_view() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Intensity, DimType::F64);

    let mut view = PointView::new(Rc::new(layout));
    for [x, y, z, intensity] in [
        [1.0, 2.0, 3.0, 42.0],
        [4.5, 5.5, 6.5, 43.0],
        [7.25, 8.25, 9.25, 44.0],
    ] {
        let point = view.add_point();
        view.set_f64(point, &DimId::X, x);
        view.set_f64(point, &DimId::Y, y);
        view.set_f64(point, &DimId::Z, z);
        view.set_f64(point, &DimId::Intensity, intensity);
    }
    view
}

fn read_pcd(path: &Path) -> pdal_core::point::PointView {
    let mut options = Options::new();
    options.add("filename", path.display());
    PcdReader::new(&options).read().unwrap().pop().unwrap()
}

fn assert_contains_binary_compressed_marker(path: &Path) {
    let written = fs::read(path).unwrap();
    assert!(written
        .windows(b"DATA binary_compressed\n".len())
        .any(|window| window == b"DATA binary_compressed\n"));
}

fn assert_compressed_writer_points_match(view: &PointView) {
    let expected = [
        [1.0, 2.0, 3.0, 42.0],
        [4.5, 5.5, 6.5, 43.0],
        [7.25, 8.25, 9.25, 44.0],
    ];
    assert_eq!(view.len(), expected.len() as u64);
    for (point, [x, y, z, intensity]) in expected.into_iter().enumerate() {
        let point = point as u64;
        assert_near(view.get_f64(point, &DimId::X), x);
        assert_near(view.get_f64(point, &DimId::Y), y);
        assert_near(view.get_f64(point, &DimId::Z), z);
        assert_eq!(view.get_f64(point, &DimId::Intensity), intensity);
    }
}

fn assert_near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {expected}, got {actual}"
    );
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
