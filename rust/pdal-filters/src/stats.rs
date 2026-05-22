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

    pub fn merge(&mut self, other: &Summary) -> bool {
        if self.name != other.name
            || self.enumerate != other.enumerate
            || self.advanced != other.advanced
        {
            return false;
        }

        if other.cnt == 0 {
            return true;
        }

        if self.cnt == 0 {
            self.min = other.min;
            self.max = other.max;
            self.cnt = other.cnt;
            self.m1 = other.m1;
            self.m2 = other.m2;
            self.m3 = other.m3;
            self.m4 = other.m4;
            self.median = other.median;
            self.mad = other.mad;
            self.values = other.values.clone();
            self.data = other.data.clone();
            return true;
        }

        let n1 = self.cnt as f64;
        let n2 = other.cnt as f64;
        let n = n1 + n2;
        let delta = other.m1 - self.m1;
        let delta2 = delta * delta;
        let delta3 = delta2 * delta;
        let delta4 = delta3 * delta;

        self.cnt = n as u64;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        self.m1 = (n1 * self.m1 + n2 * other.m1) / n;

        if self.advanced {
            let new_m4 = self.m4
                + other.m4
                + delta4 * n1 * n2 * (n1 * n1 - n1 * n2 + n2 * n2) / (n * n * n)
                + 6.0 * delta2 * (n1 * n1 * other.m2 + n2 * n2 * self.m2) / (n * n)
                + 4.0 * delta * (n1 * other.m3 - n2 * self.m3) / n;
            let new_m3 = self.m3
                + other.m3
                + delta3 * n1 * n2 * (n1 - n2) / (n * n)
                + 3.0 * delta * (n1 * other.m2 - n2 * self.m2) / n;
            self.m4 = new_m4;
            self.m3 = new_m3;
        }

        self.m2 = self.m2 + other.m2 + delta2 * n1 * n2 / n;

        if self.enumerate != 0 {
            for (value, count) in &other.values {
                *self.values.entry(*value).or_insert(0) += count;
            }
        }

        if self.enumerate == 3 {
            self.data.extend_from_slice(&other.data);
            self.compute_global_stats();
        }

        true
    }
}

pub struct StatsFilter {
    pub summaries: HashMap<String, Summary>,
    pub advanced: bool,
}

impl StatsFilter {
    pub fn new() -> Self {
        Self {
            summaries: HashMap::new(),
            advanced: false,
        }
    }

    pub fn from_options(options: &pdal_core::options::Options) -> Self {
        let advanced = options.get_bool("advanced", false);
        let mut filter = Self {
            summaries: HashMap::new(),
            advanced,
        };
        let dims = options.get_str("dimensions", "");
        let enumerate = options.get_str("enumerate", "");
        let count = options.get_str("count", "");
        let global = options.get_str("global", "");

        let mut names = std::collections::HashSet::new();
        if !dims.is_empty() {
            for d in dims.split(',').map(|s| s.trim()) {
                names.insert(d.to_string());
            }
        }

        for d in enumerate
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            filter
                .summaries
                .insert(d.to_string(), Summary::new(d.to_string(), 1, advanced));
            names.remove(d);
        }
        for d in count.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            filter
                .summaries
                .insert(d.to_string(), Summary::new(d.to_string(), 2, advanced));
            names.remove(d);
        }
        for d in global
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            filter
                .summaries
                .insert(d.to_string(), Summary::new(d.to_string(), 3, advanced));
            names.remove(d);
        }

        for d in names {
            filter
                .summaries
                .insert(d.to_string(), Summary::new(d.to_string(), 0, advanced));
        }

        filter
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

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        if self.summaries.is_empty() {
            let layout = view.layout();
            for i in 0..layout.dim_count() {
                if let Some((id, _ty)) = layout.dim_at(i) {
                    let name = id.name().to_string();
                    self.summaries
                        .insert(name.clone(), Summary::new(name, 0, self.advanced));
                }
            }
        }

        let size = view.len();
        for i in 0..size {
            for summary in self.summaries.values_mut() {
                let dim = DimId::from_name(&summary.name);
                let val = view.get_f64(i, &dim);
                summary.insert(val);
            }
        }
        for summary in self.summaries.values_mut() {
            if summary.enumerate == 3 {
                summary.compute_global_stats();
            }
        }
        // Stats filter returns the same points
        let mut out = view.make_new();
        for i in 0..size {
            out.append_point(view, i);
        }
        Ok(vec![out])
    }
}

