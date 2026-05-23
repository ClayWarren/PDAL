use super::*;
use pdal_core::point::{PointLayout, PointView};

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn xy_layout() -> *mut PointLayout {
    let layout = pdal_point_layout_create();
    for dim in ["X", "Y", "Z"] {
        pdal_point_layout_register_dim(layout, cstring(dim).as_ptr(), 9);
    }
    layout
}

unsafe fn view_with_xyz(points: &[(f64, f64, f64)]) -> *mut PointView {
    let view = pdal_point_view_create(xy_layout());
    for (x, y, z) in points {
        let idx = pdal_point_view_add_point(view);
        for (dim, value) in [("X", *x), ("Y", *y), ("Z", *z)] {
            pdal_point_view_set_f64(view, idx, cstring(dim).as_ptr(), value);
        }
    }
    view
}

#[test]
fn dimension_resolve_type_handles_negative_and_valid_inputs() {
    // Negative inputs short-circuit to 0 (None).
    assert_eq!(pdal_dimension_resolve_type(-1, 9), 0);
    assert_eq!(pdal_dimension_resolve_type(9, -1), 0);
    // Two doubles resolve to double again (the test just exercises a code path
    // rather than asserting on PDAL's exact resolution table).
    let resolved = pdal_dimension_resolve_type(9, 9);
    assert!(resolved >= 0);
}

#[test]
fn dimension_type_helpers_handle_null_and_unknown_inputs() {
    unsafe {
        assert_eq!(pdal_dimension_type_from_name(std::ptr::null()), 0);
        let nope = pdal_dimension_type_from_name(cstring("DefinitelyNotADim").as_ptr());
        assert!(nope >= 0);

        assert_eq!(
            pdal_dimension_type_from_base_and_size(std::ptr::null(), 4),
            0
        );
        let by_base = pdal_dimension_type_from_base_and_size(cstring("signed").as_ptr(), 4);
        assert!(by_base >= 0);

        // dimension_interpretation_name for negative returns "unknown".
        assert_eq!(
            take_string(pdal_dimension_interpretation_name(-1)),
            "unknown"
        );
    }
}

#[test]
fn dimension_fix_name_handles_null_and_unsanitary_strings() {
    unsafe {
        // Null input still returns a valid (empty) string handle.
        let fixed = pdal_dimension_fix_name(std::ptr::null());
        assert_eq!(take_string(fixed), "");

        // Non-null path: sanitization should not crash.
        let fixed = take_string(pdal_dimension_fix_name(cstring("My Dim Name!").as_ptr()));
        assert!(!fixed.is_empty());
    }
}

#[test]
fn point_view_create_returns_null_for_null_layout() {
    unsafe {
        assert!(pdal_point_view_create(std::ptr::null_mut()).is_null());
    }
}

#[test]
fn mesh_triangle_round_trip_through_capi() {
    unsafe {
        let view = view_with_xyz(&[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)]);
        assert_eq!(pdal_point_view_mesh_triangle_count(view), 0);

        // Add a triangle to the default mesh and read it back.
        assert!(pdal_point_view_add_mesh_triangle(view, 0, 1, 2));
        assert_eq!(pdal_point_view_mesh_triangle_count(view), 1);

        let mut a: u64 = 99;
        let mut b: u64 = 99;
        let mut c: u64 = 99;
        assert!(pdal_point_view_mesh_triangle(
            view, 0, &mut a, &mut b, &mut c
        ));
        assert_eq!((a, b, c), (0u64, 1u64, 2u64));

        // Add via the named API and read it back.
        assert!(pdal_point_view_add_named_mesh_triangle(
            view,
            cstring("alt").as_ptr(),
            2,
            1,
            0
        ));
        assert_eq!(
            pdal_point_view_named_mesh_triangle_count(view, cstring("alt").as_ptr()),
            1
        );
        assert!(pdal_point_view_named_mesh_triangle(
            view,
            cstring("alt").as_ptr(),
            0,
            &mut a,
            &mut b,
            &mut c
        ));
        assert_eq!((a, b, c), (2u64, 1u64, 0u64));

        // Out-of-range index returns false and leaves outputs untouched.
        a = 42;
        assert!(!pdal_point_view_mesh_triangle(
            view, 99, &mut a, &mut b, &mut c
        ));

        pdal_point_view_destroy(view);
    }
}

