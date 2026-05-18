use pdal_core::options::Options;
use pdal_core::pipeline::{FilterWrapper, Pipeline};
use pdal_filters::decimation::DecimationFilter;
use pdal_io::obj::ObjReader;
use pdal_io::text_writer::TextWriter;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_obj_decimation_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/obj/box.obj");
    let temp = make_temp_dir("obj-decimation-regression");
    let installed_output = temp.join("installed.txt");
    let rust_output = temp.join("rust.txt");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.obj","filename":"{}"}},
  {{"type":"filters.decimation","step":2}},
  {{"type":"writers.text","filename":"{}","order":"X,Y,Z","quote_header":false,"precision":1}}
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
        fs::read_to_string(&installed_output).unwrap(),
        fs::read_to_string(&rust_output).unwrap()
    );
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
        .add("quote_header", false)
        .add("precision", 1);

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.obj",
        Box::new(ObjReader::new(&reader_options)),
        reader_options,
    );
    let filter = pipeline.add_stage(
        "filters.decimation",
        Box::new(FilterWrapper::new(DecimationFilter::new(&filter_options))),
        filter_options,
    );
    let writer = pipeline.add_writer(
        "writers.text",
        Box::new(TextWriter::new(&writer_options)),
        writer_options,
    );
    pipeline.add_dependency(filter, reader).unwrap();
    pipeline.add_dependency(writer, filter).unwrap();
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
