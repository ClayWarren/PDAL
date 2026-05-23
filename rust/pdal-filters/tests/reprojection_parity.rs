use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::Filter;
use pdal_filters::reprojection::ReprojectionFilter;
use pdal_io::faux::FauxReader;

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

    let mut reader = FauxReader::new(&reader_options).unwrap();
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

#[test]
fn test_reprojection_unknown_source_srs_is_error() {
    let mut layout = pdal_core::point::PointLayout::new();
    layout.register(DimId::X, pdal_core::point::DimType::F64);
    layout.register(DimId::Y, pdal_core::point::DimType::F64);
    layout.register(DimId::Z, pdal_core::point::DimType::F64);
    let mut view = PointView::new(std::rc::Rc::new(layout));
    let idx = view.add_point();
    view.set_f64(idx, &DimId::X, 1.0);
    view.set_f64(idx, &DimId::Y, 1.0);
    view.set_f64(idx, &DimId::Z, 1.0);

    let mut filter = ReprojectionFilter::new("EPSG:32610", None, true);
    let res = pdal_core::stage::Filter::run_one(&mut filter, &view);
    assert!(res.is_err());
}

#[test]
fn test_reprojection_with_explicit_in_srs() {
    let mut layout = pdal_core::point::PointLayout::new();
    layout.register(DimId::X, pdal_core::point::DimType::F64);
    layout.register(DimId::Y, pdal_core::point::DimType::F64);
    layout.register(DimId::Z, pdal_core::point::DimType::F64);
    let mut view = PointView::new(std::rc::Rc::new(layout));
    let idx = view.add_point();
    view.set_f64(idx, &DimId::X, -122.3331);
    view.set_f64(idx, &DimId::Y, 47.6097);
    view.set_f64(idx, &DimId::Z, 0.0);

    let mut filter = ReprojectionFilter::new("EPSG:32610", Some("EPSG:4326".to_string()), true);
    let out_views = pdal_core::stage::Filter::run_one(&mut filter, &view).unwrap();
    let out_view = &out_views[0];
    assert_eq!(out_view.len(), 1);
    let x = out_view.get_f64(0, &DimId::X);
    assert!((x - 550058.0).abs() < 100.0);
}

#[test]
fn test_reprojection_with_error_on_failure() {
    let mut layout = pdal_core::point::PointLayout::new();
    layout.register(DimId::X, pdal_core::point::DimType::F64);
    layout.register(DimId::Y, pdal_core::point::DimType::F64);
    layout.register(DimId::Z, pdal_core::point::DimType::F64);
    let mut view = PointView::new(std::rc::Rc::new(layout));
    let idx = view.add_point();
    view.set_f64(idx, &DimId::X, 1e9);
    view.set_f64(idx, &DimId::Y, 1e9);
    view.set_f64(idx, &DimId::Z, 0.0);
    view.set_spatial_reference(pdal_core::srs::SpatialReference::new("EPSG:4326"));

    let mut filter = ReprojectionFilter::new("EPSG:32610", None, false);
    let out_views = pdal_core::stage::Filter::run_one(&mut filter, &view).unwrap();
    assert_eq!(out_views[0].len(), 1);

    let mut filter_err = ReprojectionFilter::new("EPSG:32610", None, true);
    let mut stream_view = view.clone();
    assert!(!pdal_core::stage::Streamable::process_one(
        &mut filter_err,
        &mut stream_view,
        0
    ));
}

#[test]
fn test_reprojection_trait_and_streamable_methods() {
    let mut filter = ReprojectionFilter::new("EPSG:32610", None, true);
    assert_eq!(filter.name(), "filters.reprojection");
    assert!(pdal_core::stage::Filter::as_any(&filter)
        .downcast_ref::<ReprojectionFilter>()
        .is_some());
    pdal_core::stage::Streamable::reset(&mut filter);
}
