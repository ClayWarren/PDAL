use super::*;

#[test]
fn layout_offsets_and_field_roundtrip() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Intensity, DimType::U16);
    layout.register(DimId::X, DimType::F64); // duplicate -- ignored
    assert_eq!(layout.point_size(), 10);

    let mut view = PointView::new(Rc::new(layout));
    let p = view.add_point();
    view.set_f64(p, &DimId::X, 12.5);
    view.set_f64(p, &DimId::Intensity, 700.0);
    assert_eq!(view.len(), 1);
    assert_eq!(view.get_f64(p, &DimId::X), 12.5);
    assert_eq!(view.get_f64(p, &DimId::Intensity), 700.0);
}

#[test]
fn append_point_copies_record() {
    let mut layout = PointLayout::new();
    layout.register(DimId::OffsetTime, DimType::F64);
    let layout = Rc::new(layout);

    let mut src = PointView::new(Rc::clone(&layout));
    let p = src.add_point();
    src.set_f64(p, &DimId::OffsetTime, 42.0);

    let mut dst = src.make_new();
    assert!(dst.is_empty());
    dst.append_point(&src, p);
    assert_eq!(dst.len(), 1);
    assert_eq!(dst.get_f64(0, &DimId::OffsetTime), 42.0);
}

#[test]
fn append_point_preserves_source_index() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    let layout = Rc::new(layout);

    let mut src = PointView::new(Rc::clone(&layout));
    for i in 0..3 {
        let point = src.add_point();
        src.set_f64(point, &DimId::X, i as f64);
    }

    let mut dst = src.make_new();
    dst.append_point(&src, 2);
    dst.append_point(&src, 0);

    assert_eq!(dst.len(), 2);
    assert_eq!(dst.source_index(0), 2);
    assert_eq!(dst.source_index(1), 0);
    assert_eq!(dst.get_f64(0, &DimId::X), 2.0);
    assert_eq!(dst.get_f64(1, &DimId::X), 0.0);
}

#[test]
fn calculate_bounds_matches_point_view_contract() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));

    for (x, y, z) in [(-10.0, 5.0, 100.0), (20.0, -15.0, -50.0), (3.0, 7.0, 25.0)] {
        let point = view.add_point();
        view.set_f64(point, &DimId::X, x);
        view.set_f64(point, &DimId::Y, y);
        view.set_f64(point, &DimId::Z, z);
    }

    assert_eq!(
        view.calculate_bounds_2d(),
        Some(Bounds2D {
            minx: -10.0,
            maxx: 20.0,
            miny: -15.0,
            maxy: 7.0,
        })
    );
    assert_eq!(
        view.calculate_bounds_3d(),
        Some(Bounds3D {
            minx: -10.0,
            maxx: 20.0,
            miny: -15.0,
            maxy: 7.0,
            minz: -50.0,
            maxz: 100.0,
        })
    );
}

#[test]
fn calculate_bounds_requires_points_and_coordinate_dimensions() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));

    assert_eq!(view.calculate_bounds_2d(), None);
    assert_eq!(view.calculate_bounds_3d(), None);

    let point = view.add_point();
    view.set_f64(point, &DimId::X, 1.0);
    view.set_f64(point, &DimId::Y, 2.0);

    assert_eq!(
        view.calculate_bounds_2d(),
        Some(Bounds2D {
            minx: 1.0,
            maxx: 1.0,
            miny: 2.0,
            maxy: 2.0,
        })
    );
    assert_eq!(view.calculate_bounds_3d(), None);
}

#[test]
fn summarize_dimension_reports_basic_statistics() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Intensity, DimType::U16);
    let mut view = PointView::new(Rc::new(layout));

    for (x, intensity) in [(-10.0, 7.0), (20.0, 3.0), (2.0, 5.0)] {
        let point = view.add_point();
        view.set_f64(point, &DimId::X, x);
        view.set_f64(point, &DimId::Intensity, intensity);
    }

    assert_eq!(
        view.summarize_dimension(&DimId::X),
        Some(DimensionSummary {
            name: "X".to_string(),
            count: 3,
            minimum: -10.0,
            maximum: 20.0,
            mean: 4.0,
        })
    );
    assert_eq!(
        view.summarize_dimension(&DimId::Intensity),
        Some(DimensionSummary {
            name: "Intensity".to_string(),
            count: 3,
            minimum: 3.0,
            maximum: 7.0,
            mean: 5.0,
        })
    );
    assert_eq!(view.summarize_dimension(&DimId::Z), None);
}

#[test]
fn summarize_dimensions_follows_layout_order_and_requires_points() {
    let mut layout = PointLayout::new();
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::X, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));

    assert!(view.summarize_dimensions().is_empty());

    let point = view.add_point();
    view.set_f64(point, &DimId::Y, 2.0);
    view.set_f64(point, &DimId::X, 1.0);

    let summaries = view.summarize_dimensions();
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].name, "Y");
    assert_eq!(summaries[1].name, "X");
}