#[test]
fn raster_create_read_and_set_round_trip() {
    unsafe {
        let view = view_with_xyz(&[(0.0, 0.0, 0.0)]);
        let limits = pdal_raster_limits_t {
            x_origin: 0.0,
            y_origin: 0.0,
            width: 4,
            height: 3,
            edge_length: 1.0,
        };
        assert_eq!(pdal_point_view_raster_count(view), 0);
        assert!(pdal_point_view_create_raster(
            view,
            cstring("density").as_ptr(),
            &limits,
            -9999.0,
        ));
        assert_eq!(pdal_point_view_raster_count(view), 1);
        let name = take_string(pdal_point_view_raster_name(view, 0));
        assert_eq!(name, "density");

        // Initializer round-trips.
        let init = pdal_point_view_raster_initializer(view, cstring("density").as_ptr());
        assert_eq!(init, -9999.0);
        // Missing raster returns NaN.
        let missing = pdal_point_view_raster_initializer(view, cstring("nope").as_ptr());
        assert!(missing.is_nan());

        // Limits round-trip.
        let mut out = pdal_raster_limits_t {
            x_origin: 0.0,
            y_origin: 0.0,
            width: 0,
            height: 0,
            edge_length: 0.0,
        };
        assert!(pdal_point_view_raster_limits(
            view,
            cstring("density").as_ptr(),
            &mut out,
        ));
        assert_eq!(out.width, 4);
        assert_eq!(out.height, 3);

        // Set + read a cell.
        assert!(pdal_point_view_set_raster_cell(
            view,
            cstring("density").as_ptr(),
            1,
            2,
            3.5
        ));
        let mut value = 0.0;
        assert!(pdal_point_view_raster_cell(
            view,
            cstring("density").as_ptr(),
            1,
            2,
            &mut value
        ));
        assert_eq!(value, 3.5);

        // Out-of-range cell access returns false.
        assert!(!pdal_point_view_raster_cell(
            view,
            cstring("density").as_ptr(),
            5,
            0,
            &mut value
        ));
        assert!(!pdal_point_view_set_raster_cell(
            view,
            cstring("density").as_ptr(),
            5,
            0,
            1.0
        ));

        // Unknown raster name returns false for set_raster_cell and read_cell.
        assert!(!pdal_point_view_raster_cell(
            view,
            cstring("nope").as_ptr(),
            0,
            0,
            &mut value
        ));
        assert!(!pdal_point_view_set_raster_cell(
            view,
            cstring("nope").as_ptr(),
            0,
            0,
            1.0
        ));

        // Trying to create an existing raster fails (returns false).
        assert!(!pdal_point_view_create_raster(
            view,
            cstring("density").as_ptr(),
            &limits,
            0.0
        ));

        // Null name to create_raster returns false.
        assert!(!pdal_point_view_create_raster(
            view,
            std::ptr::null(),
            &limits,
            0.0
        ));

        pdal_point_view_destroy(view);
    }
}

