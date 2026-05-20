use super::*;

#[test]
fn point_view_carries_spatial_reference() {
    unsafe {
        let layout = pdal_point_layout_create();
        let x = CString::new("X").unwrap();
        pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
        let view = pdal_point_view_create(layout);

        let text = CString::new("EPSG:4978").unwrap();
        let srs = pdal_spatial_reference_create(text.as_ptr());
        pdal_point_view_set_spatial_reference(view, srs);

        let copied = pdal_point_view_spatial_reference(view);
        assert_eq!(
            take_string(pdal_spatial_reference_text(copied)),
            "EPSG:4978"
        );

        pdal_spatial_reference_destroy(copied);
        pdal_spatial_reference_destroy(srs);
        pdal_point_view_destroy(view);
    }
}

#[test]
fn point_view_exposes_layout_dimensions() {
    unsafe {
        let layout = pdal_point_layout_create();
        let x = CString::new("X").unwrap();
        let classification = CString::new("Classification").unwrap();
        pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
        pdal_point_layout_register_dim(layout, classification.as_ptr(), 0);
        let view = pdal_point_view_create(layout);

        assert_eq!(pdal_point_view_dim_count(view), 2);
        assert_eq!(take_string(pdal_point_view_dim_name(view, 0)), "X");
        assert_eq!(pdal_point_view_dim_type(view, 0), 9);
        assert_eq!(
            take_string(pdal_point_view_dim_name(view, 1)),
            "Classification"
        );
        assert_eq!(pdal_point_view_dim_type(view, 1), 0);

        pdal_point_view_destroy(view);
    }
}

#[test]
fn point_view_bounds_roundtrip_through_c_abi() {
    unsafe {
        let layout = pdal_point_layout_create();
        for dim in ["X", "Y", "Z"] {
            let name = CString::new(dim).unwrap();
            pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
        }
        let view = pdal_point_view_create(layout);

        for (x, y, z) in [(-10.0, 5.0, 100.0), (20.0, -15.0, -50.0), (3.0, 7.0, 25.0)] {
            let point = pdal_point_view_add_point(view);
            for (dim, value) in [("X", x), ("Y", y), ("Z", z)] {
                let name = CString::new(dim).unwrap();
                pdal_point_view_set_f64(view, point, name.as_ptr(), value);
            }
        }

        let mut bounds2d = pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        };
        assert!(pdal_point_view_calculate_bounds_2d(view, &mut bounds2d));
        assert_eq!(
            bounds2d,
            pdal_bounds2d_t {
                minx: -10.0,
                maxx: 20.0,
                miny: -15.0,
                maxy: 7.0,
            }
        );

        let mut bounds3d = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        };
        assert!(pdal_point_view_calculate_bounds_3d(view, &mut bounds3d));
        assert_eq!(
            bounds3d,
            pdal_bounds3d_t {
                minx: -10.0,
                maxx: 20.0,
                miny: -15.0,
                maxy: 7.0,
                minz: -50.0,
                maxz: 100.0,
            }
        );

        pdal_point_view_destroy(view);
    }
}

#[test]
fn point_view_bounds_c_abi_reports_unavailable_bounds() {
    unsafe {
        let layout = pdal_point_layout_create();
        let x = CString::new("X").unwrap();
        let y = CString::new("Y").unwrap();
        pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
        pdal_point_layout_register_dim(layout, y.as_ptr(), 9);
        let view = pdal_point_view_create(layout);

        let mut bounds2d = pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        };
        assert!(!pdal_point_view_calculate_bounds_2d(view, &mut bounds2d));

        let point = pdal_point_view_add_point(view);
        pdal_point_view_set_f64(view, point, x.as_ptr(), 1.0);
        pdal_point_view_set_f64(view, point, y.as_ptr(), 2.0);
        assert!(pdal_point_view_calculate_bounds_2d(view, &mut bounds2d));

        let mut bounds3d = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        };
        assert!(!pdal_point_view_calculate_bounds_3d(view, &mut bounds3d));
        assert!(!pdal_point_view_calculate_bounds_2d(
            std::ptr::null(),
            &mut bounds2d
        ));
        assert!(!pdal_point_view_calculate_bounds_2d(
            view,
            std::ptr::null_mut()
        ));

        pdal_point_view_destroy(view);
    }
}

#[test]
fn dimension_type_helpers_roundtrip_through_c_abi() {
    unsafe {
        let signed = CString::new("signed").unwrap();
        let int32 = CString::new("INT32_T").unwrap();
        let bad = CString::new("unknown").unwrap();

        assert_eq!(
            take_string(pdal_dimension_interpretation_name(0x200 | 2)),
            "uint16_t"
        );
        assert_eq!(pdal_dimension_type_from_name(int32.as_ptr()), 0x100 | 4);
        assert_eq!(
            pdal_dimension_type_from_base_and_size(signed.as_ptr(), 8),
            0x100 | 8
        );
        assert_eq!(pdal_dimension_type_from_name(bad.as_ptr()), 0);
        assert_eq!(
            pdal_dimension_type_from_base_and_size(std::ptr::null(), 8),
            0
        );
    }
}

#[test]
fn point_view_dimension_summaries_serialize_through_c_abi() {
    unsafe {
        let layout = pdal_point_layout_create();
        let x = CString::new("X").unwrap();
        let intensity = CString::new("Intensity").unwrap();
        pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
        pdal_point_layout_register_dim(layout, intensity.as_ptr(), 1);
        let view = pdal_point_view_create(layout);

        for (x_value, intensity_value) in [(-10.0, 7.0), (20.0, 3.0), (2.0, 5.0)] {
            let point = pdal_point_view_add_point(view);
            pdal_point_view_set_f64(view, point, x.as_ptr(), x_value);
            pdal_point_view_set_f64(view, point, intensity.as_ptr(), intensity_value);
        }

        let json: serde_json::Value =
            serde_json::from_str(&take_string(pdal_point_view_dimension_summaries_json(view)))
                .unwrap();
        assert_eq!(json[0]["name"], "X");
        assert_eq!(json[0]["count"], 3);
        assert_eq!(json[0]["minimum"], -10.0);
        assert_eq!(json[0]["maximum"], 20.0);
        assert_eq!(json[0]["mean"], 4.0);
        assert_eq!(json[1]["name"], "Intensity");
        assert_eq!(json[1]["minimum"], 3.0);
        assert_eq!(json[1]["maximum"], 7.0);
        assert_eq!(json[1]["mean"], 5.0);

        pdal_point_view_destroy(view);
    }
}
