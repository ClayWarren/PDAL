use super::*;
use pdal_core::options::Options;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_native::gdal::{VectorFieldType, VectorFieldValue, VectorPointWriter};
use std::rc::Rc;
use tempfile::TempDir;

#[test]
fn registry_assign_filter_supports_value_expressions() {
    let mut options = Options::new();
    options.add("value", "Y = X * 2");
    options.add("value", "Classification = Y WHERE X >= 5");
    let mut filter = create_filter("filters.assign", &options).unwrap();

    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for x in [1.0, 5.0, 10.0] {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, x);
        view.set_f64(idx, &DimId::Classification, 1.0);
    }

    let output_dims = filter.output_dimensions();
    assert!(output_dims.contains(&(DimId::Y, DimType::F64)));
    let views = filter.run(&[view.with_dimensions(&output_dims)]).unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::Y), 2.0);
    assert_eq!(views[0].get_f64(1, &DimId::Y), 10.0);
    assert_eq!(views[0].get_f64(2, &DimId::Y), 20.0);
    assert_eq!(views[0].get_f64(0, &DimId::Classification), 1.0);
    assert_eq!(views[0].get_f64(1, &DimId::Classification), 10.0);
    assert_eq!(views[0].get_f64(2, &DimId::Classification), 20.0);
}

#[test]
fn registry_radiusassign_filter_supports_value_expressions() {
    let mut options = Options::new();
    options.add("radius", 1.0);
    options.add("is3d", true);
    options.add("reference_domain", "Classification[1:1]");
    options.add("update_expression", "Classification = Z + 3 WHERE X < 1");
    let mut filter = create_filter("filters.radiusassign", &options).unwrap();

    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    for (x, y, z, class) in [
        (0.0, 0.0, 0.0, 1.0),
        (0.5, 0.0, 0.0, 0.0),
        (0.0, 0.5, -2.0, 0.0),
        (10.0, 0.0, 0.0, 0.0),
    ] {
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, x);
        view.set_f64(idx, &DimId::Y, y);
        view.set_f64(idx, &DimId::Z, z);
        view.set_f64(idx, &DimId::Classification, class);
    }

    let views = filter.run(&[view]).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::Classification), 3.0);
    assert_eq!(views[0].get_f64(1, &DimId::Classification), 3.0);
    assert_eq!(views[0].get_f64(2, &DimId::Classification), 0.0);
    assert_eq!(views[0].get_f64(3, &DimId::Classification), 0.0);
}

fn overlay_datasource() -> (TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("overlay.gpkg");
    let writer =
        VectorPointWriter::create_polygon(path.to_str().unwrap(), "GPKG", "", "zones").unwrap();
    writer
        .create_field("cls", VectorFieldType::Integer)
        .unwrap();
    writer
        .write_geometry_wkt(
            "MULTIPOLYGON (((0 0, 10 0, 10 10, 0 10, 0 0)))",
            &[VectorFieldValue::Integer(5)],
        )
        .unwrap();
    writer
        .write_geometry_wkt(
            "MULTIPOLYGON (((20 0, 30 0, 30 10, 20 10, 20 0)))",
            &[VectorFieldValue::Integer(7)],
        )
        .unwrap();
    drop(writer);
    (dir, path.display().to_string())
}

fn overlay_view(x: f64, y: f64) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    layout.register(DimId::Classification, DimType::U8);
    let mut view = PointView::new(Rc::new(layout));
    let idx = view.add_point();
    view.set_f64(idx, &DimId::X, x);
    view.set_f64(idx, &DimId::Y, y);
    view.set_f64(idx, &DimId::Z, 0.0);
    view.set_f64(idx, &DimId::Classification, 0.0);
    view
}

#[test]
fn registry_overlay_filter_supports_named_layers() {
    let (_dir, datasource) = overlay_datasource();
    let mut options = Options::new();
    options.add("dimension", "Classification");
    options.add("datasource", datasource);
    options.add("column", "cls");
    options.add("lyr_name", "zones");
    let mut filter = create_filter("filters.overlay", &options).unwrap();

    let views = filter.run(&[overlay_view(5.0, 5.0)]).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::Classification), 5.0);
}

#[test]
fn registry_overlay_filter_supports_sql_queries() {
    let (_dir, datasource) = overlay_datasource();
    let mut options = Options::new();
    options.add("dimension", "Classification");
    options.add("datasource", datasource);
    options.add("column", "cls");
    options.add("query", "SELECT * FROM zones WHERE cls = 7");
    let mut filter = create_filter("filters.overlay", &options).unwrap();

    let views = filter.run(&[overlay_view(25.0, 5.0)]).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::Classification), 7.0);
}

#[test]
fn registry_overlay_filter_prefers_layer_over_query() {
    let (_dir, datasource) = overlay_datasource();
    let mut options = Options::new();
    options.add("dimension", "Classification");
    options.add("datasource", datasource);
    options.add("column", "cls");
    options.add("layer", "zones");
    options.add("query", "SELECT * FROM zones WHERE cls = 7");
    let mut filter = create_filter("filters.overlay", &options).unwrap();

    let views = filter.run(&[overlay_view(5.0, 5.0)]).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].get_f64(0, &DimId::Classification), 5.0);
}
