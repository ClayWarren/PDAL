use pdal_core::options::Options;
use pdal_core::pipeline::{FilterWrapper, Pipeline};
use pdal_filters::delaunay::DelaunayFilter;
use pdal_io::gltf::GltfWriter;
use pdal_io::text::TextReader;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
#[ignore = "requires installed pdal on PATH"]
fn installed_pdal_matches_rust_gltf_writer_pipeline() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = repo.join("test/data/filters/delaunaytest.txt");
    let temp = make_temp_dir("gltf-writer-regression");
    let installed_output = temp.join("installed.glb");
    let rust_output = temp.join("rust.glb");
    let pipeline = temp.join("pipeline.json");

    fs::write(
        &pipeline,
        format!(
            r#"[
  {{"type":"readers.text","filename":"{}"}},
  {{"type":"filters.delaunay"}},
  {{"type":"writers.gltf","filename":"{}"}}
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

    let installed = read_glb_summary(&installed_output);
    let rust = read_glb_summary(&rust_output);
    assert!(installed.bin_length > 0);
    assert_eq!(rust.bin_length, installed.bin_length);
    assert_eq!(rust.buffer_byte_length(), installed.buffer_byte_length());
    assert_eq!(
        rust.mesh_primitive_count(),
        installed.mesh_primitive_count()
    );
    assert_eq!(rust.accessor_count(), installed.accessor_count());
    assert_eq!(rust.buffer_view_count(), installed.buffer_view_count());
}

fn run_rust_pipeline(input: &Path, output: &Path) {
    let mut reader_options = Options::new();
    reader_options.add("filename", input.display());
    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.text",
        Box::new(TextReader::new(&reader_options)),
        reader_options,
    );
    let filter = pipeline.add_stage(
        "filters.delaunay",
        Box::new(FilterWrapper::new(DelaunayFilter::new())),
        Options::new(),
    );
    let writer = pipeline.add_writer(
        "writers.gltf",
        Box::new(GltfWriter::new(&writer_options)),
        writer_options,
    );
    pipeline.add_dependency(filter, reader).unwrap();
    pipeline.add_dependency(writer, filter).unwrap();
    assert!(pipeline.execute(Vec::new()).unwrap().is_empty());
}

struct GlbSummary {
    bin_length: u32,
    json: Value,
}

impl GlbSummary {
    fn buffer_byte_length(&self) -> u64 {
        self.json["buffers"][0]["byteLength"].as_u64().unwrap()
    }

    fn mesh_primitive_count(&self) -> usize {
        self.json["meshes"][0]["primitives"]
            .as_array()
            .unwrap()
            .len()
    }

    fn accessor_count(&self) -> usize {
        self.json["accessors"].as_array().unwrap().len()
    }

    fn buffer_view_count(&self) -> usize {
        self.json["bufferViews"].as_array().unwrap().len()
    }
}

fn read_glb_summary(path: &Path) -> GlbSummary {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[0..4], b"glTF");
    assert_eq!(read_u32(&bytes[4..8]), 2);

    let total_length = read_u32(&bytes[8..12]);
    if total_length != 0 {
        assert_eq!(total_length as usize, bytes.len());
    }

    let json_length = read_u32(&bytes[12..16]) as usize;
    assert_eq!(read_u32(&bytes[16..20]), 0x4E4F534A);
    let json_start = 20;
    let json_end = json_start + json_length;
    let json = serde_json::from_slice(trim_ascii_end(&bytes[json_start..json_end])).unwrap();

    let bin_header = json_end;
    let bin_length = read_u32(&bytes[bin_header..bin_header + 4]);
    assert_eq!(read_u32(&bytes[bin_header + 4..bin_header + 8]), 0x004E4942);

    GlbSummary { bin_length, json }
}

fn trim_ascii_end(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|idx| idx + 1)
        .unwrap_or(0);
    &bytes[..end]
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
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
