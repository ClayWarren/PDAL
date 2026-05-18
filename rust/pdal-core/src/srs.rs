//! Spatial reference primitives for the Rust port.
//!
//! This slice stores the canonical user/WKT text and coordinate epoch. It does
//! not wrap GDAL/PROJ yet; reprojection and authority normalization remain
//! explicit future FFI work.

use crate::metadata::{MetadataNode, MetadataValue};

/// A spatial reference carried by stages, tables, and views.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpatialReference {
    text: String,
    epoch: f64,
}

impl SpatialReference {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            epoch: 0.0,
        }
    }

    pub fn with_epoch(text: impl Into<String>, epoch: f64) -> Self {
        Self {
            text: text.into(),
            epoch,
        }
    }

    pub fn empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn epoch(&self) -> f64 {
        self.epoch
    }

    pub fn set_epoch(&mut self, epoch: f64) {
        self.epoch = epoch;
    }

    pub fn to_metadata(&self) -> MetadataNode {
        let mut root = MetadataNode::new("srs");
        root.add_value("wkt", MetadataValue::String(self.text.clone()));
        if self.epoch != 0.0 {
            root.add_value("coordinate_epoch", MetadataValue::F64(self.epoch));
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_text_and_epoch() {
        let srs = SpatialReference::with_epoch("EPSG:4326", 2020.0);
        assert!(!srs.empty());
        assert_eq!(srs.text(), "EPSG:4326");
        assert_eq!(srs.epoch(), 2020.0);
    }

    #[test]
    fn exports_minimal_metadata() {
        let srs = SpatialReference::with_epoch("EPSG:4978", 2010.0);
        let metadata = srs.to_metadata();
        assert_eq!(metadata.name(), "srs");
        assert_eq!(
            metadata
                .find_child("wkt")
                .and_then(MetadataNode::value)
                .map(MetadataValue::as_string),
            Some("EPSG:4978".into())
        );
        assert_eq!(
            metadata
                .find_child("coordinate_epoch")
                .and_then(MetadataNode::value),
            Some(&MetadataValue::F64(2010.0))
        );
    }
}