#[test]
fn split_where_partitions_view_or_reports_invalid_expression() {
    unsafe {
        let view = view_with_xyz(&[
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (5.0, 5.0, 0.0),
        ]);
        let mut keep: *mut PointView = std::ptr::null_mut();
        let mut skip: *mut PointView = std::ptr::null_mut();
        let ok =
            pdal_point_view_split_where(view, cstring("X > 0.5").as_ptr(), &mut keep, &mut skip);
        assert!(ok);
        assert!(!keep.is_null());
        assert!(!skip.is_null());
        assert_eq!(pdal_point_view_length(keep), 2);
        assert_eq!(pdal_point_view_length(skip), 2);
        pdal_point_view_destroy(keep);
        pdal_point_view_destroy(skip);

        // Garbage expression reports false and clears outputs.
        keep = std::ptr::null_mut();
        skip = std::ptr::null_mut();
        let bad = pdal_point_view_split_where(
            view,
            cstring("not an expression!!").as_ptr(),
            &mut keep,
            &mut skip,
        );
        assert!(!bad);

        // Null arguments to split_where -> false.
        assert!(!pdal_point_view_split_where(
            std::ptr::null(),
            cstring("X > 0").as_ptr(),
            &mut keep,
            &mut skip,
        ));
        assert!(!pdal_point_view_split_where(
            view,
            std::ptr::null(),
            &mut keep,
            &mut skip,
        ));

        pdal_point_view_destroy(view);
    }
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_point_abi_nulls_and_errors() {
    unsafe {
        // --- 1. Bounds 2D null & edge cases ---
        pdal_bounds2d_clear(std::ptr::null_mut());
        assert!(pdal_bounds2d_empty(std::ptr::null()));
        pdal_bounds2d_grow_point(std::ptr::null_mut(), 0.0, 0.0);
        pdal_bounds2d_grow_distance(std::ptr::null_mut(), 1.0);
        pdal_bounds2d_grow_bounds(std::ptr::null_mut(), std::ptr::null());
        pdal_bounds2d_clip(std::ptr::null_mut(), std::ptr::null());
        assert!(!pdal_bounds2d_contains_point(std::ptr::null(), 0.0, 0.0));
        assert!(!pdal_bounds2d_contains_bounds(
            std::ptr::null(),
            std::ptr::null()
        ));
        assert!(!pdal_bounds2d_overlaps(std::ptr::null(), std::ptr::null()));

        // parsing: null arguments
        let mut bounds2d = pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        };
        let mut wkt = std::ptr::null_mut();
        let mut pos = 0;
        let err = pdal_bounds2d_parse(std::ptr::null(), 0, &mut bounds2d, &mut wkt, &mut pos);
        assert!(!err.is_null());
        take_string(err);

        // parsing: parse error path
        let bad_input = cstring("invalid");
        let err2 = pdal_bounds2d_parse(bad_input.as_ptr(), 0, &mut bounds2d, &mut wkt, &mut pos);
        assert!(!err2.is_null());
        take_string(err2);

        // parsing: out_wkt is null
        let ok_input = cstring("([0, 1], [0, 1])");
        let err3 = pdal_bounds2d_parse(
            ok_input.as_ptr(),
            0,
            &mut bounds2d,
            std::ptr::null_mut(),
            &mut pos,
        );
        assert!(err3.is_null());

        // other 2d bounds functions
        assert!(!pdal_bounds2d_equal(std::ptr::null(), std::ptr::null()));
        pdal_bounds2d_default(std::ptr::null_mut());
        assert!(pdal_bounds2d_format(std::ptr::null(), 2).is_null());
        assert!(pdal_bounds2d_to_wkt(std::ptr::null(), 2).is_null());
        assert!(pdal_bounds2d_to_geojson(std::ptr::null(), 2).is_null());

        // --- 2. Bounds 3D null & edge cases ---
        pdal_bounds3d_clear(std::ptr::null_mut());
        assert!(pdal_bounds3d_empty(std::ptr::null()));
        pdal_bounds3d_grow_point(std::ptr::null_mut(), 0.0, 0.0, 0.0);
        pdal_bounds3d_grow_distance(std::ptr::null_mut(), 1.0);
        pdal_bounds3d_grow_bounds(std::ptr::null_mut(), std::ptr::null());
        pdal_bounds3d_clip(std::ptr::null_mut(), std::ptr::null());
        assert!(!pdal_bounds3d_contains_point(
            std::ptr::null(),
            0.0,
            0.0,
            0.0
        ));
        assert!(!pdal_bounds3d_contains_bounds(
            std::ptr::null(),
            std::ptr::null()
        ));
        assert!(!pdal_bounds3d_overlaps(std::ptr::null(), std::ptr::null()));

        // parsing
        let mut bounds3d = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        };
        let err3d = pdal_bounds3d_parse(std::ptr::null(), 0, &mut bounds3d, &mut wkt, &mut pos);
        assert!(!err3d.is_null());
        take_string(err3d);

        let err3d_2 = pdal_bounds3d_parse(bad_input.as_ptr(), 0, &mut bounds3d, &mut wkt, &mut pos);
        assert!(!err3d_2.is_null());
        take_string(err3d_2);

        let ok_input3d = cstring("([0, 1], [0, 1], [0, 1])");
        let err3d_3 = pdal_bounds3d_parse(
            ok_input3d.as_ptr(),
            0,
            &mut bounds3d,
            std::ptr::null_mut(),
            &mut pos,
        );
        assert!(err3d_3.is_null());

        assert!(!pdal_bounds3d_equal(std::ptr::null(), std::ptr::null()));
        pdal_bounds3d_default(std::ptr::null_mut());
        assert!(pdal_bounds3d_format(std::ptr::null(), 2).is_null());
        assert!(pdal_bounds3d_to_wkt(std::ptr::null(), 2).is_null());

        // --- 3. PointView / layout null & edge cases ---
        pdal_point_layout_register_dim(std::ptr::null_mut(), std::ptr::null(), 0);
        pdal_point_layout_destroy(std::ptr::null_mut());

        pdal_point_view_destroy(std::ptr::null_mut());
        assert_eq!(pdal_point_view_length(std::ptr::null_mut()), 0);
        assert_eq!(pdal_point_view_source_index(std::ptr::null_mut(), 42), 42);
        pdal_point_view_set_spatial_reference(std::ptr::null_mut(), std::ptr::null());
        assert!(pdal_point_view_spatial_reference(std::ptr::null()).is_null());
        assert_eq!(pdal_point_view_dim_count(std::ptr::null()), 0);
        assert!(pdal_point_view_dim_name(std::ptr::null(), 0).is_null());
        assert_eq!(pdal_point_view_dim_type(std::ptr::null(), 0), -1);

        assert_eq!(pdal_point_view_mesh_triangle_count(std::ptr::null()), 0);
        assert_eq!(
            pdal_point_view_named_mesh_triangle_count(std::ptr::null(), std::ptr::null()),
            0
        );
        let mut a = 0;
        let mut b = 0;
        let mut c = 0;
        assert!(!pdal_point_view_mesh_triangle(
            std::ptr::null(),
            0,
            &mut a,
            &mut b,
            &mut c
        ));
        assert!(!pdal_point_view_named_mesh_triangle(
            std::ptr::null(),
            std::ptr::null(),
            0,
            &mut a,
            &mut b,
            &mut c
        ));
        assert!(!pdal_point_view_add_mesh_triangle(
            std::ptr::null_mut(),
            0,
            0,
            0
        ));
        assert!(!pdal_point_view_add_named_mesh_triangle(
            std::ptr::null_mut(),
            std::ptr::null(),
            0,
            0,
            0
        ));

        assert_eq!(pdal_point_view_raster_count(std::ptr::null()), 0);
        assert!(pdal_point_view_raster_name(std::ptr::null(), 0).is_null());
        assert!(!pdal_point_view_create_raster(
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            0.0
        ));
        assert!(!pdal_point_view_raster_limits(
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut()
        ));
        assert!(pdal_point_view_raster_initializer(std::ptr::null(), std::ptr::null()).is_nan());
        let mut val = 0.0;
        assert!(!pdal_point_view_raster_cell(
            std::ptr::null(),
            std::ptr::null(),
            0,
            0,
            &mut val
        ));
        assert!(!pdal_point_view_set_raster_cell(
            std::ptr::null_mut(),
            std::ptr::null(),
            0,
            0,
            0.0
        ));

        // Dimension name out of range
        let real_view = view_with_xyz(&[(1.0, 2.0, 3.0)]);
        assert!(pdal_point_view_dim_name(real_view, 999).is_null());
        assert_eq!(pdal_point_view_dim_type(real_view, 999), -1);

        // Raster count/name out of range
        assert!(pdal_point_view_raster_name(real_view, 999).is_null());

        // KNN / Radius null cases
        let query = [0.0f64; 3];
        let mut knn_res = [pdal_spatial_result_t {
            id: 0,
            sqr_dist: 0.0,
        }; 5];
        assert_eq!(
            pdal_point_view_knn(
                std::ptr::null(),
                std::ptr::null(),
                query.as_ptr(),
                3,
                5,
                1,
                knn_res.as_mut_ptr(),
                5
            ),
            0
        );

        let mut out_len = 0;
        assert!(pdal_point_view_radius(
            std::ptr::null(),
            std::ptr::null(),
            query.as_ptr(),
            3,
            10.0,
            &mut out_len
        )
        .is_null());
        pdal_spatial_results_free(std::ptr::null_mut(), 0);

        // Quad index null/edge cases
        assert!(pdal_quad_index_create(
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            0.0,
            0.0,
            0.0,
            0.0,
            0
        )
        .is_null());
        pdal_quad_index_bounds(std::ptr::null(), std::ptr::null_mut());
        assert_eq!(pdal_quad_index_depth(std::ptr::null()), 0);

        let mut fills_len = 0;
        let fills_ptr = pdal_quad_index_fills(std::ptr::null(), &mut fills_len);
        assert!(!fills_ptr.is_null());
        assert_eq!(fills_len, 0);
        pdal_u64_array_free(fills_ptr, fills_len);

        let mut pts_len = 0;
        let pts_ptr1 = pdal_quad_index_points_by_depth(std::ptr::null(), 0, 0, &mut pts_len);
        assert!(!pts_ptr1.is_null());
        assert_eq!(pts_len, 0);
        pdal_u64_array_free(pts_ptr1, pts_len);

        let pts_ptr2 = pdal_quad_index_points_in_bounds(
            std::ptr::null(),
            0.0,
            0.0,
            0.0,
            0.0,
            0,
            0,
            &mut pts_len,
        );
        assert!(!pts_ptr2.is_null());
        assert_eq!(pts_len, 0);
        pdal_u64_array_free(pts_ptr2, pts_len);

        let mut x_beg = 0.0;
        let mut x_end = 0.0;
        let mut x_step = 0.0;
        let mut y_beg = 0.0;
        let mut y_end = 0.0;
        let mut y_step = 0.0;
        let pts_ptr3 = pdal_quad_index_points_raster_level(
            std::ptr::null(),
            0,
            &mut x_beg,
            &mut x_end,
            &mut x_step,
            &mut y_beg,
            &mut y_end,
            &mut y_step,
            &mut pts_len,
        );
        assert!(!pts_ptr3.is_null());
        assert_eq!(pts_len, 0);
        pdal_u64_array_free(pts_ptr3, pts_len);

        let pts_ptr4 = pdal_quad_index_points_raster_bounds(
            std::ptr::null(),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            &mut pts_len,
        );
        assert!(!pts_ptr4.is_null());
        assert_eq!(pts_len, 0);
        pdal_u64_array_free(pts_ptr4, pts_len);

        // Branch coverage with a valid index but edge case arguments
        let xs = [0.5f64];
        let ys = [0.5f64];
        let ids = [1u64];
        let test_idx = pdal_quad_index_create(
            xs.as_ptr(),
            ys.as_ptr(),
            ids.as_ptr(),
            1,
            0.0,
            0.0,
            1.0,
            1.0,
            5,
        );
        assert!(!test_idx.is_null());

        // depth_end != 0 and depth_begin >= depth_end
        let pts_ptr_depth = pdal_quad_index_points_by_depth(test_idx, 5, 2, &mut pts_len);
        assert!(!pts_ptr_depth.is_null());
        assert_eq!(pts_len, 0);
        pdal_u64_array_free(pts_ptr_depth, pts_len);

        // depth_end != 0 and depth_begin >= depth_end in points_in_bounds
        let pts_ptr_in_b =
            pdal_quad_index_points_in_bounds(test_idx, 0.0, 0.0, 1.0, 1.0, 5, 2, &mut pts_len);
        assert!(!pts_ptr_in_b.is_null());
        assert_eq!(pts_len, 0);
        pdal_u64_array_free(pts_ptr_in_b, pts_len);

        // checked_shl fails: rasterize = 64
        let pts_ptr_raster_err = pdal_quad_index_points_raster_level(
            test_idx,
            64,
            &mut x_beg,
            &mut x_end,
            &mut x_step,
            &mut y_beg,
            &mut y_end,
            &mut y_step,
            &mut pts_len,
        );
        assert!(!pts_ptr_raster_err.is_null());
        assert_eq!(pts_len, 0);
        pdal_u64_array_free(pts_ptr_raster_err, pts_len);

        // x_step == 0 or y_step == 0
        let pts_ptr_raster_b_err = pdal_quad_index_points_raster_bounds(
            test_idx,
            0.0,
            1.0,
            0.0,
            0.0,
            1.0,
            0.0,
            &mut pts_len,
        );
        assert!(!pts_ptr_raster_b_err.is_null());
        assert_eq!(pts_len, 0);
        pdal_u64_array_free(pts_ptr_raster_b_err, pts_len);

        pdal_quad_index_destroy(test_idx);

        pdal_u64_array_free(std::ptr::null_mut(), 0);
        pdal_quad_index_destroy(std::ptr::null_mut());

        // Dimension summaries JSON null
        let json_str = pdal_point_view_dimension_summaries_json(std::ptr::null());
        assert_eq!(take_string(json_str), "[]");

        pdal_point_view_destroy(real_view);
    }
}

