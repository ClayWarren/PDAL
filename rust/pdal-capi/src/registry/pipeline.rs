//! Pipeline-from-JSON construction for the registry.
//!
//! Split out of `registry.rs` to keep modules under ~1k LOC.

use crate::error::{clear_last_error, set_last_error};
use crate::pipeline_abi::PipelineHandle;
use pdal_core::driver::{infer_reader_driver, infer_writer_driver};
use pdal_core::options::Options;
use pdal_core::pipeline::Pipeline;
use pdal_core::stage::StageError;

use serde_json::Value;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::registry::{create_stage, CreatedStage};

pub fn pipeline_from_json(json: &str) -> Result<Pipeline, StageError> {
    let stripped = pdal_core::pipeline_reader::strip_json_comments(json);
    let value: Value = serde_json::from_str(&stripped)
        .map_err(|err| StageError(format!("Invalid pipeline JSON: {err}")))?;
    let stages = pipeline_stages(&value)?;

    let mut pipeline = Pipeline::new();
    let mut tags = HashMap::new();
    let mut previous: Option<usize> = None;

    for (position, stage_val) in stages.iter().enumerate() {
        let string_stage;
        let object = if let Some(object) = stage_val.as_object() {
            object
        } else if let Some(filename) = stage_val.as_str() {
            string_stage = filename_stage_object(filename);
            &string_stage
        } else {
            return Err(StageError(format!(
                "Pipeline stage {position} must be a JSON object or filename string."
            )));
        };

        validate_type_field(object, position)?;
        let tag = tag_from_object(object)?;
        let options = options_from_object(object)?;
        let driver_name = stage_name(object, position, stages.len(), &options)?;
        if driver_name.starts_with("readers.") && object.contains_key("inputs") {
            return Err(StageError(
                "JSON pipeline: Inputs not permitted for reader.".to_string(),
            ));
        }
        let stage = create_stage(&driver_name, &options)?;

        let idx = match stage {
            CreatedStage::Reader(r) => pipeline.add_reader(&driver_name, r, options),
            CreatedStage::Filter(f) => pipeline.add_stage(&driver_name, f, options),
            CreatedStage::Writer(w) => pipeline.add_writer(&driver_name, w, options),
        };

        if let Some(tag) = tag {
            pipeline.set_tag(idx, tag)?;
            tags.insert(tag.to_string(), idx);
        }

        let explicit_inputs = add_explicit_inputs(&mut pipeline, idx, object, &tags)?;
        if !explicit_inputs && position > 0 {
            if let Some(input) = previous {
                pipeline.add_dependency(idx, input)?;
            }
        }
        previous = Some(idx);
    }

    Ok(pipeline)
}

fn validate_type_field(
    object: &serde_json::Map<String, Value>,
    position: usize,
) -> Result<(), StageError> {
    match object.get("type") {
        None | Some(Value::Null) | Some(Value::String(_)) => Ok(()),
        Some(_) => Err(StageError(format!(
            "Pipeline stage {position} has a non-string 'type'."
        ))),
    }
}

fn tag_from_object(object: &serde_json::Map<String, Value>) -> Result<Option<&str>, StageError> {
    let tag = match object.get("tag") {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(tag)) => tag.as_str(),
        Some(_) => {
            return Err(StageError(
                "JSON pipeline: tag must be specified as a string.".to_string(),
            ));
        }
    };
    if !valid_tag_name(tag) {
        return Err(StageError(format!(
            "JSON pipeline: Invalid tag name '{tag}'. Must start with letter. Remainder can be letters, digits or underscores."
        )));
    }
    Ok(Some(tag))
}

fn valid_tag_name(tag: &str) -> bool {
    let mut chars = tag.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn filename_stage_object(filename: &str) -> serde_json::Map<String, Value> {
    let mut object = serde_json::Map::new();
    object.insert("filename".to_string(), Value::String(filename.to_string()));
    object
}

fn pipeline_stages(value: &Value) -> Result<&Vec<Value>, StageError> {
    if let Some(stages) = value.as_array() {
        return Ok(stages);
    }
    let Some(object) = value.as_object() else {
        return Err(StageError(
            "Pipeline JSON must be an array or an object with a 'pipeline' array.".to_string(),
        ));
    };
    object
        .get("pipeline")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            StageError("Pipeline JSON object must contain a 'pipeline' array.".to_string())
        })
}

fn stage_name(
    object: &serde_json::Map<String, Value>,
    position: usize,
    len: usize,
    options: &Options,
) -> Result<String, StageError> {
    if let Some(name) = object.get("type").and_then(Value::as_str) {
        return Ok(name.to_string());
    }
    let filename = options.get_str("filename", "");
    if filename.is_empty() {
        return Err(StageError(format!(
            "Pipeline stage {position} is missing a 'type'."
        )));
    }
    if position == 0 {
        infer_reader_driver(&filename)
            .map(str::to_string)
            .ok_or_else(|| StageError(format!("Unable to infer reader for '{filename}'.")))
    } else if position + 1 == len {
        infer_writer_driver(&filename)
            .map(str::to_string)
            .ok_or_else(|| StageError(format!("Unable to infer writer for '{filename}'.")))
    } else {
        Err(StageError(format!(
            "Pipeline stage {position} needs an explicit 'type'."
        )))
    }
}

pub(crate) fn options_from_object(
    object: &serde_json::Map<String, Value>,
) -> Result<Options, StageError> {
    Options::from_pipeline_stage_object(object).map_err(StageError)
}

fn add_explicit_inputs(
    pipeline: &mut Pipeline,
    idx: usize,
    object: &serde_json::Map<String, Value>,
    tags: &HashMap<String, usize>,
) -> Result<bool, StageError> {
    let Some(inputs) = object.get("inputs") else {
        return Ok(false);
    };
    let input_names = match inputs {
        Value::String(name) => vec![name.as_str()],
        Value::Array(values) => values
            .iter()
            .map(|value| {
                value.as_str().ok_or_else(|| {
                    StageError("Pipeline 'inputs' entries must be tag strings.".to_string())
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(StageError(
                "Pipeline 'inputs' must be a tag string or array of tag strings.".to_string(),
            ));
        }
    };
    for name in input_names {
        let input = tags
            .get(name)
            .copied()
            .ok_or_else(|| StageError(format!("Unknown pipeline input tag '{name}'.")))?;
        pipeline.add_dependency(idx, input)?;
    }
    Ok(true)
}

/// Create a pipeline from JSON.
///
/// # Safety
/// `json` must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_create_json(json: *const c_char) -> *mut PipelineHandle {
    clear_last_error();
    if json.is_null() {
        set_last_error("null json string");
        return std::ptr::null_mut();
    }
    let json_str = CStr::from_ptr(json).to_string_lossy();
    match pipeline_from_json(&json_str) {
        Ok(pipeline) => Box::into_raw(Box::new(PipelineHandle { pipeline })),
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}
