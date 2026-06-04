use super::argv_to_vec;
use crate::registry::pipeline_from_json;
use pdal_core::point::DimId;
use pdal_kernels::{build_translate_plan, serialize_pipeline_json, TranslateKernelPlan};
use std::os::raw::c_char;

pub(super) unsafe fn run_translate_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    let plan = match build_translate_plan(&args) {
        TranslateKernelPlan::Run(plan) => plan,
        TranslateKernelPlan::Return(code) => return code,
    };
    execute_translate_pipeline(
        plan.stages,
        plan.allowed_dims,
        plan.metadata_file,
        plan.serialization_file,
        plan.stream_allowed,
        plan.stream_required,
    )
}

fn execute_translate_pipeline(
    stages: Vec<serde_json::Value>,
    allowed_dims: Vec<String>,
    metadata_file: Option<String>,
    serialization_file: Option<String>,
    stream_allowed: bool,
    stream_required: bool,
) -> i32 {
    let stage_types = translate_stage_types(&stages);
    let pipeline_json = serde_json::Value::Array(stages);
    if let Some(path) = serialization_file {
        match serialize_pipeline_json(&pipeline_json.to_string()) {
            Ok(serialized) => {
                if let Err(err) = std::fs::write(&path, serialized) {
                    eprintln!(
                        "PDAL: kernels.translate: Unable to write pipeline serialization '{path}': {err}"
                    );
                    return 1;
                }
            }
            Err(err) => {
                eprintln!("PDAL: kernels.translate: {err}");
                return 1;
            }
        }
        return 0;
    }

    let mut pipeline = match pipeline_from_json(&pipeline_json.to_string()) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.translate: {err}");
            return 1;
        }
    };
    pipeline.set_allowed_dims(
        allowed_dims
            .iter()
            .map(|name| DimId::from_name(name))
            .collect(),
    );

    if stream_allowed && metadata_file.is_none() {
        match pipeline.execute_streaming() {
            Ok(Some(_)) => return 0,
            Ok(None) if stream_required => {
                eprintln!("PDAL: kernels.translate: Pipeline is not streamable.");
                return 1;
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("PDAL: kernels.translate: {err}");
                return 1;
            }
        }
    }

    match pipeline.execute_with_result(Vec::new()) {
        Ok(result) => {
            if let Some(path) = metadata_file {
                let handle = crate::pipeline_abi::PipelineHandle { pipeline };
                let mut summary = serde_json::from_str::<serde_json::Value>(
                    &crate::pipeline_abi::pipeline_result_to_json_for_kernel(result, &handle),
                )
                .unwrap_or_else(|_| serde_json::json!({}));
                summary["stages"] = serde_json::Value::Array(
                    stage_types
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                );
                let summary =
                    serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string());
                if let Err(err) = std::fs::write(&path, summary) {
                    eprintln!("PDAL: kernels.translate: Unable to write metadata '{path}': {err}");
                    return 1;
                }
            }
            0
        }
        Err(err) => {
            eprintln!("PDAL: kernels.translate: {err}");
            1
        }
    }
}

fn translate_stage_types(stages: &[serde_json::Value]) -> Vec<String> {
    stages
        .iter()
        .filter_map(|stage| {
            stage
                .get("type")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect()
}
