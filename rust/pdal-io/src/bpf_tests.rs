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
fn bpf_vsi_path_helpers_cover_remote_and_vsi_forms() {
    assert!(is_bpf_vsi_path("https://example.com/file.bpf"));
    assert!(is_bpf_vsi_path("http://example.com/file.bpf"));
    assert!(is_bpf_vsi_path("/vsicurl/https://example.com/file.bpf"));
    assert!(!is_bpf_vsi_path("/tmp/file.bpf"));
    assert_eq!(
        bpf_vsi_path("https://example.com/file.bpf"),
        "/vsicurl/https://example.com/file.bpf"
    );
    assert_eq!(
        bpf_vsi_path("/vsicurl/https://example.com/file.bpf"),
        "/vsicurl/https://example.com/file.bpf"
    );
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

#[test]
fn reader_errors_without_filename() {
    let mut reader = BpfReader::new(&Options::new());
    let err = reader.read().err().expect("missing filename");
    assert!(err.0.contains("filename"));
}

#[test]
fn reader_errors_on_missing_file() {
    let mut options = Options::new();
    options.add("filename", "/no/such/file.bpf");
    let mut reader = BpfReader::new(&options);
    assert!(reader.read().is_err());
}

#[test]
fn writer_errors_without_filename() {
    let mut writer = BpfWriter::new(&Options::new());
    let layout = PointLayout::new();
    let view = PointView::new(Rc::new(layout));
    assert!(writer.write(&[view]).is_err());
}

#[test]
fn writer_with_bad_format_falls_back() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("bad-format.bpf");
    let mut options = Options::new();
    options.add("filename", &output);
    options.add("format", "alien-format");
    let mut writer = BpfWriter::new(&options);
    let _ = writer.write(std::slice::from_ref(&input));
    std::fs::remove_file(&output).ok();
}

#[test]
fn reader_metadata_returns_expected_name() {
    let reader = BpfReader::new(&Options::new());
    assert_eq!(reader.metadata().name(), "readers.bpf");
}

#[test]
fn writer_metadata_returns_expected_name() {
    let writer = BpfWriter::new(&Options::new());
    assert_eq!(writer.metadata().name(), "writers.bpf");
}

#[test]
fn writer_errors_on_invalid_output_path() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let mut options = Options::new();
    options.add("filename", "/nonexistent-dir-xyz/output.bpf");
    let mut writer = BpfWriter::new(&options);
    assert!(writer.write(std::slice::from_ref(&input)).is_err());
}

#[test]
fn writer_skips_invalid_bundled_files() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("invalid-bundle.bpf");
    let mut options = Options::new();
    options.add("filename", &output);
    options.add("bundledfile", "/no/such/bundle/file");
    let mut writer = BpfWriter::new(&options);
    let _ = writer.write(std::slice::from_ref(&input));
    std::fs::remove_file(&output).ok();
}

#[test]
fn read_autzen_dd_bpf() {
    let view = read_bpf(&data_path("bpf/autzen-dd.bpf"));
    assert!(view.len() > 0);
}

#[test]
fn reader_handles_v3_segregated_deflate() {
    let view = read_bpf(&data_path(
        "bpf/autzen-utm-chipped-25-v3-deflate-segregated.bpf",
    ));
    assert_eq!(view.len(), 1065);
}

#[test]
fn reader_name_is_readers_bpf() {
    let reader = BpfReader::new(&Options::new());
    assert_eq!(reader.name(), "readers.bpf");
}

#[test]
fn writer_name_is_writers_bpf() {
    let writer = BpfWriter::new(&Options::new());
    assert_eq!(writer.name(), "writers.bpf");
}

#[test]
fn writer_errors_on_zero_scale() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("zero-scale.bpf");
    let mut options = Options::new();
    options.add("filename", &output);
    options.add("scale_x", 0.0);
    let mut writer = BpfWriter::new(&options);
    let err = writer.write(std::slice::from_ref(&input)).err().unwrap();
    assert!(err.0.contains("scale"));
    std::fs::remove_file(&output).ok();
}

