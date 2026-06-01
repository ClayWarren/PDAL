use super::*;

#[test]
fn las_summary_abi_tracks_bounds_total_and_return_counts() {
    unsafe {
        let summary = pdal_las_summary_create();
        assert!(!summary.is_null());

        pdal_las_summary_add_point(summary, 10.0, 20.0, 30.0, 1);
        pdal_las_summary_add_point(summary, -5.0, 40.0, 15.0, 3);
        pdal_las_summary_add_point(summary, 100.0, -2.0, 0.5, 16);

        assert_eq!(pdal_las_summary_total_num_points(summary), 3);
        assert_eq!(pdal_las_summary_return_count(summary, 0), 1);
        assert_eq!(pdal_las_summary_return_count(summary, 2), 1);
        assert_eq!(pdal_las_summary_return_count(summary, 15), 0);

        let mut bounds = pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        };
        pdal_las_summary_bounds(summary, &mut bounds);
        assert_eq!(bounds.minx, -5.0);
        assert_eq!(bounds.maxx, 100.0);
        assert_eq!(bounds.miny, -2.0);
        assert_eq!(bounds.maxy, 40.0);
        assert_eq!(bounds.minz, 0.5);
        assert_eq!(bounds.maxz, 30.0);

        pdal_las_summary_clear(summary);
        assert_eq!(pdal_las_summary_total_num_points(summary), 0);
        assert_eq!(pdal_las_summary_return_count(summary, 0), 0);

        pdal_las_summary_destroy(summary);
        pdal_las_summary_destroy(std::ptr::null_mut());
        pdal_las_summary_clear(std::ptr::null_mut());
        pdal_las_summary_add_point(std::ptr::null_mut(), 1.0, 2.0, 3.0, 1);
        assert_eq!(pdal_las_summary_total_num_points(std::ptr::null()), 0);
        assert_eq!(pdal_las_summary_return_count(std::ptr::null(), 0), 0);
    }
}

#[test]
fn las_header_abi_matches_supported_formats_and_legacy_counts() {
    assert_eq!(pdal_las_base_count(0), 20);
    assert_eq!(pdal_las_base_count(1), 28);
    assert_eq!(pdal_las_base_count(2), 26);
    assert_eq!(pdal_las_base_count(3), 34);
    assert_eq!(pdal_las_base_count(6), 30);
    assert_eq!(pdal_las_base_count(7), 36);
    assert_eq!(pdal_las_base_count(8), 38);
    assert_eq!(pdal_las_base_count(0x86), 30);
    assert_eq!(pdal_las_base_count(5), 0);

    for format in [0, 1, 2, 3, 6, 7, 8] {
        assert!(pdal_las_point_format_supported(format));
    }
    assert!(!pdal_las_point_format_supported(4));
    assert!(!pdal_las_point_format_supported(9));

    assert_eq!(pdal_las_legacy_point_count(123, 2, 3), 123);
    assert_eq!(pdal_las_legacy_point_count(123, 4, 6), 0);
    assert_eq!(
        pdal_las_legacy_point_count(u64::from(u32::MAX) + 1, 2, 3),
        0
    );
    assert_eq!(pdal_las_legacy_points_by_return(42, 2, 2, 3), 42);
    assert_eq!(pdal_las_legacy_points_by_return(42, 5, 2, 3), 0);
    assert_eq!(pdal_las_legacy_points_by_return(42, 0, 4, 6), 0);
}

