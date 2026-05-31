use super::*;

fn parse(json: &str) -> Value {
    parse_pipeline_descriptors(json).expect("parse should succeed")
}

#[test]
fn classifies_reader_filter_writer_by_type_and_position() {
    let desc = parse(r#"["input.las", {"type": "filters.sort", "dimension": "X"}, "output.laz"]"#);
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
    let err =
        parse_pipeline_descriptors(r#"[{"type": "readers.las", "filename": "a", "tag": "1bad"}]"#)
            .unwrap_err();
    assert!(err.contains("Invalid tag name '1bad'"));
}

#[test]
fn rejects_undefined_input_tag() {
    let err =
        parse_pipeline_descriptors(r#"[{"type": "filters.merge", "inputs": ["nope"]}, "out.las"]"#)
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
