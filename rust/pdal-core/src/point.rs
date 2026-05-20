//! The point-buffer model -- the Rust analog of PDAL's `Dimension`,
//! `PointLayout`, `PointTable` and `PointView`.
//!
//! A [`PointLayout`] fixes the set of dimensions and their byte offsets within
//! a fixed-size point record. A [`PointView`] is a contiguous buffer of such
//! records. PDAL shares one `PointTable` across many views; this spike folds
//! the storage into the view for simplicity -- the shared table is a planned
//! follow-up, not a behavioural difference for a single filter.

use crate::raster::{RasterData, RasterLimits};
use crate::srs::SpatialReference;
use std::collections::BTreeMap;
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
    Dimension,
    StartPulse,
    ReflectedPulse,
    Azimuth,
    Pitch,
    Roll,
    Pdop,
    PulseWidth,
    PassiveSignal,
    PassiveX,
    PassiveY,
    PassiveZ,
    ReturnNumber,
    NumberOfReturns,
    ScanDirectionFlag,
    EdgeOfFlightLine,
    ScanAngleRank,
    PointSourceId,
    UserData,
    EchoRange,
    EchoNorm,
    EchoPos,
    Image,
    Reflectance,
    Deviation,
    Reliability,
    Amplitude,
    Red,
    Green,
    Blue,
    Infrared,
    Alpha,
    Flag,
    Mark,
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
            DimId::Dimension => "Dimension",
            DimId::StartPulse => "StartPulse",
            DimId::ReflectedPulse => "ReflectedPulse",
            DimId::Azimuth => "Azimuth",
            DimId::Pitch => "Pitch",
            DimId::Roll => "Roll",
            DimId::Pdop => "Pdop",
            DimId::PulseWidth => "PulseWidth",
            DimId::PassiveSignal => "PassiveSignal",
            DimId::PassiveX => "PassiveX",
            DimId::PassiveY => "PassiveY",
            DimId::PassiveZ => "PassiveZ",
            DimId::ReturnNumber => "ReturnNumber",
            DimId::NumberOfReturns => "NumberOfReturns",
            DimId::ScanDirectionFlag => "ScanDirectionFlag",
            DimId::EdgeOfFlightLine => "EdgeOfFlightLine",
            DimId::ScanAngleRank => "ScanAngleRank",
            DimId::PointSourceId => "PointSourceId",
            DimId::UserData => "UserData",
            DimId::EchoRange => "EchoRange",
            DimId::EchoNorm => "EchoNorm",
            DimId::EchoPos => "EchoPos",
            DimId::Image => "Image",
            DimId::Reflectance => "Reflectance",
            DimId::Deviation => "Deviation",
            DimId::Reliability => "Reliability",
            DimId::Amplitude => "Amplitude",
            DimId::Red => "Red",
            DimId::Green => "Green",
            DimId::Blue => "Blue",
            DimId::Infrared => "Infrared",
            DimId::Alpha => "Alpha",
            DimId::Flag => "Flag",
            DimId::Mark => "Mark",
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
            "Dimension" => DimId::Dimension,
            "StartPulse" => DimId::StartPulse,
            "ReflectedPulse" => DimId::ReflectedPulse,
            "Azimuth" => DimId::Azimuth,
            "Pitch" => DimId::Pitch,
            "Roll" => DimId::Roll,
            "Pdop" => DimId::Pdop,
            "PulseWidth" => DimId::PulseWidth,
            "PassiveSignal" => DimId::PassiveSignal,
            "PassiveX" => DimId::PassiveX,
            "PassiveY" => DimId::PassiveY,
            "PassiveZ" => DimId::PassiveZ,
            "ReturnNumber" => DimId::ReturnNumber,
            "NumberOfReturns" => DimId::NumberOfReturns,
            "ScanDirectionFlag" => DimId::ScanDirectionFlag,
            "EdgeOfFlightLine" => DimId::EdgeOfFlightLine,
            "ScanAngleRank" => DimId::ScanAngleRank,
            "PointSourceId" => DimId::PointSourceId,
            "UserData" | "Userdata" => DimId::UserData,
            "EchoRange" => DimId::EchoRange,
            "EchoNorm" => DimId::EchoNorm,
            "EchoPos" => DimId::EchoPos,
            "Image" => DimId::Image,
            "Reflectance" => DimId::Reflectance,
            "Deviation" => DimId::Deviation,
            "Reliability" => DimId::Reliability,
            "Amplitude" => DimId::Amplitude,
            "Red" => DimId::Red,
            "Green" => DimId::Green,
            "Blue" => DimId::Blue,
            "Infrared" => DimId::Infrared,
            "Alpha" => DimId::Alpha,
            "Flag" => DimId::Flag,
            "Mark" => DimId::Mark,
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

