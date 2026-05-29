//! Byte-exact COPC output structures: the `copc` info VLR payload and the
//! hierarchy-page entries.
//!
//! These mirror what `pdal-io::copc_hierarchy` reads back (and the LAZ/COPC
//! 1.0 spec): the info VLR is a fixed 160-byte payload, and each hierarchy
//! entry is 32 bytes (`level, x, y, z` as i32, `offset` u64, `byte_size` i32,
//! `point_count` i32). Higher-level Output assembly (LAS header, LAZ chunk
//! encoding, file layout) builds on these.

use byteorder::{LittleEndian, WriteBytesExt};

use super::voxel_key::VoxelKey;

/// Fixed size of the `copc` info VLR payload.
pub const COPC_INFO_PAYLOAD_SIZE: usize = 160;
/// Fixed size of one hierarchy-page entry.
pub const HIERARCHY_ENTRY_SIZE: usize = 32;

/// The fields of the COPC `info` VLR (record id 1, user id "copc").
#[derive(Clone, Copy, Debug, Default)]
pub struct CopcInfo {
    pub center_x: f64,
    pub center_y: f64,
    pub center_z: f64,
    pub halfsize: f64,
    pub spacing: f64,
    pub root_hier_offset: u64,
    pub root_hier_size: u64,
    pub gpstime_minimum: f64,
    pub gpstime_maximum: f64,
}

impl CopcInfo {
    /// Serialize the 160-byte info VLR payload (trailing 11 reserved u64s are
    /// zero), matching the COPC 1.0 layout the reader parses.
    pub fn to_bytes(&self) -> [u8; COPC_INFO_PAYLOAD_SIZE] {
        let mut buf = Vec::with_capacity(COPC_INFO_PAYLOAD_SIZE);
        buf.write_f64::<LittleEndian>(self.center_x).unwrap();
        buf.write_f64::<LittleEndian>(self.center_y).unwrap();
        buf.write_f64::<LittleEndian>(self.center_z).unwrap();
        buf.write_f64::<LittleEndian>(self.halfsize).unwrap();
        buf.write_f64::<LittleEndian>(self.spacing).unwrap();
        buf.write_u64::<LittleEndian>(self.root_hier_offset)
            .unwrap();
        buf.write_u64::<LittleEndian>(self.root_hier_size).unwrap();
        buf.write_f64::<LittleEndian>(self.gpstime_minimum).unwrap();
        buf.write_f64::<LittleEndian>(self.gpstime_maximum).unwrap();
        // 11 reserved u64 fields (zero).
        for _ in 0..11 {
            buf.write_u64::<LittleEndian>(0).unwrap();
        }
        let mut out = [0u8; COPC_INFO_PAYLOAD_SIZE];
        out.copy_from_slice(&buf);
        out
    }
}

/// One hierarchy-page entry: a node key, its chunk's file offset, compressed
/// byte size, and point count. A `byte_size`/`point_count` of 0 is a valid
/// empty node; a negative `byte_size` (written as a sub-page offset by the
/// hierarchy emitter) is also representable.
#[derive(Clone, Copy, Debug)]
pub struct HierarchyEntry {
    pub key: VoxelKey,
    pub offset: u64,
    pub byte_size: i32,
    pub point_count: i32,
}