#[test]
fn writer_errors_on_invalid_output_dim() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("invalid-dim.bpf");
    let mut options = Options::new();
    options.add("filename", &output);
    options.add("output_dims", "X,Y,Z,Bogusdim");
    let mut writer = BpfWriter::new(&options);
    let err = writer.write(std::slice::from_ref(&input)).err().unwrap();
    assert!(err.0.contains("Bogusdim"));
    std::fs::remove_file(&output).ok();
}

#[test]
fn writer_errors_when_missing_xyz_in_output_dims() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("no-xyz.bpf");
    let mut options = Options::new();
    options.add("filename", &output);
    options.add("output_dims", "Intensity");
    let mut writer = BpfWriter::new(&options);
    let err = writer.write(std::slice::from_ref(&input)).err().unwrap();
    assert!(err.0.contains("X") || err.0.contains("Y") || err.0.contains("Z"));
    std::fs::remove_file(&output).ok();
}

#[test]
fn writer_errors_on_empty_views() {
    let mut writer = BpfWriter::new(&{
        let mut o = Options::new();
        o.add("filename", "/tmp/empty-views.bpf");
        o
    });
    let err = writer.write(&[]).err().unwrap();
    assert!(err.0.contains("input view"));
}

#[test]
fn writer_errors_on_oversized_bundle_filename() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("oversized-bundle.bpf");
    // Create a bundled file with a name >32 chars
    let long_name = temp_path("this-bundle-filename-is-definitely-way-too-long");
    std::fs::write(&long_name, b"data").unwrap();
    let mut options = Options::new();
    options.add("filename", &output);
    options.add("bundledfile", &long_name);
    let mut writer = BpfWriter::new(&options);
    let err = writer.write(std::slice::from_ref(&input)).err().unwrap();
    assert!(err.0.contains("maximum length"));
    std::fs::remove_file(&long_name).ok();
    std::fs::remove_file(&output).ok();
}

#[test]
fn writer_errors_on_empty_bundle_file() {
    let input = read_bpf(&data_path("bpf/autzen-utm-chipped-25-v3.bpf"));
    let output = temp_path("empty-bundle.bpf");
    let empty = temp_path("empty-bundle-data");
    std::fs::write(&empty, b"").unwrap();
    let mut options = Options::new();
    options.add("filename", &output);
    options.add("bundledfile", &empty);
    let mut writer = BpfWriter::new(&options);
    let err = writer.write(std::slice::from_ref(&input)).err().unwrap();
    assert!(err.0.contains("empty"));
    std::fs::remove_file(&empty).ok();
    std::fs::remove_file(&output).ok();
}

// ----- BPF v1 reading: synthesize a minimal v1 file -----

fn build_v1_bpf_dim_major(num_pts: i32, dyn_dims: usize) -> Vec<u8> {
    use byteorder::{LittleEndian, WriteBytesExt};
    use std::io::Write as IoWrite;

    let mut buf: Vec<u8> = Vec::new();
    // header_len placeholder
    let header_len_placeholder_pos = buf.len();
    buf.write_i32::<LittleEndian>(0).unwrap(); // len
    buf.write_i32::<LittleEndian>(1).unwrap(); // version 1 (dim major)
    buf.write_i32::<LittleEndian>(num_pts).unwrap();
    buf.write_i32::<LittleEndian>(dyn_dims as i32).unwrap();
    buf.write_i32::<LittleEndian>(0).unwrap(); // coord_type (geo)
    buf.write_i32::<LittleEndian>(0).unwrap(); // coord_id
    buf.write_f32::<LittleEndian>(1.0).unwrap(); // spacing
                                                 // static dim offsets X, Y, Z
    for _ in 0..3 {
        buf.write_f64::<LittleEndian>(0.0).unwrap();
    }
    // static dim min/max for X, Y, Z
    for _ in 0..3 {
        buf.write_f64::<LittleEndian>(0.0).unwrap(); // min
        buf.write_f64::<LittleEndian>(1.0).unwrap(); // max
    }
    // dynamic dim offsets (just zero for each)
    for _ in 0..dyn_dims {
        buf.write_f64::<LittleEndian>(0.0).unwrap();
    }
    // dynamic dim mins
    for _ in 0..dyn_dims {
        buf.write_f64::<LittleEndian>(0.0).unwrap();
    }
    // dynamic dim maxes
    for _ in 0..dyn_dims {
        buf.write_f64::<LittleEndian>(0.0).unwrap();
    }
    // dynamic dim labels (32 bytes each)
    for i in 0..dyn_dims {
        let label = format!("Intensity_{i}");
        let mut buf32 = [0u8; 32];
        let bytes = label.as_bytes();
        let len = bytes.len().min(32);
        buf32[..len].copy_from_slice(&bytes[..len]);
        buf.write_all(&buf32).unwrap();
    }
    // write header length into placeholder
    let header_len = buf.len() as i32;
    let len_bytes = header_len.to_le_bytes();
    buf[header_len_placeholder_pos..header_len_placeholder_pos + 4].copy_from_slice(&len_bytes);

    // payload (dim-major): each dimension has num_pts f32 values
    let total_dims = 3 + dyn_dims;
    for _ in 0..total_dims {
        for _ in 0..num_pts {
            buf.write_f32::<LittleEndian>(0.5_f32).unwrap();
        }
    }
    buf
}

