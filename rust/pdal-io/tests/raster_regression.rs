use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::Filter;
use pdal_filters::faceraster::FaceRasterFilter;
use pdal_io::raster_writer::RasterWriter;
use std::path::PathBuf;
use std::rc::Rc;

#[test]
fn faceraster_output_writes_through_raster_writer() {
    let output = temp_path("faceraster.tif");
    let mut filter_options = Options::new();
    filter_options.add("resolution", 1.0);
    filter_options.add("origin_x", 0.0);
    filter_options.add("origin_y", 0.0);
    filter_options.add("width", 2);
    filter_options.add("height", 2);
    filter_options.add("nodata", -9999.0);
    let mut filter = FaceRasterFilter::new(&filter_options);
    let view = filter.run_one(&triangle_view()).unwrap().pop().unwrap();

    let mut writer_options = Options::new();
    writer_options.add("filename", output.display());
    let mut writer = RasterWriter::new(&writer_options);
    writer.write(&[view]).unwrap();

    pdal_core::gdal::register_drivers();
    let raster = pdal_core::gdal::Raster::open(output.to_str().unwrap()).unwrap();
    let mut values = vec![0.0; 4];
    raster.read_band(1, 2, 2, &mut values).unwrap();
    assert_eq!(values, vec![3.5, -9999.0, 1.5, 2.5]);
}

fn triangle_view() -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    for (x, y, z) in [(0.0, 0.0, 0.0), (2.0, 0.0, 2.0), (0.0, 2.0, 4.0)] {
        let point = view.add_point();
        view.set_f64(point, &DimId::X, x);
        view.set_f64(point, &DimId::Y, y);
        view.set_f64(point, &DimId::Z, z);
    }
    view.create_mesh().add(0, 1, 2);
    view
}

fn temp_path(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("pdal-rust-raster-{}-{name}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}
