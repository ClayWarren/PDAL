use pdal_core::options::Options;
use pdal_core::pipeline::{Pipeline, Reader, Writer};
use pdal_core::point::{DimId, DimType};
use pdal_io::faux::FauxReader;
use pdal_io::las::LasReader;
use pdal_io::las_writer::LasWriter;
use std::fs;
use std::path::PathBuf;

use byteorder::{LittleEndian, WriteBytesExt};
use las::{Builder, Point, Vlr};

#[test]
fn rust_las_preserves_extra_bytes() {
    let temp = make_temp_dir("las-extrabytes");
    let output = temp.join("extra.las");

    // 1. Create a point with an extra dimension
    let mut reader_options = Options::new();
    reader_options.add("count", 1);
    reader_options.add("bounds", "([0,0],[1,1],[0,1])");

    let mut pipeline = Pipeline::new();
    let _reader = pipeline.add_reader(
        "readers.faux",
        Box::new(FauxReader::new(&reader_options).unwrap()),
        reader_options,
    );

    // Add a filter to inject an extra dimension
    // Actually, I'll just use a simple reader and add a point manually to the view
    // Or I can use filters.assign?
    // Let's just run the pipeline and then modify the output view.
    let mut views = pipeline.execute(Vec::new()).unwrap();
    assert_eq!(views.len(), 1);
    let view = views.remove(0);

    // PointLayout is shared and immutable once built, so we can't easily add a dimension
    // to an existing view's layout in this spike.
    // Instead, I'll create a NEW layout and view.
    let mut layout = view.layout().as_ref().clone();
    let extra_dim = DimId::Other("MyExtra".to_string());
    layout.register(extra_dim.clone(), DimType::F64);

    let mut new_view = pdal_core::point::PointView::new(std::rc::Rc::new(layout));
    let id = new_view.add_point();
    new_view.set_f64(id, &DimId::X, 1.0);
    new_view.set_f64(id, &DimId::Y, 2.0);
    new_view.set_f64(id, &DimId::Z, 3.0);
    new_view.set_f64(id, &extra_dim, 42.5);

    // 2. Write to LAS
    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());
    let mut writer = LasWriter::new(&writer_options);
    writer.write(&[new_view]).unwrap();

    // 3. Read back
    let mut reader_options = Options::new();
    reader_options.add("filename", output.display());
    let mut reader = LasReader::new(&reader_options);
    let read_views = reader.read().unwrap();
    assert_eq!(read_views.len(), 1);
    let read_view = &read_views[0];

    assert_eq!(read_view.len(), 1);
    assert_eq!(read_view.get_f64(0, &DimId::X), 1.0);
    assert_eq!(read_view.get_f64(0, &extra_dim), 42.5);
}

#[test]
fn rust_las_applies_extra_byte_scale_and_offset() {
    let temp = make_temp_dir("las-scaled-extrabytes");
    let output = temp.join("scaled-extra.las");
    let scaled_dim = DimId::Other("ScaledExtra".to_string());

    let mut builder = Builder::default();
    builder.point_format = las::point::Format::new(3).unwrap();
    builder.point_format.extra_bytes = 2;
    builder.vlrs.push(scaled_extra_bytes_vlr());
    let header = builder.into_header().unwrap();

    let mut writer = las::Writer::from_path(&output, header).unwrap();
    let mut point = Point {
        x: 1.0,
        y: 2.0,
        z: 3.0,
        gps_time: Some(0.0),
        color: Some(las::Color {
            red: 0,
            green: 0,
            blue: 0,
        }),
        ..Default::default()
    };
    point.extra_bytes = 8u16.to_le_bytes().to_vec();
    writer.write_point(point).unwrap();
    writer.close().unwrap();

    let mut reader_options = Options::new();
    reader_options.add("filename", output.display());
    let mut reader = LasReader::new(&reader_options);
    let read_views = reader.read().unwrap();
    assert_eq!(read_views.len(), 1);
    assert_eq!(read_views[0].get_f64(0, &scaled_dim), 14.0);
}

fn scaled_extra_bytes_vlr() -> Vlr {
    let mut data = Vec::new();
    data.write_u16::<LittleEndian>(0).unwrap();
    data.write_u8(3).unwrap();
    data.write_u8((1 << 3) | (1 << 4)).unwrap();

    let mut name = [0u8; 32];
    name[..11].copy_from_slice(b"ScaledExtra");
    data.extend_from_slice(&name);

    data.write_u32::<LittleEndian>(0).unwrap();
    data.extend_from_slice(&[0u8; 24]);
    data.extend_from_slice(&[0u8; 24]);
    data.extend_from_slice(&[0u8; 24]);
    for value in [0.5, 0.0, 0.0] {
        data.write_f64::<LittleEndian>(value).unwrap();
    }
    for value in [10.0, 0.0, 0.0] {
        data.write_f64::<LittleEndian>(value).unwrap();
    }
    data.extend_from_slice(&[0u8; 32]);

    Vlr {
        user_id: "LASF_Spec".to_string(),
        record_id: 4,
        description: "Extra Bytes Record".to_string(),
        data,
    }
}

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}