impl Streamable for StatsFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        if self.summaries.is_empty() {
            let layout = view.layout();
            for i in 0..layout.dim_count() {
                if let Some((id, _ty)) = layout.dim_at(i) {
                    let name = id.name().to_string();
                    self.summaries
                        .insert(name.clone(), Summary::new(name, 0, self.advanced));
                }
            }
        }
        for summary in self.summaries.values_mut() {
            let dim = DimId::from_name(&summary.name);
            let val = view.get_f64(idx, &dim);
            summary.insert(val);
        }
        true
    }

    fn reset(&mut self) {
        for summary in self.summaries.values_mut() {
            *summary = Summary::new(summary.name.clone(), summary.enumerate, summary.advanced);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::options::Options;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view(values: &[(f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Classification, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, class) in values {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Classification, *class);
        }
        view
    }

    #[test]
    fn summary_computes_basic_and_advanced_statistics() {
        let mut summary = Summary::new("X".to_string(), 3, true);
        for value in [1.0, 2.0, 3.0, 4.0, 5.0] {
            summary.insert(value);
        }
        summary.compute_global_stats();

        assert_eq!(summary.cnt, 5);
        assert_eq!(summary.min, 1.0);
        assert_eq!(summary.max, 5.0);
        assert_eq!(summary.m1, 3.0);
        assert_eq!(summary.variance(), 2.5);
        assert!((summary.stddev() - 2.5_f64.sqrt()).abs() < 1e-12);
        assert_eq!(summary.median, 3.0);
        assert_eq!(summary.mad, 1.0);
        assert_eq!(summary.values.get(&3.0f64.to_bits()), Some(&1));
        assert!(summary.skewness().abs() < 1e-12);
        assert!(summary.kurtosis().is_finite());
    }

    #[test]
    fn merge_combines_compatible_summaries() {
        let mut whole = Summary::new("test".to_string(), 0, true);
        let mut part = Summary::new("test".to_string(), 0, true);
        for value in [1.0, 2.0, 3.0, 4.0, 5.0] {
            whole.insert(value);
            part.insert(value);
        }

        assert!(whole.merge(&part));
        assert_eq!(whole.cnt, 10);
        assert_eq!(whole.min, 1.0);
        assert_eq!(whole.max, 5.0);
        assert_eq!(whole.m1, 3.0);
    }

    #[test]
    fn run_summarizes_selected_dimensions_and_preserves_points() {
        let mut options = Options::new();
        options
            .add("dimensions", "X")
            .add("count", "Classification")
            .add("advanced", "true");
        let mut filter = StatsFilter::from_options(&options);
        let input = view(&[(10.0, 2.0), (20.0, 2.0), (30.0, 5.0)]);

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);
        assert_eq!(out.len(), 3);
        assert_eq!(out.get_f64(2, &DimId::X), 30.0);

        let x = filter.summaries.get("X").unwrap();
        assert_eq!(x.cnt, 3);
        assert_eq!(x.min, 10.0);
        assert_eq!(x.max, 30.0);
        assert_eq!(x.m1, 20.0);

        let classes = filter.summaries.get("Classification").unwrap();
        assert_eq!(classes.enumerate, 2);
        assert_eq!(classes.values.get(&2.0f64.to_bits()), Some(&2));
        assert_eq!(classes.values.get(&5.0f64.to_bits()), Some(&1));
    }

    #[test]
    fn empty_configuration_summarizes_every_layout_dimension_and_reset_clears_state() {
        let mut filter = StatsFilter::new();
        let input = view(&[(1.0, 7.0), (3.0, 8.0)]);

        filter.process_one(&mut input.clone(), 0);
        filter.process_one(&mut input.clone(), 1);
        assert_eq!(filter.summaries.get("X").unwrap().cnt, 2);
        assert_eq!(filter.summaries.get("Classification").unwrap().cnt, 2);

        filter.reset();
        assert_eq!(filter.summaries.get("X").unwrap().cnt, 0);
        assert_eq!(filter.summaries.get("Classification").unwrap().cnt, 0);
    }
}
