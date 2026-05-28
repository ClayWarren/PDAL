//! Octree node key for the COPC writer.
//!
//! Port of `io/private/copcwriter/VoxelKey.hpp`. A `VoxelKey` identifies an
//! octree node by its `(level, x, y, z)` integer coordinates, matching the COPC
//! hierarchy key encoding written to the hierarchy EVLR.

/// An octree node key: depth `level` and integer cell coordinates within that
/// level. The root is `(0, 0, 0, 0)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VoxelKey {
    // Ordering mirrors C++ operator<: x, then y, then z, then level.
    x: i32,
    y: i32,
    z: i32,
    level: i32,
}

impl VoxelKey {
    /// The octree root key, `level 0` at the origin.
    pub const ROOT: VoxelKey = VoxelKey {
        x: 0,
        y: 0,
        z: 0,
        level: 0,
    };

    pub fn new(x: i32, y: i32, z: i32, level: i32) -> Self {
        VoxelKey { x, y, z, level }
    }

    /// The child key in octant `dir` (0..8), where bit 0 is X, bit 1 is Y, and
    /// bit 2 is Z. Matches `VoxelKey::child` in the C++ writer.
    pub fn child(self, dir: i32) -> Self {
        VoxelKey {
            x: (self.x << 1) | (dir & 0x1),
            y: (self.y << 1) | ((dir >> 1) & 0x1),
            z: (self.z << 1) | ((dir >> 2) & 0x1),
            level: self.level + 1,
        }
    }

    /// The parent key (level clamped at 0). Matches `VoxelKey::parent`.
    pub fn parent(self) -> Self {
        VoxelKey {
            x: self.x >> 1,
            y: self.y >> 1,
            z: self.z >> 1,
            level: (self.level - 1).max(0),
        }
    }

    pub fn x(self) -> i32 {
        self.x
    }
    pub fn y(self) -> i32 {
        self.y
    }
    pub fn z(self) -> i32 {
        self.z
    }
    pub fn level(self) -> i32 {
        self.level
    }
}

impl std::fmt::Display for VoxelKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Matches the C++ "level-x-y-z" string form.
        write!(f, "{}-{}-{}-{}", self.level, self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_encodes_octant_bits() {
        let root = VoxelKey::ROOT;
        // dir 0 -> (level 1, 0,0,0); dir 7 -> (level 1, 1,1,1).
        assert_eq!(root.child(0), VoxelKey::new(0, 0, 0, 1));
        assert_eq!(root.child(7), VoxelKey::new(1, 1, 1, 1));
        // bit 0 = x, bit 1 = y, bit 2 = z.
        assert_eq!(root.child(1), VoxelKey::new(1, 0, 0, 1));
        assert_eq!(root.child(2), VoxelKey::new(0, 1, 0, 1));
        assert_eq!(root.child(4), VoxelKey::new(0, 0, 1, 1));
    }

    #[test]
    fn parent_is_inverse_of_child_and_clamps_root() {
        let key = VoxelKey::new(3, 5, 6, 4);
        for dir in 0..8 {
            assert_eq!(key.child(dir).parent(), key);
        }
        // Root's parent stays at the root (level clamped at 0).
        assert_eq!(VoxelKey::ROOT.parent(), VoxelKey::ROOT);
    }

    #[test]
    fn display_matches_cpp_level_x_y_z() {
        assert_eq!(VoxelKey::new(1, 2, 3, 4).to_string(), "4-1-2-3");
        assert_eq!(VoxelKey::ROOT.to_string(), "0-0-0-0");
    }

    #[test]
    fn ordering_matches_cpp_x_y_z_level() {
        // C++ operator< compares x, then y, then z, then level.
        assert!(VoxelKey::new(0, 0, 0, 5) < VoxelKey::new(1, 0, 0, 0));
        assert!(VoxelKey::new(1, 0, 0, 0) < VoxelKey::new(1, 1, 0, 0));
        assert!(VoxelKey::new(1, 1, 0, 0) < VoxelKey::new(1, 1, 1, 0));
        assert!(VoxelKey::new(1, 1, 1, 0) < VoxelKey::new(1, 1, 1, 1));
    }
}
