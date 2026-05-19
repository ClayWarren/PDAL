//! Spatial Reference Systems and transformations.

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

pub fn calculate_zone(lon: f64, lat: f64) -> i32 {
    let lon = normalize_longitude(lon);

    let zone = if (56.0..64.0).contains(&lat) && (3.0..12.0).contains(&lon) {
        32
    } else if (72.0..84.0).contains(&lat) {
        if (0.0..9.0).contains(&lon) {
            31
        } else if (9.0..21.0).contains(&lon) {
            33
        } else if (21.0..33.0).contains(&lon) {
            35
        } else if (33.0..42.0).contains(&lon) {
            37
        } else {
            0
        }
    } else {
        ((lon + 180.0) / 6.0).floor() as i32 + 1
    };

    if lat < 0.0 {
        -zone
    } else {
        zone
    }
}

fn normalize_longitude(longitude: f64) -> f64 {
    let longitude = longitude % 360.0;
    if longitude <= -180.0 {
        longitude + 360.0
    } else if longitude > 180.0 {
        longitude - 360.0
    } else {
        longitude
    }
}

/// A coordinate transformation between two SRSs (PDAL's `SrsTransform`).
pub struct SrsTransform {
    inner: pdal_native::srs::SrsTransform,
}

impl SrsTransform {
    pub fn new(src: &SpatialReference, dst: &SpatialReference) -> Result<Self, String> {
        let inner = pdal_native::srs::SrsTransform::new(src.wkt(), dst.wkt())?;
        Ok(Self { inner })
    }

    pub fn new_pipeline(coord_op: &str) -> Result<Self, String> {
        let inner = pdal_native::srs::SrsTransform::new_pipeline(coord_op)?;
        Ok(Self { inner })
    }

    pub fn transform(&self, x: &mut f64, y: &mut f64, z: &mut f64) -> bool {
        self.inner.transform(x, y, z)
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

    #[test]
    fn calculate_zone_matches_pdal_special_cases() {
        let mut zone = 1;
        let mut lon = -537.0;
        while lon < 537.0 {
            assert_eq!(calculate_zone(lon, 25.0), zone);
            assert_eq!(calculate_zone(lon, -25.0), -zone);
            zone += 1;
            if zone > 60 {
                zone = 1;
            }
            lon += 6.0;
        }

        assert_eq!(calculate_zone(5.0, 60.0), 32);
        assert_eq!(calculate_zone(5.0, 80.0), 31);
        assert_eq!(calculate_zone(10.0, 80.0), 33);
        assert_eq!(calculate_zone(25.0, 80.0), 35);
        assert_eq!(calculate_zone(40.0, 80.0), 37);
    }
}
