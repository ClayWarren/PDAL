use pdal_core::bounds::Bounds3D;

pub const RETURN_COUNT: usize = 15;

#[derive(Debug, Clone, PartialEq)]
pub struct LasSummary {
    total_num_points: u64,
    return_counts: [u64; RETURN_COUNT],
    bounds: Bounds3D,
}

impl Default for LasSummary {
    fn default() -> Self {
        Self {
            total_num_points: 0,
            return_counts: [0; RETURN_COUNT],
            bounds: Bounds3D::empty(),
        }
    }
}

impl LasSummary {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn add_point(&mut self, x: f64, y: f64, z: f64, return_number: i32) {
        self.total_num_points += 1;
        self.bounds.grow_point(x, y, z);

        if (1..=RETURN_COUNT as i32).contains(&return_number) {
            self.return_counts[return_number as usize - 1] += 1;
        }
    }

    pub fn total_num_points(&self) -> u64 {
        self.total_num_points
    }

    pub fn bounds(&self) -> Bounds3D {
        self.bounds
    }

    pub fn return_count(&self, return_number: usize) -> u64 {
        self.return_counts.get(return_number).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_tracks_bounds_total_and_one_based_return_numbers() {
        let mut summary = LasSummary::default();

        summary.add_point(10.0, 20.0, 30.0, 1);
        summary.add_point(-5.0, 40.0, 15.0, 3);
        summary.add_point(100.0, -2.0, 0.5, 0);
        summary.add_point(8.0, 2.0, 4.0, 16);

        assert_eq!(summary.total_num_points(), 4);
        assert_eq!(summary.return_count(0), 1);
        assert_eq!(summary.return_count(1), 0);
        assert_eq!(summary.return_count(2), 1);
        assert_eq!(summary.return_count(14), 0);
        assert_eq!(
            summary.bounds(),
            Bounds3D {
                minx: -5.0,
                maxx: 100.0,
                miny: -2.0,
                maxy: 40.0,
                minz: 0.5,
                maxz: 30.0,
            }
        );
    }

    #[test]
    fn clear_restores_empty_state() {
        let mut summary = LasSummary::default();
        summary.add_point(1.0, 2.0, 3.0, 1);

        summary.clear();

        assert_eq!(summary.total_num_points(), 0);
        assert_eq!(summary.return_count(0), 0);
        assert!(summary.bounds().is_empty());
    }
}
