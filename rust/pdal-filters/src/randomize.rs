//! RandomizeFilter: Randomize points in a view.

use pdal_core::point::PointView;
use pdal_core::stage::{Filter, StageError, Streamable};

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u32) -> Self {
        Self { state: seed as u64 }
    }

    fn next_range(&mut self, n: usize) -> usize {
        if n <= 1 {
            return 0;
        }
        // Standard high-quality 64-bit LCG multiplier/adder (Knuth/MMIX)
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let val = (self.state >> 32) as usize;
        val % n
    }
}

pub struct RandomizeFilter {
    pub seed_source: Option<u32>,
    pub running_seed: u32,
}

impl RandomizeFilter {
    pub fn new(seed: Option<u32>) -> Self {
        let initial_seed = seed.unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u32)
                .unwrap_or(42)
        });
        Self {
            seed_source: seed,
            running_seed: initial_seed,
        }
    }
}

impl Filter for RandomizeFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.randomize"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let n = input.len();
        if n == 0 {
            return Ok(vec![input.make_new()]);
        }

        let seed = if let Some(s) = self.seed_source {
            s
        } else {
            // Update running seed to guarantee sequential executions get unique seeds
            let next = self
                .running_seed
                .wrapping_mul(1103515245)
                .wrapping_add(12345);
            self.running_seed = next;
            next
        };

        // Generate sequential indices
        let mut indices: Vec<usize> = (0..n as usize).collect();

        // Fisher-Yates shuffle
        let mut rng = Lcg::new(seed);
        for i in (1..n as usize).rev() {
            let j = rng.next_range(i + 1);
            indices.swap(i, j);
        }

        // Reconstruct shuffled PointView
        let mut out = input.make_new();
        for &idx in &indices {
            out.append_point(input, idx as u64);
        }

        Ok(vec![out])
    }
}

impl Streamable for RandomizeFilter {
    fn process_one(
        &mut self,
        _view: &mut pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        // Shuffling points is inherently batch-only and not streamable
        false
    }

    fn reset(&mut self) {}
}
