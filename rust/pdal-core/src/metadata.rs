//! Metadata tree primitives for the Rust port.
//!
//! This is intentionally a structural model only. It preserves PDAL's named
//! node tree and typed scalar values without trying to serialize every C++
//! `MetadataNode` behavior in the first slice.

/// A scalar metadata value.
#[derive(Clone, Debug, PartialEq)]
pub enum MetadataValue {
    String(String),
    I64(i64),
    U64(u64),
    F64(f64),
    Bool(bool),
}

impl MetadataValue {
    pub fn kind_id(&self) -> u8 {
        match self {
            MetadataValue::String(_) => 0,
            MetadataValue::I64(_) => 1,
            MetadataValue::U64(_) => 2,
            MetadataValue::F64(_) => 3,
            MetadataValue::Bool(_) => 4,
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            MetadataValue::String(value) => value.clone(),
            MetadataValue::I64(value) => value.to_string(),
            MetadataValue::U64(value) => value.to_string(),
            MetadataValue::F64(value) => value.to_string(),
            MetadataValue::Bool(value) => value.to_string(),
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            MetadataValue::I64(value) => *value,
            MetadataValue::U64(value) => *value as i64,
            MetadataValue::F64(value) => *value as i64,
            MetadataValue::Bool(value) => i64::from(*value),
            MetadataValue::String(value) => value.parse().unwrap_or_default(),
        }
    }

    pub fn as_u64(&self) -> u64 {
        match self {
            MetadataValue::I64(value) => *value as u64,
            MetadataValue::U64(value) => *value,
            MetadataValue::F64(value) => *value as u64,
            MetadataValue::Bool(value) => u64::from(*value),
            MetadataValue::String(value) => value.parse().unwrap_or_default(),
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            MetadataValue::I64(value) => *value as f64,
            MetadataValue::U64(value) => *value as f64,
            MetadataValue::F64(value) => *value,
            MetadataValue::Bool(value) => {
                if *value {
                    1.0
                } else {
                    0.0
                }
            }
            MetadataValue::String(value) => value.parse().unwrap_or_default(),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            MetadataValue::I64(value) => *value != 0,
            MetadataValue::U64(value) => *value != 0,
            MetadataValue::F64(value) => *value != 0.0,
            MetadataValue::Bool(value) => *value,
            MetadataValue::String(value) => matches!(value.as_str(), "true" | "1"),
        }
    }
}

pub fn json_scalar_value(type_name: &str, value: &str) -> String {
    if type_name == "json" {
        return value.to_string();
    }

    if type_name == "double" && matches!(value, "NaN" | "Infinity" | "-Infinity") {
        return quote_json_string(value);
    }

    if matches!(
        type_name,
        "string" | "base64Binary" | "uuid" | "matrix" | "spatialreference" | "bounds"
    ) {
        return quote_json_string(value);
    }

    escape_json(value)
}

pub fn scalar_as_i64(type_name: &str, value: &str) -> Option<i64> {
    if type_name == "base64Binary" {
        let bytes = decode_base64(value)?;
        let array: [u8; 8] = bytes.get(..8)?.try_into().ok()?;
        return Some(i64::from_ne_bytes(array));
    }
    value.parse().ok()
}

pub fn scalar_as_u64(type_name: &str, value: &str) -> Option<u64> {
    if type_name == "base64Binary" {
        let bytes = decode_base64(value)?;
        let array: [u8; 8] = bytes.get(..8)?.try_into().ok()?;
        return Some(u64::from_ne_bytes(array));
    }
    value.parse().ok()
}

pub fn scalar_as_f64(type_name: &str, value: &str) -> Option<f64> {
    if type_name == "base64Binary" {
        let bytes = decode_base64(value)?;
        let array: [u8; 8] = bytes.get(..8)?.try_into().ok()?;
        return Some(f64::from_ne_bytes(array));
    }
    value.parse().ok()
}