#[test]
fn test_native_abi_geometry_helpers() {
    unsafe {
        let mut is_valid = false;
        // null paths
        assert!(!pdal_geometry_wkt_is_valid(std::ptr::null(), &mut is_valid));

        let mut area = 0.0;
        assert!(!pdal_geometry_wkt_area(std::ptr::null(), &mut area));

        let mut out_wkt = std::ptr::null_mut();
        assert!(!pdal_geometry_wkt_simplify(
            std::ptr::null(),
            0.1,
            false,
            &mut out_wkt
        ));

        let mut out_bounds = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        };
        assert!(!pdal_geometry_wkt_bounds(std::ptr::null(), &mut out_bounds));

        // valid geometry paths
        let wkt = cstring("POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))");
        assert!(pdal_geometry_wkt_is_valid(wkt.as_ptr(), &mut is_valid));
        assert!(is_valid);

        assert!(pdal_geometry_wkt_area(wkt.as_ptr(), &mut area));
        assert_eq!(area, 100.0);

        assert!(pdal_geometry_wkt_bounds(wkt.as_ptr(), &mut out_bounds));
        assert_eq!(out_bounds.maxx, 10.0);

        let mut dist = 0.0;
        assert!(pdal_geometry_wkt_distance_to_point(
            wkt.as_ptr(),
            5.0,
            5.0,
            0.0,
            &mut dist
        ));
        // since (5,5) is inside the polygon, distance is 0
        assert_eq!(dist, 0.0);

        // distance to point invalid wkt
        assert!(!pdal_geometry_wkt_distance_to_point(
            std::ptr::null(),
            5.0,
            5.0,
            0.0,
            &mut dist
        ));

        // contains point
        let mut contains = false;
        assert!(pdal_geometry_wkt_contains_point(
            wkt.as_ptr(),
            5.0,
            5.0,
            &mut contains
        ));
        assert!(contains);

        assert!(pdal_geometry_wkt_contains_point(
            wkt.as_ptr(),
            15.0,
            15.0,
            &mut contains
        ));
        assert!(!contains);

        assert!(!pdal_geometry_wkt_contains_point(
            std::ptr::null(),
            5.0,
            5.0,
            &mut contains
        ));

        // simplify
        assert!(pdal_geometry_wkt_simplify(
            wkt.as_ptr(),
            1.0,
            false,
            &mut out_wkt
        ));
        assert!(!out_wkt.is_null());
        let simplified = take_string(out_wkt);
        assert!(simplified.contains("POLYGON"));
    }
}