#[test]
fn all_dimension_storage_types_roundtrip_through_f64_accessors() {
    let cases = [
        (DimType::U8, 255.0, 255.0),
        (DimType::U16, 65_535.0, 65_535.0),
        (DimType::U32, 1_000_000.0, 1_000_000.0),
        (DimType::U64, 1_000_000.0, 1_000_000.0),
        (DimType::I8, -12.0, -12.0),
        (DimType::I16, -1234.0, -1234.0),
        (DimType::I32, -123_456.0, -123_456.0),
        (DimType::I64, -123_456.0, -123_456.0),
        (DimType::F32, 12.25, 12.25),
        (DimType::F64, -99.5, -99.5),
    ];

    for (idx, (ty, value, expected)) in cases.into_iter().enumerate() {
        let dim = DimId::Other(format!("dim{idx}"));
        let mut layout = PointLayout::new();
        layout.register(dim.clone(), ty);
        let mut view = PointView::new(Rc::new(layout));

        let point = view.add_point();
        view.set_f64(point, &dim, value);

        assert_eq!(view.get_f64(point, &dim), expected);
    }
}

#[test]
fn resolve_pdal_dimension_types_matches_cpp_contract() {
    const NONE: u32 = 0;
    const U8: u32 = 0x200 | 1;
    const S8: u32 = 0x100 | 1;
    const U16: u32 = 0x200 | 2;
    const S16: u32 = 0x100 | 2;
    const S32: u32 = 0x100 | 4;
    const S64: u32 = 0x100 | 8;
    const FLOAT: u32 = 0x400 | 4;
    const DOUBLE: u32 = 0x400 | 8;

    assert_eq!(resolve_pdal_dimension_type(U8, NONE), U8);
    assert_eq!(resolve_pdal_dimension_type(U8, U8), U8);
    assert_eq!(resolve_pdal_dimension_type(U8, S8), S16);
    assert_eq!(resolve_pdal_dimension_type(U8, S16), S16);
    assert_eq!(resolve_pdal_dimension_type(U16, S16), S32);
    assert_eq!(resolve_pdal_dimension_type(FLOAT, S32), FLOAT);
    assert_eq!(resolve_pdal_dimension_type(DOUBLE, S64), DOUBLE);
}

#[test]
fn dimension_type_names_match_cpp_contract() {
    assert_eq!(pdal_dimension_interpretation_name(0x000), "unknown");
    assert_eq!(pdal_dimension_interpretation_name(0x100 | 1), "int8_t");
    assert_eq!(pdal_dimension_interpretation_name(0x200 | 2), "uint16_t");
    assert_eq!(pdal_dimension_interpretation_name(0x100 | 4), "int32_t");
    assert_eq!(pdal_dimension_interpretation_name(0x200 | 8), "uint64_t");
    assert_eq!(pdal_dimension_interpretation_name(0x400 | 4), "float");
    assert_eq!(pdal_dimension_interpretation_name(0x400 | 8), "double");
    assert_eq!(pdal_dimension_interpretation_name(999), "unknown");
}

#[test]
fn dimension_type_parsing_matches_cpp_contract() {
    assert_eq!(pdal_dimension_type_from_name("char"), 0x100 | 1);
    assert_eq!(pdal_dimension_type_from_name("int16"), 0x100 | 2);
    assert_eq!(pdal_dimension_type_from_name("INT32_T"), 0x100 | 4);
    assert_eq!(pdal_dimension_type_from_name("ulong"), 0x200 | 8);
    assert_eq!(pdal_dimension_type_from_name("float32"), 0x400 | 4);
    assert_eq!(pdal_dimension_type_from_name("float64"), 0x400 | 8);
    assert_eq!(pdal_dimension_type_from_name("nonsense"), 0);

    assert_eq!(
        pdal_dimension_type_from_base_and_size("signed", 1),
        0x100 | 1
    );
    assert_eq!(
        pdal_dimension_type_from_base_and_size("unsigned", 2),
        0x200 | 2
    );
    assert_eq!(
        pdal_dimension_type_from_base_and_size("floating", 4),
        0x400 | 4
    );
    assert_eq!(pdal_dimension_type_from_base_and_size("floating", 2), 0);
    assert_eq!(pdal_dimension_type_from_base_and_size("unknown", 4), 0);
    assert_eq!(pdal_dimension_type_from_base_and_size("signed", 3), 0);
}

#[test]
fn fix_dimension_name_matches_cpp_contract() {
    assert_eq!(fix_dimension_name("Pulse width"), "Pulse width");
    assert_eq!(fix_dimension_name("DimensionName42"), "DimensionName42");
    assert_eq!(fix_dimension_name("with#punctuation."), "with_punctuation_");
    assert_eq!(fix_dimension_name("42DimensionName42"), "_2DimensionName42");
}

#[test]
fn make_new_keeps_layout_and_spatial_reference_without_points() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    view.set_spatial_reference(SpatialReference::with_epoch("EPSG:4326", 2020.0));
    view.add_point();

    let new_view = view.make_new();

    assert!(new_view.is_empty());
    assert_eq!(new_view.layout().point_size(), view.layout().point_size());
    assert_eq!(new_view.spatial_reference().wkt(), "EPSG:4326");
    assert_eq!(new_view.spatial_reference().epoch(), 2020.0);
}

#[test]
fn triangular_mesh_tracks_face_indices() {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    let mut view = PointView::new(Rc::new(layout));
    for _ in 0..3 {
        view.add_point();
    }

    view.create_mesh().add(0, 1, 2);
    let mesh = view.mesh().unwrap();

    assert_eq!(mesh.len(), 1);
    assert_eq!(mesh.triangles()[0].a, 0);
    assert_eq!(mesh.triangles()[0].b, 1);
    assert_eq!(mesh.triangles()[0].c, 2);
    assert!(view.make_new().mesh().is_none());
}
