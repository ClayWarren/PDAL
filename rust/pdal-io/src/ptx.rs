//! `readers.ptx` -- Leica PTX ASCII scan format.
//!
//! Port of `io/PtxReader.cpp`. A PTX file holds one or more clouds, each a
//! 10-line header followed by a `columns * rows` grid of point lines:
//!
//! ```text
//! columns
//! rows
//! sx sy sz            (scanner position -- skipped)
//! s11 s21 s31         (scanner 3x3 transform -- skipped)
//! s12 s22 s32
//! s13 s23 s33
//! t11 t21 t31 t41     (4x4 transform, applied to each point)
//! t12 t22 t32 t42
//! t13 t23 t33 t43
//! t14 t24 t34 t44
//! ```
//!
//! A point line is `X Y Z Intensity` or `X Y Z Intensity Red Green Blue`.
//! Intensity is `0.0..1.0` in the file and is mapped to `0..4096`. With
//! `discard_missing_points` (default true), grid cells with `X Y Z` exactly
//! `0 0 0` are dropped.

use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::fs;
use std::path::Path;
use std::rc::Rc;

/// One PTX cloud header: grid size and the 4x4 transform.
struct PtxHeader {
    columns: usize,
    rows: usize,
    transform: [f64; 16],
}

impl PtxHeader {
    /// Apply the 4x4 transform to a point's coordinates.
    fn apply(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let t = &self.transform;
        (
            x * t[0] + y * t[4] + z * t[8] + t[12],
            x * t[1] + y * t[5] + z * t[9] + t[13],
            x * t[2] + y * t[6] + z * t[10] + t[14],
        )
    }
}

/// Reader for the Leica PTX ASCII scan format.
pub struct PtxReader {
    filename: String,
    discard_missing_points: bool,
}

impl PtxReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            discard_missing_points: options.get_bool("discard_missing_points", true),
        }
    }
}

impl Reader for PtxReader {
    fn name(&self) -> &str {
        "readers.ptx"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "PtxReader requires a filename option.".to_string(),
            ));
        }
        let text = fs::read_to_string(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Unable to open file '{}'.", self.filename)))?;
        let lines: Vec<&str> = text.lines().collect();

        // Peek the first cloud's header and first data line to fix dimensions.
        let mut peek = 0usize;
        parse_header(&lines, &mut peek, &self.filename)?;
        let first_data = lines
            .get(peek)
            .map(|line| split_fields(line))
            .unwrap_or_default();
        let dims = dims_for(first_data.len()).ok_or_else(|| {
            StageError(format!(
                "Invalid number of fields for the first point in file '{}'.",
                self.filename
            ))
        })?;

        let mut layout = PointLayout::new();
        for dim in &dims {
            layout.register(dim.clone(), dim_type(dim));
        }
        let mut view = PointView::new(Rc::new(layout));

        // Each cloud is a header followed by columns*rows grid lines.
        let mut cursor = 0usize;
        while cursor < lines.len() {
            let header = parse_header(&lines, &mut cursor, &self.filename)?;
            let grid = header.columns.saturating_mul(header.rows);
            for _ in 0..grid {
                let Some(raw) = lines.get(cursor) else {
                    break;
                };
                cursor += 1;
                if raw.is_empty() {
                    continue;
                }
                let fields = split_fields(raw);
                // Lines with an unexpected field count are skipped, as in PDAL.
                if fields.len() != dims.len() {
                    continue;
                }

                let mut values = vec![0.0f64; dims.len()];
                for (i, field) in fields.iter().enumerate() {
                    let mut value = field.parse::<f64>().unwrap_or(0.0);
                    // PTX intensity is 0.0..1.0; PDAL maps it to 0..4096.
                    if dims[i] == DimId::Intensity {
                        value = (value * 4096.0).round();
                    }
                    values[i] = value;
                }

                // Fully populated PTX grids carry "0 0 0" gap points.
                if self.discard_missing_points
                    && values[0] == 0.0
                    && values[1] == 0.0
                    && values[2] == 0.0
                {
                    continue;
                }

                let (x, y, z) = header.apply(values[0], values[1], values[2]);
                values[0] = x;
                values[1] = y;
                values[2] = z;

                let point = view.add_point();
                for (dim, value) in dims.iter().zip(&values) {
                    view.set_f64(point, dim, *value);
                }
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.ptx")
    }
}

/// Parse a 10-line PTX cloud header, advancing `cursor` past it.
fn parse_header(
    lines: &[&str],
    cursor: &mut usize,
    filename: &str,
) -> Result<PtxHeader, StageError> {
    fn take<'a>(
        lines: &[&'a str],
        cursor: &mut usize,
        filename: &str,
    ) -> Result<&'a str, StageError> {
        let line = lines
            .get(*cursor)
            .copied()
            .ok_or_else(|| StageError(format!("Unable to read header for file '{filename}'.")))?;
        *cursor += 1;
        Ok(line)
    }

    let columns: usize = take(lines, cursor, filename)?.trim().parse().map_err(|_| {
        StageError(format!(
            "Invalid column size in header for file '{filename}'."
        ))
    })?;
    let rows: usize = take(lines, cursor, filename)?
        .trim()
        .parse()
        .map_err(|_| StageError(format!("Invalid row size in header for file '{filename}'.")))?;

    // Skip the scanner position and 3x3 scanner transform.
    for _ in 0..4 {
        take(lines, cursor, filename)?;
    }

    // Read the 4x4 transform, row-major as written in the file.
    let mut transform = [0.0f64; 16];
    for ty in 0..4 {
        let row = take(lines, cursor, filename)?;
        let fields = split_fields(row);
        if fields.len() != 4 {
            return Err(StageError(format!(
                "Invalid transform row '{row}' in header for file '{filename}'."
            )));
        }
        for (tx, field) in fields.iter().enumerate() {
            transform[tx + ty * 4] = field.parse().map_err(|_| {
                StageError(format!(
                    "Invalid transform value '{field}' in header for file '{filename}'."
                ))
            })?;
        }
    }

    Ok(PtxHeader {
        columns,
        rows,
        transform,
    })
}

