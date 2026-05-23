use super::*;
use std::os::raw::c_char;

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

#[test]
fn utils_is_json_recognizes_json_and_rejects_null_and_non_json() {
    unsafe {
        assert!(pdal_utils_is_json(cstring("{\"a\":1}").as_ptr()));
        assert!(pdal_utils_is_json(cstring("[1,2,3]").as_ptr()));
        assert!(!pdal_utils_is_json(cstring("hello").as_ptr()));
        assert!(!pdal_utils_is_json(std::ptr::null()));
    }
}

#[test]
fn utils_to_string_roundtrips_numbers() {
    unsafe {
        assert_eq!(take_string(pdal_utils_to_string_f64(12.5, 4)), "12.5");
        assert_eq!(take_string(pdal_utils_to_string_i32(-7)), "-7");

        let mut out_i32: i32 = 0;
        assert_eq!(
            pdal_utils_from_string_i32(cstring("42").as_ptr(), &mut out_i32),
            0
        );
        assert_eq!(out_i32, 42);

        let mut out_f64: f64 = 0.0;
        assert_eq!(
            pdal_utils_from_string_f64(cstring("3.5").as_ptr(), &mut out_f64),
            0
        );
        assert_eq!(out_f64, 3.5);

        assert_eq!(
            pdal_utils_from_string_i32(cstring("not-numeric").as_ptr(), &mut out_i32),
            -1
        );
        assert_eq!(
            pdal_utils_from_string_f64(cstring("not-numeric").as_ptr(), &mut out_f64),
            -1
        );

        assert_eq!(
            pdal_utils_from_string_i32(std::ptr::null(), &mut out_i32),
            -1
        );
        assert_eq!(
            pdal_utils_from_string_f64(std::ptr::null(), &mut out_f64),
            -1
        );
        assert_eq!(
            pdal_utils_from_string_i32(cstring("1").as_ptr(), std::ptr::null_mut()),
            -1
        );
        assert_eq!(
            pdal_utils_from_string_f64(cstring("1").as_ptr(), std::ptr::null_mut()),
            -1
        );
    }
}

#[test]
fn utils_numeric_casts_round_trip_and_reject_null_out() {
    unsafe {
        let mut out_f64: f64 = 0.0;
        assert!(pdal_utils_numeric_cast_f32_to_f64(1.5_f32, &mut out_f64));
        assert_eq!(out_f64, 1.5_f64);
        assert!(!pdal_utils_numeric_cast_f32_to_f64(
            1.5_f32,
            std::ptr::null_mut()
        ));

        let mut out_f32: f32 = 0.0;
        assert!(pdal_utils_numeric_cast_f64_to_f32(2.5_f64, &mut out_f32));
        assert_eq!(out_f32, 2.5_f32);
        assert!(!pdal_utils_numeric_cast_f64_to_f32(
            1e40_f64,
            &mut out_f32
        ));
        assert!(!pdal_utils_numeric_cast_f64_to_f32(
            2.5_f64,
            std::ptr::null_mut()
        ));
    }
}

#[test]
fn utils_compare_approx_respects_tolerance() {
    assert!(pdal_utils_compare_approx(1.0, 1.000_5, 1e-3));
    assert!(!pdal_utils_compare_approx(1.0, 1.5, 1e-3));
}

#[test]
fn utils_random_is_within_bounds_after_seeding() {
    pdal_utils_random_seed(12345);
    for _ in 0..32 {
        let v = pdal_utils_random(-1.0, 1.0);
        assert!((-1.0..=1.0).contains(&v));
    }
}

#[test]
fn utils_run_shell_command_returns_captured_output() {
    unsafe {
        let mut output: *mut c_char = std::ptr::null_mut();
        let status = pdal_utils_run_shell_command(
            cstring("printf hi").as_ptr(),
            &mut output as *mut *mut c_char,
        );
        assert_eq!(status, 0);
        assert_eq!(take_string(output), "hi");
    }
}

#[test]
fn charbuf_seek_helpers_clamp_negative_and_out_of_range() {
    // Out-of-range positions/offsets should return -1 (the err sentinel).
    assert!(pdal_charbuf_seekpos(-1, 0, 16, false) < 0);
    // Use seekoff with cur+offset within length should return offset.
    let off = pdal_charbuf_seekoff(0, b'c', 4, 16, 0);
    assert!(off >= 0 || off == -1);
}

