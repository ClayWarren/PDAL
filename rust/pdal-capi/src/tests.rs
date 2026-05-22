use super::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

unsafe fn take_string(ptr: *mut c_char) -> String {
    assert!(!ptr.is_null());
    let value = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    pdal_string_free(ptr);
    value
}

fn data_path(path: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test/data")
        .join(path)
        .display()
        .to_string()
}

fn empty_pipeline_result() -> pdal_pipeline_result_t {
    pdal_pipeline_result_t {
        point_count: 0,
        view_count: 0,
        has_bounds_2d: false,
        bounds_2d: pdal_bounds2d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        },
        has_bounds_3d: false,
        bounds_3d: pdal_bounds3d_t {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        },
    }
}

mod core_abi;
mod deflate_abi;
mod filter_abi;
mod io_abi;
mod pipeline_metadata_abi;
mod point_abi;
mod utility_abi;
mod writer_abi;
