//! EPT JSON/location helpers and binary-tile decoding.
//! Split out of `ept.rs` to keep modules under ~1k LOC. These are leaf
//! helpers the reader calls by function; the stage types/filters that the
//! reader constructs by literal stay in the parent module.

use super::*;

pub(super) fn read_json(path: &Path) -> Result<Value, StageError> {
    let location = path.to_string_lossy();
    let text = crate::source::read_to_string(&location)
        .map_err(|err| StageError(format!("Can't open EPT file '{}': {err}", path.display())))?;
    serde_json::from_str(&text).map_err(|err| {
        StageError(format!(
            "EPT file '{}' is not valid JSON: {err}",
            path.display()
        ))
    })
}

pub(super) fn read_json_location(location: &str) -> Result<Value, StageError> {
    let text = crate::source::read_to_string(location)
        .map_err(|err| StageError(format!("Can't open EPT file '{location}': {err}")))?;
    serde_json::from_str(&text)
        .map_err(|err| StageError(format!("EPT file '{location}' is not valid JSON: {err}")))
}

pub(super) fn location_parent(location: &str) -> String {
    location
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

pub(super) fn addon_metadata_path(path: &str) -> String {
    if path.ends_with("ept-addon.json") {
        path.to_string()
    } else {
        join_location(path, "ept-addon.json")
    }
}

pub(super) fn join_location(base: &str, relative: &str) -> String {
    if is_remote_location(relative) || Path::new(relative).is_absolute() {
        relative.to_string()
    } else {
        format!(
            "{}/{}",
            base.trim_end_matches('/'),
            relative.trim_start_matches("./")
        )
    }
}

pub(super) fn is_remote_location(value: &str) -> bool {
    value.contains("://")
}

#[derive(Clone)]
pub(super) struct EptSchema {
    pub(super) entries: Vec<SchemaEntry>,
    pub(super) point_size: usize,
    pub(super) layout: Rc<PointLayout>,
}

#[derive(Clone)]
pub(super) struct SchemaEntry {
    dim: DimId,
    ty: DimType,
    size: usize,
    scale: f64,
    offset: f64,
}

impl EptSchema {
    pub(super) fn parse(info: &Value) -> Result<Self, StageError> {
        let schema = info["schema"]
            .as_array()
            .ok_or_else(|| StageError("EPT file is missing schema.".to_string()))?;
        let mut entries = Vec::with_capacity(schema.len());
        let mut layout = PointLayout::new();
        let mut point_size = 0;
        for item in schema {
            let name = item["name"]
                .as_str()
                .ok_or_else(|| StageError("EPT schema entry is missing name.".to_string()))?;
            let kind = item["type"]
                .as_str()
                .ok_or_else(|| StageError(format!("EPT schema '{name}' is missing type.")))?;
            let size = item["size"]
                .as_u64()
                .ok_or_else(|| StageError(format!("EPT schema '{name}' is missing size.")))?
                as usize;
            let storage_ty = dim_type(kind, size)?;
            let scale = item["scale"].as_f64().unwrap_or(1.0);
            let offset = item["offset"].as_f64().unwrap_or(0.0);
            let ty = if scale != 1.0 || offset != 0.0 {
                DimType::F64
            } else {
                storage_ty
            };
            let dim = DimId::from_name(name);
            layout.register(dim.clone(), ty);
            entries.push(SchemaEntry {
                dim,
                ty: storage_ty,
                size,
                scale,
                offset,
            });
            point_size += size;
        }
        Ok(Self {
            entries,
            point_size,
            layout: Rc::new(layout),
        })
    }
}

pub(super) fn dim_type(kind: &str, size: usize) -> Result<DimType, StageError> {
    match (kind, size) {
        ("unsigned", 1) => Ok(DimType::U8),
        ("unsigned", 2) => Ok(DimType::U16),
        ("unsigned", 4) => Ok(DimType::U32),
        ("unsigned", 8) => Ok(DimType::U64),
        ("signed", 1) => Ok(DimType::I8),
        ("signed", 2) => Ok(DimType::I16),
        ("signed", 4) => Ok(DimType::I32),
        ("signed", 8) => Ok(DimType::I64),
        ("float", 4) => Ok(DimType::F32),
        ("float", 8) => Ok(DimType::F64),
        _ => Err(StageError(format!(
            "Unsupported EPT schema type '{kind}' with size {size}."
        ))),
    }
}

pub(super) fn read_binary_tile(
    path: &Path,
    schema: &EptSchema,
    srs: &str,
) -> Result<PointView, StageError> {
    let location = path.to_string_lossy();
    let bytes = crate::source::read_bytes(&location)
        .map_err(|err| StageError(format!("Can't open EPT tile '{}': {err}", path.display())))?;
    view_from_binary_tile(path, bytes, schema, srs)
}

pub(super) fn read_zstandard_tile(
    path: &Path,
    schema: &EptSchema,
    srs: &str,
) -> Result<PointView, StageError> {
    let location = path.to_string_lossy();
    let bytes = crate::source::read_bytes(&location)
        .map_err(|err| StageError(format!("Can't open EPT tile '{}': {err}", path.display())))?;
    let decoded = zstd::stream::decode_all(Cursor::new(bytes)).map_err(|err| {
        StageError(format!(
            "Can't decompress EPT tile '{}': {err}",
            path.display()
        ))
    })?;
    view_from_binary_tile(path, decoded, schema, srs)
}

pub(super) fn view_from_binary_tile(
    path: &Path,
    bytes: Vec<u8>,
    schema: &EptSchema,
    srs: &str,
) -> Result<PointView, StageError> {
    if schema.point_size == 0 || !bytes.len().is_multiple_of(schema.point_size) {
        return Err(StageError(format!(
            "EPT tile '{}' size does not match schema.",
            path.display()
        )));
    }

    let mut view = PointView::new(Rc::clone(&schema.layout));
    if !srs.is_empty() {
        view.set_spatial_reference(SpatialReference::new(srs));
    }
    for record in bytes.chunks_exact(schema.point_size) {
        let point = view.add_point();
        let mut offset = 0;
        for entry in &schema.entries {
            let raw = read_binary_value(&record[offset..offset + entry.size], entry.ty);
            let mut scaled = raw * entry.scale + entry.offset;
            // Match C++ behavior: EptInfo stores dims at their storage type, and
            // TileContents::transform() writes XYZ back through the storage type
            // (I32 etc.), truncating fractional bits. The Rust reader must
            // replicate this truncation to match C++ dimension values.
            if entry.scale != 1.0 || entry.offset != 0.0 {
                scaled = truncate_storage(scaled, entry.ty);
            }
            view.set_f64(point, &entry.dim, scaled);
            offset += entry.size;
        }
    }
    Ok(view)
}

/// Symmetric round half away from zero, matching C++ `Utils::sround`.
pub(super) fn sround(v: f64) -> f64 {
    if v > 0.0 {
        (v + 0.5).floor()
    } else {
        (v - 0.5).ceil()
    }
}

/// Emulate C++ `Utils::numericCast<double, T>` which first rounds via
/// `sround` for integer storage types, then static_casts to T.
pub(super) fn truncate_storage(v: f64, ty: DimType) -> f64 {
    match ty {
        DimType::U8 => (sround(v) as u8) as f64,
        DimType::I8 => (sround(v) as i8) as f64,
        DimType::U16 => (sround(v) as u16) as f64,
        DimType::I16 => (sround(v) as i16) as f64,
        DimType::U32 => (sround(v) as u32) as f64,
        DimType::I32 => (sround(v) as i32) as f64,
        DimType::U64 => (sround(v) as u64) as f64,
        DimType::I64 => (sround(v) as i64) as f64,
        DimType::F32 => (v as f32) as f64,
        DimType::F64 => v,
    }
}

pub(super) fn read_binary_value(bytes: &[u8], ty: DimType) -> f64 {
    match ty {
        DimType::U8 => f64::from(bytes[0]),
        DimType::I8 => f64::from(bytes[0] as i8),
        DimType::U16 => f64::from(u16::from_le_bytes(bytes.try_into().unwrap())),
        DimType::I16 => f64::from(i16::from_le_bytes(bytes.try_into().unwrap())),
        DimType::U32 => u32::from_le_bytes(bytes.try_into().unwrap()) as f64,
        DimType::I32 => i32::from_le_bytes(bytes.try_into().unwrap()) as f64,
        DimType::U64 => u64::from_le_bytes(bytes.try_into().unwrap()) as f64,
        DimType::I64 => i64::from_le_bytes(bytes.try_into().unwrap()) as f64,
        DimType::F32 => f64::from(f32::from_le_bytes(bytes.try_into().unwrap())),
        DimType::F64 => f64::from_le_bytes(bytes.try_into().unwrap()),
    }
}

pub(super) fn apply_addons(
    view: &mut PointView,
    addons: &[EptAddon],
    key: &str,
) -> Result<(), StageError> {
    for addon in addons {
        let path = addon.data_path(key);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(StageError(format!(
                    "Can't open EPT addon data '{}': {err}",
                    path
                )))
            }
        };
        let expected_len = view.len() as usize * addon.size;
        if bytes.len() != expected_len {
            return Err(StageError(format!(
                "EPT addon data '{}' has {} bytes but expected {}.",
                path,
                bytes.len(),
                expected_len
            )));
        }
        for (idx, record) in bytes.chunks_exact(addon.size).enumerate() {
            view.set_f64(idx as u64, &addon.dim, read_binary_value(record, addon.ty));
        }
    }
    Ok(())
}
