//! The point-buffer model -- the Rust analog of PDAL's `Dimension`,
//! `PointLayout`, `PointTable` and `PointView`.
//!
//! A [`PointLayout`] fixes the set of dimensions and their byte offsets within
//! a fixed-size point record. A [`PointView`] is a contiguous buffer of such
//! records. PDAL shares one `PointTable` across many views; this spike folds
//! the storage into the view for simplicity -- the shared table is a planned
//! follow-up, not a behavioural difference for a single filter.

use crate::srs::SpatialReference;
use std::rc::Rc;

/// Index of a point within a view.
pub type PointId = u64;

/// Identifier for a point dimension.
///
/// PDAL has a large fixed enum of well-known dimensions; the spike models the
/// ones the current slice needs, with `Other` as an escape hatch for the rest.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DimId {
    X,
    Y,
    Z,
    Intensity,
    OffsetTime,
    Classification,
    ClusterID,
    HeightAboveGround,
    LocalOutlierFactor,
    LocalReachabilityDistance,
    RadialDensity,
    NNDistance,
    Reciprocity,
    Rank,
    Coplanar,
    PlaneFit,
    Eigenvalue0,
    Eigenvalue1,
    Eigenvalue2,
    OptimalKNN,
    OptimalRadius,
    H3,
    GpsTime,
    W,
    TextureU,
    TextureV,
    TextureW,
    NormalX,
    NormalY,
    NormalZ,
    StartPulse,
    ReflectedPulse,
    Azimuth,
    Pitch,
    Roll,
    Pdop,
    PulseWidth,
    XVelocity,
    YVelocity,
    ZVelocity,
    WanderAngle,
    XBodyAccel,
    YBodyAccel,
    ZBodyAccel,
    XBodyAngRate,
    YBodyAngRate,
    ZBodyAngRate,
    NorthPositionRMS,
    EastPositionRMS,
    DownPositionRMS,
    NorthVelocityRMS,
    EastVelocityRMS,
    DownVelocityRMS,
    RollRMS,
    PitchRMS,
    HeadingRMS,
    Other(String),
}

impl DimId {
    /// The canonical dimension name.
    pub fn name(&self) -> &str {
        match self {
            DimId::X => "X",
            DimId::Y => "Y",
            DimId::Z => "Z",
            DimId::Intensity => "Intensity",
            DimId::OffsetTime => "OffsetTime",
            DimId::Classification => "Classification",
            DimId::ClusterID => "ClusterID",
            DimId::HeightAboveGround => "HeightAboveGround",
            DimId::LocalOutlierFactor => "LocalOutlierFactor",
            DimId::LocalReachabilityDistance => "LocalReachabilityDistance",
            DimId::RadialDensity => "RadialDensity",
            DimId::NNDistance => "NNDistance",
            DimId::Reciprocity => "Reciprocity",
            DimId::Rank => "Rank",
            DimId::Coplanar => "Coplanar",
            DimId::PlaneFit => "PlaneFit",
            DimId::Eigenvalue0 => "Eigenvalue0",
            DimId::Eigenvalue1 => "Eigenvalue1",
            DimId::Eigenvalue2 => "Eigenvalue2",
            DimId::OptimalKNN => "OptimalKNN",
            DimId::OptimalRadius => "OptimalRadius",
            DimId::H3 => "H3",
            DimId::GpsTime => "GpsTime",
            DimId::W => "W",
            DimId::TextureU => "TextureU",
            DimId::TextureV => "TextureV",
            DimId::TextureW => "TextureW",
            DimId::NormalX => "NormalX",
            DimId::NormalY => "NormalY",
            DimId::NormalZ => "NormalZ",
            DimId::StartPulse => "StartPulse",
            DimId::ReflectedPulse => "ReflectedPulse",
            DimId::Azimuth => "Azimuth",
            DimId::Pitch => "Pitch",
            DimId::Roll => "Roll",
            DimId::Pdop => "Pdop",
            DimId::PulseWidth => "PulseWidth",
            DimId::XVelocity => "XVelocity",
            DimId::YVelocity => "YVelocity",
            DimId::ZVelocity => "ZVelocity",
            DimId::WanderAngle => "WanderAngle",
            DimId::XBodyAccel => "XBodyAccel",
            DimId::YBodyAccel => "YBodyAccel",
            DimId::ZBodyAccel => "ZBodyAccel",
            DimId::XBodyAngRate => "XBodyAngRate",
            DimId::YBodyAngRate => "YBodyAngRate",
            DimId::ZBodyAngRate => "ZBodyAngRate",
            DimId::NorthPositionRMS => "NorthPositionRMS",
            DimId::EastPositionRMS => "EastPositionRMS",
            DimId::DownPositionRMS => "DownPositionRMS",
            DimId::NorthVelocityRMS => "NorthVelocityRMS",
            DimId::EastVelocityRMS => "EastVelocityRMS",
            DimId::DownVelocityRMS => "DownVelocityRMS",
            DimId::RollRMS => "RollRMS",
            DimId::PitchRMS => "PitchRMS",
            DimId::HeadingRMS => "HeadingRMS",
            DimId::Other(s) => s,
        }
    }

