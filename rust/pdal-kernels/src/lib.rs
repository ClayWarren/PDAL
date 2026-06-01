//! CLI kernels for the PDAL Rust port.
//!
//! Kernels are a late migration phase because they sit above the core,
//! pipeline, filters, and I/O layers. This crate owns the Rust-native command
//! contract used by the C ABI kernel runner.

mod density;
mod fauxplugin;
mod merge;
mod random;
mod registry;
mod sort;
mod split;
mod stage_options;
mod text;
mod tile;

pub use density::build_density_pipeline;
pub use fauxplugin::FauxPluginKernel;
pub use merge::build_merge_pipeline;
pub use random::build_random_pipeline;
pub use registry::{Kernel, KernelArgs, KernelError, KernelRegistry, KernelSpec, KERNEL_LIST_JSON};
pub use sort::build_sort_pipeline;
pub use split::{build_split_plan, numbered_split_output, SplitKernelPlan, SplitPlan};
pub use text::word_wrap;
pub use tile::{build_tile_plan, TileKernelPlan, TilePlan};

pub enum KernelPipelinePlan {
    Pipeline(serde_json::Value),
    Return(i32),
}
