use pdal_core::options::Options;
use pdal_core::pipeline::{Pipeline, Writer};
use pdal_core::point::{PointLayout, PointView};
use pdal_io::sbet::SbetReader;
use pdal_io::sbet_writer::SbetWriter;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_sbet_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/sbet/2-points.sbet");
    let temp = make_temp_dir("sbet-regression");
    let installed_output = temp.join("installed.sbet");
    let rust_output = temp.join("rust.sbet");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.sbet","filename":"{}","angles_as_degrees":false}},
  {{"type":"writers.sbet","filename":"{}","angles_are_degrees":false}}
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

    assert_eq!(
        fs::read(&installed_output).unwrap(),
        fs::read(&rust_output).unwrap()
    );
}

fn run_rust_pipeline(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    reader_options.add("angles_as_degrees", false);
    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());
    writer_options.add("angles_are_degrees", false);

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.sbet",
        Box::new(SbetReader::new(&reader_options)),
        reader_options,
    );
    let writer = pipeline.add_writer(
        "writers.sbet",
        Box::new(SbetWriter::new(&writer_options)),
        writer_options,
    );
    pipeline.add_dependency(writer, reader).unwrap();
    assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
}

#[test]
fn test_rust_sbet_pipeline_standalone() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/sbet/2-points.sbet");
    let temp = make_temp_dir("sbet-standalone");
    let rust_output = temp.join("rust.sbet");
    run_rust_pipeline(&input, &rust_output);
    assert!(rust_output.exists());
    let content = fs::read(&rust_output).unwrap();
    assert!(!content.is_empty());
}

#[test]
fn test_sbet_writer_rejects_empty_filename() {
    let mut options = Options::new();
    options.add("filename", "");
    let mut writer = SbetWriter::new(&options);
    let layout = std::rc::Rc::new(PointLayout::new());
    let view = PointView::new(layout);
    let result = writer.write(&[view]);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().0.contains("filename"),
        "error should mention missing filename"
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
