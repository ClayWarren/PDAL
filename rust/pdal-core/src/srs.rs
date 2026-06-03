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

/// Ordered set of spatial references tracked by a point table.
#[derive(Clone, Debug, Default)]
pub struct SpatialReferenceList {
    refs: Vec<SpatialReference>,
}

impl SpatialReferenceList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.refs.clear();
    }

    pub fn add(&mut self, srs: SpatialReference) {
        if let Some(pos) = self.refs.iter().position(|existing| existing == &srs) {
            if pos != 0 {
                let srs = self.refs.remove(pos);
                self.refs.insert(0, srs);
            }
        } else {
            self.refs.insert(0, srs);
        }
    }

    pub fn is_unique(&self) -> bool {
        self.refs.len() <= 1
    }

    pub fn any(&self) -> SpatialReference {
        self.refs.first().cloned().unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.refs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
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

pub fn wgs84_code_from_zone(zone: i32) -> Option<String> {
    let abs_zone = zone.unsigned_abs();
    if abs_zone == 0 || abs_zone > 60 {
        return None;
    }

    let prefix = if zone > 0 { "EPSG:326" } else { "EPSG:327" };
    Some(format!("{prefix}{abs_zone:02}"))
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
///
/// Backed by GDAL's `OCTTransform` (via [`pdal_native::srs::GdalSrsTransform`]),
/// matching C++ `pdal::SrsTransform`, which uses
/// `OGRCreateCoordinateTransformation` + `Transform(1, &x, &y, &z)`. This is a
/// full 3D transform: the earlier proj-crate `convert` path was 2D-only and
/// silently passed Z through unchanged, which broke projected<->geocentric
/// reprojection.
pub struct SrsTransform {
    inner: pdal_native::srs::GdalSrsTransform,
}

impl SrsTransform {
    pub fn new(src: &SpatialReference, dst: &SpatialReference) -> Result<Self, String> {
        // `GdalSrsTransform` expects WKT (it imports via `OSRImportFromWkt`),
        // but a `SpatialReference` may hold any user-input form (e.g. an EPSG
        // code). Normalize to WKT first, matching C++ `SrsTransform`, which
        // builds its `OGRSpatialReference` from `getWKT2()`.
        let src_wkt = pdal_native::srs::user_input_to_wkt(src.wkt())?.wkt;
        let dst_wkt = pdal_native::srs::user_input_to_wkt(dst.wkt())?.wkt;
        // Empty axis-order slices => GDAL's OAMS_TRADITIONAL_GIS_ORDER on both
        // ends, as in C++ `SrsTransform::set`.
        let inner = pdal_native::srs::GdalSrsTransform::new(
            &src_wkt,
            src.epoch(),
            &dst_wkt,
            dst.epoch(),
            &[],
            &[],
        )?;
        Ok(Self { inner })
    }

    pub fn transform(&self, x: &mut f64, y: &mut f64, z: &mut f64) -> bool {
        self.inner.transform_xyz(x, y, z)
    }
}

/// A GDAL coordinate-operation transform used by `filters.projpipeline`.
pub struct CoordOperationTransform {
    inner: pdal_native::srs::GdalCoordOperationTransform,
}

impl CoordOperationTransform {
    pub fn new(coord_op: &str, reverse: bool) -> Result<Self, String> {
        let inner = pdal_native::srs::GdalCoordOperationTransform::new(coord_op, reverse)?;
        Ok(Self { inner })
    }

    pub fn transform(&self, x: &mut f64, y: &mut f64, z: &mut f64) -> bool {
        self.inner.transform_xyz(x, y, z)
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
    fn spatial_reference_list_tracks_unique_refs_and_moves_recent_to_front() {
        let srs1 = SpatialReference::new("EPSG:4326");
        let srs2 = SpatialReference::new("EPSG:32617");
        let mut list = SpatialReferenceList::new();

        assert!(list.is_empty());
        assert!(list.is_unique());
        assert!(list.any().is_empty());

        list.add(srs1.clone());
        list.add(srs1.clone());
        assert!(list.is_unique());
        assert_eq!(list.any(), srs1);
        assert_eq!(list.len(), 1);

        list.add(srs2.clone());
        assert!(!list.is_unique());
        assert_eq!(list.any(), srs2);
        assert_eq!(list.len(), 2);

        list.add(srs1.clone());
        assert!(!list.is_unique());
        assert_eq!(list.any(), srs1);
        assert_eq!(list.len(), 2);

        list.clear();
        assert!(list.is_empty());
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

    #[test]
    fn wgs84_code_from_zone_matches_pdal_contract() {
        assert_eq!(wgs84_code_from_zone(1), Some("EPSG:32601".into()));
        assert_eq!(wgs84_code_from_zone(17), Some("EPSG:32617".into()));
        assert_eq!(wgs84_code_from_zone(-17), Some("EPSG:32717".into()));
        assert_eq!(wgs84_code_from_zone(60), Some("EPSG:32660".into()));
        assert_eq!(wgs84_code_from_zone(0), None);
        assert_eq!(wgs84_code_from_zone(61), None);
        assert_eq!(wgs84_code_from_zone(-61), None);
    }
}
