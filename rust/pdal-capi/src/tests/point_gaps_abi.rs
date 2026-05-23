use super::*;
use pdal_core::point::PointView;
use std::os::raw::c_char;

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn layout_with_all_dim_types() -> *mut pdal_core::point::PointLayout {
    let layout = pdal_point_layout_create();
    let names = [
        "U8d", "U16d", "U32d", "U64d", "I8d", "I16d", "I32d", "I64d", "F32d", "F64d",
    ];
    for (i, name) in names.iter().enumerate() {
        let n = cstring(name);
        pdal_point_layout_register_dim(layout, n.as_ptr(), i as i32);
    }
    let outer = cstring("Strange");
    pdal_point_layout_register_dim(layout, outer.as_ptr(), 999);
    layout
}

#[test]
fn dim_type_round_trip_covers_every_id_variant() {
    unsafe {
        let layout = layout_with_all_dim_types();
        let view = pdal_point_view_create(layout);
        assert!(!view.is_null());
        assert_eq!(pdal_point_view_dim_count(view), 11);
        for i in 0..11u64 {
            let name_raw = pdal_point_view_dim_name(view, i);
            assert!(!name_raw.is_null());
            let _ = take_string(name_raw);
            let ty = pdal_point_view_dim_type(view, i);
            assert!(ty >= 0);
        }
        assert!(pdal_point_view_dim_name(view, 999).is_null());
        assert_eq!(pdal_point_view_dim_type(view, 999), -1);
        pdal_point_view_destroy(view);
    }
}

#[test]
fn dim_type_from_name_and_base_size_handle_inputs() {
    unsafe {
        assert_eq!(pdal_dimension_type_from_name(std::ptr::null()), 0);
        let valid = cstring("float");
        assert!(pdal_dimension_type_from_name(valid.as_ptr()) >= 0);

        assert_eq!(
            pdal_dimension_type_from_base_and_size(std::ptr::null(), 8),
            0
        );
        let base = cstring("signed");
        assert!(pdal_dimension_type_from_base_and_size(base.as_ptr(), 4) >= 0);
        assert!(pdal_dimension_type_from_base_and_size(base.as_ptr(), 1) >= 0);
        assert!(pdal_dimension_type_from_base_and_size(base.as_ptr(), 2) >= 0);
        assert!(pdal_dimension_type_from_base_and_size(base.as_ptr(), 8) >= 0);
        let bad = cstring("nonsense");
        let _ = pdal_dimension_type_from_base_and_size(bad.as_ptr(), 999);
    }
}

#[test]
fn dim_fix_name_handles_null_and_input_strings() {
    unsafe {
        let raw = pdal_dimension_fix_name(std::ptr::null());
        let _ = take_string(raw);

        let messy = cstring("X-Y Z");
        let _ = take_string(pdal_dimension_fix_name(messy.as_ptr()));
    }
}

#[test]
fn dim_interpretation_and_resolve_type_cover_neg_inputs() {
    unsafe {
        let _ = take_string(pdal_dimension_interpretation_name(-1));
        let _ = take_string(pdal_dimension_interpretation_name(0x408));
        assert_eq!(pdal_dimension_resolve_type(-1, 0x408), 0);
        assert_eq!(pdal_dimension_resolve_type(0x408, -1), 0);
        assert!(pdal_dimension_resolve_type(0x408, 0x408) > 0);
    }
}

#[test]
fn point_view_set_get_handle_nulls_and_invalid_inputs() {
    unsafe {
        let view = std::ptr::null_mut();
        let name = cstring("X");
        pdal_point_view_set_f64(view, 0, name.as_ptr(), 1.0);
        assert_eq!(pdal_point_view_get_f64(view, 0, name.as_ptr()), 0.0);

        let layout = pdal_point_layout_create();
        let n = cstring("X");
        pdal_point_layout_register_dim(layout, n.as_ptr(), 9);
        let v = pdal_point_view_create(layout);
        assert!(!v.is_null());

        pdal_point_view_set_f64(v, 0, std::ptr::null(), 7.0);
        assert_eq!(pdal_point_view_get_f64(v, 0, std::ptr::null()), 0.0);

        pdal_point_view_destroy(v);
        assert!(pdal_point_view_create(std::ptr::null_mut()).is_null());
    }
}

#[test]
fn point_view_source_index_with_null_returns_idx() {
    unsafe {
        assert_eq!(pdal_point_view_source_index(std::ptr::null_mut(), 42), 42);
    }
}

#[test]
fn point_view_length_handles_null() {
    unsafe {
        assert_eq!(pdal_point_view_length(std::ptr::null_mut()), 0);
    }
}

