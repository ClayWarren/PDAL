use super::argv_to_vec;
use crate::pipeline_abi::{pipeline_result_to_json_for_kernel, PipelineHandle};
use crate::registry::pipeline_from_json;
use pdal_core::metadata::{metadata_node_to_json_flat, MetadataNode, MetadataValue};
use pdal_core::point::PointView;
use pdal_kernels::{
    build_info_plan, point_report, query_report, schema_body, schema_report,
    serialize_pipeline_json, stac_report, stats_body, stats_report, InfoKernelPlan, InfoMode,
    InfoRunPlan,
};
use std::os::raw::c_char;

pub(in crate::kernel_abi) unsafe fn run_info_kernel(argc: i32, argv: *const *const c_char) -> i32 {
    let args = match argv_to_vec(argc, argv) {
        Ok(args) => args,
        Err(code) => return code,
    };

    let plan = match build_info_plan(&args) {
        InfoKernelPlan::Run(plan) => plan,
        InfoKernelPlan::Return(code) => return code,
    };

    let (filename, driver, mode, pc_type, serialization_file) = match plan {
        InfoRunPlan::PipelineJson {
            json,
            mode,
            pc_type,
            serialization_file,
        } => return run_info_pipeline_json(&json, mode, &pc_type, serialization_file),
        InfoRunPlan::File {
            filename,
            driver,
            mode,
            pc_type,
            serialization_file,
        } => (filename, driver, mode, pc_type, serialization_file),
    };

    if let Some(path) = serialization_file {
        let pipeline_json = serde_json::json!({
            "pipeline": [
                {
                    "type": driver,
                    "filename": filename,
                }
            ]
        });
        let text = match serialize_pipeline_json(&pipeline_json.to_string()) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                return 1;
            }
        };
        if let Err(err) = std::fs::write(&path, text) {
            eprintln!("PDAL: kernels.info: Unable to write pipeline serialization '{path}': {err}");
            return 1;
        }
    }

    let mut pipeline = match info_pipeline(&driver, &filename, mode.needs_boundary()) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.info: {err}");
            return 1;
        }
    };

    match mode {
        InfoMode::Summary => match pipeline.execute_with_result(Vec::new()) {
            Ok(result) => {
                let handle = PipelineHandle { pipeline };
                println!(
                    "{}",
                    info_summary_json(
                        pipeline_result_to_json_for_kernel(result, &handle),
                        Some(&filename),
                        Some(&driver),
                    )
                );
                0
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                1
            }
        },
        InfoMode::Stats { .. }
        | InfoMode::Schema
        | InfoMode::Metadata
        | InfoMode::All { .. }
        | InfoMode::Boundary
        | InfoMode::Stac
        | InfoMode::Points(_)
        | InfoMode::Query(_) => match pipeline.execute(Vec::new()) {
            Ok(views) => {
                let metadata = pipeline.metadata();
                println!(
                    "{}",
                    info_report(mode, &views, &metadata, &filename, &pc_type)
                );
                0
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                1
            }
        },
    }
}

fn run_info_pipeline_json(
    json: &str,
    mode: InfoMode,
    pc_type: &str,
    serialization_file: Option<String>,
) -> i32 {
    let json = if mode.needs_boundary() {
        match append_info_stage_to_pipeline_json(
            json,
            serde_json::json!({ "type": "filters.hexbin" }),
        ) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                return 1;
            }
        }
    } else {
        json.to_string()
    };

    if let Some(path) = serialization_file {
        match serialize_pipeline_json(&json) {
            Ok(serialized) => {
                if let Err(err) = std::fs::write(&path, serialized) {
                    eprintln!(
                        "PDAL: kernels.info: Unable to write pipeline serialization '{path}': {err}"
                    );
                    return 1;
                }
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                return 1;
            }
        }
    }

    let mut pipeline = match pipeline_from_json(&json) {
        Ok(pipeline) => pipeline,
        Err(err) => {
            eprintln!("PDAL: kernels.info: {err}");
            return 1;
        }
    };

    match mode {
        InfoMode::Summary => match pipeline.execute_with_result(Vec::new()) {
            Ok(result) => {
                let handle = PipelineHandle { pipeline };
                println!(
                    "{}",
                    info_summary_json(
                        pipeline_result_to_json_for_kernel(result, &handle),
                        Some("STDIN"),
                        None,
                    )
                );
                0
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                1
            }
        },
        _ => match pipeline.execute(Vec::new()) {
            Ok(views) => {
                let metadata = pipeline.metadata();
                println!("{}", info_report(mode, &views, &metadata, "STDIN", pc_type));
                0
            }
            Err(err) => {
                eprintln!("PDAL: kernels.info: {err}");
                1
            }
        },
    }
}

fn info_summary_json(summary: String, filename: Option<&str>, driver: Option<&str>) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&summary) else {
        return summary;
    };
    let Some(object) = value.as_object_mut() else {
        return summary;
    };
    if let Some(filename) = filename {
        object.insert("filename".to_string(), serde_json::json!(filename));
    }
    if let Some(driver) = driver {
        object.insert("driver".to_string(), serde_json::json!(driver));
    }
    serde_json::to_string(&value).unwrap_or(summary)
}

