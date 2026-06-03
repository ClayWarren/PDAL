//! Faithful parse + validation of C++ `PipelineReaderJSON` pipeline documents.
//!
//! This owns the parsing contract of `pdal::PipelineReaderJSON`: JSONC comment
//! stripping, root-structure validation, per-stage `type`/`tag`/`inputs`
//! validation, and reader/writer/filter role classification. It does not build
//! stages; it returns a flat, pre-validated descriptor array.

use serde_json::{json, Map, Value};
use std::collections::HashSet;

pub fn strip_json_comments(input: &str) -> String {
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
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else {
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

fn valid_tag_name(tag: &str) -> bool {
    let mut chars = tag.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

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
                    "JSON pipeline: 'inputs' tag must be specified as a string or array of strings."
                        .to_string()
                })?;
                names.push(name.to_string());
            }
            names
        }
        _ => {
            return Err(
                "JSON pipeline: 'inputs' tag must be specified as a string or array of strings."
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

pub fn parse_pipeline_descriptors(json: &str) -> Result<Value, String> {
    let stripped = strip_json_comments(json);
    let root: Value = serde_json::from_str(&stripped).map_err(|err| format!("Pipeline: {err}"))?;
    let stages = root_stages(&root)?;
    let count = stages.len();

    let mut descriptors = Vec::with_capacity(count);
    let mut seen_tags: HashSet<String> = HashSet::new();

    for (position, node) in stages.iter().enumerate() {
        if let Some(filename) = node.as_str() {
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
            return Err("JSON pipeline: Inputs not permitted for reader.".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Value {
        parse_pipeline_descriptors(json).expect("parse should succeed")
    }

    #[test]
    fn classifies_reader_filter_writer_by_type_and_position() {
        let desc =
            parse(r#"["input.las", {"type": "filters.sort", "dimension": "X"}, "output.laz"]"#);
        let arr = desc.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["role"], "reader");
        assert_eq!(arr[0]["string_node"], true);
        assert_eq!(arr[0]["filename"], "input.las");
        assert_eq!(arr[1]["role"], "filter");
        assert_eq!(arr[1]["type"], "filters.sort");
        assert_eq!(arr[1]["options"]["dimension"], "X");
        assert_eq!(arr[2]["role"], "writer");
    }

    #[test]
    fn single_filename_stage_is_a_reader() {
        let desc = parse(r#"["only.las"]"#);
        assert_eq!(desc.as_array().unwrap()[0]["role"], "reader");
    }

    #[test]
    fn explicit_reader_writer_types_win_over_position() {
        let desc = parse(
            r#"[{"type": "writers.las", "filename": "o.las"}, {"type": "readers.las", "filename": "i.las"}]"#,
        );
        let arr = desc.as_array().unwrap();
        assert_eq!(arr[0]["role"], "writer");
        assert_eq!(arr[1]["role"], "reader");
        assert_eq!(arr[1]["filename"], "i.las");
    }

    #[test]
    fn root_pipeline_object_is_accepted() {
        let desc = parse(r#"{"pipeline": ["a.las", "b.las"]}"#);
        assert_eq!(desc.as_array().unwrap().len(), 2);
    }

    #[test]
    fn tags_and_inputs_resolve_to_earlier_stages() {
        let desc = parse(
            r#"[
                {"type": "readers.las", "filename": "a.las", "tag": "A"},
                {"type": "readers.las", "filename": "b.las", "tag": "B"},
                {"type": "filters.merge", "inputs": ["A", "B"]}
            ]"#,
        );
        let arr = desc.as_array().unwrap();
        assert_eq!(arr[2]["inputs"][0], "A");
        assert_eq!(arr[2]["inputs"][1], "B");
        assert_eq!(arr[2]["explicit_inputs"], true);
    }

    #[test]
    fn strips_jsonc_comments_but_keeps_urls() {
        let desc = parse(
            "[\n  // a reader\n  {\"type\": \"readers.las\", \"filename\": \"http://x/y.las\"} /* trailing */\n]",
        );
        let arr = desc.as_array().unwrap();
        assert_eq!(arr[0]["type"], "readers.las");
        assert_eq!(arr[0]["filename"], "http://x/y.las");
    }

    #[test]
    fn rejects_non_pipeline_root() {
        let err = parse_pipeline_descriptors(r#"42"#).unwrap_err();
        assert!(err.contains("root element is not a pipeline"));
    }

    #[test]
    fn rejects_duplicate_tag() {
        let err = parse_pipeline_descriptors(
            r#"[{"type": "readers.las", "filename": "a", "tag": "A"}, {"type": "readers.las", "filename": "b", "tag": "A"}]"#,
        )
        .unwrap_err();
        assert!(err.contains("duplicate tag 'A'"));
    }

    #[test]
    fn rejects_invalid_tag_name() {
        let err = parse_pipeline_descriptors(
            r#"[{"type": "readers.las", "filename": "a", "tag": "1bad"}]"#,
        )
        .unwrap_err();
        assert!(err.contains("Invalid tag name '1bad'"));
    }

    #[test]
    fn rejects_undefined_input_tag() {
        let err = parse_pipeline_descriptors(
            r#"[{"type": "filters.merge", "inputs": ["nope"]}, "out.las"]"#,
        )
        .unwrap_err();
        assert!(err.contains("undefined stage tag 'nope'"));
    }

    #[test]
    fn rejects_non_string_type() {
        let err = parse_pipeline_descriptors(r#"[{"type": 5, "filename": "a"}]"#).unwrap_err();
        assert!(err.contains("'type' must be specified as a string"));
    }

    #[test]
    fn rejects_inputs_on_reader() {
        let err = parse_pipeline_descriptors(
            r#"[{"type": "readers.las", "filename": "a", "tag": "X"}, {"type": "readers.las", "filename": "b", "inputs": ["X"]}]"#,
        )
        .unwrap_err();
        assert!(err.contains("Inputs not permitted for"));
    }

    #[test]
    fn object_option_values_are_preserved_as_json() {
        let desc = parse(r#"[{"type": "filters.mongo", "expression": {"X": {"$gt": 1}}}]"#);
        let arr = desc.as_array().unwrap();
        assert_eq!(arr[0]["options"]["expression"]["X"]["$gt"], 1);
    }
}