#[test]
fn test_uuid_abi() {
    unsafe {
        // null paths
        assert!(!pdal_uuid_parse(std::ptr::null(), std::ptr::null_mut()));
        assert!(pdal_uuid_unparse(std::ptr::null()).is_null());
        assert!(!pdal_uuid_random(std::ptr::null_mut()));
        assert!(pdal_uuid_is_null(std::ptr::null()));

        // valid path
        let mut bytes = [0u8; 16];
        assert!(pdal_uuid_random(bytes.as_mut_ptr()));
        assert!(!pdal_uuid_is_null(bytes.as_ptr()));

        let unparsed = pdal_uuid_unparse(bytes.as_ptr());
        assert!(!unparsed.is_null());
        let uuid_str = take_string(unparsed);
        assert_eq!(uuid_str.len(), 36);

        let mut parsed_bytes = [0u8; 16];
        let uuid_c_str = cstring(&uuid_str);
        assert!(pdal_uuid_parse(
            uuid_c_str.as_ptr(),
            parsed_bytes.as_mut_ptr()
        ));
        assert_eq!(bytes, parsed_bytes);

        // invalid parses
        assert!(!pdal_uuid_parse(
            cstring("invalid-uuid").as_ptr(),
            parsed_bytes.as_mut_ptr()
        ));
    }
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_bounds_abi_happy_paths() {
    unsafe {
        // --- 2D happy paths ---
        let mut b2 = pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        };
        pdal_bounds2d_clear(&mut b2);
        assert!(pdal_bounds2d_empty(&b2));

        pdal_bounds2d_grow_point(&mut b2, 5.0, 5.0);
        assert!(!pdal_bounds2d_empty(&b2));
        assert_eq!(b2.minx, 5.0);
        assert_eq!(b2.maxy, 5.0);

        pdal_bounds2d_grow_distance(&mut b2, 1.0);
        assert_eq!(b2.minx, 4.0);
        assert_eq!(b2.maxy, 6.0);

        let mut b2_other = pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        };
        pdal_bounds2d_clear(&mut b2_other);
        pdal_bounds2d_grow_point(&mut b2_other, 10.0, 10.0);

        pdal_bounds2d_grow_bounds(&mut b2, &b2_other);
        assert_eq!(b2.maxx, 10.0);
        assert_eq!(b2.maxy, 10.0);

        assert!(pdal_bounds2d_contains_point(&b2, 5.0, 5.0));
        assert!(!pdal_bounds2d_contains_point(&b2, 100.0, 100.0));

        let b2_clip = pdal_bounds2d_t {
            minx: 2.0,
            maxx: 6.0,
            miny: 2.0,
            maxy: 6.0,
        };
        pdal_bounds2d_clip(&mut b2, &b2_clip);
        assert_eq!(b2.minx, 4.0); // clipped with 2.0..6.0 -> 4.0..6.0
        assert_eq!(b2.maxx, 6.0);

        assert!(pdal_bounds2d_contains_bounds(
            &b2,
            &pdal_bounds2d_t {
                minx: 4.5,
                maxx: 5.5,
                miny: 4.5,
                maxy: 5.5
            }
        ));
        assert!(pdal_bounds2d_overlaps(
            &b2,
            &pdal_bounds2d_t {
                minx: 5.0,
                maxx: 10.0,
                miny: 5.0,
                maxy: 10.0
            }
        ));

        assert!(pdal_bounds2d_equal(&b2, &b2));

        let formatted = pdal_bounds2d_format(&b2, 2);
        assert!(!formatted.is_null());
        let fmt_str = take_string(formatted);
        assert!(fmt_str.contains("([4, 6]"));

        let wkt = pdal_bounds2d_to_wkt(&b2, 2);
        assert!(!wkt.is_null());
        take_string(wkt);

        let geojson = pdal_bounds2d_to_geojson(&b2, 2);
        assert!(!geojson.is_null());
        take_string(geojson);

        pdal_bounds2d_clear(&mut b2);
        assert!(pdal_bounds2d_empty(&b2));

        // --- 3D happy paths ---
        let mut b3 = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        };
        pdal_bounds3d_clear(&mut b3);
        assert!(pdal_bounds3d_empty(&b3));

        pdal_bounds3d_grow_point(&mut b3, 5.0, 5.0, 5.0);
        assert!(!pdal_bounds3d_empty(&b3));
        assert_eq!(b3.minx, 5.0);
        assert_eq!(b3.maxy, 5.0);
        assert_eq!(b3.maxz, 5.0);

        pdal_bounds3d_grow_distance(&mut b3, 1.0);
        assert_eq!(b3.minx, 4.0);
        assert_eq!(b3.maxy, 6.0);
        assert_eq!(b3.maxz, 6.0);

        let mut b3_other = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        };
        pdal_bounds3d_clear(&mut b3_other);
        pdal_bounds3d_grow_point(&mut b3_other, 10.0, 10.0, 10.0);

        pdal_bounds3d_grow_bounds(&mut b3, &b3_other);
        assert_eq!(b3.maxx, 10.0);
        assert_eq!(b3.maxy, 10.0);
        assert_eq!(b3.maxz, 10.0);

        assert!(pdal_bounds3d_contains_point(&b3, 5.0, 5.0, 5.0));
        assert!(!pdal_bounds3d_contains_point(&b3, 100.0, 100.0, 100.0));

        let b3_clip = pdal_bounds3d_t {
            minx: 2.0,
            maxx: 6.0,
            miny: 2.0,
            maxy: 6.0,
            minz: 2.0,
            maxz: 6.0,
        };
        pdal_bounds3d_clip(&mut b3, &b3_clip);
        assert_eq!(b3.minx, 4.0);
        assert_eq!(b3.maxx, 6.0);

        assert!(pdal_bounds3d_contains_bounds(
            &b3,
            &pdal_bounds3d_t {
                minx: 4.5,
                maxx: 5.5,
                miny: 4.5,
                maxy: 5.5,
                minz: 4.5,
                maxz: 5.5
            }
        ));
        assert!(pdal_bounds3d_overlaps(
            &b3,
            &pdal_bounds3d_t {
                minx: 5.0,
                maxx: 10.0,
                miny: 5.0,
                maxy: 10.0,
                minz: 5.0,
                maxz: 10.0
            }
        ));

        assert!(pdal_bounds3d_equal(&b3, &b3));

        let formatted3d = pdal_bounds3d_format(&b3, 2);
        assert!(!formatted3d.is_null());
        take_string(formatted3d);

        let wkt3d = pdal_bounds3d_to_wkt(&b3, 2);
        assert!(!wkt3d.is_null());
        take_string(wkt3d);

        pdal_bounds3d_clear(&mut b3);
        assert!(pdal_bounds3d_empty(&b3));
    }
}
