use super::*;

#[test]
fn obb_abi_matches_cpp_fixture() {
    // Mirrors the C++ ObbTest.obb fixture.
    let base_center = [0.0, 0.0, 0.0];
    let base_half = [2.0, 1.0, 1.5];
    let base_quat = [0.0, 0.0, 0.0, 1.0];
    let clip_half = [2.12132034355, std::f64::consts::FRAC_1_SQRT_2, 1.0];
    let clip_quat = [0.0, 0.0, -0.3826834324, 0.9238795325];

    let hit = |center: [f64; 3]| unsafe {
        pdal_obb_intersect(
            base_center.as_ptr(),
            base_half.as_ptr(),
            base_quat.as_ptr(),
            center.as_ptr(),
            clip_half.as_ptr(),
            clip_quat.as_ptr(),
        )
    };

    assert!(hit([2.0, 1.0, 0.0]));
    assert!(hit([2.0, 1.0, -1.0]));
    assert!(hit([2.0, 1.0, -2.5]));
    assert!(!hit([2.0, 1.0, -2.51]));
    assert!(!hit([2.0, 3.0, 0.0]));
    assert!(hit([2.0, 2.0, 0.0]));
}

#[test]
fn obb_abi_rejects_null_pointers() {
    let center = [0.0, 0.0, 0.0];
    let half = [1.0, 1.0, 1.0];
    let quat = [0.0, 0.0, 0.0, 1.0];
    unsafe {
        assert!(!pdal_obb_intersect(
            std::ptr::null(),
            half.as_ptr(),
            quat.as_ptr(),
            center.as_ptr(),
            half.as_ptr(),
            quat.as_ptr(),
        ));
    }
}
