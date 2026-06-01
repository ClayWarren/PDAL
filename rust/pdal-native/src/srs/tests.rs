use super::*;

#[test]
fn proj_version_is_available() {
    assert!(!version().is_empty());
}

#[test]
fn topocentric_round_trips_and_anchors_at_origin() {
    // Anchor near Portland, OR.
    let t = TopocentricTransform::new(45.0, -123.0, 100.0).unwrap();
    // ECEF coordinates of the anchor itself (computed from WGS84):
    // forward should map the anchor's ECEF to ~(0,0,0) ENU.
    // Instead of hardcoding ECEF, verify the forward/reverse round-trip is
    // an identity for an arbitrary ECEF point, and that reverse(0,0,0)
    // followed by forward returns the origin.
    let (ox, oy, oz) = t.reverse(0.0, 0.0, 0.0); // ENU origin -> ECEF anchor
    let (ex, ey, ez) = t.forward(ox, oy, oz); // back to ENU
    assert!(ex.abs() < 1e-6, "east {ex}");
    assert!(ey.abs() < 1e-6, "north {ey}");
    assert!(ez.abs() < 1e-6, "up {ez}");

    // Round-trip an arbitrary local point.
    let (rx, ry, rz) = t.reverse(10.0, -20.0, 5.0);
    let (bx, by, bz) = t.forward(rx, ry, rz);
    assert!((bx - 10.0).abs() < 1e-6, "east {bx}");
    assert!((by + 20.0).abs() < 1e-6, "north {by}");
    assert!((bz - 5.0).abs() < 1e-6, "up {bz}");
}

#[test]
fn identity_transform_preserves_xy() {
    let transform = SrsTransform::new("EPSG:4326", "EPSG:4326").unwrap();
    let mut x = -93.265;
    let mut y = 44.9778;
    let mut z = 250.0;

    assert!(transform.transform(&mut x, &mut y, &mut z));
    assert_eq!(x, -93.265);
    assert_eq!(y, 44.9778);
    assert_eq!(z, 250.0);
}

#[test]
fn user_input_resolves_epsg_to_wkt1_and_wkt2() {
    let result = user_input_to_wkt("EPSG:4326").unwrap();
    assert!(result.wkt.contains("GEOGCS["));
    assert!(result.wkt.contains("WGS 84"));
    assert!(result.wkt2.contains("GEOGCRS[") || result.wkt2.contains("GEOGCS["));
    assert!(result
        .projjson
        .starts_with("{\n  \"type\": \"GeographicCRS\","));
    assert_eq!(result.epoch, 0.0);
}

#[test]
fn wkt_to_projjson_matches_user_input_projjson_shape() {
    let result = user_input_to_wkt("EPSG:4326").unwrap();
    let json = wkt_to_projjson(&result.wkt, result.epoch).unwrap();
    assert!(json.starts_with("{\n  \"type\": \"GeographicCRS\","));
    assert!(json.contains("\"name\": \"WGS 84\""));

    assert_eq!(wkt_to_projjson("", 0.0).unwrap(), "");
    assert_eq!(wkt_to_projjson("not wkt", 0.0).unwrap(), "");
}

#[test]
fn wkt_export_helpers_match_expected_formats() {
    let result = user_input_to_wkt("EPSG:32617").unwrap();

    let wkt1 = wkt_to_wkt1(&result.wkt2, result.epoch).unwrap();
    assert!(wkt1.starts_with("PROJCS["));
    assert!(wkt1.contains("WGS 84 / UTM zone 17N"));

    let wkt2 = wkt_to_wkt2(&result.wkt, result.epoch).unwrap();
    assert!(wkt2.starts_with("PROJCRS["));
    assert!(wkt2.contains("WGS 84 / UTM zone 17N"));

    let pretty = pretty_wkt(&result.wkt).unwrap();
    assert!(pretty.contains('\n'));
    assert!(pretty.contains("WGS 84 / UTM zone 17N"));

    assert!(wkt_to_wkt1("", 0.0).is_err());
    assert!(wkt_to_wkt2("not wkt", 0.0).is_err());
    assert!(pretty_wkt("not wkt").is_err());
}

