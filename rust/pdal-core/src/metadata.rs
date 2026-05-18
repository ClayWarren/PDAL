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
    pub fn as_string(&self) -> String {
        match self {
            MetadataValue::String(value) => value.clone(),
            MetadataValue::I64(value) => value.to_string(),
            MetadataValue::U64(value) => value.to_string(),
            MetadataValue::F64(value) => value.to_string(),
            MetadataValue::Bool(value) => value.to_string(),
        }
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
}