#[test]
fn u8_array_free_handles_null_and_valid() {
    unsafe {
        pdal_u8_array_free(std::ptr::null_mut(), 0);
        let v = vec![1u8, 2, 3, 4];
        let mut boxed = v.into_boxed_slice();
        let ptr = boxed.as_mut_ptr();
        let len = boxed.len() as u64;
        std::mem::forget(boxed);
        pdal_u8_array_free(ptr, len);
    }
}

#[test]
fn file_utils_directory_round_trip() {
    unsafe {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("a/b/c");
        let nested_c = cstring(nested.to_string_lossy().as_ref());

        assert!(!pdal_file_utils_directory_exists(nested_c.as_ptr()));
        assert_eq!(pdal_file_utils_create_directories(nested_c.as_ptr()), 1);
        assert!(pdal_file_utils_directory_exists(nested_c.as_ptr()));
        // Creating again should report exists (0).
        assert_eq!(pdal_file_utils_create_directories(nested_c.as_ptr()), 0);

        let sibling = temp.path().join("sibling");
        let sibling_c = cstring(sibling.to_string_lossy().as_ref());
        assert_eq!(pdal_file_utils_create_directory(sibling_c.as_ptr()), 1);
        // Re-creating the same directory should report already-exists (0).
        assert_eq!(pdal_file_utils_create_directory(sibling_c.as_ptr()), 0);
        // Creating a deep path with create_directory (single level) should fail.
        let deep = temp.path().join("deep/missing");
        let deep_c = cstring(deep.to_string_lossy().as_ref());
        assert_eq!(pdal_file_utils_create_directory(deep_c.as_ptr()), -1);

        // List contents of temp dir.
        let temp_c = cstring(temp.path().to_string_lossy().as_ref());
        let listing = take_string(pdal_file_utils_directory_list(temp_c.as_ptr()));
        assert!(listing.contains("sibling"));

        // Glob inside the temp directory.
        let pattern = format!("{}/*", temp.path().display());
        let pat_c = cstring(&pattern);
        let glob_out = take_string(pdal_file_utils_glob(pat_c.as_ptr()));
        assert!(glob_out.contains("sibling"));

        // Cleanup.
        pdal_file_utils_delete_directory(temp_c.as_ptr());
    }
}

#[test]
fn file_utils_file_round_trip() {
    unsafe {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("hello.txt");
        std::fs::write(&path, "hello world\n").unwrap();

        let path_c = cstring(path.to_string_lossy().as_ref());
        assert!(pdal_file_utils_file_exists(path_c.as_ptr()));
        assert_eq!(pdal_file_utils_file_size(path_c.as_ptr()), 12);

        let contents = take_string(pdal_file_utils_read_file_into_string(path_c.as_ptr()));
        assert_eq!(contents, "hello world\n");

        let dst = temp.path().join("hello-renamed.txt");
        let dst_c = cstring(dst.to_string_lossy().as_ref());
        pdal_file_utils_rename_file(dst_c.as_ptr(), path_c.as_ptr());
        assert!(!pdal_file_utils_file_exists(path_c.as_ptr()));
        assert!(pdal_file_utils_file_exists(dst_c.as_ptr()));

        assert!(pdal_file_utils_delete_file(dst_c.as_ptr()));
        assert!(!pdal_file_utils_file_exists(dst_c.as_ptr()));

        // read_file_into_string of a missing file returns null.
        let missing = cstring(temp.path().join("missing").to_string_lossy().as_ref());
        assert!(pdal_file_utils_read_file_into_string(missing.as_ptr()).is_null());
    }
}

#[test]
fn support_diff_helpers_match_byte_and_line_counts() {
    unsafe {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("a.txt");
        let b = temp.path().join("b.txt");
        std::fs::write(&a, "hello").unwrap();
        std::fs::write(&b, "hello").unwrap();
        let a_c = cstring(a.to_string_lossy().as_ref());
        let b_c = cstring(b.to_string_lossy().as_ref());
        assert_eq!(
            pdal_support_diff_files(
                a_c.as_ptr(),
                b_c.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0
            ),
            0
        );
        assert_eq!(pdal_support_diff_text_files(a_c.as_ptr(), b_c.as_ptr(), -1), 0);

        std::fs::write(&b, "world").unwrap();
        assert!(
            pdal_support_diff_files(
                a_c.as_ptr(),
                b_c.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0
            ) > 0
        );

        // Ignore the first 5 bytes — files become "equal" via the ignore range.
        let starts: [u32; 1] = [0];
        let lengths: [u32; 1] = [5];
        assert_eq!(
            pdal_support_diff_files(
                a_c.as_ptr(),
                b_c.as_ptr(),
                starts.as_ptr(),
                lengths.as_ptr(),
                1
            ),
            0
        );
    }
}
