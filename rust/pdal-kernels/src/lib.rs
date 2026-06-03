//! CLI kernels for the PDAL Rust port.
//!
//! Kernels are a late migration phase because they sit above the core,
//! pipeline, filters, and I/O layers. This crate owns the Rust-native command
//! contract used by the C ABI kernel runner.

mod density;
mod fauxplugin;
mod ground;
mod info;
mod info_report;
mod merge;
mod metrics;
mod pipeline;
mod random;
mod registry;
mod sort;
mod split;
mod stage_options;
mod text;
mod tile;
mod tindex;
mod translate;

pub use density::build_density_pipeline;
pub use fauxplugin::FauxPluginKernel;
pub use ground::build_ground_pipeline;
pub use info::{build_info_plan, InfoKernelPlan, InfoMode, InfoRunPlan, QueryRequest};
pub use info_report::{
    point_report, query_report, schema_body, schema_report, stac_report, stats_body, stats_report,
};
pub use merge::build_merge_pipeline;
pub use metrics::{
    build_chamfer_plan, build_delta_plan, build_eval_plan, build_hausdorff_plan, DeltaPlan,
    EvalPlan, MetricPairPlan, MetricPlan,
};
pub use pipeline::{
    apply_stage_options_to_pipeline_json, parse_pipeline_args, serialize_pipeline_json,
    validate_pipeline_json_shape, ParsedPipelineArgs, PipelineArgsResult,
};
pub use random::build_random_pipeline;
pub use registry::{Kernel, KernelArgs, KernelError, KernelRegistry, KernelSpec, KERNEL_LIST_JSON};
pub use sort::build_sort_pipeline;
pub use split::{build_split_plan, numbered_split_output, SplitKernelPlan, SplitPlan};
pub use stage_options::{apply_cli_stage_options, CliStageOption};
pub use text::word_wrap;
pub use tile::{build_tile_plan, TileKernelPlan, TilePlan};
pub use tindex::{
    build_tindex_merge_plan, parse_tindex_create_args, parse_tindex_merge_args, print_tindex_usage,
    tindex_next_value, BoundaryOptions, TindexCreateArgs, TindexMergeArgs, TindexMergeClip,
    TindexMergePlan, TindexParseResult, TindexResolvedClip, INVALID_TINDEX_FILTER_STAGE_MESSAGE,
};
pub use translate::{
    build_translate_plan, expand_translate_option_files, parse_option_file, translate_json_stages,
    TranslateKernelPlan, TranslatePlan,
};

pub enum KernelPipelinePlan {
    Pipeline(serde_json::Value),
    Return(i32),
}
