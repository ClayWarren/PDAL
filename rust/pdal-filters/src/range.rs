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

pub struct RangeFilter {
    pub limits: Vec<RangeLimit>,
}

impl RangeFilter {
    pub fn new(limits: Vec<RangeLimit>) -> Self {
        Self { limits }
    }

    pub fn point_passes(&self, view: &PointView, idx: u64) -> bool {
        if self.limits.is_empty() {
            return true;
        }

        // Sort limits by dimension name to ensure contiguous grouping
        // (matches C++ std::sort on m_ranges)
        let mut sorted_limits = self.limits.clone();
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
