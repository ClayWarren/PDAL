use super::*;

#[test]
fn bounds2d_empty_and_grow_match_cpp_contract() {
    let mut bounds = Bounds2D::empty();
    assert!(bounds.is_empty());

    bounds.grow_point(0.0, 201.0);
    assert_eq!(
        bounds,
        Bounds2D {
            minx: 0.0,
            maxx: 0.0,
            miny: 201.0,
            maxy: 201.0,
        }
    );

    bounds.grow_distance(2.0);
    assert_eq!(
        bounds,
        Bounds2D {
            minx: -2.0,
            maxx: 2.0,
            miny: 199.0,
            maxy: 203.0,
        }
    );

    let other = Bounds2D {
        minx: -1.0,
        maxx: 10.0,
        miny: 200.0,
        maxy: 204.0,
    };
    assert!(bounds.contains_point(0.0, 201.0));
    assert!(bounds.overlaps(&other));
    bounds.grow_bounds(&other);
    assert_eq!(bounds.maxx, 10.0);
    bounds.clip(&other);
    assert_eq!(bounds, other);
    assert!(bounds.contains_bounds(&other));
}

#[test]
fn bounds3d_empty_and_grow_match_cpp_contract() {
    let mut bounds = Bounds3D::empty();
    assert!(bounds.is_empty());

    bounds.grow_point(0.0, 201.0, 202.0);
    assert_eq!(
        bounds,
        Bounds3D {
            minx: 0.0,
            maxx: 0.0,
            miny: 201.0,
            maxy: 201.0,
            minz: 202.0,
            maxz: 202.0,
        }
    );

    bounds.grow_distance(2.0);
    assert!(bounds.contains_point(0.0, 201.0, 202.0));
    let other = Bounds3D {
        minx: -1.0,
        maxx: 1.0,
        miny: 200.0,
        maxy: 202.0,
        minz: 201.0,
        maxz: 203.0,
    };
    assert!(bounds.overlaps(&other));
    assert!(bounds.contains_bounds(&other));
    bounds.clip(&other);
    assert_eq!(bounds, other);
}

#[test]
fn parses_tuple_and_json_bounds() {
    let parsed = parse_bounds3d("([1,101],[2,102],[3,103])", 0).unwrap();
    assert_eq!(parsed.bounds.minx, 1.0);
    assert_eq!(parsed.bounds.maxz, 103.0);
    assert_eq!(parsed.pos, 25);

    let parsed = parse_bounds2d("[1, 2, 101, 102]", 0).unwrap();
    assert_eq!(parsed.bounds.maxx, 101.0);
    assert_eq!(parsed.bounds.maxy, 102.0);

    let parsed = parse_bounds2d(
        r#"{"minx": 1,"miny": 2,"maxx": 101,"maxy": 102,"crs":"EPSG:2596"}"#,
        0,
    )
    .unwrap();
    assert_eq!(parsed.wkt, "EPSG:2596");
}

#[test]
fn equal_and_default_helpers_match_definitions() {
    let a = Bounds2D {
        minx: 0.0,
        maxx: 1.0,
        miny: 0.0,
        maxy: 1.0,
    };
    let b = a;
    assert!(bounds2d_equal(&a, &b));
    let mut c = a;
    c.maxx = 2.0;
    assert!(!bounds2d_equal(&a, &c));

    let a3 = Bounds3D {
        minx: 0.0,
        maxx: 1.0,
        miny: 0.0,
        maxy: 1.0,
        minz: 0.0,
        maxz: 1.0,
    };
    let b3 = a3;
    assert!(bounds3d_equal(&a3, &b3));
    let mut c3 = a3;
    c3.maxz = 2.0;
    assert!(!bounds3d_equal(&a3, &c3));

    let d2 = default_bounds2d();
    assert_eq!(d2.minx, f64::MIN);
    assert_eq!(d2.maxx, f64::MAX);
    let d3 = default_bounds3d();
    assert_eq!(d3.minz, f64::MIN);
    assert_eq!(d3.maxz, f64::MAX);
}

#[test]
fn format_helpers_render_empty_and_populated_bounds() {
    assert_eq!(format_bounds2d(&Bounds2D::empty(), 2), "()");
    assert_eq!(format_bounds3d(&Bounds3D::empty(), 2), "()");

    let b2 = Bounds2D {
        minx: 1.0,
        maxx: 3.0,
        miny: 2.0,
        maxy: 4.0,
    };
    assert_eq!(format_bounds2d(&b2, 0), "([1, 3], [2, 4])");

    let b3 = Bounds3D {
        minx: 1.0,
        maxx: 3.0,
        miny: 2.0,
        maxy: 4.0,
        minz: 5.0,
        maxz: 6.0,
    };
    assert_eq!(format_bounds3d(&b3, 0), "([1, 3], [2, 4], [5, 6])");
}