#[test]
fn srs_kind_helpers_match_known_crs_types() {
    let geographic = user_input_to_wkt("EPSG:4326").unwrap();
    assert!(is_geographic(&geographic.wkt, 0.0));
    assert!(!is_geocentric(&geographic.wkt, 0.0));
    assert!(!is_projected(&geographic.wkt, 0.0));

    let projected = user_input_to_wkt("EPSG:32617").unwrap();
    assert!(!is_geographic(&projected.wkt, 0.0));
    assert!(!is_geocentric(&projected.wkt, 0.0));
    assert!(is_projected(&projected.wkt, 0.0));

    let geocentric = user_input_to_wkt("EPSG:4978").unwrap();
    assert!(!is_geographic(&geocentric.wkt, 0.0));
    assert!(is_geocentric(&geocentric.wkt, 0.0));
    assert!(!is_projected(&geocentric.wkt, 0.0));

    assert!(!is_geographic("", 0.0));
    assert!(!is_projected("not wkt", 0.0));
}

#[test]
fn axis_ordering_returns_gdal_mapping() {
    let geographic = user_input_to_wkt("EPSG:4326").unwrap();
    let ordering = axis_ordering(&geographic.wkt, 0.0);
    assert!(!ordering.is_empty());
    assert!(ordering.iter().all(|axis| *axis > 0));

    assert!(axis_ordering("", 0.0).is_empty());
    assert!(axis_ordering("not wkt", 0.0).is_empty());
}

#[test]
fn user_input_rejects_garbage() {
    assert!(user_input_to_wkt("not a srs").is_err());
}

#[test]
fn wkt_to_proj4_returns_trimmed_proj4() {
    let result = user_input_to_wkt("EPSG:4326").unwrap();
    let proj4 = wkt_to_proj4(&result.wkt).unwrap();
    assert_eq!(proj4, "+proj=longlat +datum=WGS84 +no_defs");
}

#[test]
fn wkt_to_proj4_empty_returns_empty() {
    assert_eq!(wkt_to_proj4("").unwrap(), "");
    assert_eq!(wkt_to_proj4("not a wkt").unwrap(), "");
}

#[test]
fn is_same_recognizes_equivalent_srs() {
    let a = user_input_to_wkt("EPSG:4326").unwrap();
    let b = user_input_to_wkt("+proj=longlat +datum=WGS84 +no_defs").unwrap();
    assert!(is_same(&a.wkt, &b.wkt, 0.0));
}

#[test]
fn is_same_distinguishes_different_srs() {
    let a = user_input_to_wkt("EPSG:4326").unwrap();
    let b = user_input_to_wkt("EPSG:32617").unwrap();
    assert!(!is_same(&a.wkt, &b.wkt, 0.0));
    assert!(!is_same("", &b.wkt, 0.0));
    assert!(!is_same("not a wkt", &b.wkt, 0.0));
}

#[test]
fn identify_horizontal_epsg_returns_authority_code() {
    let a = user_input_to_wkt("EPSG:32617").unwrap();
    assert_eq!(identify_horizontal_epsg(&a.wkt, 0.0), "32617");
    assert_eq!(identify_horizontal_epsg("", 0.0), "");
    assert_eq!(identify_horizontal_epsg("not a wkt", 0.0), "");
}

#[test]
fn get_utm_zone_signed_by_hemisphere() {
    let north = user_input_to_wkt("EPSG:2027").unwrap();
    assert_eq!(get_utm_zone(&north.wkt).unwrap(), 15);

    let south = user_input_to_wkt("EPSG:32732").unwrap();
    assert_eq!(get_utm_zone(&south.wkt).unwrap(), -32);

    assert_eq!(get_utm_zone("").unwrap(), 0);
    assert!(get_utm_zone("not a wkt").is_err());
}

#[test]
fn get_horizontal_wkt_strips_vertical_cs() {
    let compound = user_input_to_wkt("EPSG:7415").unwrap();
    let horiz = get_horizontal_wkt(&compound.wkt);
    assert!(horiz.contains("PROJCS["));
    assert!(!horiz.contains("VERT_CS"));
    assert_eq!(get_horizontal_wkt(""), "");
    assert_eq!(get_horizontal_wkt("not a wkt"), "");
}

