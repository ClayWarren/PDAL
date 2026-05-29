//! COPC file assembly: lay out the LAS 1.4 header, VLRs, LAZ point data, and
//! the hierarchy EVLR.
//!
//! Port of the file-writing parts of `io/private/copcwriter/Output.cpp`,
//! re-expressed against the `las`/`laz` crates instead of lazperf. The header
//! is built with `las::raw::Header`; VLR/EVLR headers and the `copc` info VLR
//! are written by hand to match the COPC 1.0 layout the reader expects.

use std::io::Write;

use byteorder::{LittleEndian, WriteBytesExt};
use las::{Transform, Vector};

use super::chunk_writer::encode_chunks;
use super::hierarchy;
use super::output_format::{CopcInfo, HierarchyEntry, COPC_INFO_PAYLOAD_SIZE};
use super::processor::Chunk;
use super::voxel_key::VoxelKey;
use crate::las_writer::ExtraDim;

const LAS14_HEADER_SIZE: u32 = 375;
const VLR_HEADER_SIZE: usize = 54;
const EVLR_HEADER_SIZE: u64 = 60;
const COPC_INFO_RECORD_ID: u16 = 1;
const LASZIP_RECORD_ID: u16 = 22204;
const HIERARCHY_RECORD_ID: u16 = 1000;

/// A raw VLR to place in the standard VLR area (after the header).
pub(crate) struct RawVlr {
    pub user_id: String,
    pub record_id: u16,
    pub description: String,
    pub data: Vec<u8>,
}

/// Everything the file assembler needs that isn't derived from the chunks.
pub(crate) struct CopcWriteParams {
    pub point_format: u8,
    pub num_extra_bytes: u16,
    pub extra_dims: Vec<ExtraDim>,
    pub scale: [f64; 3],
    pub offset: [f64; 3],
    /// Conforming (true) data bounds.
    pub bounds: [f64; 6], // minx, miny, minz, maxx, maxy, maxz
    pub center: [f64; 3],
    pub halfsize: f64,
    pub spacing: f64,
    pub gpstime_min: f64,
    pub gpstime_max: f64,
    /// Extended per-return point counts (LAS 1.4 large-file fields, returns 1..15).
    pub points_by_return: [u64; 15],
    pub file_source_id: u16,
    pub global_encoding: u16,
    pub creation_day: u16,
    pub creation_year: u16,
    pub system_id: String,
    pub software_id: String,
    /// LAS project-id GUID (16 bytes), from the `project_id` option.
    pub guid: [u8; 16],
    /// SRS / eb / user VLRs to write after the copc info and laz VLRs.
    pub extra_vlrs: Vec<RawVlr>,
}

fn fixed(s: &str, n: usize) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.resize(n, 0);
    v.truncate(n);
    v
}

/// Serialize a standard VLR (54-byte header + data).
fn write_vlr(out: &mut Vec<u8>, user_id: &str, record_id: u16, description: &str, data: &[u8]) {
    out.write_u16::<LittleEndian>(0).unwrap(); // reserved
    out.extend_from_slice(&fixed(user_id, 16));
    out.write_u16::<LittleEndian>(record_id).unwrap();
    out.write_u16::<LittleEndian>(data.len() as u16).unwrap();
    out.extend_from_slice(&fixed(description, 32));
    out.extend_from_slice(data);
}

/// Serialize an EVLR header (60 bytes; record length is u64).
fn write_evlr_header(
    out: &mut Vec<u8>,
    user_id: &str,
    record_id: u16,
    description: &str,
    len: u64,
) {
    out.write_u16::<LittleEndian>(0).unwrap(); // reserved
    out.extend_from_slice(&fixed(user_id, 16));
    out.write_u16::<LittleEndian>(record_id).unwrap();
    out.write_u64::<LittleEndian>(len).unwrap();
    out.extend_from_slice(&fixed(description, 32));
}

/// The `laszip encoded` VLR data for a variable-chunk LAZ of this point format.
fn laz_vlr_data(point_format: u8, num_extra_bytes: u16) -> Result<Vec<u8>, String> {
    let vlr = laz::LazVlrBuilder::new(Vec::new())
        .with_point_format(point_format, num_extra_bytes)
        .map_err(|e| format!("COPC: laz vlr build failed: {e}"))?
        .with_variable_chunk_size()
        .build();
    let mut data = Vec::new();
    vlr.write_to(&mut data)
        .map_err(|e| format!("COPC: laz vlr serialize failed: {e}"))?;
    Ok(data)
}