#[test]
fn wkt_and_geojson_renderers_handle_empty_and_populated_bounds() {
    assert_eq!(bounds2d_to_wkt(&Bounds2D::empty(), 0), "");
    assert_eq!(bounds3d_to_wkt(&Bounds3D::empty(), 0), "");
    assert_eq!(bounds2d_to_geojson(&Bounds2D::empty(), 0), "");

    let b2 = Bounds2D {
        minx: 0.0,
        maxx: 10.0,
        miny: 0.0,
        maxy: 5.0,
    };
    let wkt2 = bounds2d_to_wkt(&b2, 2);
    assert!(wkt2.starts_with("POLYGON (("));
    assert!(wkt2.contains("0.00 0.00"));
    assert!(wkt2.contains("10.00 5.00"));

    let geo = bounds2d_to_geojson(&b2, 1);
    assert_eq!(geo, "{\"bbox\":[0.0, 0.0, 10.0,5.0]}");

    let b3 = Bounds3D {
        minx: 0.0,
        maxx: 1.0,
        miny: 0.0,
        maxy: 1.0,
        minz: 0.0,
        maxz: 1.0,
    };
    let wkt3 = bounds3d_to_wkt(&b3, 0);
    assert!(wkt3.starts_with("POLYHEDRON Z ("));
    // Each of the six faces should appear.
    assert_eq!(wkt3.matches("((").count(), 6);
}

