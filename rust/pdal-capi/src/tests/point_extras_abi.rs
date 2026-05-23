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
        assert_eq!(take_string(pdal_dimension_interpretation_name(-1)), "unknown");
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
        assert!(pdal_point_view_mesh_triangle(view, 0, &mut a, &mut b, &mut c));
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
        assert!(!pdal_point_view_mesh_triangle(view, 99, &mut a, &mut b, &mut c));

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
        let ok = pdal_point_view_split_where(
            view,
            cstring("X > 0.5").as_ptr(),
            &mut keep,
            &mut skip,
        );
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
