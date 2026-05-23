//! Behavioral parity tests for GeomDistanceFilter.

use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, Streamable};
use pdal_filters::geom_distance::GeomDistanceFilter;
use std::rc::Rc;

fn make_xyz_view(points: &[(f64, f64, f64)], out_dim: &str) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::from_name(out_dim), DimType::F64);
    let layout = Rc::new(layout);

    let mut view = PointView::new(layout);
    for &(x, y, z) in points {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, x);
        view.set_f64(idx, &DimId::Y, y);
        view.set_f64(idx, &DimId::Z, z);
    }
    view
}

#[test]
fn test_geom_distance_to_point() {
    let view = make_xyz_view(&[(3.0, 4.0, 0.0), (0.0, 0.0, 0.0)], "Distance");
    let mut filter = GeomDistanceFilter::new("POINT(0 0 0)", "Distance", false).unwrap();
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs.len(), 1);
    let out = &outputs[0];

    let dist_dim = DimId::from_name("Distance");
    assert!((out.get_f64(0, &dist_dim) - 5.0).abs() < 1e-10);
    assert!((out.get_f64(1, &dist_dim)).abs() < 1e-10);
}

#[test]
fn test_geom_distance_ring_option() {
    // polygon covering (0,0) to (10,10)
    let wkt = "POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))";
    // point inside should be distance 0 from polygon, but >0 from boundary
    let view = make_xyz_view(&[(5.0, 5.0, 0.0)], "Distance");
    let mut filter = GeomDistanceFilter::new(wkt, "Distance", true).unwrap();
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    let out = &outputs[0];
    let dist_dim = DimId::from_name("Distance");
    let boundary_dist = out.get_f64(0, &dist_dim);

    // ring=true means distance to boundary of polygon, not interior
    // So the point at (5,5) should have some positive distance to the boundary
    assert!(boundary_dist > 0.0);
}

#[test]
fn test_geom_distance_invalid_wkt_is_error() {
    let result = GeomDistanceFilter::new("NOT VALID WKT", "Distance", false);
    assert!(result.is_err());
}

#[test]
fn test_geom_distance_empty_input() {
    let layout = Rc::new(PointLayout::new());
    let view = PointView::new(layout);
    let mut filter = GeomDistanceFilter::new("POINT(0 0 0)", "Distance", false).unwrap();
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs[0].len(), 0);
}

#[test]
fn test_geom_distance_names() {
    let filter = GeomDistanceFilter::new("POINT(0 0 0)", "D", false).unwrap();
    assert_eq!(filter.name(), "filters.geomdistance");
    assert!(filter.as_any().downcast_ref::<GeomDistanceFilter>().is_some());
}

#[test]
fn test_geom_distance_reset() {
    let view = make_xyz_view(&[(3.0, 4.0, 0.0)], "D");
    let mut filter = GeomDistanceFilter::new("POINT(0 0 0)", "D", false).unwrap();
    filter.reset();
    let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
    assert_eq!(outputs[0].len(), 1);
}