/// The fixed PTX dimension set for a given field count.
fn dims_for(field_count: usize) -> Option<Vec<DimId>> {
    match field_count {
        4 => Some(vec![DimId::X, DimId::Y, DimId::Z, DimId::Intensity]),
        7 => Some(vec![
            DimId::X,
            DimId::Y,
            DimId::Z,
            DimId::Intensity,
            DimId::from_name("Red"),
            DimId::from_name("Green"),
            DimId::from_name("Blue"),
        ]),
        _ => None,
    }
}

/// PDAL's default storage type for each PTX dimension.
fn dim_type(dim: &DimId) -> DimType {
    match dim {
        DimId::X | DimId::Y | DimId::Z => DimType::F64,
        _ => DimType::U16,
    }
}

/// Split a PTX line on spaces, dropping empty tokens (PDAL's `split2`).
fn split_fields(line: &str) -> Vec<&str> {
    line.split(' ')
        .map(|field| field.trim_end_matches('\r'))
        .filter(|field| !field.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_path(path: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        root.join("test/data").join(path).display().to_string()
    }

    fn read_ptx(path: &str, discard_missing: bool) -> PointView {
        let mut options = Options::new();
        options.add("filename", data_path(path));
        options.add("discard_missing_points", discard_missing);
        let mut reader = PtxReader::new(&options);
        let mut views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        views.pop().unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-5
    }

    #[test]
    fn reads_ptx_with_color() {
        let view = read_ptx("ptx/1.2-with-color.ptx", true);
        assert_eq!(view.len(), 15 * 71);

        // Identity transform: X/Y/Z are the file values exactly.
        assert_eq!(view.get_f64(0, &DimId::X), 637012.24);
        assert_eq!(view.get_f64(0, &DimId::Y), 849028.31);
        assert_eq!(view.get_f64(0, &DimId::Z), 431.66);
        // Intensity 0.034912 * 4096 rounds to 143.
        assert_eq!(view.get_f64(0, &DimId::Intensity), 143.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("Red")), 68.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("Green")), 77.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("Blue")), 88.0);

        assert_eq!(view.get_f64(489, &DimId::X), 635770.47);
        assert_eq!(view.get_f64(489, &DimId::Y), 851464.67);
        assert_eq!(view.get_f64(489, &DimId::Z), 422.28);
        // Intensity 0.019775 * 4096 rounds to 81.
        assert_eq!(view.get_f64(489, &DimId::Intensity), 81.0);
        assert_eq!(view.get_f64(489, &DimId::from_name("Red")), 105.0);
    }

    #[test]
    fn reads_ptx_without_color() {
        let view = read_ptx("ptx/no-color.ptx", true);
        assert_eq!(view.len(), 15);
        assert!(view.layout().dim(&DimId::from_name("Red")).is_none());

        assert_eq!(view.get_f64(14, &DimId::X), 635795.24);
        assert_eq!(view.get_f64(14, &DimId::Y), 849310.43);
        assert_eq!(view.get_f64(14, &DimId::Z), 426.61);
        // Intensity 0.035645 * 4096 rounds to 146.
        assert_eq!(view.get_f64(14, &DimId::Intensity), 146.0);
    }

    #[test]
    fn discards_missing_points_and_applies_transform() {
        // complex-transform.ptx has 12 grid cells; 8 are "0 0 0" gaps.
        let view = read_ptx("ptx/complex-transform.ptx", true);
        assert_eq!(view.len(), 4);

        assert!(close(view.get_f64(0, &DimId::X), -3.034408));
        assert!(close(view.get_f64(0, &DimId::Y), -3.173781));
        assert!(close(view.get_f64(0, &DimId::Z), -1.823750));
        // Intensity 0.494911 * 4096 rounds to 2027.
        assert_eq!(view.get_f64(0, &DimId::Intensity), 2027.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("Red")), 33.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("Green")), 38.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("Blue")), 24.0);
    }

    #[test]
    fn keeps_missing_points_when_discard_is_disabled() {
        let view = read_ptx("ptx/complex-transform.ptx", false);
        // All 12 grid cells are kept.
        assert_eq!(view.len(), 12);
    }

    #[test]
    fn reads_multiple_clouds_into_one_view() {
        // Two clouds (2x2 with a transform, then 4x1 identity) holding the
        // same four points; the transform makes them equal.
        let view = read_ptx("ptx/multiple-and-transform.ptx", true);
        assert_eq!(view.len(), 2 * 2 + 4 * 1);
        assert!(view.layout().dim(&DimId::from_name("Red")).is_none());

        for i in 0..4 {
            assert!(close(
                view.get_f64(i, &DimId::X),
                view.get_f64(i + 4, &DimId::X)
            ));
            assert!(close(
                view.get_f64(i, &DimId::Y),
                view.get_f64(i + 4, &DimId::Y)
            ));
            assert!(close(
                view.get_f64(i, &DimId::Z),
                view.get_f64(i + 4, &DimId::Z)
            ));
        }
    }
}