#[test]
fn calculate_bounds_2d_3d_handle_empty_and_no_dims() {
    unsafe {
        let layout = pdal_point_layout_create();
        let view = pdal_point_view_create(layout);
        let mut bounds2 = pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        };
        assert!(!pdal_point_view_calculate_bounds_2d(view, &mut bounds2));
        let mut bounds3 = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        };
        assert!(!pdal_point_view_calculate_bounds_3d(view, &mut bounds3));

        let p = pdal_point_view_add_point(view);
        assert_eq!(p, 0);

        assert!(!pdal_point_view_calculate_bounds_2d(
            view,
            std::ptr::null_mut()
        ));
        assert!(!pdal_point_view_calculate_bounds_3d(
            view,
            std::ptr::null_mut()
        ));
        pdal_point_view_destroy(view);

        assert!(!pdal_point_view_calculate_bounds_2d(
            std::ptr::null(),
            &mut bounds2
        ));
        assert!(!pdal_point_view_calculate_bounds_3d(
            std::ptr::null(),
            &mut bounds3
        ));
    }
}

#[test]
fn mesh_triangle_count_handles_null_and_no_mesh() {
    unsafe {
        assert_eq!(pdal_point_view_mesh_triangle_count(std::ptr::null()), 0);
        assert_eq!(
            pdal_point_view_named_mesh_triangle_count(std::ptr::null(), std::ptr::null()),
            0
        );

        let layout = pdal_point_layout_create();
        let view = pdal_point_view_create(layout);
        assert_eq!(pdal_point_view_mesh_triangle_count(view), 0);
        let name = cstring("missing");
        assert_eq!(
            pdal_point_view_named_mesh_triangle_count(view, name.as_ptr()),
            0
        );
        assert_eq!(
            pdal_point_view_named_mesh_triangle_count(view, std::ptr::null()),
            0
        );

        let mut a = 0u64;
        let mut b = 0u64;
        let mut c = 0u64;
        assert!(!pdal_point_view_mesh_triangle(
            view, 0, &mut a, &mut b, &mut c
        ));
        assert!(!pdal_point_view_named_mesh_triangle(
            view,
            std::ptr::null(),
            0,
            &mut a,
            &mut b,
            &mut c,
        ));

        pdal_point_view_destroy(view);
    }
}

#[test]
fn point_view_add_named_mesh_triangle_round_trips() {
    unsafe {
        let layout = pdal_point_layout_create();
        let x = cstring("X");
        let y = cstring("Y");
        let z = cstring("Z");
        pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
        pdal_point_layout_register_dim(layout, y.as_ptr(), 9);
        pdal_point_layout_register_dim(layout, z.as_ptr(), 9);
        let view = pdal_point_view_create(layout);
        for _ in 0..3 {
            pdal_point_view_add_point(view);
        }
        let mesh = cstring("mesh1");
        assert!(pdal_point_view_add_named_mesh_triangle(
            view,
            mesh.as_ptr(),
            0,
            1,
            2,
        ));
        assert_eq!(
            pdal_point_view_named_mesh_triangle_count(view, mesh.as_ptr()),
            1
        );

        let mut a = 0u64;
        let mut b = 0u64;
        let mut c = 0u64;
        assert!(pdal_point_view_named_mesh_triangle(
            view,
            mesh.as_ptr(),
            0,
            &mut a,
            &mut b,
            &mut c,
        ));
        assert_eq!((a, b, c), (0, 1, 2));
        assert!(!pdal_point_view_named_mesh_triangle(
            view,
            mesh.as_ptr(),
            999,
            &mut a,
            &mut b,
            &mut c,
        ));
        pdal_point_view_destroy(view);
    }
}

#[test]
fn spatial_reference_set_and_get_round_trip() {
    unsafe {
        let layout = pdal_point_layout_create();
        let view = pdal_point_view_create(layout);
        pdal_point_view_set_spatial_reference(view, std::ptr::null());
        let srs_ptr = pdal_point_view_spatial_reference(view);
        if !srs_ptr.is_null() {
            pdal_spatial_reference_destroy(srs_ptr);
        }
        assert!(pdal_point_view_spatial_reference(std::ptr::null()).is_null());
        pdal_point_view_set_spatial_reference(std::ptr::null_mut(), std::ptr::null());
        pdal_point_view_destroy(view);
    }
}

#[test]
fn dim_count_and_layout_destroy_handle_null_paths() {
    unsafe {
        assert_eq!(pdal_point_view_dim_count(std::ptr::null()), 0);
        assert!(pdal_point_view_dim_name(std::ptr::null(), 0).is_null());
        assert_eq!(pdal_point_view_dim_type(std::ptr::null(), 0), -1);
        pdal_point_layout_destroy(std::ptr::null_mut());
        pdal_point_view_destroy(std::ptr::null_mut());

        let layout = pdal_point_layout_create();
        pdal_point_layout_register_dim(layout, std::ptr::null(), 9);
        let view = pdal_point_view_create(layout);
        pdal_point_view_destroy(view);
    }
}

#[allow(dead_code)]
fn unused_view_param(_: &PointView, _: *const c_char) {}
