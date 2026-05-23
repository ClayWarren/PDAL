use super::*;
use pdal_core::point::PointView;
use std::os::raw::c_char;

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn xyz_view(points: &[(f64, f64, f64)]) -> *mut PointView {
    let layout = pdal_point_layout_create();
    for dim in ["X", "Y", "Z"] {
        let name = cstring(dim);
        pdal_point_layout_register_dim(layout, name.as_ptr(), 9);
    }
    let view = pdal_point_view_create(layout);
    for (x, y, z) in points {
        let idx = pdal_point_view_add_point(view);
        for (dim, value) in [("X", *x), ("Y", *y), ("Z", *z)] {
            let name = cstring(dim);
            pdal_point_view_set_f64(view, idx, name.as_ptr(), value);
        }
    }
    view
}

#[test]
fn delaunay_triangulate_happy_and_null_paths() {
    unsafe {
        let view = xyz_view(&[
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (1.0, 1.0, 0.0),
        ]);
        let mut out_len: u64 = 0;
        let ptr = pdal_delaunay_triangulate(view, &mut out_len);
        assert!(!ptr.is_null());
        assert!(out_len > 0);
        assert_eq!(out_len % 3, 0);
        pdal_free_u64_array(ptr, out_len);

        let mut out_len2: u64 = 0;
        assert!(pdal_delaunay_triangulate(std::ptr::null(), &mut out_len2).is_null());
        assert!(pdal_delaunay_triangulate(view, std::ptr::null_mut()).is_null());

        pdal_point_view_destroy(view);
    }
}

