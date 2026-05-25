//! Minimal COPC hierarchy walker for `preview()` parity.
//!
//! Parses the LAS 1.4 header, the first VLR (mandated COPC info), and the
//! hierarchy page tree stored in the EVLR region. Returns total point count
//! and dataset-coordinate bounding box after applying optional 2D/3D bounds
//! and resolution pruning. Match the C++ `CopcReader::inspect()` depth math:
//! `depthEnd = max(1, ceil(log2(spacing/resolution)) + 1)`.

use std::io::{Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};
use pdal_core::bounds::{Bounds2D, Bounds3D};

const LAS_SIGNATURE: [u8; 4] = *b"LASF";
const LAS14_HEADER_SIZE: u64 = 375;
const VLR_HEADER_SIZE: u64 = 54;
const COPC_INFO_RECORD_ID: u16 = 1;
const COPC_INFO_PAYLOAD_SIZE: usize = 160;
const HIERARCHY_ENTRY_SIZE: usize = 32;

#[derive(Debug, Clone, Copy)]
pub struct CopcInfo {
    pub center_x: f64,
    pub center_y: f64,
    pub center_z: f64,
    pub halfsize: f64,
    pub spacing: f64,
    pub root_hier_offset: u64,
    pub root_hier_size: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct LasBounds {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub min_z: f64,
    pub max_z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoxelKey {
    pub level: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct HierarchyEntry {
    pub key: VoxelKey,
    pub offset: u64,
    pub byte_size: i32,
    pub point_count: i32,
}

pub enum QueryBounds {
    Two(Bounds2D),
    Three(Bounds3D),
}

pub struct CopcPreview {
    pub point_count: u64,
    pub bounds: LasBounds,
}

pub fn read_copc_info<R: Read + Seek + ?Sized>(
    reader: &mut R,
) -> Result<(CopcInfo, LasBounds), String> {
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|e| format!("COPC: seek start failed: {e}"))?;
    let mut sig = [0u8; 4];
    reader
        .read_exact(&mut sig)
        .map_err(|e| format!("COPC: read signature failed: {e}"))?;
    if sig != LAS_SIGNATURE {
        return Err("COPC: missing LASF signature".to_string());
    }

    // Bounds live at offsets 179..227 of the LAS header.
    reader
        .seek(SeekFrom::Start(179))
        .map_err(|e| format!("COPC: seek bounds failed: {e}"))?;
    let max_x = read_f64(reader)?;
    let min_x = read_f64(reader)?;
    let max_y = read_f64(reader)?;
    let min_y = read_f64(reader)?;
    let max_z = read_f64(reader)?;
    let min_z = read_f64(reader)?;

    // COPC info VLR header is the first VLR, immediately after the 375-byte
    // LAS 1.4 header. Skip the VLR header (54 bytes) and read the 160-byte
    // info payload directly.
    let info_offset = LAS14_HEADER_SIZE + VLR_HEADER_SIZE;
    reader
        .seek(SeekFrom::Start(info_offset))
        .map_err(|e| format!("COPC: seek info VLR failed: {e}"))?;

    let center_x = read_f64(reader)?;
    let center_y = read_f64(reader)?;
    let center_z = read_f64(reader)?;
    let halfsize = read_f64(reader)?;
    let spacing = read_f64(reader)?;
    let root_hier_offset = reader
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("COPC: read root_hier_offset failed: {e}"))?;
    let root_hier_size = reader
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("COPC: read root_hier_size failed: {e}"))?;

    // Validate by also checking the VLR header points at COPC info.
    reader
        .seek(SeekFrom::Start(LAS14_HEADER_SIZE))
        .map_err(|e| format!("COPC: seek VLR header failed: {e}"))?;
    let mut vlr_header = [0u8; VLR_HEADER_SIZE as usize];
    reader
        .read_exact(&mut vlr_header)
        .map_err(|e| format!("COPC: read VLR header failed: {e}"))?;
    let user_id = std::str::from_utf8(&vlr_header[2..18])
        .unwrap_or("")
        .trim_end_matches(char::from(0));
    let record_id = u16::from_le_bytes([vlr_header[18], vlr_header[19]]);
    let record_length = u16::from_le_bytes([vlr_header[20], vlr_header[21]]);
    if !user_id.starts_with("copc") || record_id != COPC_INFO_RECORD_ID {
        return Err(format!(
            "COPC: first VLR is not COPC info (user_id={user_id:?}, record_id={record_id})"
        ));
    }
    if (record_length as usize) < COPC_INFO_PAYLOAD_SIZE {
        return Err(format!(
            "COPC: info VLR payload too small ({record_length} < {COPC_INFO_PAYLOAD_SIZE})"
        ));
    }

