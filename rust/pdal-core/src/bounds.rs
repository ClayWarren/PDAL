//! Bounding-box helpers for PDAL core.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds2D {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds3D {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
    pub minz: f64,
    pub maxz: f64,
}

impl Bounds2D {
    pub fn empty() -> Self {
        Self {
            minx: f64::MAX,
            maxx: f64::MIN,
            miny: f64::MAX,
            maxy: f64::MIN,
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::empty()
    }

    pub fn grow_point(&mut self, x: f64, y: f64) {
        self.minx = self.minx.min(x);
        self.maxx = self.maxx.max(x);
        self.miny = self.miny.min(y);
        self.maxy = self.maxy.max(y);
    }

    pub fn grow_distance(&mut self, dist: f64) {
        self.minx -= dist;
        self.maxx += dist;
        self.miny -= dist;
        self.maxy += dist;
    }

    pub fn grow_bounds(&mut self, other: &Self) {
        self.minx = self.minx.min(other.minx);
        self.maxx = self.maxx.max(other.maxx);
        self.miny = self.miny.min(other.miny);
        self.maxy = self.maxy.max(other.maxy);
    }

    pub fn clip(&mut self, other: &Self) {
        self.minx = self.minx.max(other.minx);
        self.maxx = self.maxx.min(other.maxx);
        self.miny = self.miny.max(other.miny);
        self.maxy = self.maxy.min(other.maxy);
    }

    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        self.minx <= x && x <= self.maxx && self.miny <= y && y <= self.maxy
    }

    pub fn contains_bounds(&self, other: &Self) -> bool {
        self.minx <= other.minx
            && self.maxx >= other.maxx
            && self.miny <= other.miny
            && self.maxy >= other.maxy
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.minx <= other.maxx
            && self.maxx >= other.minx
            && self.miny <= other.maxy
            && self.maxy >= other.miny
    }
}

impl Bounds3D {
    pub fn empty() -> Self {
        Self {
            minx: f64::MAX,
            maxx: f64::MIN,
            miny: f64::MAX,
            maxy: f64::MIN,
            minz: f64::MAX,
            maxz: f64::MIN,
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::empty()
    }

    pub fn grow_point(&mut self, x: f64, y: f64, z: f64) {
        self.minx = self.minx.min(x);
        self.maxx = self.maxx.max(x);
        self.miny = self.miny.min(y);
        self.maxy = self.maxy.max(y);
        self.minz = self.minz.min(z);
        self.maxz = self.maxz.max(z);
    }

    pub fn grow_bounds(&mut self, other: &Self) {
        self.minx = self.minx.min(other.minx);
        self.maxx = self.maxx.max(other.maxx);
        self.miny = self.miny.min(other.miny);
        self.maxy = self.maxy.max(other.maxy);
        self.minz = self.minz.min(other.minz);
        self.maxz = self.maxz.max(other.maxz);
    }

    pub fn grow_distance(&mut self, dist: f64) {
        self.minx -= dist;
        self.maxx += dist;
        self.miny -= dist;
        self.maxy += dist;
        self.minz -= dist;
        self.maxz += dist;
    }

    pub fn clip(&mut self, other: &Self) {
        self.minx = self.minx.max(other.minx);
        self.maxx = self.maxx.min(other.maxx);
        self.miny = self.miny.max(other.miny);
        self.maxy = self.maxy.min(other.maxy);
        if other.minz > self.minz && other.minz < self.maxz {
            self.minz = other.minz;
        }
        if other.maxz < self.maxz && other.maxz > self.minz {
            self.maxz = other.maxz;
        }
    }

    pub fn contains_point(&self, x: f64, y: f64, z: f64) -> bool {
        self.minx <= x
            && x <= self.maxx
            && self.miny <= y
            && y <= self.maxy
            && self.minz <= z
            && z <= self.maxz
    }

    pub fn contains_bounds(&self, other: &Self) -> bool {
        self.minx <= other.minx
            && self.maxx >= other.maxx
            && self.miny <= other.miny
            && self.maxy >= other.maxy
            && self.minz <= other.minz
            && other.maxz <= self.maxz
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.minx <= other.maxx
            && self.maxx >= other.minx
            && self.miny <= other.maxy
            && self.maxy >= other.miny
            && self.minz <= other.maxz
            && self.maxz >= other.minz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds2d_empty_and_grow_match_cpp_contract() {
        let mut bounds = Bounds2D::empty();
        assert!(bounds.is_empty());

        bounds.grow_point(0.0, 201.0);
        assert_eq!(
            bounds,
            Bounds2D {
                minx: 0.0,
                maxx: 0.0,
                miny: 201.0,
                maxy: 201.0,
            }
        );

        bounds.grow_distance(2.0);
        assert_eq!(
            bounds,
            Bounds2D {
                minx: -2.0,
                maxx: 2.0,
                miny: 199.0,
                maxy: 203.0,
            }
        );

        let other = Bounds2D {
            minx: -1.0,
            maxx: 10.0,
            miny: 200.0,
            maxy: 204.0,
        };
        assert!(bounds.contains_point(0.0, 201.0));
        assert!(bounds.overlaps(&other));
        bounds.grow_bounds(&other);
        assert_eq!(bounds.maxx, 10.0);
        bounds.clip(&other);
        assert_eq!(bounds, other);
        assert!(bounds.contains_bounds(&other));
    }

    #[test]
    fn bounds3d_empty_and_grow_match_cpp_contract() {
        let mut bounds = Bounds3D::empty();
        assert!(bounds.is_empty());

        bounds.grow_point(0.0, 201.0, 202.0);
        assert_eq!(
            bounds,
            Bounds3D {
                minx: 0.0,
                maxx: 0.0,
                miny: 201.0,
                maxy: 201.0,
                minz: 202.0,
                maxz: 202.0,
            }
        );

        bounds.grow_distance(2.0);
        assert!(bounds.contains_point(0.0, 201.0, 202.0));
        let other = Bounds3D {
            minx: -1.0,
            maxx: 1.0,
            miny: 200.0,
            maxy: 202.0,
            minz: 201.0,
            maxz: 203.0,
        };
        assert!(bounds.overlaps(&other));
        assert!(bounds.contains_bounds(&other));
        bounds.clip(&other);
        assert_eq!(bounds, other);
    }
}