pub fn resolve_pdal_dimension_type(t1: u32, t2: u32) -> u32 {
    const NONE: u32 = 0x000;
    const SIGNED: u32 = 0x100;
    const UNSIGNED: u32 = 0x200;
    const FLOATING: u32 = 0x400;
    const SIGNED16: u32 = SIGNED | 2;
    const SIGNED32: u32 = SIGNED | 4;
    const SIGNED64: u32 = SIGNED | 8;
    const DOUBLE: u32 = FLOATING | 8;

    fn size(ty: u32) -> u32 {
        ty & 0xff
    }

    fn base(ty: u32) -> u32 {
        ty & 0xff00
    }

    if t1 == NONE && t2 != NONE {
        return t2;
    }
    if t2 == NONE && t1 != NONE {
        return t1;
    }
    if t1 == t2 {
        return t1;
    }
    if base(t1) == base(t2) {
        return t1.max(t2);
    }
    if base(t1) == FLOATING && base(t2) != FLOATING {
        return t1;
    }
    if base(t2) == FLOATING && base(t1) != FLOATING {
        return t2;
    }
    if base(t1) == UNSIGNED && size(t1) < size(t2) {
        return t2;
    }
    if base(t2) == UNSIGNED && size(t2) < size(t1) {
        return t1;
    }

    match size(t1).max(size(t2)) {
        1 => SIGNED16,
        2 => SIGNED32,
        4 => SIGNED64,
        _ => DOUBLE,
    }
}

pub fn pdal_dimension_interpretation_name(ty: u32) -> &'static str {
    match ty {
        0x000 => "unknown",
        0x101 => "int8_t",
        0x102 => "int16_t",
        0x104 => "int32_t",
        0x108 => "int64_t",
        0x201 => "uint8_t",
        0x202 => "uint16_t",
        0x204 => "uint32_t",
        0x208 => "uint64_t",
        0x404 => "float",
        0x408 => "double",
        _ => "unknown",
    }
}

pub fn pdal_dimension_type_from_name(name: &str) -> u32 {
    match name.to_ascii_lowercase().as_str() {
        "int8_t" | "int8" | "char" => 0x100 | 1,
        "int16_t" | "int16" | "short" => 0x100 | 2,
        "int32_t" | "int32" | "int" => 0x100 | 4,
        "int64_t" | "int64" | "long" => 0x100 | 8,
        "uint8_t" | "uint8" | "uchar" => 0x200 | 1,
        "uint16_t" | "uint16" | "ushort" => 0x200 | 2,
        "uint32_t" | "uint32" | "uint" => 0x200 | 4,
        "uint64_t" | "uint64" | "ulong" => 0x200 | 8,
        "float" | "float32" => 0x400 | 4,
        "double" | "float64" => 0x400 | 8,
        _ => 0x000,
    }
}

pub fn pdal_dimension_type_from_base_and_size(base: &str, size: usize) -> u32 {
    let base = match base {
        "signed" => 0x100,
        "unsigned" => 0x200,
        "floating" | "float" => 0x400,
        _ => return 0x000,
    };
    if !matches!(size, 1 | 2 | 4 | 8) {
        return 0x000;
    }
    if matches!(size, 1 | 2) && base == 0x400 {
        return 0x000;
    }

    base | size as u32
}