#[test]
fn parser_reports_each_distinct_syntax_error() {
    // Missing opening parenthesis around the dim list.
    let err = parse_bounds2d("[1,2],[3,4])", 0).unwrap_err();
    assert!(err.contains("opening"));

    // Range without opening bracket.
    let err = parse_bounds2d("(1,2],[3,4])", 0).unwrap_err();
    assert!(err.contains("opening '['"));

    // Range with no minimum number.
    let err = parse_bounds2d("([,2],[3,4])", 0).unwrap_err();
    assert!(err.contains("minimum"));

    // Range with no comma between min/max.
    let err = parse_bounds2d("([1 2],[3,4])", 0).unwrap_err();
    assert!(err.contains("separator") || err.contains(","));

    // Range with no maximum.
    let err = parse_bounds2d("([1,],[3,4])", 0).unwrap_err();
    assert!(err.contains("maximum"));

    // Range with no closing bracket.
    let err = parse_bounds2d("([1,2,[3,4])", 0).unwrap_err();
    assert!(err.contains("closing"));

    // Bounds with no closing paren.
    let err = parse_bounds2d("([1,2],[3,4]", 0).unwrap_err();
    assert!(err.contains("closing"));

    // JSON arrays of wrong length report explicit error.
    let err = parse_bounds2d("[1,2,3]", 0).unwrap_err();
    assert!(err.contains("array size must be 4"));

    let err = parse_bounds3d("[1,2,3]", 0).unwrap_err();
    assert!(err.contains("GeoJSON array must be 6"));

    // JSON object missing required field.
    let err = parse_bounds2d(r#"{"minx":1,"miny":2,"maxx":3}"#, 0).unwrap_err();
    assert!(err.contains("must contain 'maxy'"));
}

#[test]
fn parser_handles_position_offsets_and_3d_json_object() {
    // Resume parsing partway through a longer string.
    let prefix = "garbage";
    let composite = format!("{prefix}([1,2],[3,4])");
    let parsed = parse_bounds2d(&composite, prefix.len()).unwrap();
    assert_eq!(parsed.bounds.minx, 1.0);
    assert_eq!(parsed.pos, composite.len());

    // 3D object form with srs key (falls back to "srs" when "crs" missing).
    let parsed = parse_bounds3d(
        r#"{"minx":0,"miny":0,"maxx":1,"maxy":1,"minz":-1,"maxz":1,"srs":"EPSG:4326"}"#,
        0,
    )
    .unwrap();
    assert_eq!(parsed.bounds.minz, -1.0);
    assert_eq!(parsed.bounds.maxz, 1.0);
    assert_eq!(parsed.wkt, "EPSG:4326");

    // 3D object form omitting minz/maxz defaults to Bounds3D::empty's values.
    let parsed = parse_bounds3d(r#"{"minx":0,"miny":0,"maxx":1,"maxy":1}"#, 0).unwrap();
    assert_eq!(parsed.bounds.minz, f64::MAX);
    assert_eq!(parsed.bounds.maxz, f64::MIN);
}

#[test]
fn bounds3d_clip_preserves_z_when_neighbour_window_does_not_intersect() {
    let mut bounds = Bounds3D {
        minx: 0.0,
        maxx: 10.0,
        miny: 0.0,
        maxy: 10.0,
        minz: 0.0,
        maxz: 10.0,
    };
    let other = Bounds3D {
        minx: 2.0,
        maxx: 8.0,
        miny: 2.0,
        maxy: 8.0,
        minz: 100.0,
        maxz: 200.0,
    };
    bounds.clip(&other);
    // 2D extents clipped to the other.
    assert_eq!(bounds.minx, 2.0);
    assert_eq!(bounds.maxx, 8.0);
    // z extent untouched because other z is entirely outside.
    assert_eq!(bounds.minz, 0.0);
    assert_eq!(bounds.maxz, 10.0);
}

#[test]
fn bounds2d_contains_and_overlap_edge_cases() {
    let bounds = Bounds2D {
        minx: 0.0,
        maxx: 10.0,
        miny: 0.0,
        maxy: 10.0,
    };
    assert!(bounds.contains_point(0.0, 0.0));
    assert!(bounds.contains_point(10.0, 10.0));
    assert!(!bounds.contains_point(10.0001, 5.0));
    assert!(!bounds.contains_point(-0.0001, 5.0));

    // Touching at an edge counts as overlap.
    let touching = Bounds2D {
        minx: 10.0,
        maxx: 20.0,
        miny: 0.0,
        maxy: 10.0,
    };
    assert!(bounds.overlaps(&touching));
    let disjoint = Bounds2D {
        minx: 11.0,
        maxx: 12.0,
        miny: 0.0,
        maxy: 10.0,
    };
    assert!(!bounds.overlaps(&disjoint));
}

// ----- JSON bounds parsing branches -----

#[test]
fn parse_bounds2d_accepts_geojson_array() {
    let p = parse_bounds2d("[1.0, 2.0, 3.0, 4.0]", 0).unwrap();
    assert_eq!(p.bounds.minx, 1.0);
    assert_eq!(p.bounds.miny, 2.0);
    assert_eq!(p.bounds.maxx, 3.0);
    assert_eq!(p.bounds.maxy, 4.0);
}

#[test]
fn parse_bounds2d_rejects_wrong_size_geojson_array() {
    let r = parse_bounds2d("[1.0, 2.0]", 0);
    assert!(r.is_err());
}

#[test]
fn parse_bounds2d_accepts_object_form() {
    let p = parse_bounds2d(
        r#"{"minx":1,"miny":2,"maxx":3,"maxy":4,"srs":"EPSG:4326"}"#,
        0,
    )
    .unwrap();
    assert_eq!(p.bounds.minx, 1.0);
    assert_eq!(p.wkt, "EPSG:4326");
}

#[test]
fn parse_bounds3d_accepts_geojson_array() {
    let p = parse_bounds3d("[1, 2, 3, 4, 5, 6]", 0).unwrap();
    assert_eq!(p.bounds.minx, 1.0);
    assert_eq!(p.bounds.miny, 2.0);
    assert_eq!(p.bounds.minz, 3.0);
    assert_eq!(p.bounds.maxx, 4.0);
    assert_eq!(p.bounds.maxy, 5.0);
    assert_eq!(p.bounds.maxz, 6.0);
}

#[test]
fn parse_bounds3d_rejects_wrong_size_geojson_array() {
    assert!(parse_bounds3d("[1, 2]", 0).is_err());
}

#[test]
fn parse_bounds3d_accepts_object_with_z() {
    let p = parse_bounds3d(
        r#"{"minx":1,"miny":2,"maxx":3,"maxy":4,"minz":5,"maxz":6,"crs":"foo"}"#,
        0,
    )
    .unwrap();
    assert_eq!(p.bounds.minz, 5.0);
    assert_eq!(p.bounds.maxz, 6.0);
    assert_eq!(p.wkt, "foo");
}

#[test]
fn parse_bounds3d_object_uses_empty_z_when_missing() {
    let p = parse_bounds3d(r#"{"minx":1,"miny":2,"maxx":3,"maxy":4}"#, 0).unwrap();
    // Z defaults to Bounds3D::empty() values
    let empty = Bounds3D::empty();
    assert_eq!(p.bounds.minz, empty.minz);
    assert_eq!(p.bounds.maxz, empty.maxz);
}
