//! Faithful parse + validation of C++ `PipelineReaderJSON` pipeline documents.
//!
//! This owns the *parsing contract* of `pdal::PipelineReaderJSON`: JSONC
//! comment stripping, root-structure validation, per-stage `type`/`tag`/
//! `inputs` validation, and reader/writer/filter role classification. It does
//! not build stages; it returns a flat, pre-validated descriptor array that the
//! C++ wrapper consumes to construct the `Stage*` DAG (glob expansion, plugin
//! loading, `FileSpec`/`Options` construction, `makeReader/Writer/Filter`, and
//! input wiring stay in C++ because they own C++ objects).

use crate::error::{set_last_error, string_to_c_ptr};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Strip `//` line and `/* */` block comments from JSONC text, honoring string
/// literals (so `//` inside a quoted value such as an `http://` URL is kept).
/// Mirrors nlohmann's `parse(..., ignore_comments = true)`.
fn strip_json_comments(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escaped = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            out.push(c as char);
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == b'"' {
            in_string = true;
            out.push('"');
            i += 1;
        } else if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            // Line comment: skip to end of line.
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            // Block comment: skip to closing */.
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else {
            // Non-ASCII bytes pass through unchanged; build from the original
            // str slice to keep UTF-8 intact.
            let ch_len = utf8_len(c);
            out.push_str(&input[i..i + ch_len]);
            i += ch_len;
        }
    }
    out
}

fn utf8_len(first: u8) -> usize {
    match first {
        b if b < 0x80 => 1,
        b if b >> 5 == 0b110 => 2,
        b if b >> 4 == 0b1110 => 3,
        _ => 4,
    }
}

/// Validate a pipeline stage tag, matching `Stage::parseTagName`: non-empty,
/// first char ASCII alphabetic, remaining chars alphanumeric or underscore.
fn valid_tag_name(tag: &str) -> bool {
    let mut chars = tag.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Classify a stage's role from its explicit `type` and position, mirroring the
/// C++ `parsePipeline` branch order.
fn classify_role(stage_type: &str, position: usize, count: usize) -> &'static str {
    let last = count - 1;
    if (stage_type.is_empty() && (position == 0 || position != last))
        || stage_type.starts_with("readers.")
    {
        "reader"
    } else if stage_type.is_empty() || stage_type.starts_with("writers.") {
        "writer"
    } else {
        "filter"
    }
}

fn root_stages(value: &Value) -> Result<&Vec<Value>, String> {
    if let Some(array) = value.as_array() {
        return Ok(array);
    }
    if let Some(object) = value.as_object() {
        if let Some(array) = object.get("pipeline").and_then(Value::as_array) {
            return Ok(array);
        }
    }
    Err("Pipeline: root element is not a pipeline.".to_string())
}

fn extract_type(object: &Map<String, Value>) -> Result<String, String> {
    match object.get("type") {
        None => Ok(String::new()),
        Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err("JSON pipeline: 'type' must be specified as a string.".to_string()),
    }
}

fn extract_tag(object: &Map<String, Value>, seen: &HashSet<String>) -> Result<String, String> {
    let tag = match object.get("tag") {
        None | Some(Value::Null) => return Ok(String::new()),
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err("JSON pipeline: tag must be specified as a string.".to_string());
        }
    };
    if seen.contains(&tag) {
        return Err(format!("JSON pipeline: duplicate tag '{tag}'."));
    }
    if !valid_tag_name(&tag) {
        return Err(format!(
            "JSON pipeline: Invalid tag name '{tag}'.  Must start with letter.  \
             Remainder can be letters, digits or underscores."
        ));
    }
    Ok(tag)
}

