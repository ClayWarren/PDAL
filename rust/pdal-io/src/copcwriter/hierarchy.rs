//! COPC hierarchy EVLR emission.
//!
//! Port of `Output::emitRoot`/`emitChildren` from
//! `io/private/copcwriter/Output.cpp`. Serializes the octree into one or more
//! hierarchy pages of 32-byte entries. A leaf entry carries its chunk's file
//! offset, byte size, and point count; a sub-page reference carries the page's
//! offset and byte size with `point_count == -1` (the COPC sentinel that tells
//! a reader to descend into a child page).
//!
//! Pages are laid out children-first: a parent page is written only after the
//! sub-pages it references, so their offsets are known. Small/shallow trees
//! produce a single root page.

use std::collections::HashMap;

use super::output_format::HierarchyEntry;
use super::voxel_key::VoxelKey;

/// Levels below a page's root before a new sub-page is started (C++ `LevelBreak`).
const LEVEL_BREAK: i32 = 4;
/// A subtree with at most this many cumulative nodes stays inline rather than
/// becoming a sub-page (C++ `MinHierarchySize`).
const MIN_HIERARCHY_SIZE: i64 = 50;

/// Per-node chunk position (offsets absolute in the output file).
pub(crate) type LeafEntries = HashMap<VoxelKey, HierarchyEntry>;

/// Result of emitting the hierarchy: the concatenated page bytes, and the root
/// page's offset (within these bytes) and byte size, which the `copc` info VLR
/// records as `root_hier_offset`/`root_hier_size` (after adding the absolute
/// base offset of the hierarchy in the file).
pub(crate) struct EmittedHierarchy {
    pub bytes: Vec<u8>,
    pub root_offset: u64,
    pub root_size: i32,
}

/// Emit the hierarchy for the octree described by `leaves` (per written node)
/// and `child_counts` (cumulative descendant counts, C++ `calcCounts`).
pub(crate) fn emit(
    leaves: &LeafEntries,
    child_counts: &HashMap<VoxelKey, i64>,
) -> EmittedHierarchy {
    let mut buf = Vec::new();
    let (root_offset, root_size) = emit_root(VoxelKey::ROOT, leaves, child_counts, &mut buf);
    EmittedHierarchy {
        bytes: buf,
        root_offset,
        root_size,
    }
}

/// Emit one page rooted at `key` (plus its inline descendants), writing any
/// sub-pages first. Returns `(offset_within_buf, byte_size)` of this page.
fn emit_root(
    key: VoxelKey,
    leaves: &LeafEntries,
    counts: &HashMap<VoxelKey, i64>,
    buf: &mut Vec<u8>,
) -> (u64, i32) {
    let stop_level = key.level() + LEVEL_BREAK;
    let mut entries: Vec<HierarchyEntry> = Vec::new();
    entries.push(leaf_entry(key, leaves));
    emit_children(key, leaves, counts, &mut entries, stop_level, buf);

    let start = buf.len() as u64;
    for entry in &entries {
        entry.write_to(buf);
    }
    let size = (buf.len() as u64 - start) as i32;
    (start, size)
}

fn emit_children(
    parent: VoxelKey,
    leaves: &LeafEntries,
    counts: &HashMap<VoxelKey, i64>,
    entries: &mut Vec<HierarchyEntry>,
    stop_level: i32,
    buf: &mut Vec<u8>,
) {
    for i in 0..8 {
        let c = parent.child(i);
        let Some(&cnt) = counts.get(&c) else {
            continue;
        };
        if c.level() != stop_level || cnt <= MIN_HIERARCHY_SIZE {
            // Inline this child (and recurse).
            entries.push(leaf_entry(c, leaves));
            emit_children(c, leaves, counts, entries, stop_level, buf);
        } else {
            // Start a sub-page for this child; reference it with point_count -1.
            let (offset, size) = emit_root(c, leaves, counts, buf);
            entries.push(HierarchyEntry {
                key: c,
                offset,
                byte_size: size,
                point_count: -1,
            });
        }
    }
}

/// The leaf chunk entry for `key`. Written nodes always have one; a node that
/// is in `counts` but somehow lacks a chunk is treated as an empty node.
fn leaf_entry(key: VoxelKey, leaves: &LeafEntries) -> HierarchyEntry {
    leaves.get(&key).copied().unwrap_or(HierarchyEntry {
        key,
        offset: 0,
        byte_size: 0,
        point_count: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copcwriter::output_format::HIERARCHY_ENTRY_SIZE;
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::Cursor;

    fn leaf(key: VoxelKey, offset: u64, size: i32, count: i32) -> HierarchyEntry {
        HierarchyEntry {
            key,
            offset,
            byte_size: size,
            point_count: count,
        }
    }

    /// Parse a hierarchy page's entries back from the buffer.
    fn parse_page(buf: &[u8], offset: u64, size: i32) -> Vec<HierarchyEntry> {
        let mut entries = Vec::new();
        let n = size as usize / HIERARCHY_ENTRY_SIZE;
        let mut c = Cursor::new(&buf[offset as usize..]);
        for _ in 0..n {
            let level = c.read_i32::<LittleEndian>().unwrap();
            let x = c.read_i32::<LittleEndian>().unwrap();
            let y = c.read_i32::<LittleEndian>().unwrap();
            let z = c.read_i32::<LittleEndian>().unwrap();
            let off = c.read_u64::<LittleEndian>().unwrap();
            let bs = c.read_i32::<LittleEndian>().unwrap();
            let pc = c.read_i32::<LittleEndian>().unwrap();
            entries.push(HierarchyEntry {
                key: VoxelKey::new(x, y, z, level),
                offset: off,
                byte_size: bs,
                point_count: pc,
            });
        }
        entries
    }

    #[test]
    fn small_tree_emits_single_page_with_root_and_children() {
        // Root with two level-1 children, all written.
        let c0 = VoxelKey::new(0, 0, 0, 1);
        let c3 = VoxelKey::new(1, 1, 0, 1);
        let mut leaves = LeafEntries::new();
        leaves.insert(VoxelKey::ROOT, leaf(VoxelKey::ROOT, 8, 100, 10));
        leaves.insert(c0, leaf(c0, 108, 80, 7));
        leaves.insert(c3, leaf(c3, 188, 60, 5));

        let mut counts = HashMap::new();
        counts.insert(c0, 0);
        counts.insert(c3, 0);
        counts.insert(VoxelKey::ROOT, 2);

        let emitted = emit(&leaves, &counts);
        // Shallow tree -> single page: root + 2 children = 3 entries.
        assert_eq!(emitted.root_offset, 0);
        assert_eq!(emitted.root_size as usize, 3 * HIERARCHY_ENTRY_SIZE);

        let entries = parse_page(&emitted.bytes, emitted.root_offset, emitted.root_size);
        assert_eq!(entries.len(), 3);
        // Root entry present with its chunk data.
        let root = entries.iter().find(|e| e.key == VoxelKey::ROOT).unwrap();
        assert_eq!(root.point_count, 10);
        assert_eq!(root.offset, 8);
        // Children present.
        assert!(entries.iter().any(|e| e.key == c0 && e.point_count == 7));
        assert!(entries.iter().any(|e| e.key == c3 && e.point_count == 5));
        // No sub-page references in a shallow tree.
        assert!(entries.iter().all(|e| e.point_count >= 0));
    }
}
