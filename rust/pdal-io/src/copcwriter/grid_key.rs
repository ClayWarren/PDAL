//! Sampling-grid cell key for COPC voxel subsampling.
//!
//! Port of `io/private/copcwriter/GridKey.hpp`. Packs three small (`< 255`)
//! cell indices into one integer; used as the key of a node's occupancy grid
//! when selecting representative points for a level.

/// A packed `(i, j, k)` sampling-grid cell key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridKey {
    key: i32,
}

impl GridKey {
    /// Pack `(i, j, k)`. Each index must be `< 255`, matching the C++ asserts;
    /// the packing is `(i << 16) | (j << 8) | k`.
    pub fn new(i: i32, j: i32, k: i32) -> Self {
        debug_assert!(i < u8::MAX as i32);
        debug_assert!(j < u8::MAX as i32);
        debug_assert!(k < u8::MAX as i32);
        GridKey {
            key: (i << 16) | (j << 8) | k,
        }
    }

    pub fn i(self) -> i32 {
        self.key >> 16
    }
    pub fn j(self) -> i32 {
        (self.key >> 8) & 0xFF
    }
    pub fn k(self) -> i32 {
        self.key & 0xFF
    }
    pub fn key(self) -> i32 {
        self.key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_and_unpacks() {
        let g = GridKey::new(5, 200, 17);
        assert_eq!(g.i(), 5);
        assert_eq!(g.j(), 200);
        assert_eq!(g.k(), 17);
        // Equality is on the packed value.
        assert_eq!(g, GridKey::new(5, 200, 17));
        assert_ne!(g, GridKey::new(5, 200, 18));
    }
}
