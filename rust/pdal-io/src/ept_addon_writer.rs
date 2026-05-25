//! `writers.ept_addon` -- write addon dimension data alongside an existing
//! Entwine Point Tile (EPT) dataset.
//!
//! The C++ wrapper still owns the upstream `ept::Artifact` plumbing (it stays
//! in lockstep with the C++ `readers.ept` non-Rust path), but the actual file
//! I/O — binary chunk writes, hierarchy JSON, and addon metadata — happens
//! here. Each addon dimension is written with one call to [`write_addon`].
//!
//! The wire shape mirrors what `EptAddonWriter::writeOne` did in
//! `io/EptAddonWriter.cpp` before this port: per-tile binary buffers sized by
//! `overlap.m_count`, indexed into via `(EptNodeId - 1, EptPointId)`, then
//! emitted to `<addon_dir>/ept-data/<key>.bin`, with hierarchy JSON files
//! mirroring the source EPT hierarchy structure (subtrees split on
//! `hierarchy_step`).

use std::fs;
use std::path::{Path, PathBuf};

use pdal_core::point::{DimId, DimType, PointView};
use serde_json::{json, Map, Value};

/// One node in the source EPT hierarchy.
///
/// `count` and `node_id` are extracted from the C++ `ept::Overlap` so the
/// Rust writer can size per-tile buffers identically to the legacy path. The
/// `node_id` is 1-based; node 0 indicates a point that does not come from the
/// EPT reader and is dropped by the writer.
#[derive(Debug, Clone, Copy)]
pub struct AddonOverlap {
    pub depth: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub count: u64,
    pub node_id: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct AddonRootBounds {
    pub minx: f64,
    pub miny: f64,
    pub minz: f64,
    pub maxx: f64,
    pub maxy: f64,
    pub maxz: f64,
}

/// Bundles all inputs for one addon dimension's write call.
pub struct AddonWriteRequest<'a> {
    pub view: &'a PointView,
    pub node_id_dim: &'a str,
    pub point_id_dim: &'a str,
    pub source_dim: &'a str,
    pub addon_file: &'a str,
    pub addon_type: DimType,
    pub hierarchy_step: u64,
    pub root_bounds: AddonRootBounds,
    pub overlaps: &'a [AddonOverlap],
}

/// Write one addon dimension's worth of data: per-tile binary chunks, the
/// hierarchy JSON tree, and the top-level `ept-addon.json` metadata file.
///
/// `view` provides `EptNodeId`/`EptPointId`/the source dimension. `addon_file`
/// points at the top-level `ept-addon.json` (e.g. `<addon>/ept-addon.json`);
/// the binary chunks land next to it under `ept-data/`, the hierarchy under
/// `ept-hierarchy/`.
pub fn write_addon(req: AddonWriteRequest<'_>) -> Result<(), String> {
    let AddonWriteRequest {
        view,
        node_id_dim,
        point_id_dim,
        source_dim,
        addon_file,
        addon_type,
        hierarchy_step,
        root_bounds,
        overlaps,
    } = req;
    let node_id = DimId::from_name(node_id_dim);
    let point_id = DimId::from_name(point_id_dim);
    let source = DimId::from_name(source_dim);

    let item_size = addon_type.size();
    let path = Path::new(addon_file);
    let addon_dir = path
        .parent()
        .ok_or_else(|| format!("Invalid addon file path '{addon_file}'"))?;

    // Allocate per-overlap buffers (indexed by node_id - 1) the same way the
    // C++ writer did. This keeps point-to-slot indexing simple.
    let max_node_id = overlaps.iter().map(|o| o.node_id).max().unwrap_or(0);
    let mut buffers: Vec<Vec<u8>> = vec![Vec::new(); max_node_id as usize];
    for overlap in overlaps {
        if overlap.node_id == 0 || overlap.count == 0 {
            continue;
        }
        let idx = (overlap.node_id - 1) as usize;
        buffers[idx] = vec![0u8; overlap.count as usize * item_size];
    }

    // Fill buffers from the view. Points with node_id == 0 are dropped (they
    // did not come from the EPT reader so they have nowhere to land).
    for i in 0..view.len() {
        let n = view.get_f64(i, &node_id) as u64;
        if n == 0 {
            continue;
        }
        let pid = view.get_f64(i, &point_id) as u64;
        let buf = buffers
            .get_mut((n - 1) as usize)
            .ok_or_else(|| format!("EptNodeId {n} out of range"))?;
        let off = pid as usize * item_size;
        if off + item_size > buf.len() {
            return Err(format!(
                "EptPointId {pid} out of range for tile {n} (buffer size {})",
                buf.len()
            ));
        }
        let value = view.get_f64(i, &source);
        write_dim_value(value, addon_type, &mut buf[off..off + item_size])?;
    }

    // Write binary chunks.
    let data_dir: PathBuf = addon_dir.join("ept-data");
    fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create '{}': {e}", data_dir.display()))?;
    for overlap in overlaps {
        if overlap.node_id == 0 {
            continue;
        }
        let buf = &buffers[(overlap.node_id - 1) as usize];
        let filename = data_dir.join(format!("{}.bin", key_to_string(overlap)));
        fs::write(&filename, buf)
            .map_err(|e| format!("Failed to write '{}': {e}", filename.display()))?;
    }

    // Write hierarchy JSON.
    let hier_dir = addon_dir.join("ept-hierarchy");
    fs::create_dir_all(&hier_dir)
        .map_err(|e| format!("Failed to create '{}': {e}", hier_dir.display()))?;

    let by_key: std::collections::HashMap<KeyId, &AddonOverlap> = overlaps
        .iter()
        .map(|o| (KeyId::new(o.depth, o.x, o.y, o.z), o))
        .collect();

    // Root key is depth 0 covering root_bounds (we only need depth here; the
    // bounds are recorded in the source EPT's ept.json, not the addon's).
    let _ = root_bounds; // bounds are informational; the hierarchy keys carry tree position
    let mut root_map: Map<String, Value> = Map::new();
    let root_key = KeyId::new(0, 0, 0, 0);
    write_hierarchy_node(
        &by_key,
        hierarchy_step,
        &hier_dir,
        &mut root_map,
        root_key,
    )?;
    let root_filename = hier_dir.join(format!("{}.json", root_key));
    fs::write(&root_filename, Value::Object(root_map).to_string())
        .map_err(|e| format!("Failed to write '{}': {e}", root_filename.display()))?;

    // Write the top-level ept-addon.json metadata file (matches what C++
    // EptAddonWriter writes; the addon storage method also wrote a sibling
    // metadata file, but the C++ writer overwrites it here so we match).
    let meta = json!({
        "type": dim_type_name(addon_type),
        "size": item_size,
        "version": "1.0.0",
        "dataType": "binary",
    });
    fs::write(addon_file, meta.to_string())
        .map_err(|e| format!("Failed to write '{addon_file}': {e}"))?;

    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct KeyId {
    depth: i32,
    x: i32,
    y: i32,
    z: i32,
}

impl KeyId {
    fn new(depth: i32, x: i32, y: i32, z: i32) -> Self {
        Self { depth, x, y, z }
    }
    fn bisect(&self, dir: u32) -> Self {
        // Mirror ept::Key::bisect: child's depth is parent + 1, and the child's
        // (x,y,z) is parent shifted left by one with `dir` bits OR'd in.
        let nx = (self.x << 1) | ((dir & 1) as i32);
        let ny = (self.y << 1) | (((dir >> 1) & 1) as i32);
        let nz = (self.z << 1) | (((dir >> 2) & 1) as i32);
        Self {
            depth: self.depth + 1,
            x: nx,
            y: ny,
            z: nz,
        }
    }
}

impl std::fmt::Display for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}-{}-{}", self.depth, self.x, self.y, self.z)
    }
}

