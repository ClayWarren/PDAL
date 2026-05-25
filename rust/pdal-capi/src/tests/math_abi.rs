use super::*;

/// Column-major 3x3 matrix matching the C++ `EigenTest.ComputeValues` input.
const A: [f64; 9] = [
    1.8339, -2.2588, 0.8622, 0.3188, -1.3077, -0.4336, 0.3426, 3.5784, 2.7694,
];

#[test]
fn math_abi_grad_x_matches_reference() {
    // Column-major expected X gradient from EigenTest.ComputeValues.
    let expected: [f64; 9] = [
        -1.5151, 0.9511, -1.2958, -0.7457, 2.9186, 0.9536, 0.0238, 4.8861, 3.2030,
    ];
    let mut out = [0.0f64; 9];
    unsafe {
        pdal_math_grad_x(A.as_ptr(), 3, 3, out.as_mut_ptr());
    }
    for (got, want) in out.iter().zip(expected.iter()) {
        assert!((got - want).abs() < 1e-4, "grad_x: {got} vs {want}");
    }
}

#[test]
fn math_abi_grad_y_matches_reference() {
    // Column-major expected Y gradient from EigenTest.ComputeValues.
    let expected: [f64; 9] = [
        -4.0927, -0.4859, 3.1210, -1.6265, -0.3762, 0.8741, 3.2358, 1.2134, -0.8090,
    ];
    let mut out = [0.0f64; 9];
    unsafe {
        pdal_math_grad_y(A.as_ptr(), 3, 3, out.as_mut_ptr());
    }
    for (got, want) in out.iter().zip(expected.iter()) {
        assert!((got - want).abs() < 1e-4, "grad_y: {got} vs {want}");
    }
}

#[test]
fn math_abi_dilate_then_erode_diamond() {
    // A single set cell in a 3x3 column-major raster.
    let mut raster = [0.0f64, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
    unsafe {
        pdal_math_dilate_diamond(raster.as_mut_ptr(), 3, 3, 1);
    }
    // Dilation fills the center cell and its four diamond neighbors.
    assert_eq!(raster, [0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0]);

    unsafe {
        pdal_math_erode_diamond(raster.as_mut_ptr(), 3, 3, 1);
    }
    // Erosion of the plus shape leaves only the center cell set.
    assert_eq!(raster, [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn math_abi_compute_centroid() {
    let xyz = [0.0, 0.0, 0.0, 2.0, 4.0, 6.0, 4.0, 8.0, 12.0];
    let mut out = [0.0f64; 3];
    unsafe {
        pdal_math_compute_centroid(xyz.as_ptr(), 3, out.as_mut_ptr());
    }
    assert_eq!(out, [2.0, 4.0, 6.0]);

    // A zero count yields the origin.
    out = [9.0, 9.0, 9.0];
    unsafe {
        pdal_math_compute_centroid(xyz.as_ptr(), 0, out.as_mut_ptr());
    }
    assert_eq!(out, [0.0, 0.0, 0.0]);
}

#[test]
fn math_abi_point_view_to_xyz_copies_rows() {
    unsafe {
        let layout = pdal_point_layout_create();
        let x = CString::new("X").unwrap();
        let y = CString::new("Y").unwrap();
        let z = CString::new("Z").unwrap();
        pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
        pdal_point_layout_register_dim(layout, y.as_ptr(), 9);
        pdal_point_layout_register_dim(layout, z.as_ptr(), 9);
        let view = pdal_point_view_create(layout);

        let point = pdal_point_view_add_point(view);
        pdal_point_view_set_f64(view, point, x.as_ptr(), 1.0);
        pdal_point_view_set_f64(view, point, y.as_ptr(), 2.0);
        pdal_point_view_set_f64(view, point, z.as_ptr(), 3.0);

        let mut out = [0.0; 3];
        assert_eq!(pdal_math_point_view_to_xyz(view, out.as_mut_ptr(), 0), 3);
        assert_eq!(out, [0.0; 3]);
        assert_eq!(pdal_math_point_view_to_xyz(view, out.as_mut_ptr(), 3), 3);
        assert_eq!(out, [1.0, 2.0, 3.0]);
        assert_eq!(
            pdal_math_point_view_to_xyz(std::ptr::null(), out.as_mut_ptr(), 3),
            0
        );

        pdal_point_view_destroy(view);
    }
}

#[test]
fn math_abi_tolerates_null_and_empty() {
    let mut out = [0.0f64; 4];
    unsafe {
        // Null pointers are ignored rather than dereferenced.
        pdal_math_grad_x(std::ptr::null(), 2, 2, out.as_mut_ptr());
        pdal_math_dilate_diamond(std::ptr::null_mut(), 2, 2, 1);
        pdal_math_compute_centroid(std::ptr::null(), 3, out.as_mut_ptr());
        // A zero-size raster is a no-op.
        pdal_math_grad_x(A.as_ptr(), 0, 0, out.as_mut_ptr());
    }
    assert_eq!(out, [0.0; 4]);
}