    /// Construct from string name.
    pub fn from_name(name: &str) -> Self {
        match name {
            "X" => DimId::X,
            "Y" => DimId::Y,
            "Z" => DimId::Z,
            "Intensity" => DimId::Intensity,
            "OffsetTime" => DimId::OffsetTime,
            "Classification" => DimId::Classification,
            "ClusterID" => DimId::ClusterID,
            "HeightAboveGround" => DimId::HeightAboveGround,
            "LocalOutlierFactor" => DimId::LocalOutlierFactor,
            "LocalReachabilityDistance" => DimId::LocalReachabilityDistance,
            "RadialDensity" => DimId::RadialDensity,
            "NNDistance" => DimId::NNDistance,
            "Reciprocity" => DimId::Reciprocity,
            "Rank" => DimId::Rank,
            "Coplanar" => DimId::Coplanar,
            "PlaneFit" => DimId::PlaneFit,
            "Eigenvalue0" => DimId::Eigenvalue0,
            "Eigenvalue1" => DimId::Eigenvalue1,
            "Eigenvalue2" => DimId::Eigenvalue2,
            "OptimalKNN" => DimId::OptimalKNN,
            "OptimalRadius" => DimId::OptimalRadius,
            "H3" => DimId::H3,
            "GpsTime" => DimId::GpsTime,
            "W" => DimId::W,
            "TextureU" => DimId::TextureU,
            "TextureV" => DimId::TextureV,
            "TextureW" => DimId::TextureW,
            "NormalX" => DimId::NormalX,
            "NormalY" => DimId::NormalY,
            "NormalZ" => DimId::NormalZ,
            "StartPulse" => DimId::StartPulse,
            "ReflectedPulse" => DimId::ReflectedPulse,
            "Azimuth" => DimId::Azimuth,
            "Pitch" => DimId::Pitch,
            "Roll" => DimId::Roll,
            "Pdop" => DimId::Pdop,
            "PulseWidth" => DimId::PulseWidth,
            "XVelocity" => DimId::XVelocity,
            "YVelocity" => DimId::YVelocity,
            "ZVelocity" => DimId::ZVelocity,
            "WanderAngle" => DimId::WanderAngle,
            "XBodyAccel" => DimId::XBodyAccel,
            "YBodyAccel" => DimId::YBodyAccel,
            "ZBodyAccel" => DimId::ZBodyAccel,
            "XBodyAngRate" => DimId::XBodyAngRate,
            "YBodyAngRate" => DimId::YBodyAngRate,
            "ZBodyAngRate" => DimId::ZBodyAngRate,
            "NorthPositionRMS" => DimId::NorthPositionRMS,
            "EastPositionRMS" => DimId::EastPositionRMS,
            "DownPositionRMS" => DimId::DownPositionRMS,
            "NorthVelocityRMS" => DimId::NorthVelocityRMS,
            "EastVelocityRMS" => DimId::EastVelocityRMS,
            "DownVelocityRMS" => DimId::DownVelocityRMS,
            "RollRMS" => DimId::RollRMS,
            "PitchRMS" => DimId::PitchRMS,
            "HeadingRMS" => DimId::HeadingRMS,
            other => DimId::Other(other.to_string()),
        }
    }
}

/// In-memory storage type of a dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DimType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

impl DimType {
    /// Size in bytes of one value of this type.
    pub fn size(self) -> usize {
        match self {
            DimType::U8 | DimType::I8 => 1,
            DimType::U16 | DimType::I16 => 2,
            DimType::U32 | DimType::I32 | DimType::F32 => 4,
            DimType::U64 | DimType::I64 | DimType::F64 => 8,
        }
    }
}

#[derive(Clone)]
struct DimEntry {
    id: DimId,
    ty: DimType,
    offset: usize,
}

/// The ordered set of registered dimensions, fixing each one's byte offset
/// within a point record. Immutable once a [`PointView`] is built from it.
#[derive(Clone, Default)]
pub struct PointLayout {
    dims: Vec<DimEntry>,
    point_size: usize,
}

impl PointLayout {
    /// An empty layout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a dimension. Re-registering an existing dimension is a no-op
    /// (PDAL likewise keeps the first registration).
    pub fn register(&mut self, id: DimId, ty: DimType) {
        if self.dims.iter().any(|d| d.id == id) {
            return;
        }
        let offset = self.point_size;
        self.point_size += ty.size();
        self.dims.push(DimEntry { id, ty, offset });
    }

