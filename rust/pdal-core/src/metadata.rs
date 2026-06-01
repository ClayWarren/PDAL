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
    Pointer(usize),
}

impl MetadataValue {
    pub fn kind_id(&self) -> u8 {
        match self {
            MetadataValue::String(_) => 0,
            MetadataValue::I64(_) => 1,
            MetadataValue::U64(_) => 2,
            MetadataValue::F64(_) => 3,
            MetadataValue::Bool(_) => 4,
            MetadataValue::Pointer(_) => 5,
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            MetadataValue::String(value) => value.clone(),
            MetadataValue::I64(value) => value.to_string(),
            MetadataValue::U64(value) => value.to_string(),
            MetadataValue::F64(value) => value.to_string(),
            MetadataValue::Bool(value) => value.to_string(),
            MetadataValue::Pointer(value) => value.to_string(),
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            MetadataValue::I64(value) => *value,
            MetadataValue::U64(value) => *value as i64,
            MetadataValue::F64(value) => *value as i64,
            MetadataValue::Bool(value) => i64::from(*value),
            MetadataValue::Pointer(value) => *value as i64,
            MetadataValue::String(value) => value.parse().unwrap_or_default(),
        }
    }

    pub fn as_u64(&self) -> u64 {
        match self {
            MetadataValue::I64(value) => *value as u64,
            MetadataValue::U64(value) => *value,
            MetadataValue::F64(value) => *value as u64,
            MetadataValue::Bool(value) => u64::from(*value),
            MetadataValue::Pointer(value) => *value as u64,
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
            MetadataValue::Pointer(value) => *value as f64,
            MetadataValue::String(value) => value.parse().unwrap_or_default(),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            MetadataValue::I64(value) => *value != 0,
            MetadataValue::U64(value) => *value != 0,
            MetadataValue::F64(value) => *value != 0.0,
            MetadataValue::Bool(value) => *value,
            MetadataValue::Pointer(value) => *value != 0,
            MetadataValue::String(value) => matches!(value.as_str(), "true" | "1"),
        }
    }

