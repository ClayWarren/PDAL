use pdal_core::options::Options;
use pdal_core::pipeline::{FilterWrapper, Pipeline, Reader};
use pdal_core::point::DimId;
use pdal_filters::decimation::DecimationFilter;
use pdal_io::ply::{PlyReader, PlyWriter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_ply_writer_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/ply/simple_text.ply");
    let temp = make_temp_dir("ply-writer-regression");
    let installed_output = temp.join("installed.ply");
    let rust_output = temp.join("rust.ply");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.ply","filename":"{}"}},
  {{"type":"filters.decimation","step":2}},
  {{"type":"writers.ply","filename":"{}","storage_mode":"ascii","precision":6}}
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

    let installed = read_ply(&installed_output);
    let rust = read_ply(&rust_output);
    assert_eq!(rust.len(), installed.len());
    for point in 0..rust.len() {
        for dim in [DimId::X, DimId::Y, DimId::Z] {
            assert_eq!(rust.get_f64(point, &dim), installed.get_f64(point, &dim));
        }
    }
}

fn run_rust_pipeline(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut filter_options = Options::new();
    filter_options.add("step", 2);
    let mut writer_options = Options::new();
    writer_options
        .add("filename", output.display())
        .add("storage_mode", "ascii")
        .add("precision", 6);

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.ply",
        Box::new(PlyReader::new(&reader_options)),
        reader_options,
    );
    let filter = pipeline.add_stage(
        "filters.decimation",
        Box::new(FilterWrapper::new(DecimationFilter::new(&filter_options))),
        filter_options,
    );
    let writer = pipeline.add_writer(
        "writers.ply",
        Box::new(PlyWriter::new(&writer_options).unwrap()),
        writer_options,
    );
    pipeline.add_dependency(filter, reader).unwrap();
    pipeline.add_dependency(writer, filter).unwrap();
    assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
}

fn read_ply(path: &Path) -> pdal_core::point::PointView {
    let mut options = Options::new();
    options.add("filename", path.display());
    PlyReader::new(&options).read().unwrap().pop().unwrap()
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