#[test]
fn ogr_spec_parse_handles_valid_invalid_and_null() {
    unsafe {
        let input = cstring(r#"{"type": "ogr", "datasource": "places.shp", "drivers": ["ESRI Shapefile"], "layer": "places"}"#);
        let raw = pdal_ogr_spec_parse_json(input.as_ptr());
        let parsed: serde_json::Value = serde_json::from_str(&take_string(raw)).unwrap();
        assert_eq!(parsed["ok"], serde_json::json!(true));
        assert_eq!(parsed["datasource"], serde_json::json!("places.shp"));
        assert_eq!(parsed["drivers"], serde_json::json!(["ESRI Shapefile"]));

        let null_raw = pdal_ogr_spec_parse_json(std::ptr::null());
        let null_parsed: serde_json::Value =
            serde_json::from_str(&take_string(null_raw)).unwrap();
        assert_eq!(null_parsed["ok"], serde_json::json!(false));

        let bad = cstring("{ not json");
        let bad_raw = pdal_ogr_spec_parse_json(bad.as_ptr());
        let bad_parsed: serde_json::Value =
            serde_json::from_str(&take_string(bad_raw)).unwrap();
        assert_eq!(bad_parsed["ok"], serde_json::json!(false));
        assert!(bad_parsed["error"].is_string());
    }
}

#[test]
fn kernel_parse_stage_option_covers_ok_invalid_unknown_and_null() {
    unsafe {
        let mut stage: *mut c_char = std::ptr::null_mut();
        let mut option: *mut c_char = std::ptr::null_mut();
        let mut value: *mut c_char = std::ptr::null_mut();

        let input = cstring("--filters.range.limits=Z[0:10]");
        let rc =
            pdal_kernel_parse_stage_option(input.as_ptr(), true, &mut stage, &mut option, &mut value);
        assert_eq!(rc, 0);
        assert_eq!(take_string(stage), "filters.range");
        assert_eq!(take_string(option), "limits");
        assert_eq!(take_string(value), "Z[0:10]");

        let mut s2: *mut c_char = std::ptr::null_mut();
        let mut o2: *mut c_char = std::ptr::null_mut();
        let mut v2: *mut c_char = std::ptr::null_mut();
        let null_rc =
            pdal_kernel_parse_stage_option(std::ptr::null(), true, &mut s2, &mut o2, &mut v2);
        assert!(null_rc != 0);
        let _ = take_string(s2);
        let _ = take_string(o2);
        let _ = take_string(v2);

        let bad = cstring("not-a-flag");
        let bad_rc = pdal_kernel_parse_stage_option(
            bad.as_ptr(),
            true,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert!(bad_rc != 0);
    }
}

#[test]
fn plugin_valid_name_round_trips() {
    unsafe {
        let path = cstring("/some/dir/libpdal_plugin_filter_foo.so");
        let kind_reader = cstring("reader");
        let kind_filter = cstring("filter");
        let kind_writer = cstring("writer");
        let ext = cstring(".so");
        let kinds: [*const c_char; 3] = [
            kind_reader.as_ptr(),
            kind_filter.as_ptr(),
            kind_writer.as_ptr(),
        ];

        let raw = pdal_plugin_valid_name(path.as_ptr(), kinds.as_ptr(), 3, ext.as_ptr());
        let value = take_string(raw);
        assert!(value.contains("foo"));

        let raw_null = pdal_plugin_valid_name(
            std::ptr::null(),
            std::ptr::null(),
            0,
            std::ptr::null(),
        );
        let _ = take_string(raw_null);

        let nul_in_list: [*const c_char; 1] = [std::ptr::null()];
        let raw_with_null_entry =
            pdal_plugin_valid_name(path.as_ptr(), nul_in_list.as_ptr(), 1, ext.as_ptr());
        let _ = take_string(raw_with_null_entry);
    }
}

#[test]
fn grid_decimation_validate_and_kept_indices() {
    unsafe {
        let ok = cstring("max");
        assert!(pdal_grid_decimation_validate(1.0, ok.as_ptr()).is_null());

        let err_raw = pdal_grid_decimation_validate(0.0, ok.as_ptr());
        assert!(!err_raw.is_null());
        let _ = take_string(err_raw);

        let null_raw = pdal_grid_decimation_validate(1.0, std::ptr::null());
        assert!(!null_raw.is_null());
        let _ = take_string(null_raw);

        let bad_type = cstring("median");
        let bad_raw = pdal_grid_decimation_validate(1.0, bad_type.as_ptr());
        assert!(!bad_raw.is_null());
        let _ = take_string(bad_raw);

        let view = xyz_view(&[
            (0.0, 0.0, 0.0),
            (0.1, 0.0, 1.0),
            (1.0, 0.0, 2.0),
            (1.1, 0.0, 3.0),
        ]);
        let mut out_len: u64 = 0;
        let kept = pdal_grid_decimation_get_kept_indices(view, 1.0, ok.as_ptr(), &mut out_len);
        assert!(!kept.is_null());
        assert!(out_len > 0);
        pdal_free_u64_array(kept, out_len);

        let mut len2: u64 = 0;
        assert!(pdal_grid_decimation_get_kept_indices(
            std::ptr::null(),
            1.0,
            ok.as_ptr(),
            &mut len2,
        )
        .is_null());
        assert!(pdal_grid_decimation_get_kept_indices(
            view,
            1.0,
            std::ptr::null(),
            &mut len2,
        )
        .is_null());
        assert!(pdal_grid_decimation_get_kept_indices(
            view,
            1.0,
            ok.as_ptr(),
            std::ptr::null_mut(),
        )
        .is_null());

        pdal_free_u64_array(std::ptr::null_mut(), 0);
        pdal_point_view_destroy(view);
    }
}

#[test]
fn icp_register_handles_happy_path_and_null_inputs() {
    unsafe {
        let fixed = xyz_view(&[
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (1.0, 1.0, 0.0),
            (0.5, 0.5, 0.0),
        ]);
        let moving = xyz_view(&[
            (0.01, 0.0, 0.0),
            (1.01, 0.0, 0.0),
            (0.01, 1.0, 0.0),
            (1.01, 1.0, 0.0),
            (0.51, 0.5, 0.0),
        ]);

        let mut transform = [0.0f64; 16];
        let mut centroid = [0.0f64; 3];
        let mut converged = false;
        let mut mse = 0.0f64;
        let identity: [f64; 16] = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];

        let result = pdal_icp_register(
            fixed,
            moving,
            50,
            5,
            1e-7,
            1e-7,
            1e-12,
            true,
            10.0,
            true,
            identity.as_ptr(),
            transform.as_mut_ptr(),
            centroid.as_mut_ptr(),
            &mut converged,
            &mut mse,
        );
        assert!(!result.is_null());
        assert!(mse.is_finite());
        pdal_point_view_destroy(result);

        let r2 = pdal_icp_register(
            fixed,
            moving,
            5,
            2,
            1e-7,
            1e-7,
            1e-12,
            false,
            0.0,
            false,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert!(!r2.is_null());
        pdal_point_view_destroy(r2);

        assert!(pdal_icp_register(
            std::ptr::null(),
            moving,
            5, 2, 1e-7, 1e-7, 1e-12,
            false, 0.0, false, std::ptr::null(),
            std::ptr::null_mut(), std::ptr::null_mut(),
            std::ptr::null_mut(), std::ptr::null_mut(),
        )
        .is_null());
        assert!(pdal_icp_register(
            fixed,
            std::ptr::null(),
            5, 2, 1e-7, 1e-7, 1e-12,
            false, 0.0, false, std::ptr::null(),
            std::ptr::null_mut(), std::ptr::null_mut(),
            std::ptr::null_mut(), std::ptr::null_mut(),
        )
        .is_null());

        pdal_point_view_destroy(fixed);
        pdal_point_view_destroy(moving);
    }
}
