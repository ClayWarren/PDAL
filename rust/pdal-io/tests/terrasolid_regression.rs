use pdal_core::options::Options;
use pdal_core::pipeline::Pipeline;
use pdal_io::pcd::PcdWriter;
use pdal_io::terrasolid::TerrasolidReader;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_terrasolid_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/terrasolid/20020715-time-color.bin");
    let temp = make_temp_dir("terrasolid-regression");
    let installed_output = temp.join("installed.pcd");
    let rust_output = temp.join("rust.pcd");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.terrasolid","filename":"{}"}},
  {{"type":"writers.pcd","filename":"{}"}}
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
        normalized_pcd(&installed_output),
        normalized_pcd(&rust_output)
    );
}

fn run_rust_pipeline(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.terrasolid",
        Box::new(TerrasolidReader::new(&reader_options)),
        reader_options,
    );
    let writer = pipeline.add_writer(
        "writers.pcd",
        Box::new(PcdWriter::new(&writer_options)),
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

fn normalized_pcd(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| {
            if line.starts_with("VIEWPOINT ") {
                "VIEWPOINT 0 0 0 1 0 0 0".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