fn extract_inputs(
    object: &Map<String, Value>,
    seen: &HashSet<String>,
) -> Result<Vec<String>, String> {
    let Some(value) = object.get("inputs") else {
        return Ok(Vec::new());
    };
    let names = match value {
        Value::String(name) => vec![name.clone()],
        Value::Array(items) => {
            let mut names = Vec::with_capacity(items.len());
            for item in items {
                let name = item.as_str().ok_or_else(|| {
                    "JSON pipeline: 'inputs' tag must  be specified as a string or array of strings."
                        .to_string()
                })?;
                names.push(name.to_string());
            }
            names
        }
        _ => {
            return Err(
                "JSON pipeline: 'inputs' tag must  be specified as a string or array of strings."
                    .to_string(),
            );
        }
    };
    for name in &names {
        if !seen.contains(name) {
            return Err(format!(
                "JSON pipeline: Invalid pipeline: undefined stage tag '{name}'."
            ));
        }
    }
    Ok(names)
}

/// Remaining option keys (everything except `type`/`tag`/`inputs`/`filename`)
/// as a raw JSON object; the C++ wrapper builds `Options` from it (preserving
/// `plugin` handling and value typing in C++).
fn extract_options(object: &Map<String, Value>) -> Map<String, Value> {
    let mut options = Map::new();
    for (key, value) in object {
        if matches!(key.as_str(), "type" | "tag" | "inputs" | "filename") {
            continue;
        }
        options.insert(key.clone(), value.clone());
    }
    options
}

/// Parse a pipeline document into the flat descriptor array. See module docs.
pub fn parse_pipeline_descriptors(json: &str) -> Result<Value, String> {
    let stripped = strip_json_comments(json);
    let root: Value = serde_json::from_str(&stripped).map_err(|err| format!("Pipeline: {err}"))?;
    let stages = root_stages(&root)?;
    let count = stages.len();

    let mut descriptors = Vec::with_capacity(count);
    let mut seen_tags: HashSet<String> = HashSet::new();

    for (position, node) in stages.iter().enumerate() {
        if let Some(filename) = node.as_str() {
            // Bare string nodes are filenames with no type/tag/inputs/options.
            let role = classify_role("", position, count);
            descriptors.push(json!({
                "role": role,
                "type": "",
                "tag": "",
                "inputs": [],
                "explicit_inputs": false,
                "string_node": true,
                "filename": filename,
                "options": {},
            }));
            continue;
        }

        let object = node.as_object().ok_or_else(|| {
            "Pipeline: stage element is not an object or filename string.".to_string()
        })?;

        let stage_type = extract_type(object)?;
        let tag = extract_tag(object, &seen_tags)?;
        let inputs = extract_inputs(object, &seen_tags)?;
        let explicit_inputs = !inputs.is_empty();
        let options = extract_options(object);
        let role = classify_role(&stage_type, position, count);

        if role == "reader" && explicit_inputs {
            // Matches the C++ "Inputs not permitted for reader" guard. The
            // path is filled in by the C++ wrapper after glob expansion; here
            // we surface the same error class without the per-file name.
            return Err("JSON pipeline: Inputs not permitted for  reader.".to_string());
        }

        let filename = object.get("filename").cloned().unwrap_or(Value::Null);

        descriptors.push(json!({
            "role": role,
            "type": stage_type,
            "tag": tag,
            "inputs": inputs,
            "explicit_inputs": explicit_inputs,
            "string_node": false,
            "filename": filename,
            "options": options,
        }));

        if !tag.is_empty() {
            seen_tags.insert(tag);
        }
    }

    Ok(Value::Array(descriptors))
}

/// Parse a pipeline JSON document into a descriptor array (see module docs).
///
/// On success returns a newly-allocated JSON string (free with
/// `pdal_string_free`). On error returns null and sets the last error to the
/// C++-compatible message.
///
/// # Safety
/// `json` must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn pdal_pipeline_reader_parse_json(json: *const c_char) -> *mut c_char {
    if json.is_null() {
        set_last_error("Pipeline: null pipeline JSON.");
        return std::ptr::null_mut();
    }
    let json = CStr::from_ptr(json).to_string_lossy().into_owned();
    match parse_pipeline_descriptors(&json) {
        Ok(descriptors) => string_to_c_ptr(descriptors.to_string()),
        Err(message) => {
            set_last_error(message);
            std::ptr::null_mut()
        }
    }
}

#[cfg(test)]
mod tests;