    Ok((
        CopcInfo {
            center_x,
            center_y,
            center_z,
            halfsize,
            spacing,
            root_hier_offset,
            root_hier_size,
        },
        LasBounds {
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        },
    ))
}

fn read_hierarchy_page<R: Read + Seek + ?Sized>(
    reader: &mut R,
    offset: u64,
    byte_size: u64,
) -> Result<Vec<HierarchyEntry>, String> {
    let entries = (byte_size as usize) / HIERARCHY_ENTRY_SIZE;
    let mut buf = vec![0u8; byte_size as usize];
    reader
        .seek(SeekFrom::Start(offset))
        .map_err(|e| format!("COPC: seek hierarchy page failed: {e}"))?;
    reader
        .read_exact(&mut buf)
        .map_err(|e| format!("COPC: read hierarchy page failed: {e}"))?;

    let mut out = Vec::with_capacity(entries);
    let mut cursor = std::io::Cursor::new(&buf[..]);
    for _ in 0..entries {
        let level = cursor
            .read_i32::<LittleEndian>()
            .map_err(|e| format!("COPC: read entry level failed: {e}"))?;
        let x = cursor
            .read_i32::<LittleEndian>()
            .map_err(|e| format!("COPC: read entry x failed: {e}"))?;
        let y = cursor
            .read_i32::<LittleEndian>()
            .map_err(|e| format!("COPC: read entry y failed: {e}"))?;
        let z = cursor
            .read_i32::<LittleEndian>()
            .map_err(|e| format!("COPC: read entry z failed: {e}"))?;
        let entry_offset = cursor
            .read_u64::<LittleEndian>()
            .map_err(|e| format!("COPC: read entry offset failed: {e}"))?;
        let entry_size = cursor
            .read_i32::<LittleEndian>()
            .map_err(|e| format!("COPC: read entry size failed: {e}"))?;
        let point_count = cursor
            .read_i32::<LittleEndian>()
            .map_err(|e| format!("COPC: read entry point_count failed: {e}"))?;
        out.push(HierarchyEntry {
            key: VoxelKey { level, x, y, z },
            offset: entry_offset,
            byte_size: entry_size,
            point_count,
        });
    }
    Ok(out)
}

/// Compute the dataset-coordinate bbox for a voxel key relative to the COPC
/// octree root at `info.center` with extent `info.halfsize`.
pub fn voxel_bounds(info: &CopcInfo, key: &VoxelKey) -> [f64; 6] {
    let level = key.level.max(0) as u32;
    let cells = 1u64 << level;
    let extent = info.halfsize * 2.0;
    let cell = extent / cells as f64;
    let origin_x = info.center_x - info.halfsize;
    let origin_y = info.center_y - info.halfsize;
    let origin_z = info.center_z - info.halfsize;
    let min_x = origin_x + key.x as f64 * cell;
    let min_y = origin_y + key.y as f64 * cell;
    let min_z = origin_z + key.z as f64 * cell;
    [
        min_x,
        min_y,
        min_z,
        min_x + cell,
        min_y + cell,
        min_z + cell,
    ]
}

fn key_intersects(bbox: &[f64; 6], bounds: &QueryBounds) -> bool {
    match bounds {
        QueryBounds::Two(b) => {
            !(bbox[3] < b.minx || bbox[0] > b.maxx || bbox[4] < b.miny || bbox[1] > b.maxy)
        }
        QueryBounds::Three(b) => {
            !(bbox[3] < b.minx
                || bbox[0] > b.maxx
                || bbox[4] < b.miny
                || bbox[1] > b.maxy
                || bbox[5] < b.minz
                || bbox[2] > b.maxz)
        }
    }
}