    pub fn as_pointer(&self) -> usize {
        match self {
            MetadataValue::Pointer(value) => *value,
            _ => 0,
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

pub fn metadata_node_to_json(node: &MetadataNode) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    object.insert("name".to_string(), serde_json::json!(node.name()));
    if node.kind() == MetadataKind::Array {
        object.insert("kind".to_string(), serde_json::json!("array"));
    }

    if let Some(value) = node.value() {
        object.insert("value".to_string(), metadata_value_to_json(value));
        object.insert(
            "value_type".to_string(),
            serde_json::json!(metadata_value_type(value)),
        );
    }
    if let Some(type_name) = node.type_name() {
        object.insert("type".to_string(), serde_json::json!(type_name));
    }
    if let Some(description) = node.description() {
        object.insert("description".to_string(), serde_json::json!(description));
    }
    if !node.children().is_empty() {
        object.insert(
            "children".to_string(),
            serde_json::Value::Array(node.children().iter().map(metadata_node_to_json).collect()),
        );
    }

    serde_json::Value::Object(object)
}

pub fn metadata_node_to_json_flat(node: &MetadataNode) -> serde_json::Value {
    if node.children().is_empty() {
        if let Some(value) = node.value() {
            return metadata_value_to_json(value);
        }
    }

    let mut object = serde_json::Map::new();
    for child in node.children() {
        let value = metadata_node_to_json_flat(child);
        if child.kind() == MetadataKind::Array {
            match object.get_mut(child.name()) {
                Some(serde_json::Value::Array(values)) => values.push(value),
                Some(existing) => {
                    let previous = std::mem::replace(existing, serde_json::Value::Null);
                    *existing = serde_json::Value::Array(vec![previous, value]);
                }
                None => {
                    object.insert(
                        child.name().to_string(),
                        serde_json::Value::Array(vec![value]),
                    );
                }
            }
        } else {
            object.insert(child.name().to_string(), value);
        }
    }
    serde_json::Value::Object(object)
}

fn metadata_value_to_json(value: &MetadataValue) -> serde_json::Value {
    match value {
        MetadataValue::String(value) => serde_json::json!(value),
        MetadataValue::I64(value) => serde_json::json!(value),
        MetadataValue::U64(value) => serde_json::json!(value),
        MetadataValue::F64(value) => serde_json::json!(value),
        MetadataValue::Bool(value) => serde_json::json!(value),
        MetadataValue::Pointer(value) => serde_json::json!(value),
    }
}

fn metadata_value_type(value: &MetadataValue) -> &'static str {
    match value {
        MetadataValue::String(_) => "string",
        MetadataValue::I64(_) => "i64",
        MetadataValue::U64(_) => "u64",
        MetadataValue::F64(_) => "f64",
        MetadataValue::Bool(_) => "bool",
        MetadataValue::Pointer(_) => "pointer",
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataKind {
    Instance,
    Array,
}

impl MetadataKind {
    pub fn as_u8(self) -> u8 {
        match self {
            MetadataKind::Instance => 0,
            MetadataKind::Array => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetadataNode {
    name: String,
    kind: MetadataKind,
    value: Option<MetadataValue>,
    type_name: Option<String>,
    description: Option<String>,
    children: Vec<MetadataNode>,
}

impl MetadataNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: MetadataKind::Instance,
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

    pub fn kind(&self) -> MetadataKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: MetadataKind) {
        self.kind = kind;
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

    pub fn add_child(&mut self, mut child: MetadataNode) {
        if self
            .children
            .iter()
            .any(|existing| existing.name == child.name)
        {
            child.kind = MetadataKind::Array;
            for existing in self
                .children
                .iter_mut()
                .filter(|existing| existing.name == child.name)
            {
                existing.kind = MetadataKind::Array;
            }
        }
        self.children.push(child);
    }

    pub fn add_list_child(&mut self, mut child: MetadataNode) {
        child.kind = MetadataKind::Array;
        for existing in self
            .children
            .iter_mut()
            .filter(|existing| existing.name == child.name)
        {
            existing.kind = MetadataKind::Array;
        }
        self.children.push(child);
    }

    pub fn add_value(
        &mut self,
        name: impl Into<String>,
        value: MetadataValue,
    ) -> &mut MetadataNode {
        self.add_child(MetadataNode::new(name));
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
        assert_eq!(root.children()[0].kind(), MetadataKind::Instance);
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
    fn duplicate_children_become_array_kind() {
        let mut root = MetadataNode::new("root");
        root.add_value("item", MetadataValue::U64(1));
        root.add_value("item", MetadataValue::U64(2));

        assert_eq!(root.child_count(), 2);
        assert_eq!(root.children()[0].kind(), MetadataKind::Array);
        assert_eq!(root.children()[1].kind(), MetadataKind::Array);
    }

    #[test]
    fn explicit_list_child_is_array_from_first_entry() {
        let mut root = MetadataNode::new("root");
        let mut child = MetadataNode::new("item");
        child.set_value(MetadataValue::U64(1));
        root.add_list_child(child);

        assert_eq!(root.child_count(), 1);
        assert_eq!(root.children()[0].kind(), MetadataKind::Array);
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

    #[test]
    fn metadata_value_kind_id_unique_per_variant() {
        assert_eq!(MetadataValue::String("a".into()).kind_id(), 0);
        assert_eq!(MetadataValue::I64(1).kind_id(), 1);
        assert_eq!(MetadataValue::U64(1).kind_id(), 2);
        assert_eq!(MetadataValue::F64(1.0).kind_id(), 3);
        assert_eq!(MetadataValue::Bool(true).kind_id(), 4);
        assert_eq!(MetadataValue::Pointer(1).kind_id(), 5);
    }

    #[test]
    fn metadata_value_as_string_covers_each_variant() {
        assert_eq!(MetadataValue::String("hello".into()).as_string(), "hello");
        assert_eq!(MetadataValue::I64(-7).as_string(), "-7");
        assert_eq!(MetadataValue::U64(42).as_string(), "42");
        assert_eq!(MetadataValue::F64(1.5).as_string(), "1.5");
        assert_eq!(MetadataValue::Bool(true).as_string(), "true");
        assert_eq!(MetadataValue::Pointer(42).as_string(), "42");
    }

    #[test]
    fn metadata_value_as_i64_covers_each_variant() {
        assert_eq!(MetadataValue::I64(-5).as_i64(), -5);
        assert_eq!(MetadataValue::U64(7).as_i64(), 7);
        assert_eq!(MetadataValue::F64(3.7).as_i64(), 3);
        assert_eq!(MetadataValue::Bool(true).as_i64(), 1);
        assert_eq!(MetadataValue::Bool(false).as_i64(), 0);
        assert_eq!(MetadataValue::Pointer(42).as_i64(), 42);
        assert_eq!(MetadataValue::String("42".into()).as_i64(), 42);
        assert_eq!(MetadataValue::String("nope".into()).as_i64(), 0);
    }

    #[test]
    fn metadata_value_as_u64_covers_each_variant() {
        assert_eq!(MetadataValue::I64(5).as_u64(), 5);
        assert_eq!(MetadataValue::U64(7).as_u64(), 7);
        assert_eq!(MetadataValue::F64(3.7).as_u64(), 3);
        assert_eq!(MetadataValue::Bool(true).as_u64(), 1);
        assert_eq!(MetadataValue::Pointer(42).as_u64(), 42);
        assert_eq!(MetadataValue::String("42".into()).as_u64(), 42);
    }

    #[test]
    fn metadata_value_as_f64_covers_each_variant() {
        assert_eq!(MetadataValue::I64(2).as_f64(), 2.0);
        assert_eq!(MetadataValue::U64(7).as_f64(), 7.0);
        assert_eq!(MetadataValue::F64(1.5).as_f64(), 1.5);
        assert_eq!(MetadataValue::Bool(true).as_f64(), 1.0);
        assert_eq!(MetadataValue::Bool(false).as_f64(), 0.0);
        assert_eq!(MetadataValue::Pointer(42).as_f64(), 42.0);
        assert_eq!(MetadataValue::String("3.15".into()).as_f64(), 3.15);
    }

    #[test]
    fn metadata_value_as_bool_covers_each_variant() {
        assert!(MetadataValue::I64(1).as_bool());
        assert!(!MetadataValue::I64(0).as_bool());
        assert!(MetadataValue::U64(2).as_bool());
        assert!(!MetadataValue::U64(0).as_bool());
        assert!(MetadataValue::F64(0.1).as_bool());
        assert!(!MetadataValue::F64(0.0).as_bool());
        assert!(MetadataValue::Bool(true).as_bool());
        assert!(MetadataValue::Pointer(42).as_bool());
        assert!(!MetadataValue::Pointer(0).as_bool());
        assert!(MetadataValue::String("true".into()).as_bool());
        assert!(MetadataValue::String("1".into()).as_bool());
        assert!(!MetadataValue::String("nope".into()).as_bool());
    }

    #[test]
    fn metadata_value_pointer_round_trips_raw_address() {
        assert_eq!(MetadataValue::Pointer(0x1234).as_pointer(), 0x1234);
        assert_eq!(MetadataValue::String("0x1234".into()).as_pointer(), 0);
    }

    #[test]
    fn json_scalar_value_quotes_string_types() {
        assert_eq!(json_scalar_value("string", "hi"), "\"hi\"");
        assert_eq!(json_scalar_value("matrix", "[1,2]"), "\"[1,2]\"");
        assert_eq!(json_scalar_value("double", "NaN"), "\"NaN\"");
        assert_eq!(json_scalar_value("double", "Infinity"), "\"Infinity\"");
        assert_eq!(json_scalar_value("double", "-Infinity"), "\"-Infinity\"");
        assert_eq!(json_scalar_value("json", "{\"a\":1}"), "{\"a\":1}");
        assert_eq!(json_scalar_value("int32", "42"), "42");
    }

    #[test]
    fn scalar_as_i64_handles_base64_and_parse() {
        // 8 little-endian bytes for i64(42)
        let bytes = 42i64.to_ne_bytes();
        // Base64 encode
        let encoded = pdal_base64_encode(&bytes);
        assert_eq!(scalar_as_i64("base64Binary", &encoded), Some(42));
        // Truncated input -> None
        assert_eq!(scalar_as_i64("base64Binary", "AAAA"), None);
        // Plain string parse
        assert_eq!(scalar_as_i64("int32", "123"), Some(123));
        assert_eq!(scalar_as_i64("int32", "not"), None);
    }

    #[test]
    fn scalar_as_u64_handles_base64_and_parse() {
        let bytes = 42u64.to_ne_bytes();
        let encoded = pdal_base64_encode(&bytes);
        assert_eq!(scalar_as_u64("base64Binary", &encoded), Some(42));
        assert_eq!(scalar_as_u64("base64Binary", "AAAA"), None);
        assert_eq!(scalar_as_u64("uint32", "5"), Some(5));
    }

    #[test]
    fn scalar_as_f64_handles_base64_and_parse() {
        let bytes = 1.5_f64.to_ne_bytes();
        let encoded = pdal_base64_encode(&bytes);
        assert!((scalar_as_f64("base64Binary", &encoded).unwrap() - 1.5).abs() < 1e-12);
        assert_eq!(scalar_as_f64("base64Binary", "AAAA"), None);
        assert!((scalar_as_f64("double", "3.5").unwrap() - 3.5).abs() < 1e-12);
    }

    #[test]
    fn scalar_as_bool_handles_boolean_and_numeric_str() {
        assert_eq!(scalar_as_bool("boolean", "true"), Some(true));
        assert_eq!(scalar_as_bool("boolean", "false"), Some(false));
        assert_eq!(scalar_as_bool("boolean", "other"), None);
        assert_eq!(scalar_as_bool("int32", "1"), Some(true));
        assert_eq!(scalar_as_bool("int32", "0"), Some(false));
        assert_eq!(scalar_as_bool("int32", "garbage"), None);
    }

    #[test]
    fn decode_base64_rejects_invalid_chars() {
        // Char outside base64 alphabet (excluding =) -> None
        assert!(decode_base64("!!").is_none());
        // Padding in disallowed positions
        assert!(decode_base64("====").is_none());
    }

    #[test]
    fn escape_json_handles_control_chars() {
        assert!(escape_json("a\\b").contains("\\\\"));
        assert!(escape_json("a\nb").contains("\\n"));
        assert!(escape_json("a\tb").contains("\\t"));
        assert!(escape_json("a\rb").contains("\\r"));
        // backspace U+08, formfeed U+0C
        assert!(escape_json("a\u{08}b").contains("\\b"));
        assert!(escape_json("a\u{0c}b").contains("\\f"));
        // Other control chars escaped as \uNNNN
        let s = escape_json("a\u{01}b");
        assert!(s.contains("\\u0001"));
    }

    #[test]
    fn add_or_update_replaces_existing_child() {
        let mut root = MetadataNode::new("root");
        let mut a1 = MetadataNode::new("kid");
        a1.add_value("v", MetadataValue::U64(1));
        root.add_child(a1);
        let mut a2 = MetadataNode::new("kid");
        a2.add_value("v", MetadataValue::U64(2));
        root.add_or_update(a2);
        // Still only one "kid" child
        assert_eq!(root.children_named("kid").len(), 1);
    }

    #[test]
    fn add_or_update_inserts_when_missing() {
        let mut root = MetadataNode::new("root");
        let kid = MetadataNode::new("newkid");
        root.add_or_update(kid);
        assert!(root.find_child("newkid").is_some());
    }

    // Helper: tiny base64 encoder used only by these unit tests.
    fn pdal_base64_encode(bytes: &[u8]) -> String {
        const CHARS: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
            out.push(CHARS[(b0 >> 2) as usize] as char);
            out.push(CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
            if chunk.len() > 1 {
                out.push(CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                out.push('=');
            }
            if chunk.len() > 2 {
                out.push(CHARS[(b2 & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
        out
    }
}
