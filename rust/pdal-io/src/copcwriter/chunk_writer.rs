//! LAZ variable-chunk encoding for the COPC writer.
//!
//! COPC stores each octree node's points as one variable-size LAZ chunk in a
//! single LAZ point stream. This module drives `laz::LasZipCompressor` in
//! variable-chunk mode: it packs each node's points to raw LAS records (reusing
//! the `las_writer` packer), compresses them, and finishes a chunk per node,
//! recording each chunk's byte offset/size and point count for the hierarchy.
//!
//! Replaces the lazperf usage in `io/private/copcwriter/Output.cpp`'s
//! `writeCompressed`/chunk-table paths with the `laz` crate.

use std::io::Cursor;

use las::point::Format;
use las::{Transform, Vector};
use laz::{LasZipCompressor, LazVlrBuilder};

use crate::las_writer::{point_from_view, ExtraDim};

use super::processor::Chunk;
use super::voxel_key::VoxelKey;

/// One node's chunk position within the LAZ point data section. `offset` is
/// relative to the start of the point data (i.e. includes the 8-byte
/// chunk-table-offset prefix), so the absolute file offset is
/// `point_data_offset + offset`. Empty nodes have `offset/byte_size/point_count`
/// all zero, matching the C++ `Output::newChunk` empty path.
#[derive(Clone, Copy, Debug)]
pub struct NodeChunk {
    pub key: VoxelKey,
    pub offset: u64,
    pub byte_size: i32,
    pub point_count: i32,
}

/// The encoded LAZ point data plus per-node chunk positions.
pub struct EncodedChunks {
    /// `[chunk_table_offset:8][chunk 0][chunk 1]...[chunk table]` -- the full
    /// LAZ point data section written after the LAS header + VLRs.
    pub point_data: Vec<u8>,
    pub nodes: Vec<NodeChunk>,
    pub total_points: u64,
}