/// Build the LAS 1.4 header bytes (exactly 375 bytes) via `las::raw::Header`.
#[allow(clippy::too_many_arguments)]
fn header_bytes(
    params: &CopcWriteParams,
    point_record_length: u16,
    offset_to_point_data: u32,
    vlr_count: u32,
    total_points: u64,
    evlr_offset: u64,
) -> Result<Vec<u8>, String> {
    let mut h = las::raw::Header {
        file_signature: *b"LASF",
        file_source_id: params.file_source_id,
        // Bit 4 (WKT) set, like the C++ writer.
        global_encoding: params.global_encoding | (1 << 4),
        guid: params.guid,
        version: las::Version::new(1, 4),
        system_identifier: {
            let mut a = [0u8; 32];
            a.copy_from_slice(&fixed(&params.system_id, 32));
            a
        },
        generating_software: {
            let mut a = [0u8; 32];
            a.copy_from_slice(&fixed(&params.software_id, 32));
            a
        },
        file_creation_day_of_year: params.creation_day,
        file_creation_year: params.creation_year,
        header_size: LAS14_HEADER_SIZE as u16,
        offset_to_point_data,
        number_of_variable_length_records: vlr_count,
        // High bit marks LAZ compression.
        point_data_record_format: params.point_format | 0x80,
        point_data_record_length: point_record_length,
        number_of_point_records: 0, // legacy fields zero for COPC (LAS 1.4)
        number_of_points_by_return: [0; 5],
        x_scale_factor: params.scale[0],
        y_scale_factor: params.scale[1],
        z_scale_factor: params.scale[2],
        x_offset: params.offset[0],
        y_offset: params.offset[1],
        z_offset: params.offset[2],
        min_x: params.bounds[0],
        min_y: params.bounds[1],
        min_z: params.bounds[2],
        max_x: params.bounds[3],
        max_y: params.bounds[4],
        max_z: params.bounds[5],
        start_of_waveform_data_packet_record: Some(0),
        evlr: Some(las::raw::header::Evlr {
            start_of_first_evlr: evlr_offset,
            number_of_evlrs: 1,
        }),
        large_file: Some(las::raw::header::LargeFile {
            number_of_point_records: total_points,
            number_of_points_by_return: params.points_by_return,
        }),
        padding: Vec::new(),
    };
    // raw::Header expects max/min ordering as stored fields; the struct uses
    // max_x/min_x naming but the spec order is max then min. The field names
    // above already map correctly.
    let _ = &mut h;
    let mut buf = Vec::new();
    h.write_to(&mut buf)
        .map_err(|e| format!("COPC: header serialize failed: {e}"))?;
    buf.resize(LAS14_HEADER_SIZE as usize, 0);
    Ok(buf)
}