fn intersect_bounds(las: LasBounds, bounds: &QueryBounds) -> LasBounds {
    match bounds {
        QueryBounds::Two(b) => LasBounds {
            min_x: las.min_x.max(b.minx),
            max_x: las.max_x.min(b.maxx),
            min_y: las.min_y.max(b.miny),
            max_y: las.max_y.min(b.maxy),
            min_z: las.min_z,
            max_z: las.max_z,
        },
        QueryBounds::Three(b) => LasBounds {
            min_x: las.min_x.max(b.minx),
            max_x: las.max_x.min(b.maxx),
            min_y: las.min_y.max(b.miny),
            max_y: las.max_y.min(b.maxy),
            min_z: las.min_z.max(b.minz),
            max_z: las.max_z.min(b.maxz),
        },
    }
}

/// C++ parity: `depthEnd = max(1, ceil(log2(spacing/resolution)) + 1)`.
/// A node at `level >= depthEnd` is pruned.
pub fn depth_end(spacing: f64, resolution: f64) -> Option<i32> {
    if resolution <= 0.0 {
        return None;
    }
    let depth = (1.0_f64).max(((spacing / resolution).log2()).ceil() + 1.0) as i32;
    Some(depth)
}

pub fn walk_preview<R: Read + Seek + ?Sized>(
    reader: &mut R,
    info: &CopcInfo,
    full_bounds: LasBounds,
    query_bounds: Option<&QueryBounds>,
    resolution: f64,
) -> Result<CopcPreview, String> {
    let depth_limit = depth_end(info.spacing, resolution);
    let mut total: u64 = 0;
    let mut pending: Vec<(u64, u64)> = vec![(info.root_hier_offset, info.root_hier_size)];
    while let Some((offset, size)) = pending.pop() {
        if size == 0 {
            continue;
        }
        let entries = read_hierarchy_page(reader, offset, size)?;
        for entry in entries {
            if let Some(limit) = depth_limit {
                if entry.key.level >= limit {
                    continue;
                }
            }
            let bbox = voxel_bounds(info, &entry.key);
            if let Some(qb) = query_bounds {
                if !key_intersects(&bbox, qb) {
                    continue;
                }
            }
            if entry.point_count < 0 {
                // Sub-hierarchy page pointer.
                pending.push((entry.offset, entry.byte_size as u64));
            } else {
                total = total.saturating_add(entry.point_count as u64);
            }
        }
    }
    let out_bounds = match query_bounds {
        Some(b) => intersect_bounds(full_bounds, b),
        None => full_bounds,
    };
    Ok(CopcPreview {
        point_count: total,
        bounds: out_bounds,
    })
}

fn read_f64<R: Read + ?Sized>(reader: &mut R) -> Result<f64, String> {
    reader
        .read_f64::<LittleEndian>()
        .map_err(|e| format!("COPC: f64 read failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::{Path, PathBuf};

    fn data_path(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("test/data")
            .join(path)
    }

    #[test]
    fn reads_lone_star_info_and_full_preview() {
        let file = File::open(data_path("copc/lone-star.copc.laz")).unwrap();
        let mut reader = BufReader::new(file);
        let (info, bounds) = read_copc_info(&mut reader).unwrap();
        assert!(info.spacing > 0.0);
        assert!(info.halfsize > 0.0);
        let preview = walk_preview(&mut reader, &info, bounds, None, 0.0).unwrap();
        assert_eq!(preview.point_count, 518_862);
    }

    #[test]
    fn depth_end_matches_cpp_formula() {
        // spacing=300, resolution=1000 -> log2(0.3) ≈ -1.737 -> ceil = -1 -> +1 = 0 -> max(1, 0) = 1
        assert_eq!(depth_end(300.0, 1000.0), Some(1));
        // spacing=1000, resolution=1000 -> log2(1) = 0 -> ceil + 1 = 1 -> max(1,1) = 1
        assert_eq!(depth_end(1000.0, 1000.0), Some(1));
        // spacing=1000, resolution=10 -> log2(100) ≈ 6.64 -> ceil = 7 -> +1 = 8
        assert_eq!(depth_end(1000.0, 10.0), Some(8));
        // resolution=0 -> no limit
        assert_eq!(depth_end(1000.0, 0.0), None);
    }
}
