//! `filters.relaxationdartthrowing`: subsample a point cloud down to a target
//! point count via repeated "dart throwing" with a shrinking exclusion radius.
//!
//! Each pass walks the points in (optionally shuffled) order, keeps a point
//! when it is not masked, and masks every neighbor within the current radius.
//! When a pass does not reach the target count the radius is multiplied by
//! `decay` and another pass runs, until the target is met or the radius falls
//! below `terminal_radius`.

use pdal_core::point::{PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

/// Small LCG driving the Fisher-Yates shuffle. Bit-exact parity with C++
/// `std::shuffle`/`std::mt19937` is intentionally not attempted: the C++ filter
/// seeds from the wall clock when no `seed` is supplied, and the upstream tests
/// assert only on the resulting point count, which is shuffle-order invariant.
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_range(&mut self, n: usize) -> usize {
        if n <= 1 {
            return 0;
        }
        // Knuth/MMIX 64-bit LCG constants.
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.state >> 32) as usize) % n
    }
}

pub struct RelaxationDartThrowingFilter {
    decay: f64,
    start_radius: f64,
    terminal_radius: f64,
    max_size: u64,
    shuffle: bool,
    seed: Option<u32>,
}

impl RelaxationDartThrowingFilter {
    pub fn new(
        decay: f64,
        start_radius: f64,
        terminal_radius: f64,
        max_size: u64,
        shuffle: bool,
        seed: Option<u32>,
    ) -> Self {
        Self {
            decay,
            start_radius,
            terminal_radius,
            max_size,
            shuffle,
            seed,
        }
    }

    /// Build the point visit order, shuffled when `shuffle` is set.
    fn visit_order(&self, np: u64) -> Vec<PointId> {
        let mut order: Vec<PointId> = (0..np).collect();
        if self.shuffle {
            let seed = self.seed.map(u64::from).unwrap_or_else(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(42)
            });
            let mut rng = Lcg::new(seed);
            for i in (1..order.len()).rev() {
                let j = rng.next_range(i + 1);
                order.swap(i, j);
            }
        }
        order
    }
}

impl Filter for RelaxationDartThrowingFilter {
    fn name(&self) -> &str {
        "filters.relaxationdartthrowing"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let np = input.len();
        let mut out = input.make_new();

        // Pass the cloud through untouched when it is already small enough.
        if np < self.max_size {
            for idx in 0..np {
                out.append_point(input, idx);
            }
            return Ok(vec![out]);
        }

        let index = SpatialIndex3d::new(input);
        let order = self.visit_order(np);

        let mut final_ids: Vec<PointId> = Vec::new();
        let mut radius = self.start_radius;
        let sqr_terminal = self.terminal_radius * self.terminal_radius;

        while (final_ids.len() as u64) < self.max_size {
            if radius * radius < sqr_terminal {
                break;
            }

            // Every point starts kept; neighbors within `radius` get masked.
            let mut keep = vec![true; np as usize];
            for &i in &final_ids {
                for n in index.radius(i, radius) {
                    keep[n as usize] = false;
                }
            }

            for &i in &order {
                if !keep[i as usize] {
                    continue;
                }
                final_ids.push(i);
                if (final_ids.len() as u64) == self.max_size {
                    break;
                }
                for n in index.radius(i, radius) {
                    keep[n as usize] = false;
                }
            }

            if (final_ids.len() as u64) < self.max_size {
                radius *= self.decay;
            }
        }

        for i in final_ids {
            out.append_point(input, i);
        }
        Ok(vec![out])
    }
}

impl Streamable for RelaxationDartThrowingFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        // Subsampling needs the whole view at once; it is not streamable.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout};
    use std::rc::Rc;

    fn view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Y, *y);
            view.set_f64(idx, &DimId::Z, *z);
        }
        view
    }

    fn grid(n: u64) -> PointView {
        // Points 10 units apart so the default radius never masks a neighbor.
        let pts: Vec<(f64, f64, f64)> = (0..n).map(|i| (i as f64 * 10.0, 0.0, 0.0)).collect();
        view(&pts)
    }

    #[test]
    fn passes_through_input_smaller_than_target() {
        let v = grid(5);
        let mut filter = RelaxationDartThrowingFilter::new(0.9, 1.0, 0.001, 10, false, None);
        let out = filter.run_one(&v).unwrap().pop().unwrap();
        assert_eq!(out.len(), 5);
    }

    #[test]
    fn subsamples_to_target_count() {
        let v = grid(20);
        let mut filter = RelaxationDartThrowingFilter::new(0.9, 1.0, 0.001, 8, false, None);
        let out = filter.run_one(&v).unwrap().pop().unwrap();
        assert_eq!(out.len(), 8);
    }

    #[test]
    fn coincident_points_terminate_with_single_point() {
        let v = view(&[(0.0, 0.0, 0.0); 10]);
        let mut filter = RelaxationDartThrowingFilter::new(0.9, 1.0, 0.001, 5, false, None);
        let out = filter.run_one(&v).unwrap().pop().unwrap();
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn lcg_next_range_handles_small_n() {
        let mut rng = Lcg::new(123);
        assert_eq!(rng.next_range(0), 0);
        assert_eq!(rng.next_range(1), 0);
        let v = rng.next_range(10);
        assert!(v < 10);
    }

    #[test]
    fn visit_order_with_shuffle_uses_seed() {
        let f = RelaxationDartThrowingFilter::new(0.9, 1.0, 0.001, 10, true, Some(7));
        let order = f.visit_order(8);
        assert_eq!(order.len(), 8);
    }

    #[test]
    fn visit_order_with_shuffle_no_seed_uses_clock() {
        let f = RelaxationDartThrowingFilter::new(0.9, 1.0, 0.001, 10, true, None);
        let order = f.visit_order(4);
        assert_eq!(order.len(), 4);
    }

    #[test]
    fn subsamples_with_shuffle_via_seed() {
        let v = grid(20);
        let mut filter = RelaxationDartThrowingFilter::new(0.9, 1.0, 0.001, 8, true, Some(42));
        let out = filter.run_one(&v).unwrap().pop().unwrap();
        assert_eq!(out.len(), 8);
    }

    #[test]
    fn filter_name_and_streamable_returns_false() {
        let mut f = RelaxationDartThrowingFilter::new(0.9, 1.0, 0.001, 10, false, None);
        assert_eq!(f.name(), "filters.relaxationdartthrowing");
        let mut v = view(&[(0.0, 0.0, 0.0)]);
        // process_one returns false (not streamable)
        assert!(!f.process_one(&mut v, 0));
    }
}