pub fn scalar_as_bool(type_name: &str, value: &str) -> Option<bool> {
    if type_name == "boolean" {
        return match value {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
    }
    match value {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

fn quote_json_string(value: &str) -> String {
    format!("\"{}\"", escape_json(value).replace('"', "\\\""))
}

fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch < ' ' => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn decode_base64(value: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(value.len() * 3 / 4);
    let mut chunk = [0_u8; 4];
    let mut len = 0_usize;

    for byte in value.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        chunk[len] = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => 64,
            _ => return None,
        };
        len += 1;

        if len == 4 {
            if chunk[0] == 64 || chunk[1] == 64 {
                return None;
            }
            out.push((chunk[0] << 2) | (chunk[1] >> 4));
            if chunk[2] != 64 {
                out.push((chunk[1] << 4) | (chunk[2] >> 2));
            }
            if chunk[3] != 64 {
                out.push((chunk[2] << 6) | chunk[3]);
            }
            len = 0;
        }
    }

    if len == 0 {
        Some(out)
    } else {
        None
    }
}

/// A named metadata node.
#[derive(Clone, Debug, PartialEq)]
pub struct MetadataNode {
    name: String,
    value: Option<MetadataValue>,
    type_name: Option<String>,
    description: Option<String>,
    children: Vec<MetadataNode>,
}

impl MetadataNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: None,
            type_name: None,
            description: None,
            children: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn value(&self) -> Option<&MetadataValue> {
        self.value.as_ref()
    }

    pub fn type_name(&self) -> Option<&str> {
        self.type_name.as_deref()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn children(&self) -> &[MetadataNode] {
        &self.children
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn set_value(&mut self, value: MetadataValue) {
        self.value = Some(value);
    }

    pub fn set_type_name(&mut self, type_name: impl Into<String>) {
        self.type_name = Some(type_name.into());
    }

    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = Some(description.into());
    }

    pub fn add_child(&mut self, child: MetadataNode) {
        self.children.push(child);
    }

    pub fn add_value(
        &mut self,
        name: impl Into<String>,
        value: MetadataValue,
    ) -> &mut MetadataNode {
        self.children.push(MetadataNode::new(name));
        let child = self.children.last_mut().expect("child just pushed");
        child.set_value(value);
        child
    }

    pub fn find_child(&self, name: &str) -> Option<&MetadataNode> {
        self.children.iter().find(|child| child.name == name)
    }

    pub fn find_child_mut(&mut self, name: &str) -> Option<&mut MetadataNode> {
        self.children.iter_mut().find(|child| child.name == name)
    }

    pub fn add_or_update(&mut self, child: MetadataNode) {
        if let Some(existing) = self.find_child_mut(child.name()) {
            *existing = child;
        } else {
            self.add_child(child);
        }
    }

    pub fn child(&self, index: usize) -> Option<&MetadataNode> {
        self.children.get(index)
    }

    pub fn children_named(&self, name: &str) -> Vec<&MetadataNode> {
        self.children
            .iter()
            .filter(|child| child.name() == name)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_typed_values_and_children() {
        let mut root = MetadataNode::new("root");
        root.add_value("count", MetadataValue::U64(7));
        root.add_value("valid", MetadataValue::Bool(true));

        assert_eq!(
            root.find_child("count").and_then(MetadataNode::value),
            Some(&MetadataValue::U64(7))
        );
        assert_eq!(
            root.find_child("valid").and_then(MetadataNode::value),
            Some(&MetadataValue::Bool(true))
        );
    }

    #[test]
    fn add_or_update_replaces_matching_child() {
        let mut root = MetadataNode::new("root");
        root.add_value("srs", MetadataValue::String("old".into()));

        let mut replacement = MetadataNode::new("srs");
        replacement.set_value(MetadataValue::String("new".into()));
        root.add_or_update(replacement);

        assert_eq!(root.children().len(), 1);
        assert_eq!(
            root.find_child("srs")
                .and_then(MetadataNode::value)
                .map(MetadataValue::as_string),
            Some("new".into())
        );
    }

    #[test]
    fn typed_values_convert_like_pdal_metadata_scalars() {
        assert_eq!(MetadataValue::String("42".into()).as_i64(), 42);
        assert_eq!(MetadataValue::String("42".into()).as_u64(), 42);
        assert_eq!(MetadataValue::String("2.5".into()).as_f64(), 2.5);
        assert!(MetadataValue::String("true".into()).as_bool());
        assert!(MetadataValue::String("1".into()).as_bool());
        assert!(!MetadataValue::String("false".into()).as_bool());

        assert_eq!(MetadataValue::I64(-7).as_string(), "-7");
        assert_eq!(MetadataValue::U64(7).as_i64(), 7);
        assert_eq!(MetadataValue::F64(3.9).as_u64(), 3);
        assert_eq!(MetadataValue::Bool(true).as_f64(), 1.0);
        assert_eq!(MetadataValue::Bool(false).as_i64(), 0);
    }

    #[test]
    fn node_preserves_type_name_description_and_child_order() {
        let mut root = MetadataNode::new("root");
        let first = root.add_value("first", MetadataValue::String("one".into()));
        first.set_type_name("string");
        first.set_description("first child");
        root.add_value("second", MetadataValue::U64(2));

        assert_eq!(root.child_count(), 2);
        assert_eq!(root.children()[0].name(), "first");
        assert_eq!(root.children()[0].type_name(), Some("string"));
        assert_eq!(root.children()[0].description(), Some("first child"));
        assert_eq!(root.children()[1].name(), "second");
    }

    #[test]
    fn add_or_update_preserves_replacement_subtree() {
        let mut root = MetadataNode::new("root");
        let mut child = MetadataNode::new("child");
        child.add_value("old", MetadataValue::U64(1));
        root.add_child(child);

        let mut replacement = MetadataNode::new("child");
        replacement.add_value("new", MetadataValue::U64(2));
        replacement.add_value("newer", MetadataValue::U64(3));
        root.add_or_update(replacement);

        assert_eq!(root.child_count(), 1);
        let child = root.child(0).expect("replacement child");
        assert_eq!(child.children().len(), 2);
        assert_eq!(child.children()[0].name(), "new");
        assert_eq!(child.children()[1].name(), "newer");
        assert_eq!(root.children_named("child").len(), 1);
    }

    #[test]
    fn json_scalar_value_matches_cpp_metadata_contract() {
        assert_eq!(json_scalar_value("json", "{\"key\":42}"), "{\"key\":42}");
        assert_eq!(json_scalar_value("double", "NaN"), "\"NaN\"");
        assert_eq!(json_scalar_value("double", "Infinity"), "\"Infinity\"");
        assert_eq!(json_scalar_value("string", "a\"b"), "\"a\\\"b\"");
        assert_eq!(json_scalar_value("integer", "-7"), "-7");
        assert_eq!(json_scalar_value("boolean", "true"), "true");
    }

    #[test]
    fn scalar_values_convert_from_text_and_base64() {
        assert_eq!(scalar_as_i64("integer", "-7"), Some(-7));
        assert_eq!(scalar_as_u64("nonNegativeInteger", "7"), Some(7));
        assert_eq!(scalar_as_f64("double", "1.25"), Some(1.25));
        assert_eq!(scalar_as_bool("boolean", "true"), Some(true));
        assert_eq!(scalar_as_bool("boolean", "maybe"), None);

        let encoded = "zczMzMzcXkA=";
        assert_eq!(scalar_as_f64("base64Binary", encoded), Some(123.45));
    }
}
