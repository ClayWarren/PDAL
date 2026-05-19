use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OgrSpecOptions {
    pub datasource: String,
    pub layer: String,
    pub sql: String,
    pub dialect: String,
    pub geometry: String,
    pub drivers: Vec<String>,
    pub open_options: Vec<String>,
}

pub fn parse_ogr_spec_json(input: &str) -> Result<OgrSpecOptions, String> {
    let value: Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    parse_ogr_spec_value(value)
}

fn parse_ogr_spec_value(value: Value) -> Result<OgrSpecOptions, String> {
    let Value::Object(object) = value else {
        return Err("'ogr' option must be a JSON object with 'type':'ogr' specified!".into());
    };

    let Some(type_value) = object.get("type") else {
        return Err("'ogr' option must be a JSON object with 'type':'ogr' specified!".into());
    };
    let type_name = string_field(type_value, "type")?;
    if !type_name.eq_ignore_ascii_case("ogr") {
        return Err("'ogr' option must have 'type':'ogr' specified!".into());
    }

    let mut options = OgrSpecOptions::default();
    for (key, value) in object {
        let field = key.to_ascii_lowercase();
        if value.is_null() || value == Value::String(String::new()) {
            return Err(format!("invalid value for field '{field}' in OGR JSON!"));
        }

        match field.as_str() {
            "datasource" => options.datasource = string_field(&value, &field)?,
            "drivers" => options.drivers = string_array_field(&value, &field)?,
            "openoptions" => options.open_options = string_array_field(&value, &field)?,
            "layer" => options.layer = string_field(&value, &field)?,
            "sql" => options.sql = string_field(&value, &field)?,
            "options" => parse_nested_options(value, &mut options)?,
            "type" => {}
            _ => return Err(format!("unexpected field '{field}' in OGR JSON!")),
        }
    }

    if options.datasource.is_empty() {
        return Err("'ogr' option must contain a 'datasource' field!".into());
    }

    Ok(options)
}

fn parse_nested_options(value: Value, options: &mut OgrSpecOptions) -> Result<(), String> {
    let Value::Object(object) = value else {
        return Err("invalid value for field 'options' in OGR JSON!".into());
    };

    for (key, value) in object {
        let field = key.to_ascii_lowercase();
        match field.as_str() {
            "dialect" => options.dialect = string_field(&value, &field)?,
            "geometry" => options.geometry = string_field(&value, &field)?,
            _ => return Err("invalid value for 'options' field in OGR JSON!".into()),
        }
    }
    Ok(())
}

fn string_field(value: &Value, field: &str) -> Result<String, String> {
    let Value::String(value) = value else {
        return Err(format!("invalid value for field '{field}' in OGR JSON!"));
    };
    Ok(value.clone())
}

fn string_array_field(value: &Value, field: &str) -> Result<Vec<String>, String> {
    let Value::Array(values) = value else {
        return Err(format!("invalid value for field '{field}' in OGR JSON!"));
    };
    values
        .iter()
        .map(|value| string_field(value, field))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_ogr_options() {
        let options = parse_ogr_spec_value(json!({
            "type": "OGR",
            "datasource": "attributes.json",
            "drivers": ["GeoJSON"],
            "openoptions": ["FLATTEN_NESTED_ATTRIBUTES=YES"],
            "layer": "attributes",
            "sql": "select * from attributes",
            "options": {
                "dialect": "OGRSQL",
                "geometry": "{\"type\":\"Polygon\"}"
            }
        }))
        .unwrap();

        assert_eq!(options.datasource, "attributes.json");
        assert_eq!(options.drivers, vec!["GeoJSON"]);
        assert_eq!(options.open_options, vec!["FLATTEN_NESTED_ATTRIBUTES=YES"]);
        assert_eq!(options.layer, "attributes");
        assert_eq!(options.sql, "select * from attributes");
        assert_eq!(options.dialect, "OGRSQL");
        assert_eq!(options.geometry, "{\"type\":\"Polygon\"}");
    }

    #[test]
    fn reports_cpp_error_messages() {
        assert_eq!(
            parse_ogr_spec_value(json!([{"type": "ogr"}])).unwrap_err(),
            "'ogr' option must be a JSON object with 'type':'ogr' specified!"
        );
        assert_eq!(
            parse_ogr_spec_value(json!({"type": "test", "datasource": "x"})).unwrap_err(),
            "'ogr' option must have 'type':'ogr' specified!"
        );
        assert_eq!(
            parse_ogr_spec_value(json!({"type": "ogr", "datasource": "x", "foo": "test"}))
                .unwrap_err(),
            "unexpected field 'foo' in OGR JSON!"
        );
        assert_eq!(
            parse_ogr_spec_value(json!({"type": "ogr", "datasource": "x", "sql": ""})).unwrap_err(),
            "invalid value for field 'sql' in OGR JSON!"
        );
        assert_eq!(
            parse_ogr_spec_value(json!({"type": "ogr"})).unwrap_err(),
            "'ogr' option must contain a 'datasource' field!"
        );
    }
}
