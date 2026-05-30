//! C ABI for the PDAL Rust port spike.
//!
//! Every function in this crate is `extern "C"` and intended to be called from
//! C or C++ through the header `include/pdal_capi.h`.

#![allow(clippy::missing_safety_doc)]

mod artifact_abi;
mod column_storage_abi;
mod config_abi;
mod deflate_abi;
mod driver_abi;
mod ept_addon_abi;
mod error;
mod file_spec_abi;
mod filter_abi;
mod filter_expression_abi;
mod filter_grid_abi;
mod filter_icp_abi;
mod filter_mesh_abi;
mod filter_runtime;
mod gridpnp_abi;
mod info_abi;
mod io_abi;
mod kernel_abi;
mod log_abi;
mod math_abi;
mod metadata_abi;
mod metrics_abi;
mod native_abi;
mod obb_abi;
mod ogr_spec_abi;
mod options;
mod pipeline_abi;
mod plugin_abi;
mod point_abi;
mod program_args_abi;
mod registry;
mod scaling_abi;
mod segmentation_abi;
mod slpk_abi;
mod srs;
mod stage_abi;
mod stats_abi;
mod thread_pool_abi;
mod tile_abi;
mod tool_abi;
mod trajectory_abi;
mod utils_abi;
mod uuid_abi;
mod vsi_abi;
mod writer_abi;
mod xml_schema_abi;
mod zstd_abi;

pub use artifact_abi::*;
pub use column_storage_abi::*;
pub use config_abi::*;
pub use deflate_abi::*;
pub use driver_abi::*;
pub use ept_addon_abi::*;
pub use error::*;
pub use file_spec_abi::*;
pub use filter_abi::*;
pub use filter_expression_abi::*;
pub use filter_grid_abi::*;
pub use filter_icp_abi::*;
pub use filter_mesh_abi::*;
pub use filter_runtime::*;
pub use info_abi::*;
pub use io_abi::*;
pub use kernel_abi::*;
pub use log_abi::*;
pub use math_abi::*;
pub use metadata_abi::*;
pub use metrics_abi::*;
pub use native_abi::*;
pub use obb_abi::*;
pub use ogr_spec_abi::*;
pub use options::*;
pub use pipeline_abi::*;
pub use plugin_abi::*;
pub use point_abi::*;
pub use program_args_abi::*;
pub use registry::*;
pub use scaling_abi::*;
pub use segmentation_abi::*;
pub use slpk_abi::*;
pub use srs::*;
pub use stage_abi::*;
pub use stats_abi::*;
pub use thread_pool_abi::*;
pub use tile_abi::*;
pub use tool_abi::*;
pub use utils_abi::*;
pub use uuid_abi::*;
pub use vsi_abi::*;
pub use writer_abi::*;
pub use xml_schema_abi::*;
pub use zstd_abi::*;

#[cfg(test)]
mod tests;
