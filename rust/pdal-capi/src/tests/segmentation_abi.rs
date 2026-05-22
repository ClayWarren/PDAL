use super::*;
use std::ptr;

fn extract(xyz: &[f64], count: usize, min: u64, max: u64, tol: f64, is_3d: bool) -> Vec<u64> {
    let mut sizes: *mut u64 = ptr::null_mut();
    let mut cluster_count: u64 = 0;
    let mut ids: *mut u64 = ptr::null_mut();
    let mut point_count: u64 = 0;
    unsafe {
        assert!(pdal_segmentation_extract_clusters(
            xyz.as_ptr(),
            count,
            min,
            max,
            tol,
            is_3d,
            &mut sizes,
            &mut cluster_count,
            &mut ids,
            &mut point_count,
        ));
        let result = if sizes.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(sizes, cluster_count as usize).to_vec()
        };
        pdal_u64_array_free(sizes, cluster_count);
        pdal_u64_array_free(ids, point_count);
        result
    }
}

#[test]
fn segmentation_abi_extract_clusters() {
    // Two near points and one distant point.
    let xyz = [0.0, 0.0, 0.0, 10.0, 10.0, 10.0, 0.5, 0.5, 0.5];
    assert_eq!(extract(&xyz, 3, 1, 10, 1.0, true), vec![2, 1]);
    // min_points drops the lone point; max_points drops the pair.
    assert_eq!(extract(&xyz, 3, 2, 10, 1.0, true), vec![2]);
    assert_eq!(extract(&xyz, 3, 1, 1, 1.0, true), vec![1]);
}

#[test]
fn segmentation_abi_segment_returns() {
    // (rn, nr): only, first, last, intermediate.
    let rn = [1u8, 1, 3, 2];
    let nr = [1u8, 3, 3, 3];
    let mut out = [0u8; 4];
    unsafe {
        pdal_segmentation_segment_returns(
            rn.as_ptr(),
            nr.as_ptr(),
            4,
            false,
            false,
            true,
            true,
            out.as_mut_ptr(),
        );
    }
    // "last" + "only" classes go to the first output.
    assert_eq!(out, [1, 0, 1, 0]);
}

#[test]
fn segmentation_abi_rejects_null() {
    let mut sizes: *mut u64 = ptr::null_mut();
    let mut cluster_count: u64 = 0;
    let mut ids: *mut u64 = ptr::null_mut();
    let mut point_count: u64 = 0;
    unsafe {
        assert!(!pdal_segmentation_extract_clusters(
            ptr::null(),
            0,
            1,
            10,
            1.0,
            true,
            &mut sizes,
            &mut cluster_count,
            &mut ids,
            &mut point_count,
        ));
    }
}
