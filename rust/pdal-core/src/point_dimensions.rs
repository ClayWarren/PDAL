/// Identifier for a point dimension.
///
/// PDAL has a large fixed enum of well-known dimensions; the Rust port models
/// the ones currently needed, with `Other` as an escape hatch for the rest.
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
    Synthetic,
    KeyPoint,
    Withheld,
    Overlap,
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
            DimId::Synthetic => "Synthetic",
            DimId::KeyPoint => "KeyPoint",
            DimId::Withheld => "Withheld",
            DimId::Overlap => "Overlap",
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
            "X" | "x" => DimId::X,
            "Y" | "y" => DimId::Y,
            "Z" | "z" => DimId::Z,
            "Intensity" | "intensity" => DimId::Intensity,
            "OffsetTime" | "offsettime" => DimId::OffsetTime,
            "Classification" | "classification" => DimId::Classification,
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
            "GpsTime" | "gpstime" => DimId::GpsTime,
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
            "ReturnNumber" | "returnnumber" => DimId::ReturnNumber,
            "NumberOfReturns" | "numberofreturns" => DimId::NumberOfReturns,
            "ScanDirectionFlag" | "scandirectionflag" => DimId::ScanDirectionFlag,
            "EdgeOfFlightLine" | "edgeofflightline" => DimId::EdgeOfFlightLine,
            "Synthetic" | "synthetic" => DimId::Synthetic,
            "KeyPoint" | "keypoint" => DimId::KeyPoint,
            "Withheld" | "withheld" => DimId::Withheld,
            "Overlap" | "overlap" => DimId::Overlap,
            "ScanAngleRank" | "scananglerank" => DimId::ScanAngleRank,
            "PointSourceId" | "pointsourceid" => DimId::PointSourceId,
            "UserData" | "Userdata" | "userdata" => DimId::UserData,
            "EchoRange" => DimId::EchoRange,
            "EchoNorm" => DimId::EchoNorm,
            "EchoPos" => DimId::EchoPos,
            "Image" => DimId::Image,
            "Reflectance" => DimId::Reflectance,
            "Deviation" => DimId::Deviation,
            "Reliability" => DimId::Reliability,
            "Amplitude" => DimId::Amplitude,
            "Red" | "red" => DimId::Red,
            "Green" | "green" => DimId::Green,
            "Blue" | "blue" => DimId::Blue,
            "Infrared" | "infrared" => DimId::Infrared,
            "Alpha" | "alpha" => DimId::Alpha,
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