pub fn fix_dimension_name(name: &str) -> String {
    let mut output = name.to_string();
    if output.is_empty() {
        return output;
    }

    let mut chars: Vec<char> = output.chars().collect();
    if !chars[0].is_ascii_alphabetic() {
        chars[0] = '_';
    }
    for c in &mut chars {
        if !(c.is_ascii_alphabetic() || c.is_ascii_digit() || *c == '_' || *c == ' ') {
            *c = '_';
        }
    }
    output = chars.into_iter().collect();
    output
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
pub struct Triangle {
    pub a: PointId,
    pub b: PointId,
    pub c: PointId,
}

#[derive(Clone, Default)]
pub struct TriangularMesh {
    triangles: Vec<Triangle>,
}

impl TriangularMesh {
    pub fn add(&mut self, a: PointId, b: PointId, c: PointId) {
        self.triangles.push(Triangle { a, b, c });
    }

    pub fn len(&self) -> usize {
        self.triangles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    pub fn triangles(&self) -> &[Triangle] {
        &self.triangles
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds2D {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
}

impl Bounds2D {
    fn new(x: f64, y: f64) -> Self {
        Self {
            minx: x,
            maxx: x,
            miny: y,
            maxy: y,
        }
    }

    fn grow(&mut self, x: f64, y: f64) {
        self.minx = self.minx.min(x);
        self.maxx = self.maxx.max(x);
        self.miny = self.miny.min(y);
        self.maxy = self.maxy.max(y);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds3D {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
    pub minz: f64,
    pub maxz: f64,
}

impl Bounds3D {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            minx: x,
            maxx: x,
            miny: y,
            maxy: y,
            minz: z,
            maxz: z,
        }
    }

    fn grow(&mut self, x: f64, y: f64, z: f64) {
        self.minx = self.minx.min(x);
        self.maxx = self.maxx.max(x);
        self.miny = self.miny.min(y);
        self.maxy = self.maxy.max(y);
        self.minz = self.minz.min(z);
        self.maxz = self.maxz.max(z);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DimensionSummary {
    pub name: String,
    pub count: u64,
    pub minimum: f64,
    pub maximum: f64,
    pub mean: f64,
}

#[derive(Clone)]
pub struct PointView {
    layout: Rc<PointLayout>,
    data: Vec<u8>,
    source_indices: Vec<PointId>,
    spatial_reference: SpatialReference,
    meshes: BTreeMap<String, TriangularMesh>,
    rasters: Vec<RasterData>,
}

impl PointView {
    /// A new, empty view over the given layout.
    pub fn new(layout: Rc<PointLayout>) -> Self {
        PointView {
            layout,
            data: Vec::new(),
            source_indices: Vec::new(),
            spatial_reference: SpatialReference::default(),
            meshes: BTreeMap::new(),
            rasters: Vec::new(),
        }
    }

    /// A new empty view sharing this view's layout (PDAL's `makeNew`).
    pub fn make_new(&self) -> PointView {
        PointView {
            layout: Rc::clone(&self.layout),
            data: Vec::new(),
            source_indices: Vec::new(),
            spatial_reference: self.spatial_reference.clone(),
            meshes: BTreeMap::new(),
            rasters: Vec::new(),
        }
    }

    /// Return a copy of this view whose layout includes the requested
    /// dimensions. Existing values are copied through typed accessors.
    pub fn with_dimensions(&self, dims: &[(DimId, DimType)]) -> PointView {
        if dims.iter().all(|(dim, _)| self.layout.dim(dim).is_some()) {
            return self.clone();
        }

        let mut layout = (*self.layout).clone();
        for (dim, ty) in dims {
            layout.register(dim.clone(), *ty);
        }

        let mut output = PointView::new(Rc::new(layout));
        output.spatial_reference = self.spatial_reference.clone();
        output.meshes = self.meshes.clone();
        output.rasters = self.rasters.clone();

        for idx in 0..self.len() {
            let out_idx = output.add_point();
            for dim_idx in 0..self.layout.dim_count() {
                if let Some((dim, _)) = self.layout.dim_at(dim_idx) {
                    output.set_f64(out_idx, dim, self.get_f64(idx, dim));
                }
            }
            if let Some(source_index) = self.source_indices.get(idx as usize) {
                output.source_indices[out_idx as usize] = *source_index;
            }
        }

        output
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

    pub fn create_mesh(&mut self) -> &mut TriangularMesh {
        self.meshes.entry(String::new()).or_default()
    }

    pub fn create_named_mesh(&mut self, name: &str) -> Option<&mut TriangularMesh> {
        if self.meshes.contains_key(name) {
            return None;
        }
        self.meshes
            .insert(name.to_string(), TriangularMesh::default());
        self.meshes.get_mut(name)
    }

    pub fn mesh(&self) -> Option<&TriangularMesh> {
        self.mesh_named("")
    }

    pub fn mesh_named(&self, name: &str) -> Option<&TriangularMesh> {
        self.meshes.get(name).or_else(|| {
            name.is_empty()
                .then(|| self.meshes.values().next())
                .flatten()
        })
    }

    pub fn mesh_mut_named(&mut self, name: &str) -> Option<&mut TriangularMesh> {
        if self.meshes.contains_key(name) {
            return self.meshes.get_mut(name);
        }
        if name.is_empty() {
            let key = self.meshes.keys().next()?.clone();
            return self.meshes.get_mut(&key);
        }
        None
    }

    pub fn add_raster(&mut self, raster: RasterData) {
        self.rasters.push(raster);
    }

    pub fn create_raster(
        &mut self,
        name: &str,
        limits: RasterLimits,
        initializer: f64,
    ) -> Option<&mut RasterData> {
        if self.raster(name).is_some() {
            return None;
        }
        self.rasters
            .push(RasterData::new(name.to_string(), limits, initializer));
        self.rasters.last_mut()
    }

    pub fn raster(&self, name: &str) -> Option<&RasterData> {
        self.rasters
            .iter()
            .find(|raster| raster.name() == name)
            .or_else(|| name.is_empty().then(|| self.rasters.first()).flatten())
    }

    pub fn raster_mut(&mut self, name: &str) -> Option<&mut RasterData> {
        let idx = self
            .rasters
            .iter()
            .position(|raster| raster.name() == name)
            .or_else(|| {
                name.is_empty()
                    .then_some(0)
                    .filter(|_| !self.rasters.is_empty())
            })?;
        self.rasters.get_mut(idx)
    }

    pub fn rasters(&self) -> &[RasterData] {
        &self.rasters
    }

    pub fn calculate_bounds_2d(&self) -> Option<Bounds2D> {
        if self.is_empty()
            || self.layout.dim(&DimId::X).is_none()
            || self.layout.dim(&DimId::Y).is_none()
        {
            return None;
        }

        let mut bounds = Bounds2D::new(self.get_f64(0, &DimId::X), self.get_f64(0, &DimId::Y));
        for idx in 1..self.len() {
            bounds.grow(self.get_f64(idx, &DimId::X), self.get_f64(idx, &DimId::Y));
        }
        Some(bounds)
    }

    pub fn calculate_bounds_3d(&self) -> Option<Bounds3D> {
        if self.is_empty()
            || self.layout.dim(&DimId::X).is_none()
            || self.layout.dim(&DimId::Y).is_none()
            || self.layout.dim(&DimId::Z).is_none()
        {
            return None;
        }

        let mut bounds = Bounds3D::new(
            self.get_f64(0, &DimId::X),
            self.get_f64(0, &DimId::Y),
            self.get_f64(0, &DimId::Z),
        );
        for idx in 1..self.len() {
            bounds.grow(
                self.get_f64(idx, &DimId::X),
                self.get_f64(idx, &DimId::Y),
                self.get_f64(idx, &DimId::Z),
            );
        }
        Some(bounds)
    }

    pub fn summarize_dimension(&self, dim: &DimId) -> Option<DimensionSummary> {
        if self.is_empty() || self.layout.dim(dim).is_none() {
            return None;
        }

        let mut minimum = self.get_f64(0, dim);
        let mut maximum = minimum;
        let mut sum = minimum;
        for idx in 1..self.len() {
            let value = self.get_f64(idx, dim);
            minimum = minimum.min(value);
            maximum = maximum.max(value);
            sum += value;
        }

        Some(DimensionSummary {
            name: dim.name().to_string(),
            count: self.len(),
            minimum,
            maximum,
            mean: sum / self.len() as f64,
        })
    }

    pub fn summarize_dimensions(&self) -> Vec<DimensionSummary> {
        (0..self.layout.dim_count())
            .filter_map(|idx| {
                self.layout
                    .dim_at(idx)
                    .and_then(|(dim, _)| self.summarize_dimension(dim))
            })
            .collect()
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
mod tests;