impl HierarchyEntry {
    /// Serialize the 32-byte entry, matching `copc_hierarchy::read_hierarchy_page`.
    pub fn write_to(&self, buf: &mut Vec<u8>) {
        buf.write_i32::<LittleEndian>(self.key.level()).unwrap();
        buf.write_i32::<LittleEndian>(self.key.x()).unwrap();
        buf.write_i32::<LittleEndian>(self.key.y()).unwrap();
        buf.write_i32::<LittleEndian>(self.key.z()).unwrap();
        buf.write_u64::<LittleEndian>(self.offset).unwrap();
        buf.write_i32::<LittleEndian>(self.byte_size).unwrap();
        buf.write_i32::<LittleEndian>(self.point_count).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copc_hierarchy;
    use std::io::Cursor;

    // LAS 1.4 header size and VLR header size, matching the reader.
    const LAS14_HEADER_SIZE: usize = 375;
    const VLR_HEADER_SIZE: usize = 54;

    /// Build a minimal buffer the COPC reader's `read_copc_info` accepts: LASF
    /// signature, header bounds, a "copc" record-1 VLR header, and the payload.
    fn minimal_copc_buffer(info: &CopcInfo, bounds: [f64; 6]) -> Vec<u8> {
        let total = LAS14_HEADER_SIZE + VLR_HEADER_SIZE + COPC_INFO_PAYLOAD_SIZE;
        let mut buf = vec![0u8; total];
        buf[0..4].copy_from_slice(b"LASF");
        // Bounds at offset 179: maxx,minx,maxy,miny,maxz,minz.
        let [minx, maxx, miny, maxy, minz, maxz] = bounds;
        let mut b = Vec::new();
        for v in [maxx, minx, maxy, miny, maxz, minz] {
            b.write_f64::<LittleEndian>(v).unwrap();
        }
        buf[179..179 + 48].copy_from_slice(&b);
        // VLR header at 375: reserved(2), user_id(16)="copc", record_id(2)=1,
        // record_length(2)=160, description(32).
        let vh = LAS14_HEADER_SIZE;
        buf[vh + 2..vh + 6].copy_from_slice(b"copc");
        buf[vh + 18] = 1; // record_id low byte
        buf[vh + 20] = (COPC_INFO_PAYLOAD_SIZE & 0xff) as u8;
        buf[vh + 21] = ((COPC_INFO_PAYLOAD_SIZE >> 8) & 0xff) as u8;
        // Payload.
        buf[vh + VLR_HEADER_SIZE..].copy_from_slice(&info.to_bytes());
        buf
    }

    #[test]
    fn copc_info_round_trips_through_reader() {
        let info = CopcInfo {
            center_x: 1.0,
            center_y: 2.0,
            center_z: 3.0,
            halfsize: 50.0,
            spacing: 1.25,
            root_hier_offset: 99999,
            root_hier_size: 320,
            gpstime_minimum: 10.0,
            gpstime_maximum: 20.0,
        };
        let bounds = [-49.0, 51.0, -48.0, 52.0, -47.0, 53.0];
        let buf = minimal_copc_buffer(&info, bounds);
        let mut cursor = Cursor::new(buf);
        let (read, read_bounds) = copc_hierarchy::read_copc_info(&mut cursor).unwrap();
        assert_eq!(read.center_x, 1.0);
        assert_eq!(read.center_y, 2.0);
        assert_eq!(read.center_z, 3.0);
        assert_eq!(read.halfsize, 50.0);
        assert_eq!(read.spacing, 1.25);
        assert_eq!(read.root_hier_offset, 99999);
        assert_eq!(read.root_hier_size, 320);
        assert_eq!(read_bounds.min_x, -49.0);
        assert_eq!(read_bounds.max_z, 53.0);
    }

    #[test]
    fn hierarchy_entry_is_32_bytes_and_decodes() {
        let entry = HierarchyEntry {
            key: VoxelKey::new(2, 3, 4, 1),
            offset: 0xdead_beef,
            byte_size: 256,
            point_count: 42,
        };
        let mut buf = Vec::new();
        entry.write_to(&mut buf);
        assert_eq!(buf.len(), HIERARCHY_ENTRY_SIZE);

        // Decode the same way the reader does.
        use byteorder::ReadBytesExt;
        let mut c = Cursor::new(&buf[..]);
        assert_eq!(c.read_i32::<LittleEndian>().unwrap(), 1); // level
        assert_eq!(c.read_i32::<LittleEndian>().unwrap(), 2); // x
        assert_eq!(c.read_i32::<LittleEndian>().unwrap(), 3); // y
        assert_eq!(c.read_i32::<LittleEndian>().unwrap(), 4); // z
        assert_eq!(c.read_u64::<LittleEndian>().unwrap(), 0xdead_beef);
        assert_eq!(c.read_i32::<LittleEndian>().unwrap(), 256);
        assert_eq!(c.read_i32::<LittleEndian>().unwrap(), 42);
    }
}
