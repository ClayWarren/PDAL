use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::HashMap;

pub struct Summary {
    pub name: String,
    pub enumerate: u32, // 0 = NoEnum, 1 = Enumerate, 2 = Count, 3 = Global
    pub advanced: bool,
    pub min: f64,
    pub max: f64,
    pub cnt: u64,
    pub m1: f64,
    pub m2: f64,
    pub m3: f64,
    pub m4: f64,
    pub median: f64,
    pub mad: f64,
    pub values: HashMap<u64, u64>,
    pub data: Vec<f64>,
}

impl Summary {
    pub fn new(name: String, enumerate: u32, advanced: bool) -> Self {
        Self {
            name,
            enumerate,
            advanced,
            min: f64::MAX,
            max: -f64::MAX,
            cnt: 0,
            m1: 0.0,
            m2: 0.0,
            m3: 0.0,
            m4: 0.0,
            median: 0.0,
            mad: 0.0,
            values: HashMap::new(),
            data: Vec::new(),
        }
    }

    pub fn insert(&mut self, value: f64) {
        self.cnt += 1;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }

        if self.enumerate != 0 {
            let bits = value.to_bits();
            *self.values.entry(bits).or_insert(0) += 1;
        }
        if self.enumerate == 3 {
            self.data.push(value);
        }

        let n = self.cnt as f64;
        let delta = value - self.m1;
        let delta_n = delta / n;
        let term1 = delta * delta_n * (n - 1.0);

        self.m1 += delta_n;

        if self.advanced {
            let delta_n2 = delta_n * delta_n;
            self.m4 += term1 * delta_n2 * (n * n - 3.0 * n + 3.0) + (6.0 * delta_n2 * self.m2)
                - (4.0 * delta_n * self.m3);
            self.m3 += term1 * delta_n * (n - 2.0) - 3.0 * delta_n * self.m2;
        }
        self.m2 += term1;
    }

    pub fn variance(&self) -> f64 {
        if self.cnt <= 1 {
            0.0
        } else {
            self.m2 / (self.cnt as f64 - 1.0)
        }
    }

    pub fn stddev(&self) -> f64 {
        self.variance().sqrt()
    }

    pub fn skewness(&self) -> f64 {
        if self.m2 == 0.0 || self.cnt <= 2 || !self.advanced {
            return 0.0;
        }
        let c = self.cnt as f64;
        let pop_skew = c.sqrt() * self.m3 / self.m2.powf(1.5);
        pop_skew * c.sqrt() * (c - 1.0).sqrt() / (c - 2.0)
    }

    pub fn kurtosis(&self) -> f64 {
        if self.m2 == 0.0 || self.cnt <= 3 || !self.advanced {
            return 0.0;
        }
        let c = self.cnt as f64;
        let pop_kurt = c * self.m4 / (self.m2 * self.m2);
        let sample_kurt = pop_kurt * (c + 1.0) * (c - 1.0) / ((c - 2.0) * (c - 3.0));
        sample_kurt - 3.0 * (c - 1.0) * (c - 1.0) / ((c - 2.0) * (c - 3.0))
    }

    pub fn compute_global_stats(&mut self) {
        if self.data.is_empty() {
            return;
        }
        let mut sorted = self.data.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        self.median = sorted[mid];

        let mut diffs: Vec<f64> = sorted.iter().map(|&v| (v - self.median).abs()).collect();
        diffs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        self.mad = diffs[mid];
    }
}

pub struct StatsFilter {
    pub summaries: HashMap<String, Summary>,
}

impl StatsFilter {
    pub fn new() -> Self {
        Self {
            summaries: HashMap::new(),
        }
    }
}

impl Default for StatsFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Filter for StatsFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.stats"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let size = view.len();
        for i in 0..size {
            for (name, summary) in self.summaries.iter_mut() {
                let dim = DimId::from_name(name);
                let val = view.get_f64(i, &dim);
                summary.insert(val);
            }
        }
        for summary in self.summaries.values_mut() {
            if summary.enumerate == 3 {
                summary.compute_global_stats();
            }
        }
        let mut out = PointView::new(view.layout().clone());
        for i in 0..size {
            out.append_point(view, i);
        }
        Ok(vec![out])
    }
}

impl Streamable for StatsFilter {
    fn process_one(&mut self, _view: &pdal_core::point::PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {
        for summary in self.summaries.values_mut() {
            *summary = Summary::new(summary.name.clone(), summary.enumerate, summary.advanced);
        }
    }
}