#[test]
fn las_vlr_header_abi_roundtrips_vlr_and_evlr_headers() {
    unsafe {
        let mut header = pdal_las_vlr_header_t {
            record_sig: 0,
            user_id: [0; 17],
            record_id: 42,
            data_size: 123,
            description: [0; 33],
        };
        write_c_chars(&mut header.user_id, "PDAL");
        write_c_chars(&mut header.description, "metadata");

        let mut bytes = vec![0_u8; 54];
        assert!(pdal_las_vlr_header_write(
            &header,
            false,
            bytes.as_mut_ptr(),
            bytes.len() as u64
        ));
        let mut parsed = pdal_las_vlr_header_t {
            record_sig: 0,
            user_id: [0; 17],
            record_id: 0,
            data_size: 0,
            description: [0; 33],
        };
        assert!(pdal_las_vlr_header_parse(
            bytes.as_ptr(),
            bytes.len() as u64,
            false,
            &mut parsed
        ));
        assert_eq!(parsed.record_id, 42);
        assert_eq!(parsed.data_size, 123);
        assert_eq!(read_c_chars(&parsed.user_id), "PDAL");
        assert_eq!(read_c_chars(&parsed.description), "metadata");

        header.data_size = u64::from(u16::MAX) + 1;
        assert!(!pdal_las_vlr_header_write(
            &header,
            false,
            bytes.as_mut_ptr(),
            bytes.len() as u64
        ));

        let mut evlr_bytes = vec![0_u8; 60];
        assert!(pdal_las_vlr_header_write(
            &header,
            true,
            evlr_bytes.as_mut_ptr(),
            evlr_bytes.len() as u64
        ));
        assert!(pdal_las_vlr_header_parse(
            evlr_bytes.as_ptr(),
            evlr_bytes.len() as u64,
            true,
            &mut parsed
        ));
        assert_eq!(parsed.data_size, u64::from(u16::MAX) + 1);
        assert!(!pdal_las_vlr_header_parse(
            evlr_bytes.as_ptr(),
            10,
            true,
            &mut parsed
        ));
    }
}

#[test]
fn las_vlr_text_abi_truncates_at_first_nul() {
    unsafe {
        let bytes = b"PROJCS[\"example\"]\0ignored";
        let text = pdal_las_vlr_text(bytes.as_ptr(), bytes.len() as u64);
        assert!(!text.is_null());
        assert_eq!(
            std::ffi::CStr::from_ptr(text).to_str().unwrap(),
            "PROJCS[\"example\"]"
        );
        pdal_string_free(text);

        let empty = pdal_las_vlr_text(std::ptr::null(), 0);
        assert!(!empty.is_null());
        assert_eq!(std::ffi::CStr::from_ptr(empty).to_str().unwrap(), "");
        pdal_string_free(empty);
    }
}

fn write_c_chars<const N: usize>(dst: &mut [i8; N], value: &str) {
    for (out, byte) in dst.iter_mut().zip(value.bytes()) {
        *out = byte as i8;
    }
}

fn read_c_chars<const N: usize>(src: &[i8; N]) -> String {
    let bytes: Vec<u8> = src
        .iter()
        .take_while(|&&ch| ch != 0)
        .map(|&ch| ch as u8)
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn las_tile_abi_owns_buffer_and_advances_cursor() {
    unsafe {
        let tile = pdal_las_tile_create(7, 6);
        assert!(!tile.is_null());
        assert_eq!(pdal_las_tile_chunk(tile), 7);
        assert_eq!(pdal_las_tile_size(tile), 6);

        let data = pdal_las_tile_data(tile);
        assert!(!data.is_null());
        *data.add(0) = 11;
        *data.add(5) = 99;
        assert_eq!(*pdal_las_tile_data_const(tile).add(0), 11);
        assert_eq!(*pdal_las_tile_pos(tile), 11);

        assert!(pdal_las_tile_advance(tile, 4));
        assert_eq!(*pdal_las_tile_pos(tile), 0);
        assert!(!pdal_las_tile_advance(tile, 2));
        assert!(pdal_las_tile_pos(tile).is_null());
        assert!(!pdal_las_tile_advance(tile, -1));

        pdal_las_tile_destroy(tile);
        pdal_las_tile_destroy(std::ptr::null_mut());
        assert_eq!(pdal_las_tile_size(std::ptr::null()), 0);
        assert!(pdal_las_tile_data(std::ptr::null_mut()).is_null());
    }
}
