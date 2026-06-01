//! CLI kernels for the PDAL Rust port.
//!
//! Kernels are a late migration phase because they sit above the core,
//! pipeline, filters, and I/O layers. This crate owns the Rust-native command
//! contract used by the C ABI kernel runner.

mod density;
mod fauxplugin;
mod ground;
mod merge;
mod metrics;
mod random;
mod registry;
mod sort;
mod split;
mod stage_options;
mod text;
mod tile;
mod translate;

pub use density::build_density_pipeline;
pub use fauxplugin::FauxPluginKernel;
pub use ground::build_ground_pipeline;
pub use merge::build_merge_pipeline;
pub use metrics::{
    build_chamfer_plan, build_delta_plan, build_eval_plan, build_hausdorff_plan, DeltaPlan,
    EvalPlan, MetricPairPlan, MetricPlan,
};
pub use random::build_random_pipeline;
pub use registry::{Kernel, KernelArgs, KernelError, KernelRegistry, KernelSpec, KERNEL_LIST_JSON};
pub use sort::build_sort_pipeline;
pub use split::{build_split_plan, numbered_split_output, SplitKernelPlan, SplitPlan};
pub use stage_options::CliStageOption;
pub use text::word_wrap;
pub use tile::{build_tile_plan, TileKernelPlan, TilePlan};
pub use translate::{
    build_translate_plan, expand_translate_option_files, parse_option_file, translate_json_stages,
    TranslateKernelPlan, TranslatePlan,
};

pub enum KernelPipelinePlan {
    Pipeline(serde_json::Value),
    Return(i32),
}
