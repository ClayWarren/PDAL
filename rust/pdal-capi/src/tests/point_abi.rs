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
fn bounds_parse_roundtrips_through_c_abi() {
    unsafe {
        let input = CString::new("([1,101],[2,102],[3,103])").unwrap();
        let mut bounds = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        };
        let mut wkt = std::ptr::null_mut();
        let mut pos = 0;
        let err = pdal_bounds3d_parse(input.as_ptr(), 0, &mut bounds, &mut wkt, &mut pos);
        assert!(err.is_null());
        assert_eq!(bounds.minx, 1.0);
        assert_eq!(bounds.maxz, 103.0);
        assert_eq!(pos, 25);
        assert_eq!(take_string(wkt), "");

        let input =
            CString::new(r#"{"minx": 1,"miny": 2,"maxx": 101,"maxy": 102,"crs":"EPSG:2596"}"#)
                .unwrap();
        let mut bounds2d = pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        };
        let mut wkt = std::ptr::null_mut();
        let err = pdal_bounds2d_parse(input.as_ptr(), 0, &mut bounds2d, &mut wkt, &mut pos);
        assert!(err.is_null());
        assert_eq!(bounds2d.maxx, 101.0);
        assert_eq!(take_string(wkt), "EPSG:2596");
    }
}

#[test]
fn bounds_operations_roundtrip_through_c_abi() {
    unsafe {
        let mut bounds2d = pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        };
        pdal_bounds2d_clear(&mut bounds2d);
        assert!(pdal_bounds2d_empty(&bounds2d));
        assert!(pdal_bounds2d_empty(std::ptr::null()));
        pdal_bounds2d_grow_point(&mut bounds2d, 1.0, 2.0);
        pdal_bounds2d_grow_point(&mut bounds2d, 3.0, 4.0);
        pdal_bounds2d_grow_distance(&mut bounds2d, 1.0);
        assert!(pdal_bounds2d_contains_point(&bounds2d, 1.0, 2.0));
        assert!(!pdal_bounds2d_contains_point(std::ptr::null(), 1.0, 2.0));

        let other2d = pdal_bounds2d_t {
            minx: 0.5,
            maxx: 2.0,
            miny: 1.5,
            maxy: 3.0,
        };
        assert!(pdal_bounds2d_contains_bounds(&bounds2d, &other2d));
        assert!(pdal_bounds2d_overlaps(&bounds2d, &other2d));
        pdal_bounds2d_clip(&mut bounds2d, &other2d);
        assert_eq!(bounds2d, other2d);
        pdal_bounds2d_grow_bounds(&mut bounds2d, &other2d);
        assert!(!pdal_bounds2d_contains_bounds(&bounds2d, std::ptr::null()));
        pdal_bounds2d_grow_point(std::ptr::null_mut(), 0.0, 0.0);

        let mut bounds3d = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        };
        pdal_bounds3d_clear(&mut bounds3d);
        assert!(pdal_bounds3d_empty(&bounds3d));
        assert!(pdal_bounds3d_empty(std::ptr::null()));
        pdal_bounds3d_grow_point(&mut bounds3d, 1.0, 2.0, 3.0);
        pdal_bounds3d_grow_point(&mut bounds3d, 4.0, 5.0, 6.0);
        pdal_bounds3d_grow_distance(&mut bounds3d, 1.0);
        assert!(pdal_bounds3d_contains_point(&bounds3d, 1.0, 2.0, 3.0));
        assert!(!pdal_bounds3d_contains_point(
            std::ptr::null(),
            1.0,
            2.0,
            3.0
        ));

        let other3d = pdal_bounds3d_t {
            minx: 0.5,
            maxx: 2.0,
            miny: 1.5,
            maxy: 3.0,
            minz: 2.5,
            maxz: 4.0,
        };
        assert!(pdal_bounds3d_contains_bounds(&bounds3d, &other3d));
        assert!(pdal_bounds3d_overlaps(&bounds3d, &other3d));
        pdal_bounds3d_clip(&mut bounds3d, &other3d);
        assert_eq!(bounds3d, other3d);
        pdal_bounds3d_grow_bounds(&mut bounds3d, &other3d);
        assert!(!pdal_bounds3d_overlaps(&bounds3d, std::ptr::null()));
        pdal_bounds3d_grow_point(std::ptr::null_mut(), 0.0, 0.0, 0.0);
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
fn dimension_names_cover_ported_layout_mapping() {
    unsafe {
        let layout = pdal_point_layout_create();
        let names = [
            "W",
            "TextureU",
            "TextureV",
            "TextureW",
            "ClusterID",
            "HeightAboveGround",
            "LocalOutlierFactor",
            "LocalReachabilityDistance",
            "RadialDensity",
            "NNDistance",
            "Reciprocity",
            "Rank",
            "Coplanar",
            "PlaneFit",
            "Eigenvalue0",
            "Eigenvalue1",
            "Eigenvalue2",
            "OptimalKNN",
            "OptimalRadius",
            "H3",
            "GpsTime",
            "StartPulse",
            "ReflectedPulse",
            "Azimuth",
            "Pitch",
            "Roll",
            "Pdop",
            "PulseWidth",
            "PassiveSignal",
            "PassiveX",
            "PassiveY",
            "PassiveZ",
            "ReturnNumber",
            "NumberOfReturns",
            "ScanAngleRank",
            "PointSourceId",
            "EdgeOfFlightLine",
            "Flag",
            "Mark",
            "Alpha",
            "EchoRange",
            "Userdata",
            "EchoNorm",
            "EchoPos",
            "Reflectance",
            "Deviation",
            "Reliability",
            "Amplitude",
            "NormalX",
            "NormalY",
            "NormalZ",
            "Dimension",
            "Image",
            "Infrared",
            "XVelocity",
            "YVelocity",
            "ZVelocity",
            "WanderAngle",
            "XBodyAccel",
            "YBodyAccel",
            "ZBodyAccel",
            "XBodyAngRate",
            "YBodyAngRate",
            "ZBodyAngRate",
            "NorthPositionRMS",
            "EastPositionRMS",
            "DownPositionRMS",
            "NorthVelocityRMS",
            "EastVelocityRMS",
            "DownVelocityRMS",
            "RollRMS",
            "PitchRMS",
            "HeadingRMS",
            "Red",
            "Green",
            "Blue",
            "CustomDim",
        ];
        let cstrings: Vec<CString> = names
            .iter()
            .map(|name| CString::new(*name).unwrap())
            .collect();
        for name in &cstrings {
            pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
        }
        let view = pdal_point_view_create(layout);
        let point = pdal_point_view_add_point(view);
        for (idx, name) in cstrings.iter().enumerate() {
            pdal_point_view_set_f64(view, point, name.as_ptr(), idx as f64);
        }

        assert_eq!(pdal_point_view_dim_count(view), names.len() as u64);
        assert_eq!(
            pdal_point_view_get_f64(view, point, CString::new("CustomDim").unwrap().as_ptr()),
            (names.len() - 1) as f64
        );
        assert_eq!(pdal_point_view_dim_type(view, names.len() as u64), -1);
        assert!(pdal_point_view_dim_name(view, names.len() as u64).is_null());
        assert_eq!(
            pdal_point_view_get_f64(std::ptr::null_mut(), 0, cstrings[0].as_ptr()),
            0.0
        );
        assert_eq!(pdal_point_view_source_index(std::ptr::null_mut(), 42), 42);

        pdal_point_view_destroy(view);

        let throwaway = pdal_point_layout_create();
        pdal_point_layout_destroy(throwaway);
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

#[test]
fn point_view_named_mesh_roundtrips_through_c_abi() {
    unsafe {
        let layout = pdal_point_layout_create();
        let x = CString::new("X").unwrap();
        pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
        let view = pdal_point_view_create(layout);
        for _ in 0..3 {
            pdal_point_view_add_point(view);
        }

        let mesh_name = CString::new("surface").unwrap();
        assert!(pdal_point_view_add_named_mesh_triangle(
            view,
            mesh_name.as_ptr(),
            0,
            1,
            2
        ));
        assert_eq!(
            pdal_point_view_named_mesh_triangle_count(view, mesh_name.as_ptr()),
            1
        );

        let mut a = 99;
        let mut b = 99;
        let mut c = 99;
        assert!(pdal_point_view_named_mesh_triangle(
            view,
            mesh_name.as_ptr(),
            0,
            &mut a,
            &mut b,
            &mut c
        ));
        assert_eq!((a, b, c), (0, 1, 2));

        pdal_point_view_destroy(view);
    }
}

#[test]
fn point_view_raster_roundtrips_through_c_abi() {
    unsafe {
        let layout = pdal_point_layout_create();
        let view = pdal_point_view_create(layout);
        let name = CString::new("faceraster").unwrap();
        let limits = pdal_raster_limits_t {
            x_origin: 10.0,
            y_origin: 20.0,
            width: 2,
            height: 2,
            edge_length: 0.5,
        };

        assert!(pdal_point_view_create_raster(
            view,
            name.as_ptr(),
            &limits,
            -9999.0
        ));
        assert!(!pdal_point_view_create_raster(
            view,
            name.as_ptr(),
            &limits,
            -9999.0
        ));
        assert_eq!(pdal_point_view_raster_count(view), 1);
        assert_eq!(
            take_string(pdal_point_view_raster_name(view, 0)),
            "faceraster"
        );

        let mut copied_limits = pdal_raster_limits_t {
            x_origin: 0.0,
            y_origin: 0.0,
            width: 0,
            height: 0,
            edge_length: 0.0,
        };
        assert!(pdal_point_view_raster_limits(
            view,
            name.as_ptr(),
            &mut copied_limits
        ));
        assert_eq!(copied_limits, limits);
        assert_eq!(
            pdal_point_view_raster_initializer(view, name.as_ptr()),
            -9999.0
        );

        assert!(pdal_point_view_set_raster_cell(
            view,
            name.as_ptr(),
            1,
            0,
            42.0
        ));
        let mut value = 0.0;
        assert!(pdal_point_view_raster_cell(
            view,
            name.as_ptr(),
            1,
            0,
            &mut value
        ));
        assert_eq!(value, 42.0);
        assert!(!pdal_point_view_raster_cell(
            view,
            name.as_ptr(),
            2,
            0,
            &mut value
        ));

        pdal_point_view_destroy(view);
    }
}