/// Assemble and write a COPC file. Returns the total point count written.
pub(crate) fn write_copc(
    path: &str,
    params: &CopcWriteParams,
    chunks: &[Chunk],
    child_counts: &std::collections::HashMap<VoxelKey, i64>,
) -> Result<u64, String> {
    let transforms = Vector {
        x: Transform {
            scale: params.scale[0],
            offset: params.offset[0],
        },
        y: Transform {
            scale: params.scale[1],
            offset: params.offset[1],
        },
        z: Transform {
            scale: params.scale[2],
            offset: params.offset[2],
        },
    };

    let mut enc = encode_chunks(
        chunks,
        params.point_format,
        &params.extra_dims,
        params.num_extra_bytes,
        &transforms,
    )?;

    // VLR area: copc info (must be first), laz vlr, then SRS/eb/user VLRs.
    let laz_data = laz_vlr_data(params.point_format, params.num_extra_bytes)?;

    let mut vlr_count = 2 + params.extra_vlrs.len() as u32;
    // Compute vlr area size to know the point offset (copc info is fixed-size).
    let mut vlr_area_size =
        (VLR_HEADER_SIZE + COPC_INFO_PAYLOAD_SIZE) + (VLR_HEADER_SIZE + laz_data.len());
    for v in &params.extra_vlrs {
        vlr_area_size += VLR_HEADER_SIZE + v.data.len();
    }
    let point_offset = LAS14_HEADER_SIZE as u64 + vlr_area_size as u64;

    // Leaf entries with absolute file offsets.
    let mut leaves = hierarchy::LeafEntries::new();
    for node in &enc.nodes {
        let offset = if node.point_count == 0 {
            0
        } else {
            point_offset + node.offset
        };
        leaves.insert(
            node.key,
            HierarchyEntry {
                key: node.key,
                offset,
                byte_size: node.byte_size,
                point_count: node.point_count,
            },
        );
    }

    let emitted = hierarchy::emit(&leaves, child_counts);

    // The LAZ chunk-table offset (first 8 bytes of the point data) is written
    // by LasZipCompressor relative to its own stream start; in the file it must
    // be an absolute offset. Re-base it by the point data's file offset.
    if enc.point_data.len() >= 8 {
        let relative = u64::from_le_bytes(enc.point_data[0..8].try_into().unwrap());
        let absolute = point_offset + relative;
        enc.point_data[0..8].copy_from_slice(&absolute.to_le_bytes());
    }

    let evlr_offset = point_offset + enc.point_data.len() as u64;
    let root_hier_offset = evlr_offset + EVLR_HEADER_SIZE + emitted.root_offset;

    // copc info VLR data (now that we know the root hierarchy location).
    let info = CopcInfo {
        center_x: params.center[0],
        center_y: params.center[1],
        center_z: params.center[2],
        halfsize: params.halfsize,
        spacing: params.spacing,
        root_hier_offset,
        root_hier_size: emitted.root_size as u64,
        gpstime_minimum: params.gpstime_min,
        gpstime_maximum: params.gpstime_max,
    };

    let point_record_length = las::point::Format::new(params.point_format)
        .map_err(|e| format!("COPC: {e}"))?
        .len()
        + params.num_extra_bytes;

    let header = header_bytes(
        params,
        point_record_length,
        point_offset as u32,
        vlr_count,
        enc.total_points,
        evlr_offset,
    )?;
    debug_assert_eq!(header.len(), LAS14_HEADER_SIZE as usize);
    let _ = &mut vlr_count;

    // Assemble.
    let mut file = Vec::with_capacity(point_offset as usize + enc.point_data.len());
    file.extend_from_slice(&header);
    write_vlr(
        &mut file,
        "copc",
        COPC_INFO_RECORD_ID,
        "COPC",
        &info.to_bytes(),
    );
    write_vlr(
        &mut file,
        "laszip encoded",
        LASZIP_RECORD_ID,
        "https://laszip.org",
        &laz_data,
    );
    for v in &params.extra_vlrs {
        write_vlr(&mut file, &v.user_id, v.record_id, &v.description, &v.data);
    }
    debug_assert_eq!(file.len() as u64, point_offset);
    file.extend_from_slice(&enc.point_data);
    write_evlr_header(
        &mut file,
        "copc",
        HIERARCHY_RECORD_ID,
        "EPT Hierarchy",
        emitted.bytes.len() as u64,
    );
    file.extend_from_slice(&emitted.bytes);

    std::fs::File::create(path)
        .and_then(|mut f| f.write_all(&file))
        .map_err(|e| format!("COPC: write '{path}' failed: {e}"))?;

    Ok(enc.total_points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copc_hierarchy;
    use pdal_core::point::{DimId, DimType, PointLayout, PointView};
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::BufReader;
    use std::rc::Rc;

    fn xyz_view(pts: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut v = PointView::new(Rc::new(layout));
        for &(x, y, z) in pts {
            let id = v.add_point();
            v.set_f64(id, &DimId::X, x);
            v.set_f64(id, &DimId::Y, y);
            v.set_f64(id, &DimId::Z, z);
        }
        v
    }

    #[test]
    fn writes_a_copc_the_reader_accepts() {
        let dir = std::env::temp_dir();
        let path = dir
            .join("copcwriter_output_test.copc.laz")
            .to_string_lossy()
            .into_owned();
        let _ = std::fs::remove_file(&path);

        // One root chunk with a few points.
        let chunks = vec![Chunk {
            key: VoxelKey::ROOT,
            view: xyz_view(&[(1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (7.0, 8.0, 9.0)]),
        }];
        let mut child_counts = HashMap::new();
        child_counts.insert(VoxelKey::ROOT, 0i64);

        let params = CopcWriteParams {
            point_format: 0,
            num_extra_bytes: 0,
            extra_dims: Vec::new(),
            scale: [0.01, 0.01, 0.01],
            offset: [0.0, 0.0, 0.0],
            bounds: [1.0, 2.0, 3.0, 7.0, 8.0, 9.0],
            center: [4.0, 5.0, 6.0],
            halfsize: 4.0,
            spacing: 1.0,
            gpstime_min: 0.0,
            gpstime_max: 0.0,
            points_by_return: [0; 15],
            file_source_id: 0,
            global_encoding: 0,
            creation_day: 1,
            creation_year: 2026,
            system_id: "PDAL".into(),
            software_id: "pdal-rs".into(),
            guid: [0u8; 16],
            extra_vlrs: Vec::new(),
        };

        let total = write_copc(&path, &params, &chunks, &child_counts).unwrap();
        assert_eq!(total, 3);

        // The COPC reader parses the info VLR + bounds.
        let mut reader = BufReader::new(File::open(&path).unwrap());
        let (info, bounds) = copc_hierarchy::read_copc_info(&mut reader).unwrap();
        assert_eq!(info.center_x, 4.0);
        assert_eq!(info.halfsize, 4.0);
        assert!(info.root_hier_offset > 0);
        assert!(info.root_hier_size > 0);
        assert_eq!(bounds.min_x, 1.0);
        assert_eq!(bounds.max_z, 9.0);

        // The las crate reads it back as LAZ with 3 points.
        let las_reader = las::Reader::from_path(&path).unwrap();
        assert_eq!(las_reader.header().number_of_points(), 3);

        let _ = std::fs::remove_file(&path);
    }
}