fn key_to_string(overlap: &AddonOverlap) -> String {
    format!("{}-{}-{}-{}", overlap.depth, overlap.x, overlap.y, overlap.z)
}

fn write_hierarchy_node(
    by_key: &std::collections::HashMap<KeyId, &AddonOverlap>,
    step: u64,
    hier_dir: &Path,
    curr: &mut Map<String, Value>,
    key: KeyId,
) -> Result<(), String> {
    let Some(overlap) = by_key.get(&key) else {
        return Ok(());
    };
    if overlap.count == 0 {
        return Ok(());
    }
    let key_name = key.to_string();
    let is_root = key.depth == 0;
    if step != 0 && !is_root && (key.depth as u64).is_multiple_of(step) {
        // Split point: write current subtree to its own file and place -1 in
        // the parent file to indicate "see this subtree file".
        curr.insert(key_name.clone(), Value::from(-1i64));
        let mut next: Map<String, Value> = Map::new();
        next.insert(key_name.clone(), Value::from(overlap.count));
        for dir in 0..8 {
            write_hierarchy_node(by_key, step, hier_dir, &mut next, key.bisect(dir))?;
        }
        let filename = hier_dir.join(format!("{key_name}.json"));
        fs::write(&filename, Value::Object(next).to_string())
            .map_err(|e| format!("Failed to write '{}': {e}", filename.display()))?;
    } else {
        curr.insert(key_name, Value::from(overlap.count));
        for dir in 0..8 {
            write_hierarchy_node(by_key, step, hier_dir, curr, key.bisect(dir))?;
        }
    }
    Ok(())
}

fn dim_type_name(ty: DimType) -> &'static str {
    match ty {
        DimType::U8 | DimType::U16 | DimType::U32 | DimType::U64 => "unsigned",
        DimType::I8 | DimType::I16 | DimType::I32 | DimType::I64 => "signed",
        DimType::F32 | DimType::F64 => "float",
    }
}

fn write_dim_value(value: f64, ty: DimType, dst: &mut [u8]) -> Result<(), String> {
    match ty {
        DimType::U8 => dst.copy_from_slice(&(value as u8).to_le_bytes()),
        DimType::U16 => dst.copy_from_slice(&(value as u16).to_le_bytes()),
        DimType::U32 => dst.copy_from_slice(&(value as u32).to_le_bytes()),
        DimType::U64 => dst.copy_from_slice(&(value as u64).to_le_bytes()),
        DimType::I8 => dst.copy_from_slice(&(value as i8).to_le_bytes()),
        DimType::I16 => dst.copy_from_slice(&(value as i16).to_le_bytes()),
        DimType::I32 => dst.copy_from_slice(&(value as i32).to_le_bytes()),
        DimType::I64 => dst.copy_from_slice(&(value as i64).to_le_bytes()),
        DimType::F32 => dst.copy_from_slice(&(value as f32).to_le_bytes()),
        DimType::F64 => dst.copy_from_slice(&value.to_le_bytes()),
    }
    Ok(())
}