    /// Size in bytes of one point record.
    pub fn point_size(&self) -> usize {
        self.point_size
    }

    /// The `(byte offset, type)` of a dimension, if registered.
    pub fn dim(&self, id: &DimId) -> Option<(usize, DimType)> {
        self.dims
            .iter()
            .find(|d| &d.id == id)
            .map(|d| (d.offset, d.ty))
    }

    /// Number of dimensions in registration order.
    pub fn dim_count(&self) -> usize {
        self.dims.len()
    }

    /// Dimension id and type at registration index `idx`.
    pub fn dim_at(&self, idx: usize) -> Option<(&DimId, DimType)> {
        self.dims.get(idx).map(|d| (&d.id, d.ty))
    }
}

/// A buffer of points sharing one layout.
#[derive(Clone)]
pub struct PointView {
    layout: Rc<PointLayout>,
    data: Vec<u8>,
    source_indices: Vec<PointId>,
    spatial_reference: SpatialReference,
}

impl PointView {
    /// A new, empty view over the given layout.
    pub fn new(layout: Rc<PointLayout>) -> Self {
        PointView {
            layout,
            data: Vec::new(),
            source_indices: Vec::new(),
            spatial_reference: SpatialReference::default(),
        }
    }

    /// A new empty view sharing this view's layout (PDAL's `makeNew`).
    pub fn make_new(&self) -> PointView {
        PointView {
            layout: Rc::clone(&self.layout),
            data: Vec::new(),
            source_indices: Vec::new(),
            spatial_reference: self.spatial_reference.clone(),
        }
    }

    /// The view's layout.
    pub fn layout(&self) -> &Rc<PointLayout> {
        &self.layout
    }

    /// The view's spatial reference.
    pub fn spatial_reference(&self) -> &SpatialReference {
        &self.spatial_reference
    }

    /// Set the view's spatial reference.
    pub fn set_spatial_reference(&mut self, spatial_reference: SpatialReference) {
        self.spatial_reference = spatial_reference;
    }

    /// Number of points in the view.
    pub fn len(&self) -> u64 {
        let ps = self.layout.point_size();
        self.data.len().checked_div(ps).unwrap_or(0) as u64
    }

    /// Whether the view holds no points.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Truncate the view to `len` points.
    pub fn truncate(&mut self, len: u64) {
        let ps = self.layout.point_size();
        self.data.truncate((len as usize) * ps);
        self.source_indices.truncate(len as usize);
    }

    /// Append a zero-initialised point and return its index.
    pub fn add_point(&mut self) -> PointId {
        let id = self.len();
        let ps = self.layout.point_size();
        self.data.resize(self.data.len() + ps, 0);
        self.source_indices.push(id);
        id
    }

    /// Copy point `src_idx` from `src` onto the end of this view (PDAL's
    /// `appendPoint`). The two views must share an identical layout.
    pub fn append_point(&mut self, src: &PointView, src_idx: PointId) {
        let ps = self.layout.point_size();
        let start = (src_idx as usize) * ps;
        self.data.extend_from_slice(&src.data[start..start + ps]);
        self.source_indices.push(src.source_index(src_idx));
    }

    /// Original source row copied into this point. Used by the C++ bridge to
    /// preserve PDAL PointView table IDs when filters return subsets.
    pub fn source_index(&self, idx: PointId) -> PointId {
        self.source_indices
            .get(idx as usize)
            .copied()
            .unwrap_or(idx)
    }

    /// Read a dimension of point `idx` as `f64` (PDAL's `getFieldAs`).
    /// Unregistered dimensions read as `0.0`.
    pub fn get_f64(&self, idx: PointId, dim: &DimId) -> f64 {
        match self.layout.dim(dim) {
            Some((off, ty)) => {
                let base = (idx as usize) * self.layout.point_size() + off;
                read_value(&self.data[base..base + ty.size()], ty)
            }
            None => 0.0,
        }
    }

    /// Write a dimension of point `idx` from an `f64` (PDAL's `setField`),
    /// converting to the dimension's storage type. Unregistered dimensions
    /// are ignored.
    pub fn set_f64(&mut self, idx: PointId, dim: &DimId, value: f64) {
        if let Some((off, ty)) = self.layout.dim(dim) {
            let base = (idx as usize) * self.layout.point_size() + off;
            write_value(&mut self.data[base..base + ty.size()], ty, value);
        }
    }
}

/// Decode a little-endian dimension value to `f64`.
fn read_value(buf: &[u8], ty: DimType) -> f64 {
    match ty {
        DimType::U8 => buf[0] as f64,
        DimType::I8 => (buf[0] as i8) as f64,
        DimType::U16 => u16::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::I16 => i16::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::U32 => u32::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::I32 => i32::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::U64 => u64::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::I64 => i64::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::F32 => f32::from_le_bytes(buf.try_into().unwrap()) as f64,
        DimType::F64 => f64::from_le_bytes(buf.try_into().unwrap()),
    }
}

