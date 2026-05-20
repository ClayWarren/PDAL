use super::bpf_base64::{decode_base64, encode_base64};
use super::*;
use pdal_core::metadata::MetadataNode;
use pdal_core::pipeline::Writer;
use pdal_core::point::DimId;
use std::path::Path;

fn data_path(name: &str) -> String {
    format!("{}/../../test/data/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn temp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("pdal-rust-bpf-{}-{name}", std::process::id()))
        .to_string_lossy()
        .into_owned()
}

fn read_bpf(path: &str) -> PointView {
    let mut options = Options::new();
    options.add("filename", path);
    let mut reader = BpfReader::new(&options);
    reader.read().unwrap().remove(0)
}

#[test]
fn reads_uncompressed_dim_major_bpf() {
    let view = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));

    assert_eq!(view.len(), 1065);
    assert!((view.get_f64(0, &DimId::X) - 494057.30).abs() < 0.25);
    assert!((view.get_f64(0, &DimId::Y) - 4877433.35).abs() < 0.25);
    assert!((view.get_f64(0, &DimId::Z) - 130.63).abs() < 0.01);
    assert!(view.layout().dim(&DimId::Intensity).is_some());
}

#[test]
fn reads_uncompressed_point_major_bpf() {
    let view = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3-interleaved.bpf"));

    assert_eq!(view.len(), 1065);
    assert!((view.get_f64(1, &DimId::X) - 494133.82).abs() < 0.25);
    assert!((view.get_f64(1, &DimId::Y) - 4877439.82).abs() < 0.25);
}

#[test]
fn reads_uncompressed_byte_major_bpf() {
    let view = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3-segregated.bpf"));

    assert_eq!(view.len(), 1065);
    assert!((view.get_f64(2, &DimId::Z) - 130.46).abs() < 0.01);
}

#[test]
fn writer_roundtrips_each_interleave() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));

    for format in ["dimension", "point", "byte"] {
        for compression in [false, true] {
            let output = temp_path(&format!("roundtrip-{format}-{compression}.bpf"));
            let mut options = Options::new();
            options.add("filename", &output);
            options.add("format", format);
            options.add("compression", compression);
            let mut writer = BpfWriter::new(&options);
            writer.write(std::slice::from_ref(&input)).unwrap();

            let roundtrip = read_bpf(&output);
            assert_eq!(roundtrip.len(), input.len());
            for idx in [0, 17, 1064] {
                for dim in [DimId::X, DimId::Y, DimId::Z, DimId::Intensity] {
                    assert!(
                        (roundtrip.get_f64(idx, &dim) - input.get_f64(idx, &dim)).abs() < 0.01,
                        "format {format}, idx {idx}, dim {}",
                        dim.name()
                    );
                }
            }
            std::fs::remove_file(output).ok();
        }
    }
}

#[test]
fn writer_respects_output_dims() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("output-dims.bpf");
    let mut options = Options::new();
    options.add("filename", &output);
    options.add("output_dims", "X,Y,Z,Red,Green");
    let mut writer = BpfWriter::new(&options);
    writer.write(std::slice::from_ref(&input)).unwrap();

    let roundtrip = read_bpf(&output);
    assert_eq!(roundtrip.layout().dim_count(), 5);
    assert!(roundtrip.layout().dim(&DimId::Blue).is_none());
    assert!((roundtrip.get_f64(0, &DimId::Red) - input.get_f64(0, &DimId::Red)).abs() < 0.01);
    assert!((roundtrip.get_f64(0, &DimId::Green) - input.get_f64(0, &DimId::Green)).abs() < 0.01);
    std::fs::remove_file(output).ok();
}

#[test]
fn writer_roundtrips_with_scale_and_offset() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("scaling.bpf");
    let mut options = Options::new();
    options.add("filename", &output);
    options.add("format", "point");
    options.add("offset_x", 494000.0);
    options.add("offset_y", 4870000.0);
    options.add("offset_z", 130.0);
    options.add("scale_x", 0.001);
    options.add("scale_y", 0.01);
    options.add("scale_z", 10.0);
    let mut writer = BpfWriter::new(&options);
    writer.write(std::slice::from_ref(&input)).unwrap();

    let roundtrip = read_bpf(&output);
    for idx in [0, 17, 1064] {
        for dim in [DimId::X, DimId::Y, DimId::Z] {
            assert!(
                (roundtrip.get_f64(idx, &dim) - input.get_f64(idx, &dim)).abs() < 0.01,
                "idx {idx}, dim {}",
                dim.name()
            );
        }
    }
    std::fs::remove_file(output).ok();
}

#[test]
fn writer_roundtrips_header_data_metadata() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("header-data.bpf");
    let payload = b"This is a test.\0";
    let mut options = Options::new();
    options.add("filename", &output);
    options.add("header_data", encode_base64(payload));
    let mut writer = BpfWriter::new(&options);
    writer.write(std::slice::from_ref(&input)).unwrap();

    let mut reader_options = Options::new();
    reader_options.add("filename", &output);
    let mut reader = BpfReader::new(&reader_options);
    reader.read().unwrap();
    let metadata = reader.metadata();
    let encoded = metadata
        .find_child("header_data")
        .and_then(MetadataNode::value)
        .expect("header_data metadata")
        .as_string();
    assert_eq!(decode_base64(&encoded).unwrap(), payload);
    assert_eq!(
        metadata
            .find_child("count")
            .and_then(MetadataNode::value)
            .unwrap()
            .as_u64(),
        1065
    );
    std::fs::remove_file(output).ok();
}

#[test]
fn writer_roundtrips_bundled_files_metadata() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("bundled.bpf");
    let bundle1 = temp_path("bundle1");
    let bundle2 = temp_path("bundle2");
    std::fs::write(&bundle1, b"This is a test").unwrap();
    std::fs::write(&bundle2, b"This is another test").unwrap();

    let mut options = Options::new();
    options.add("filename", &output);
    options.add("bundledfile", &bundle1);
    options.add("bundledfile", &bundle2);
    let mut writer = BpfWriter::new(&options);
    writer.write(std::slice::from_ref(&input)).unwrap();

    let mut reader_options = Options::new();
    reader_options.add("filename", &output);
    let mut reader = BpfReader::new(&reader_options);
    reader.read().unwrap();
    let metadata = reader.metadata();
    let bundles = metadata.children_named("bundled_file");
    assert_eq!(bundles.len(), 2);
    assert_eq!(
        bundle_value(&metadata, file_name(&bundle1)),
        b"This is a test"
    );
    assert_eq!(
        bundle_value(&metadata, file_name(&bundle2)),
        b"This is another test"
    );

    std::fs::remove_file(output).ok();
    std::fs::remove_file(bundle1).ok();
    std::fs::remove_file(bundle2).ok();
}

#[test]
fn reads_compressed_bpf() {
    let view = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3-deflate.bpf"));

    assert_eq!(view.len(), 1065);
    assert!((view.get_f64(0, &DimId::X) - 494057.3).abs() < 0.25);
    assert!((view.get_f64(17, &DimId::Z) - 130.03).abs() < 0.25);
}

fn bundle_value(metadata: &MetadataNode, name: &str) -> Vec<u8> {
    let encoded = metadata
        .children_named("bundled_file")
        .into_iter()
        .find_map(|bundle| bundle.find_child(name))
        .and_then(MetadataNode::value)
        .expect("bundle metadata")
        .as_string();
    decode_base64(&encoded).unwrap()
}

fn file_name(path: &str) -> &str {
    Path::new(path).file_name().unwrap().to_str().unwrap()
}
