use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedFileSpec {
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub query: BTreeMap<String, String>,
}

pub fn parse_file_spec_json(input: &str) -> Result<ParsedFileSpec, String> {
    let value: Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    parse_file_spec_value(value)
}

fn parse_file_spec_value(value: Value) -> Result<ParsedFileSpec, String> {
    match value {
        Value::Null => Err("'filename' argument contains no data".into()),
        Value::String(path) => Ok(ParsedFileSpec {
            path,
            headers: BTreeMap::new(),
            query: BTreeMap::new(),
        }),
        Value::Object(mut object) => parse_file_spec_object(&mut object),
        _ => Err("'filename' must be specified as a string.".into()),
    }
}

fn parse_file_spec_object(object: &mut Map<String, Value>) -> Result<ParsedFileSpec, String> {
    let path = match object.remove("path") {
        Some(Value::Null) => String::new(),
        Some(Value::String(path)) => path,
        Some(_) => {
            return Err("'filename' object 'path' member must be specified as a string.".into())
        }
        None => return Err("'filename' object must contain 'path' member.".into()),
    };

    let headers = extract_string_map(object, "headers")?;
    let query = extract_string_map(object, "query")?;
    if !object.is_empty() {
        return Err(format!(
            "Invalid item in filename object: {}",
            Value::Object(object.clone())
        ));
    }

    Ok(ParsedFileSpec {
        path,
        headers,
        query,
    })
}

fn extract_string_map(
    object: &mut Map<String, Value>,
    name: &str,
) -> Result<BTreeMap<String, String>, String> {
    let Some(value) = object.remove(name) else {
        return Ok(BTreeMap::new());
    };
    if value.is_null() {
        return Ok(BTreeMap::new());
    }
    let Value::Object(values) = value else {
        return Err(format!(
            "'filename' sub-argument '{name}' must be an object of string key-value pairs."
        ));
    };

    let mut output = BTreeMap::new();
    for (key, value) in values {
        let Value::String(value) = value else {
            return Err(format!(
                "'filename' sub-argument '{name}' must be an object of string key-value pairs."
            ));
        };
        output.insert(key, value);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_string_and_object_specs() {
        let spec = parse_file_spec_value(json!("foo.laz")).unwrap();
        assert_eq!(spec.path, "foo.laz");
        assert!(spec.headers.is_empty());
        assert!(spec.query.is_empty());

        let spec = parse_file_spec_value(json!({
            "path": "foo.laz",
            "headers": {"some_header_key": "some_header_val"},
            "query": {"some_query_key": "some_query_val"}
        }))
        .unwrap();
        assert_eq!(spec.path, "foo.laz");
        assert_eq!(spec.headers["some_header_key"], "some_header_val");
        assert_eq!(spec.query["some_query_key"], "some_query_val");
    }

    #[test]
    fn reports_cpp_error_messages() {
        assert_eq!(
            parse_file_spec_value(json!({
                "path": "foo.laz",
                "query": ["some_query_key", "some_query_val"]
            }))
            .unwrap_err(),
            "'filename' sub-argument 'query' must be an object of string key-value pairs."
        );
        assert_eq!(
            parse_file_spec_value(json!({"path": ["foo.laz"]})).unwrap_err(),
            "'filename' object 'path' member must be specified as a string."
        );
        assert_eq!(
            parse_file_spec_value(json!({"path": "foo.laz", "foo": "test"})).unwrap_err(),
            "Invalid item in filename object: {\"foo\":\"test\"}"
        );
        assert_eq!(
            parse_file_spec_value(json!({"headers": {}})).unwrap_err(),
            "'filename' object must contain 'path' member."
        );
        assert_eq!(
            parse_file_spec_value(Value::Null).unwrap_err(),
            "'filename' argument contains no data"
        );
    }
}