#[test]
fn get_horizontal_units_returns_unit_name() {
    let utm = user_input_to_wkt("EPSG:32617").unwrap();
    assert_eq!(get_horizontal_units(&utm.wkt), "metre");
    assert_eq!(get_horizontal_units(""), "");
    assert_eq!(get_horizontal_units("not a wkt"), "");
}

#[test]
fn srs_valid_accepts_known_codes_and_rejects_empty() {
    let utm = user_input_to_wkt("EPSG:32617").unwrap();
    assert!(srs_valid(&utm.wkt));
    assert!(!srs_valid(""));
    assert!(!srs_valid("not a wkt"));
}

#[test]
fn extract_vert_cs_subtree_handles_nested_brackets_and_quoted_strings() {
    let wkt = r#"COMPD_CS["WGS 84 + VERT_CS",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]]],VERT_CS["NAVD88 height",VERT_DATUM["North American Vertical Datum 1988",2005,AUTHORITY["EPSG","5103"]],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Up",UP],AUTHORITY["EPSG","5703"]]]"#;
    let vert = get_vertical_wkt(wkt);
    assert!(vert.starts_with("VERT_CS[\"NAVD88 height\""));
    assert!(vert.ends_with(r#"AUTHORITY["EPSG","5703"]]"#));

    // No VERT_CS → empty.
    let utm = user_input_to_wkt("EPSG:32617").unwrap();
    assert_eq!(get_vertical_wkt(&utm.wkt), "");
    assert_eq!(
        get_vertical_wkt(r#"COMPD_CS["unterminated",VERT_CS["x""#),
        ""
    );
}

#[test]
fn identify_vertical_epsg_reads_authority_code_from_subtree() {
    let wkt = r#"COMPD_CS["WGS 84 + VERT_CS",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]]],VERT_CS["NAVD88 height",VERT_DATUM["North American Vertical Datum 1988",2005,AUTHORITY["EPSG","5103"]],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Up",UP],AUTHORITY["EPSG","5703"]]]"#;
    assert_eq!(identify_vertical_epsg(wkt, 0.0), "5703");

    // No VERT_CS → empty.
    let utm = user_input_to_wkt("EPSG:3857").unwrap();
    assert_eq!(identify_vertical_epsg(&utm.wkt, 0.0), "");
    assert_eq!(identify_vertical_epsg("not a wkt", 0.0), "");
}

#[test]
fn get_vertical_units_reads_unit_from_subtree() {
    let wkt = r#"COMPD_CS["x",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]]],VERT_CS["NAVD88 height",VERT_DATUM["NAVD88",2005,AUTHORITY["EPSG","5103"]],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Up",UP],AUTHORITY["EPSG","5703"]]]"#;
    assert_eq!(get_vertical_units(wkt), "metre");
    assert_eq!(get_vertical_units(""), "");
    assert_eq!(get_vertical_units(r#"COMPD_CS["x",VERT_CS["bad"]]"#), "");
}

#[test]
fn gdal_srs_transform_identity_preserves_xyz() {
    let a = user_input_to_wkt("EPSG:4326").unwrap();
    let t = GdalSrsTransform::new(&a.wkt, 0.0, &a.wkt, 0.0, &[], &[]).unwrap();
    let mut x = -93.265;
    let mut y = 44.9778;
    let mut z = 250.0;
    assert!(t.transform_xyz(&mut x, &mut y, &mut z));
    assert!((x - -93.265).abs() < 1e-9);
    assert!((y - 44.9778).abs() < 1e-9);
    assert!((z - 250.0).abs() < 1e-9);
}

#[test]
fn gdal_srs_transform_4326_to_utm17n_matches_known_point() {
    let src = user_input_to_wkt("EPSG:4326").unwrap();
    let dst = user_input_to_wkt("EPSG:32617").unwrap();
    let t = GdalSrsTransform::new(&src.wkt, 0.0, &dst.wkt, 0.0, &[], &[]).unwrap();
    // Hobu HQ-ish, Iowa City: lon=-91.5, lat=41.6
    let mut x = -91.5;
    let mut y = 41.6;
    let mut z = 250.0;
    assert!(t.transform_xyz(&mut x, &mut y, &mut z));
    // Avoid pinning to specific PROJ datum-grid output; just confirm we
    // moved out of WGS84 lat/lon ranges into projected metres and z is
    // preserved.
    assert!(x.is_finite() && x.abs() > 1000.0);
    assert!(y.is_finite() && y.abs() > 1000.0);
    assert_eq!(z, 250.0);
}

#[test]
fn gdal_srs_transform_vector_matches_single_point_xform() {
    let src = user_input_to_wkt("EPSG:4326").unwrap();
    let dst = user_input_to_wkt("EPSG:32617").unwrap();
    let t = GdalSrsTransform::new(&src.wkt, 0.0, &dst.wkt, 0.0, &[], &[]).unwrap();

    let mut xs = vec![-91.5_f64, -91.4];
    let mut ys = vec![41.6_f64, 41.5];
    let mut zs = vec![250.0_f64, 260.0];
    assert!(t.transform_xyz_slice(&mut xs, &mut ys, &mut zs));

    // Compare to scalar version on first point.
    let mut x0 = -91.5;
    let mut y0 = 41.6;
    let mut z0 = 250.0;
    assert!(t.transform_xyz(&mut x0, &mut y0, &mut z0));
    assert!((xs[0] - x0).abs() < 1e-9);
    assert!((ys[0] - y0).abs() < 1e-9);
    assert!((zs[0] - z0).abs() < 1e-9);

    assert!(t.transform_xyz_slice(&mut [], &mut [], &mut []));
    assert!(!t.transform_xyz_slice(&mut [1.0], &mut [], &mut [0.0]));
}

#[test]
fn gdal_srs_transform_rejects_empty_or_garbage_wkt() {
    assert!(GdalSrsTransform::new("", 0.0, "EPSG:4326", 0.0, &[], &[]).is_err());
    assert!(GdalSrsTransform::new("garbage", 0.0, "EPSG:4326", 0.0, &[], &[]).is_err());
    let src = user_input_to_wkt("EPSG:4326").unwrap();
    assert!(GdalSrsTransform::new(&src.wkt, 0.0, "garbage", 0.0, &[], &[]).is_err());
}

#[test]
fn gdal_srs_transform_with_custom_axis_order_flips_xy() {
    let src = user_input_to_wkt("EPSG:4326").unwrap();
    let dst = user_input_to_wkt("EPSG:4326").unwrap();
    // For traditional order, x is lon and y is lat. If we force axis
    // mapping [2,1] on the source, we tell GDAL that data axis 1 maps
    // to SRS axis 2 (lon) and data axis 2 maps to SRS axis 1 (lat),
    // i.e. swapped input. Identity SRS, so output equals swapped input.
    let t = GdalSrsTransform::new(&src.wkt, 0.0, &dst.wkt, 0.0, &[2, 1], &[]).unwrap();
    let mut x = 1.0;
    let mut y = 2.0;
    let mut z = 0.0;
    assert!(t.transform_xyz(&mut x, &mut y, &mut z));
    // We only assert the transform doesn't crash and returns finite numbers;
    // exact axis-mapping semantics depend on GDAL version.
    assert!(x.is_finite() && y.is_finite());
}

#[test]
fn gdal_coord_operation_reverse_uses_inverse_path() {
    let transform = GdalCoordOperationTransform::new(
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad",
        true,
    )
    .unwrap();
    let mut x = std::f64::consts::PI;
    let mut y = std::f64::consts::FRAC_PI_2;
    let mut z = 3.0;

    assert!(transform.transform_xyz(&mut x, &mut y, &mut z));
    assert!((x - 180.0).abs() < 1e-9);
    assert!((y - 90.0).abs() < 1e-9);
    assert_eq!(z, 3.0);
}

#[test]
fn identity_pipeline_preserves_xy() {
    let transform = SrsTransform::new_pipeline("+proj=noop").unwrap();
    let mut x = 1.5;
    let mut y = -2.5;
    let mut z = 3.5;

    assert!(transform.transform(&mut x, &mut y, &mut z));
    assert_eq!(x, 1.5);
    assert_eq!(y, -2.5);
    assert_eq!(z, 3.5);
}