/// Encode the ordered `chunks` (one per octree node) into a single LAZ
/// variable-chunk stream.
pub fn encode_chunks(
    chunks: &[Chunk],
    point_format: u8,
    extra_dims: &[ExtraDim],
    num_extra_bytes: u16,
    transforms: &Vector<Transform>,
) -> Result<EncodedChunks, String> {
    let mut format =
        Format::new(point_format).map_err(|e| format!("COPC: bad point format: {e}"))?;
    format.extra_bytes = num_extra_bytes;
    let has_gps_time = format.has_gps_time;
    let has_color = format.has_color;

    let vlr = LazVlrBuilder::new(Vec::new())
        .with_point_format(point_format, num_extra_bytes)
        .map_err(|e| format!("COPC: laz vlr build failed: {e}"))?
        .with_variable_chunk_size()
        .build();

    let mut compressor = LasZipCompressor::new(Cursor::new(Vec::new()), vlr)
        .map_err(|e| format!("COPC: compressor init failed: {e}"))?;
    compressor
        .reserve_offset_to_chunk_table()
        .map_err(|e| format!("COPC: reserve chunk-table offset failed: {e}"))?;

    let mut nodes = Vec::with_capacity(chunks.len());
    let mut total_points = 0u64;
    let mut record = Vec::new();

    for chunk in chunks {
        let count = chunk.view.len();
        if count == 0 {
            // Empty must-write node: a hierarchy entry, but no chunk bytes.
            nodes.push(NodeChunk {
                key: chunk.key,
                offset: 0,
                byte_size: 0,
                point_count: 0,
            });
            continue;
        }

        let chunk_start = compressor.get().position();
        for i in 0..count {
            let point = point_from_view(
                &chunk.view,
                i,
                extra_dims,
                has_gps_time,
                has_color,
                point_format,
                transforms,
                false,
                0,
            )
            .map_err(|e| format!("COPC: {e:?}"))?;
            let raw = point
                .into_raw(transforms)
                .map_err(|e| format!("COPC: point to raw failed: {e}"))?;
            record.clear();
            raw.write_to(&mut record, &format)
                .map_err(|e| format!("COPC: raw point write failed: {e}"))?;
            compressor
                .compress_one(&record)
                .map_err(|e| format!("COPC: compress failed: {e}"))?;
        }
        compressor
            .finish_current_chunk()
            .map_err(|e| format!("COPC: finish chunk failed: {e}"))?;
        let chunk_end = compressor.get().position();

        total_points += count;
        nodes.push(NodeChunk {
            key: chunk.key,
            offset: chunk_start,
            byte_size: (chunk_end - chunk_start) as i32,
            point_count: count as i32,
        });
    }

    compressor
        .done()
        .map_err(|e| format!("COPC: finalize chunk table failed: {e}"))?;
    let point_data = compressor.into_inner().into_inner();

    Ok(EncodedChunks {
        point_data,
        nodes,
        total_points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use las::point::Format;
    use laz::{LasZipDecompressor, LazVlrBuilder};
    use pdal_core::point::{DimId, DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn xyz_view(xs: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for &(x, y, z) in xs {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, x);
            view.set_f64(id, &DimId::Y, y);
            view.set_f64(id, &DimId::Z, z);
        }
        view
    }

    fn unit_transforms() -> Vector<Transform> {
        Vector {
            x: Transform {
                scale: 0.01,
                offset: 0.0,
            },
            y: Transform {
                scale: 0.01,
                offset: 0.0,
            },
            z: Transform {
                scale: 0.01,
                offset: 0.0,
            },
        }
    }

    #[test]
    fn encodes_two_nodes_and_round_trips_through_decompressor() {
        let a = xyz_view(&[(1.0, 2.0, 3.0), (1.5, 2.5, 3.5)]);
        let b = xyz_view(&[(10.0, 11.0, 12.0)]);
        let chunks = vec![
            Chunk {
                key: VoxelKey::new(0, 0, 0, 1),
                view: a,
            },
            Chunk {
                key: VoxelKey::ROOT,
                view: b,
            },
        ];
        let transforms = unit_transforms();
        let enc = encode_chunks(&chunks, 0, &[], 0, &transforms).unwrap();

        assert_eq!(enc.total_points, 3);
        assert_eq!(enc.nodes.len(), 2);
        // First chunk starts right after the 8-byte chunk-table offset.
        assert_eq!(enc.nodes[0].offset, 8);
        // Chunks are non-empty and laid out in order.
        assert!(enc.nodes[0].byte_size > 0);
        assert!(enc.nodes[1].offset >= enc.nodes[0].offset + enc.nodes[0].byte_size as u64);
        assert_eq!(enc.nodes[0].point_count, 2);
        assert_eq!(enc.nodes[1].point_count, 1);

        // Round-trip: decompress the stream and confirm all 3 points decode.
        let format = Format::new(0).unwrap();
        let vlr = LazVlrBuilder::new(Vec::new())
            .with_point_format(0, 0)
            .unwrap()
            .with_variable_chunk_size()
            .build();
        let mut dec = LasZipDecompressor::new(Cursor::new(enc.point_data), vlr).unwrap();
        let point_size = format.len() as usize;
        let mut out = vec![0u8; point_size * 3];
        dec.decompress_many(&mut out).unwrap();

        // First decoded point's X (scaled i32 at offset 0) should be 1.0/0.01 = 100.
        let x0 = i32::from_le_bytes(out[0..4].try_into().unwrap());
        assert_eq!(x0, 100);
    }

    #[test]
    fn empty_node_produces_zero_entry_and_no_bytes() {
        let chunks = vec![Chunk {
            key: VoxelKey::new(1, 1, 1, 1),
            view: xyz_view(&[]),
        }];
        let enc = encode_chunks(&chunks, 0, &[], 0, &unit_transforms()).unwrap();
        assert_eq!(enc.total_points, 0);
        assert_eq!(enc.nodes.len(), 1);
        assert_eq!(enc.nodes[0].offset, 0);
        assert_eq!(enc.nodes[0].byte_size, 0);
        assert_eq!(enc.nodes[0].point_count, 0);
    }
}