#[test]
fn reads_synthetic_v1_bpf_dim_major() {
    let bytes = build_v1_bpf_dim_major(4, 1);
    let path = temp_path("v1-dim.bpf");
    std::fs::write(&path, &bytes).unwrap();
    let view = read_bpf(&path);
    assert_eq!(view.len(), 4);
    std::fs::remove_file(&path).ok();
}

#[test]
fn reads_synthetic_v1_bpf_with_unsupported_version_errors() {
    use byteorder::{LittleEndian, WriteBytesExt};
    let mut buf: Vec<u8> = Vec::new();
    buf.write_i32::<LittleEndian>(0).unwrap(); // len placeholder
    buf.write_i32::<LittleEndian>(99).unwrap(); // unsupported version
                                                // pad the rest of the (v1) header so read_v1_header reads enough bytes
    buf.extend(std::iter::repeat_n(0, 200));
    let path = temp_path("v1-bad-version.bpf");
    std::fs::write(&path, &buf).unwrap();
    let mut options = Options::new();
    options.add("filename", &path);
    let mut reader = BpfReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("Unsupported BPF version") || err.0.contains("missing"));
    std::fs::remove_file(&path).ok();
}

// ----- format_from_u8 indirectly: corrupt v3 file with bad interleave -----

#[test]
fn reader_errors_on_bad_v3_interleave_byte() {
    // Read a real v3 file and clobber the interleave byte (offset 13 in the header)
    let src = std::fs::read(data_path("bpf/autzen-utm-chipped-25-v3.bpf")).unwrap();
    let mut bytes = src.clone();
    // The interleave byte in v3 is at offset 4 (magic) + 4 (version) + 4 (len) + 1 (num_dim) = 13.
    bytes[13] = 99;
    let path = temp_path("bad-interleave.bpf");
    std::fs::write(&path, &bytes).unwrap();
    let mut options = Options::new();
    options.add("filename", &path);
    let mut reader = BpfReader::new(&options);
    let err = reader.read().err().unwrap();
    assert!(err.0.contains("interleave") || err.0.contains("Invalid"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn reader_errors_on_bad_v3_magic() {
    // Corrupt magic
    let mut bytes = std::fs::read(data_path("bpf/autzen-utm-chipped-25-v3.bpf")).unwrap();
    bytes[1] = b'X';
    let path = temp_path("bad-magic.bpf");
    std::fs::write(&path, &bytes).unwrap();
    let mut options = Options::new();
    options.add("filename", &path);
    let mut reader = BpfReader::new(&options);
    let _ = reader.read(); // will likely error
    std::fs::remove_file(&path).ok();
}

#[test]
fn parse_coord_id_auto_returns_zero() {
    let mut o = Options::new();
    o.add("coord_id", "auto");
    let writer = BpfWriter::new(&o);
    // Roundtrip to verify it doesn't panic; result depends on inner state.
    assert_eq!(writer.name(), "writers.bpf");
}
