use pdal_core::options::Options;
use pdal_core::pipeline::{Pipeline, Reader};
use pdal_core::point::DimId;
use pdal_io::fbi::FbiReader;
use pdal_io::fbi_writer::FbiWriter;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_fbi_roundtrip() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/fbi/1.2-with-color.fbi");
    let temp = make_temp_dir("fbi-regression");
    let installed_output = temp.join("installed.fbi");
    let rust_output = temp.join("rust.fbi");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.fbi","filename":"{}"}},
  {{"type":"writers.fbi","filename":"{}"}}
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

    let installed_views = read_fbi(&installed_output);
    let rust_views = read_fbi(&rust_output);
    assert_eq!(installed_views.len(), rust_views.len());
    assert_eq!(installed_views[0].len(), rust_views[0].len());

    assert_eq!(
        installed_views[0].get_f64(0, &DimId::X),
        rust_views[0].get_f64(0, &DimId::X)
    );
    assert_eq!(rust_views[0].get_f64(0, &DimId::X), 635618.98);
}

fn run_rust_pipeline(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.fbi",
        Box::new(FbiReader::new(&reader_options)),
        reader_options,
    );
    let writer = pipeline.add_writer(
        "writers.fbi",
        Box::new(FbiWriter::new(&writer_options)),
        writer_options,
    );
    pipeline.add_dependency(writer, reader).unwrap();
    assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
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

fn read_fbi(path: &Path) -> Vec<pdal_core::point::PointView> {
    let mut options = Options::new();
    options.add("filename", path.display());
    FbiReader::new(&options).read().unwrap()
}
