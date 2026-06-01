//! RangeFilter: Pass only points given a dimension/range.

use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

#[derive(Clone)]
pub struct RangeLimit {
    pub dim_name: String,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub inclusive_lower: bool,
    pub inclusive_upper: bool,
    pub negate: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedRangeLimit {
    pub dim_name: String,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub inclusive_lower: bool,
    pub inclusive_upper: bool,
    pub negate: bool,
    pub consumed: usize,
}

fn skip_spaces(input: &str, mut pos: usize) -> usize {
    while pos < input.len() && input.as_bytes()[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

const UNBOUNDED_LOWER: f64 = -f64::MAX;
const UNBOUNDED_UPPER: f64 = f64::MAX;

fn parse_number(input: &str, mut pos: usize) -> Result<(f64, usize), String> {
    pos = skip_spaces(input, pos);
    let start = pos;
    while pos < input.len() {
        let ch = input.as_bytes()[pos];
        if ch.is_ascii_digit() || matches!(ch, b'+' | b'-' | b'.' | b'e' | b'E') {
            pos += 1;
        } else {
            break;
        }
    }
    if start == pos {
        return Err("No valid minimum value for range.".to_string());
    }
    let value = input[start..pos]
        .parse::<f64>()
        .map_err(|_| "No valid minimum value for range.".to_string())?;
    Ok((value, pos))
}

pub fn parse_range_limit(input: &str) -> Result<ParsedRangeLimit, String> {
    let mut pos = skip_spaces(input, 0);
    let mut negate = false;

    let name_start = pos;
    while pos < input.len() {
        let ch = input.as_bytes()[pos];
        if ch.is_ascii_alphanumeric() || matches!(ch, b'_') {
            pos += 1;
        } else {
            break;
        }
    }
    if pos == name_start {
        return Err("No dimension name.".to_string());
    }
    let dim_name = input[name_start..pos].to_string();
    pos = skip_spaces(input, pos);

    if input.as_bytes().get(pos) == Some(&b'!') {
        negate = true;
        pos += 1;
    }

    let inclusive_lower = match input.as_bytes().get(pos) {
        Some(b'(') => false,
        Some(b'[') => true,
        _ => return Err("Missing '(' or '['.".to_string()),
    };
    pos += 1;

    let (lower_bound, next_pos) = match parse_number(input, pos) {
        Ok(value) => value,
        Err(_) => (UNBOUNDED_LOWER, pos),
    };
    pos = next_pos;
    pos = skip_spaces(input, pos);
    if input.as_bytes().get(pos) != Some(&b':') {
        return Err("Missing ':' limit separator.".to_string());
    }
    pos += 1;

    let (upper_bound, next_pos) = match parse_number(input, pos) {
        Ok(value) => value,
        Err(_) => (UNBOUNDED_UPPER, pos),
    };
    pos = next_pos;
    pos = skip_spaces(input, pos);

    let inclusive_upper = match input.as_bytes().get(pos) {
        Some(b')') => false,
        Some(b']') => true,
        _ => return Err("Missing ')' or ']'.".to_string()),
    };
    pos += 1;
    pos = skip_spaces(input, pos);

    Ok(ParsedRangeLimit {
        dim_name,
        lower_bound,
        upper_bound,
        inclusive_lower,
        inclusive_upper,
        negate,
        consumed: pos,
    })
}

impl RangeLimit {
    pub fn value_passes(&self, v: f64) -> bool {
        let mut fail = (self.inclusive_lower && v < self.lower_bound)
            || (!self.inclusive_lower && v <= self.lower_bound)
            || (self.inclusive_upper && v > self.upper_bound)
            || (!self.inclusive_upper && v >= self.upper_bound);
        if v.is_nan() {
            fail = true;
        }
        if self.negate {
            fail = !fail;
        }
        !fail
    }
}

/// Test whether a point satisfies a set of dimension ranges, matching C++
/// `DimRange::pointPasses`: ranges are grouped by dimension; within a dimension
/// the point passes if it satisfies any range (OR), and it must pass every
/// dimension that has ranges (AND). An empty range set passes everything.
///
/// Shared by `filters.range` and by `filters.smrf`'s `ignore` option, which
/// uses the identical DimRange semantics to exclude points from segmentation.
pub fn ranges_point_passes(limits: &[RangeLimit], view: &PointView, idx: u64) -> bool {
    if limits.is_empty() {
        return true;
    }

    // Sort limits by dimension name to ensure contiguous grouping
    // (matches C++ std::sort on m_ranges)
    let mut sorted_limits = limits.to_vec();
    sorted_limits.sort_by(|a, b| a.dim_name.cmp(&b.dim_name));

    let mut last_dim = &sorted_limits[0].dim_name;
    let mut passes = false;

    for r in &sorted_limits {
        if &r.dim_name != last_dim {
            if !passes {
                return false;
            }
            last_dim = &r.dim_name;
        } else if passes {
            continue;
        }

        let dim_id = DimId::from_name(&r.dim_name);
        let val = view.get_f64(idx, &dim_id);
        passes = r.value_passes(val);
    }

    passes
}

pub struct RangeFilter {
    pub limits: Vec<RangeLimit>,
}

impl RangeFilter {
    pub fn new(limits: Vec<RangeLimit>) -> Self {
        Self { limits }
    }

    pub fn point_passes(&self, view: &PointView, idx: u64) -> bool {
        ranges_point_passes(&self.limits, view, idx)
    }
}

impl Filter for RangeFilter {
    fn name(&self) -> &str {
        "filters.range"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();

        for idx in 0..input.len() {
            if self.point_passes(input, idx) {
                out.append_point(input, idx);
            }
        }

        Ok(vec![out])
    }

    fn streamable(&self) -> bool {
        true
    }

    fn stream_chunk(&mut self, chunk: &mut PointView) -> Result<(), StageError> {
        // Same predicate as `run_one`; left-compact the kept points in place.
        let n = chunk.len();
        let mut write = 0u64;
        for read in 0..n {
            if self.point_passes(chunk, read) {
                if write != read {
                    chunk.copy_point_within(read, write);
                }
                write += 1;
            }
        }
        chunk.truncate(write);
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for RangeFilter {
    fn process_one(
        &mut self,
        _view: &mut pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        // Stateful point-by-point filtering is supported via point_passes in the C ABI
        false
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_limit_strings() {
        assert!(parse_range_limit("Y[4.00e0").is_err());
        assert!(parse_range_limit("Z[4:6]").is_ok());
    }

    #[test]
    fn stream_chunk_matches_run_one() {
        use pdal_core::point::{DimType, PointLayout};
        use std::rc::Rc;

        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for x in [0.0, 5.0, 10.0, 15.0, 3.0, 7.0, -1.0] {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, x);
        }

        let limits = vec![RangeLimit {
            dim_name: "X".to_string(),
            lower_bound: 0.0,
            upper_bound: 10.0,
            inclusive_lower: true,
            inclusive_upper: true,
            negate: false,
        }];

        let mut std_filter = RangeFilter::new(limits.clone());
        assert!(std_filter.streamable());
        let standard = std_filter.run_one(&view).unwrap().remove(0);

        let mut chunk = view.clone();
        RangeFilter::new(limits).stream_chunk(&mut chunk).unwrap();

        assert_eq!(chunk.len(), standard.len());
        assert_eq!(standard.len(), 5); // 0,5,10,3,7 pass; 15 and -1 are dropped
        assert!(standard.len() < view.len()); // something was actually filtered
        for i in 0..standard.len() {
            assert_eq!(
                chunk.get_f64(i, &DimId::X),
                standard.get_f64(i, &DimId::X),
                "point {i}"
            );
            assert_eq!(chunk.source_index(i), standard.source_index(i));
        }
    }

    #[test]
    fn accepts_open_ended_and_unbounded_ranges() {
        let full = parse_range_limit("Classification[:]").unwrap();
        assert_eq!(full.dim_name, "Classification");
        assert_eq!(full.lower_bound, UNBOUNDED_LOWER);
        assert_eq!(full.upper_bound, UNBOUNDED_UPPER);
        assert_eq!(full.consumed, "Classification[:]".len());

        let upper_only = parse_range_limit("Intensity[:250]").unwrap();
        assert_eq!(upper_only.lower_bound, UNBOUNDED_LOWER);
        assert_eq!(upper_only.upper_bound, 250.0);

        let lower_only = parse_range_limit("Intensity[272:]").unwrap();
        assert_eq!(lower_only.lower_bound, 272.0);
        assert_eq!(lower_only.upper_bound, UNBOUNDED_UPPER);
    }

    #[test]
    fn allows_trailing_assignment_suffix() {
        let parsed = parse_range_limit("Classification[:]=0").unwrap();
        assert_eq!(parsed.dim_name, "Classification");
        assert_eq!(parsed.consumed, "Classification[:]".len());
    }
}
