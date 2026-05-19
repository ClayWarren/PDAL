use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_io::faux::FauxReader;
use pdal_filters::reprojection::ReprojectionFilter;
use pdal_core::point::DimId;
use pdal_core::srs::SpatialReference;

#[test]
fn test_reprojection_lat_lon_to_utm() {
    let mut reader_options = Options::new();
    reader_options.add("count", 1);
    reader_options.add("mode", "constant");
    reader_options.add("minx", -122.3331);
    reader_options.add("maxx", -122.3331);
    reader_options.add("miny", 47.6097);
    reader_options.add("maxy", 47.6097);
    reader_options.add("minz", 0.0);
    reader_options.add("maxz", 0.0);

    let mut reader = FauxReader::new(&reader_options);
    let mut views = reader.read().unwrap();
    let mut view = views.remove(0);
    view.set_spatial_reference(pdal_core::srs::SpatialReference::new("EPSG:4326"));

    let mut filter = ReprojectionFilter::new("EPSG:32610", None, true);
    let out_views = pdal_core::stage::Filter::run_one(&mut filter, &view).unwrap();
    let out_view = &out_views[0];

    assert_eq!(out_view.len(), 1);
    let x = out_view.get_f64(0, &DimId::X);
    let y = out_view.get_f64(0, &DimId::Y);
    
    // -122.3331, 47.6097 in EPSG:32610 is roughly 550000, 5273190
    assert!((x - 550058.0).abs() < 100.0);
    assert!((y - 5273190.0).abs() < 100.0);
}
