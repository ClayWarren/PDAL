//! C ABI for the PDAL Rust port spike.
//!
//! Every function in this crate is `extern "C"` and intended to be called from
//! C or C++ through the header `include/pdal_capi.h`.

mod error;
mod filter_abi;
mod io_abi;
mod metadata_abi;
mod options;
mod pipeline_abi;
mod point_abi;
mod registry;
mod srs;
mod stage_abi;
mod stats_abi;

pub use error::*;
pub use filter_abi::*;
pub use io_abi::*;
pub use metadata_abi::*;
pub use options::*;
pub use pipeline_abi::*;
pub use point_abi::*;
pub use registry::*;
pub use srs::*;
pub use stage_abi::*;
pub use stats_abi::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    unsafe fn take_string(ptr: *mut c_char) -> String {
        assert!(!ptr.is_null());
        let value = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        pdal_string_free(ptr);
        value
    }

    #[test]
    fn spatial_reference_roundtrips_through_c_abi() {
        unsafe {
            let text = CString::new("EPSG:4326").unwrap();
            let srs = pdal_spatial_reference_create_with_epoch(text.as_ptr(), 2020.0);
            assert!(!pdal_spatial_reference_empty(srs));
            assert_eq!(pdal_spatial_reference_epoch(srs), 2020.0);
            assert_eq!(take_string(pdal_spatial_reference_text(srs)), "EPSG:4326");

            pdal_spatial_reference_set_epoch(srs, 2021.5);
            assert_eq!(pdal_spatial_reference_epoch(srs), 2021.5);
            pdal_spatial_reference_destroy(srs);
        }
    }

    #[test]
    fn point_view_carries_spatial_reference() {
        unsafe {
            let layout = pdal_point_layout_create();
            let x = CString::new("X").unwrap();
            pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
            let view = pdal_point_view_create(layout);

            let text = CString::new("EPSG:4978").unwrap();
            let srs = pdal_spatial_reference_create(text.as_ptr());
            pdal_point_view_set_spatial_reference(view, srs);

            let copied = pdal_point_view_spatial_reference(view);
            assert_eq!(
                take_string(pdal_spatial_reference_text(copied)),
                "EPSG:4978"
            );

            pdal_spatial_reference_destroy(copied);
            pdal_spatial_reference_destroy(srs);
            pdal_point_view_destroy(view);
        }
    }

    #[test]
    fn point_view_exposes_layout_dimensions() {
        unsafe {
            let layout = pdal_point_layout_create();
            let x = CString::new("X").unwrap();
            let classification = CString::new("Classification").unwrap();
            pdal_point_layout_register_dim(layout, x.as_ptr(), 9);
            pdal_point_layout_register_dim(layout, classification.as_ptr(), 0);
            let view = pdal_point_view_create(layout);

            assert_eq!(pdal_point_view_dim_count(view), 2);
            assert_eq!(take_string(pdal_point_view_dim_name(view, 0)), "X");
            assert_eq!(pdal_point_view_dim_type(view, 0), 9);
            assert_eq!(
                take_string(pdal_point_view_dim_name(view, 1)),
                "Classification"
            );
            assert_eq!(pdal_point_view_dim_type(view, 1), 0);

            pdal_point_view_destroy(view);
        }
    }

    #[test]
    fn metadata_tree_roundtrips_through_c_abi() {
        unsafe {
            let root_name = CString::new("root").unwrap();
            let child_name = CString::new("child").unwrap();
            let child_value = CString::new("value").unwrap();

            let root = pdal_metadata_node_create(root_name.as_ptr());
            let child = pdal_metadata_node_create(child_name.as_ptr());
            pdal_metadata_node_set_string(child, child_value.as_ptr());
            pdal_metadata_node_add_child(root, child);

            assert_eq!(pdal_metadata_node_child_count(root), 1);
            let copied = pdal_metadata_node_child(root, 0);
            assert_eq!(take_string(pdal_metadata_node_name(copied)), "child");
            assert_eq!(take_string(pdal_metadata_node_value(copied)), "value");

            pdal_metadata_node_destroy(copied);
            pdal_metadata_node_destroy(root);
        }
    }

    #[test]
    fn metadata_numeric_values_roundtrip_through_c_abi() {
        unsafe {
            let node_name = CString::new("count").unwrap();
            let node = pdal_metadata_node_create(node_name.as_ptr());
            pdal_metadata_node_set_u64(node, 42);

            assert_eq!(pdal_metadata_node_value_kind(node), 2);
            assert_eq!(pdal_metadata_node_value_u64(node), 42);
            assert_eq!(take_string(pdal_metadata_node_value(node)), "42");

            pdal_metadata_node_destroy(node);
        }
    }

    #[test]
    fn spatial_reference_exports_metadata() {
        unsafe {
            let text = CString::new("EPSG:4326").unwrap();
            let srs = pdal_spatial_reference_create_with_epoch(text.as_ptr(), 2020.0);
            let metadata = pdal_spatial_reference_to_metadata(srs);

            assert_eq!(take_string(pdal_metadata_node_name(metadata)), "srs");
            assert_eq!(pdal_metadata_node_child_count(metadata), 2);

            let wkt = pdal_metadata_node_child(metadata, 0);
            assert_eq!(take_string(pdal_metadata_node_name(wkt)), "wkt");
            assert_eq!(take_string(pdal_metadata_node_value(wkt)), "EPSG:4326");

            pdal_metadata_node_destroy(wkt);
            pdal_metadata_node_destroy(metadata);
            pdal_spatial_reference_destroy(srs);
        }
    }
}
