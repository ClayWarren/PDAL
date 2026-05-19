use pdal_core::options::Options;
use pdal_core::pipeline::{FilterWrapper, Pipeline};
use pdal_filters::decimation::DecimationFilter;
use pdal_io::bpf::BpfReader;
use pdal_io::pcd::PcdWriter;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_bpf_decimation_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/bpf/autzen-utm-chipped-25-v3.bpf");
    let temp = make_temp_dir("bpf-decimation-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");
    let pipeline = temp.join("pipeline.json");

    std::fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.bpf","filename":"{}"}},
  {{"type":"filters.decimation","step":25}},
  {{"type":"writers.pcd","filename":"{}","order":"X,Y,Z,Intensity,Classification","precision":3}}
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
        pcd_data_lines(&installed_output),
        pcd_data_lines(&rust_output)
    );
}

fn run_rust_pipeline(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut filter_options = Options::new();
    filter_options.add("step", 25);
    let mut writer_options = Options::new();
    writer_options
        .add("filename", output.display())
        .add("order", "X,Y,Z,Intensity,Classification")
        .add("precision", 3);

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.bpf",
        Box::new(BpfReader::new(&reader_options)),
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

fn pcd_data_lines(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .skip_while(|line| *line != "DATA ascii")
        .skip(1)
        .map(str::to_string)
        .collect()
}

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn escape_json_path(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
