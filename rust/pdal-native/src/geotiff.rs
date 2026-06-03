//! GeoTIFF GeoKey helpers through libgeotiff.

use std::ffi::{c_char, c_uchar, c_void, CStr};

unsafe extern "C" {
    fn pdal_native_geotiff_wkt(
        directory: *const c_uchar,
        directory_len: usize,
        doubles: *const c_uchar,
        doubles_len: usize,
        ascii: *const c_uchar,
        ascii_len: usize,
    ) -> *mut c_char;
    fn pdal_native_geotiff_string_free(value: *mut c_char);
}

/// Convert LAS GeoTIFF VLR payloads into WKT using libgeotiff.
pub fn wkt_from_tags(directory: &[u8], doubles: &[u8], ascii: &[u8]) -> Option<String> {
    if directory.is_empty() {
        return None;
    }
    unsafe {
        let raw = pdal_native_geotiff_wkt(
            directory.as_ptr(),
            directory.len(),
            nullable_ptr(doubles),
            doubles.len(),
            nullable_ptr(ascii),
            ascii.len(),
        );
        if raw.is_null() {
            return None;
        }
        let out = CStr::from_ptr(raw).to_string_lossy().into_owned();
        pdal_native_geotiff_string_free(raw);
        (!out.trim().is_empty()).then_some(out)
    }
}

fn nullable_ptr(bytes: &[u8]) -> *const c_uchar {
    if bytes.is_empty() {
        std::ptr::null()
    } else {
        bytes.as_ptr().cast::<c_void>().cast::<c_uchar>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn las_projection_vlrs() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test/data/las/autzen_trim.las"
        );
        let bytes = std::fs::read(path).expect("read autzen fixture");
        let header_size = u16::from_le_bytes([bytes[94], bytes[95]]) as usize;
        let vlr_count = u32::from_le_bytes([bytes[100], bytes[101], bytes[102], bytes[103]]);
        let mut offset = header_size;
        let mut directory = Vec::new();
        let mut doubles = Vec::new();
        let mut ascii = Vec::new();

        for _ in 0..vlr_count {
            let header = &bytes[offset..offset + 54];
            offset += 54;
            let user_id = String::from_utf8_lossy(&header[2..18])
                .trim_end_matches('\0')
                .to_string();
            let record_id = u16::from_le_bytes([header[18], header[19]]);
            let len = u16::from_le_bytes([header[20], header[21]]) as usize;
            let data = bytes[offset..offset + len].to_vec();
            offset += len;
            if user_id == "LASF_Projection" {
                match record_id {
                    34735 => directory = data,
                    34736 => doubles = data,
                    34737 => ascii = data,
                    _ => {}
                }
            }
        }

        (directory, doubles, ascii)
    }

    #[test]
    fn converts_user_defined_las_geotiff_keys_to_wkt() {
        let (directory, doubles, ascii) = las_projection_vlrs();
        let wkt = wkt_from_tags(&directory, &doubles, &ascii).expect("geotiff wkt");

        assert!(wkt.contains("Lambert_Conformal_Conic"));
        assert!(wkt.contains("NAD_1983_HARN"));
        assert!(!wkt.contains("EPSG:32767"));
    }
}
