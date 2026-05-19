use pdal_core::options::Options;
use pdal_core::pipeline::{Pipeline, Reader, Writer};
use pdal_core::point::{DimId, DimType};
use pdal_io::faux::FauxReader;
use pdal_io::las::LasReader;
use pdal_io::las_writer::LasWriter;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn rust_las_preserves_extra_bytes() {
    let temp = make_temp_dir("las-extrabytes");
    let output = temp.join("extra.las");

    // 1. Create a point with an extra dimension
    let mut reader_options = Options::new();
    reader_options.add("count", 1);
    reader_options.add("bounds", "([0,0],[1,1],[0,1])");

    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader(
        "readers.faux",
        Box::new(FauxReader::new(&reader_options)),
        reader_options,
    );

    // Add a filter to inject an extra dimension
    // Actually, I'll just use a simple reader and add a point manually to the view
    // Or I can use filters.assign?
    // Let's just run the pipeline and then modify the output view.
    let mut views = pipeline.execute(Vec::new()).unwrap();
    assert_eq!(views.len(), 1);
    let mut view = views.remove(0);

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

fn make_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pdal-rust-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}