fn append_info_stage_to_pipeline_json(
    json: &str,
    stage: serde_json::Value,
) -> Result<String, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(json).map_err(|err| format!("Invalid pipeline JSON: {err}"))?;
    if let Some(stages) = value.as_array_mut() {
        stages.push(stage);
    } else if let Some(stages) = value
        .get_mut("pipeline")
        .and_then(serde_json::Value::as_array_mut)
    {
        stages.push(stage);
    } else {
        return Err("Pipeline JSON object must contain a 'pipeline' array.".to_string());
    }
    serde_json::to_string(&value).map_err(|err| err.to_string())
}

fn info_pipeline(
    driver: &str,
    filename: &str,
    include_boundary: bool,
) -> Result<pdal_core::pipeline::Pipeline, String> {
    let mut stages = vec![serde_json::json!({ "type": driver, "filename": filename })];
    if include_boundary {
        stages.push(serde_json::json!({ "type": "filters.hexbin" }));
    }
    pipeline_from_json(&serde_json::Value::Array(stages).to_string()).map_err(|err| err.to_string())
}

fn info_report(
    mode: InfoMode,
    views: &[PointView],
    metadata: &MetadataNode,
    filename: &str,
    pc_type: &str,
) -> String {
    match mode {
        InfoMode::Stats {
            dimensions,
            enumerate,
            breakout,
        } => stats_report(
            views,
            dimensions.as_deref(),
            enumerate.as_deref(),
            breakout.as_ref(),
        ),
        InfoMode::Schema => schema_report(views),
        InfoMode::Metadata => metadata_report(metadata),
        InfoMode::All {
            dimensions,
            enumerate,
            breakout,
        } => {
            let mut output = String::from("{\n");
            output.push_str("  \"schema\":\n");
            output.push_str(&schema_body(views, 2));
            output.push_str(",\n");
            output.push_str("  \"stats\":\n");
            output.push_str(&stats_body(
                views,
                2,
                dimensions.as_deref(),
                enumerate.as_deref(),
                breakout.as_ref(),
            ));
            output.push_str(",\n");
            output.push_str("  \"metadata\": ");
            let metadata_json = metadata_node_to_json_flat(metadata);
            output.push_str(
                &serde_json::to_string_pretty(&metadata_json).unwrap_or_else(|_| "{}".into()),
            );
            output.push_str(",\n");
            output.push_str("  \"boundary\": ");
            output.push_str(&boundary_value(metadata).to_string());
            output.push_str(",\n");
            output.push_str("  \"stac\": ");
            let stac_json = serde_json::from_str::<serde_json::Value>(&stac_report(
                views, metadata, filename, pc_type,
            ))
            .ok()
            .and_then(|value| value.get("stac").cloned())
            .unwrap_or_else(|| serde_json::json!({}));
            output.push_str(
                &serde_json::to_string_pretty(&stac_json).unwrap_or_else(|_| "{}".into()),
            );
            output.push_str("\n}\n");
            output
        }
        InfoMode::Stac => stac_report(views, metadata, filename, pc_type),
        InfoMode::Boundary => boundary_report(metadata),
        InfoMode::Points(point_ids) => point_report(views, &point_ids),
        InfoMode::Query(query) => query_report(views, query),
        InfoMode::Summary => String::new(),
    }
}

fn boundary_report(metadata: &MetadataNode) -> String {
    let value = serde_json::json!({
        "boundary": boundary_value(metadata),
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()) + "\n"
}

fn boundary_value(metadata: &MetadataNode) -> serde_json::Value {
    let hexbin = metadata.find_child("stage_1");
    let boundary = hexbin
        .and_then(|stage| stage.find_child("hex_boundary_raw"))
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_string)
        .unwrap_or_else(|| "MULTIPOLYGON EMPTY".to_string());
    let estimated_edge = hexbin
        .and_then(|stage| stage.find_child("estimated_edge"))
        .and_then(MetadataNode::value)
        .map(MetadataValue::as_f64)
        .unwrap_or(0.0);
    let geometry = pdal_native::geometry::Geometry::from_wkt(&boundary).and_then(|geometry| {
        if estimated_edge > 0.0 && boundary != "MULTIPOLYGON EMPTY" {
            geometry.simplify(1.1 * estimated_edge / 2.0, true)
        } else {
            Ok(geometry)
        }
    });
    let boundary = geometry
        .as_ref()
        .ok()
        .and_then(|geometry| geometry.to_wkt_precision(8).ok())
        .unwrap_or(boundary);
    let boundary_json = geometry
        .and_then(|geometry| geometry.to_gdal_geojson(8))
        .unwrap_or_else(|_| "{}".to_string());

    serde_json::json!({
        "boundary": boundary,
        "boundary_json": boundary_json,
    })
}

fn metadata_report(metadata: &MetadataNode) -> String {
    let value = serde_json::json!({
        "metadata": metadata_node_to_json_flat(metadata),
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()) + "\n"
}
