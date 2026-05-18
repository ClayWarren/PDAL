//! Spatial Reference Systems and transformations.

use proj::Proj;

/// A spatial reference system (PDAL's `SpatialReference`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SpatialReference {
    wkt: String,
    epoch: f64,
}

impl SpatialReference {
    pub fn new(wkt: &str) -> Self {
        Self {
            wkt: wkt.to_string(),
            epoch: 0.0,
        }
    }

    pub fn with_epoch(wkt: &str, epoch: f64) -> Self {
        Self {
            wkt: wkt.to_string(),
            epoch,
        }
    }

    pub fn wkt(&self) -> &str {
        &self.wkt
    }

    pub fn is_empty(&self) -> bool {
        self.wkt.is_empty()
    }

    pub fn epoch(&self) -> f64 {
        self.epoch
    }

    pub fn set_epoch(&mut self, epoch: f64) {
        self.epoch = epoch;
    }

    pub fn to_metadata(&self) -> crate::metadata::MetadataNode {
        let mut node = crate::metadata::MetadataNode::new("srs");
        node.add_value(
            "wkt",
            crate::metadata::MetadataValue::String(self.wkt.clone()),
        );
        if self.epoch != 0.0 {
            node.add_value("epoch", crate::metadata::MetadataValue::F64(self.epoch));
        }
        node
    }
}

/// A coordinate transformation between two SRSs (PDAL's `SrsTransform`).
pub struct SrsTransform {
    proj: Proj,
}

impl SrsTransform {
    pub fn new(src: &SpatialReference, dst: &SpatialReference) -> Result<Self, String> {
        let proj = Proj::new_known_crs(src.wkt(), dst.wkt(), None)
            .map_err(|e| format!("Failed to create projection: {}", e))?;
        Ok(Self { proj })
    }

    pub fn new_pipeline(coord_op: &str) -> Result<Self, String> {
        let proj = Proj::new(coord_op).map_err(|e| format!("Failed to create pipeline: {}", e))?;
        Ok(Self { proj })
    }

    pub fn transform(&self, x: &mut f64, y: &mut f64, _z: &mut f64) -> bool {
        // Only 2D tuple (f64, f64) implements Coord in proj 0.31 for now
        match self.proj.convert((*x, *y)) {
            Ok((nx, ny)) => {
                *x = nx;
                *y = ny;
                true
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::MetadataValue;

    #[test]
    fn empty_spatial_reference_matches_pdal_empty_contract() {
        let srs = SpatialReference::default();

        assert!(srs.is_empty());
        assert_eq!(srs.wkt(), "");
        assert_eq!(srs.epoch(), 0.0);
    }

    #[test]
    fn spatial_reference_metadata_includes_epoch_only_when_set() {
        let srs = SpatialReference::new("EPSG:4326");
        let metadata = srs.to_metadata();

        assert_eq!(metadata.name(), "srs");
        assert_eq!(
            metadata.find_child("wkt").and_then(|node| node.value()),
            Some(&MetadataValue::String("EPSG:4326".into()))
        );
        assert!(metadata.find_child("epoch").is_none());

        let srs = SpatialReference::with_epoch("EPSG:4326", 2020.5);
        let metadata = srs.to_metadata();

        assert_eq!(
            metadata.find_child("epoch").and_then(|node| node.value()),
            Some(&MetadataValue::F64(2020.5))
        );
    }
}