/// Encode an `f64` into a little-endian dimension value of type `ty`.
fn write_value(buf: &mut [u8], ty: DimType, v: f64) {
    match ty {
        DimType::U8 => buf[0] = v as u8,
        DimType::I8 => buf[0] = (v as i8) as u8,
        DimType::U16 => buf.copy_from_slice(&(v as u16).to_le_bytes()),
        DimType::I16 => buf.copy_from_slice(&(v as i16).to_le_bytes()),
        DimType::U32 => buf.copy_from_slice(&(v as u32).to_le_bytes()),
        DimType::I32 => buf.copy_from_slice(&(v as i32).to_le_bytes()),
        DimType::U64 => buf.copy_from_slice(&(v as u64).to_le_bytes()),
        DimType::I64 => buf.copy_from_slice(&(v as i64).to_le_bytes()),
        DimType::F32 => buf.copy_from_slice(&(v as f32).to_le_bytes()),
        DimType::F64 => buf.copy_from_slice(&v.to_le_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_offsets_and_field_roundtrip() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Intensity, DimType::U16);
        layout.register(DimId::X, DimType::F64); // duplicate -- ignored
        assert_eq!(layout.point_size(), 10);

        let mut view = PointView::new(Rc::new(layout));
        let p = view.add_point();
        view.set_f64(p, &DimId::X, 12.5);
        view.set_f64(p, &DimId::Intensity, 700.0);
        assert_eq!(view.len(), 1);
        assert_eq!(view.get_f64(p, &DimId::X), 12.5);
        assert_eq!(view.get_f64(p, &DimId::Intensity), 700.0);
    }

    #[test]
    fn append_point_copies_record() {
        let mut layout = PointLayout::new();
        layout.register(DimId::OffsetTime, DimType::F64);
        let layout = Rc::new(layout);

        let mut src = PointView::new(Rc::clone(&layout));
        let p = src.add_point();
        src.set_f64(p, &DimId::OffsetTime, 42.0);

        let mut dst = src.make_new();
        assert!(dst.is_empty());
        dst.append_point(&src, p);
        assert_eq!(dst.len(), 1);
        assert_eq!(dst.get_f64(0, &DimId::OffsetTime), 42.0);
    }

    #[test]
    fn append_point_preserves_source_index() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let layout = Rc::new(layout);

        let mut src = PointView::new(Rc::clone(&layout));
        for i in 0..3 {
            let point = src.add_point();
            src.set_f64(point, &DimId::X, i as f64);
        }

        let mut dst = src.make_new();
        dst.append_point(&src, 2);
        dst.append_point(&src, 0);

        assert_eq!(dst.len(), 2);
        assert_eq!(dst.source_index(0), 2);
        assert_eq!(dst.source_index(1), 0);
        assert_eq!(dst.get_f64(0, &DimId::X), 2.0);
        assert_eq!(dst.get_f64(1, &DimId::X), 0.0);
    }

    #[test]
    fn all_dimension_storage_types_roundtrip_through_f64_accessors() {
        let cases = [
            (DimType::U8, 255.0, 255.0),
            (DimType::U16, 65_535.0, 65_535.0),
            (DimType::U32, 1_000_000.0, 1_000_000.0),
            (DimType::U64, 1_000_000.0, 1_000_000.0),
            (DimType::I8, -12.0, -12.0),
            (DimType::I16, -1234.0, -1234.0),
            (DimType::I32, -123_456.0, -123_456.0),
            (DimType::I64, -123_456.0, -123_456.0),
            (DimType::F32, 12.25, 12.25),
            (DimType::F64, -99.5, -99.5),
        ];

        for (idx, (ty, value, expected)) in cases.into_iter().enumerate() {
            let dim = DimId::Other(format!("dim{idx}"));
            let mut layout = PointLayout::new();
            layout.register(dim.clone(), ty);
            let mut view = PointView::new(Rc::new(layout));

            let point = view.add_point();
            view.set_f64(point, &dim, value);

            assert_eq!(view.get_f64(point, &dim), expected);
        }
    }

    #[test]
    fn make_new_keeps_layout_and_spatial_reference_without_points() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        view.set_spatial_reference(SpatialReference::with_epoch("EPSG:4326", 2020.0));
        view.add_point();

        let new_view = view.make_new();

        assert!(new_view.is_empty());
        assert_eq!(new_view.layout().point_size(), view.layout().point_size());
        assert_eq!(new_view.spatial_reference().wkt(), "EPSG:4326");
        assert_eq!(new_view.spatial_reference().epoch(), 2020.0);
    }
}
