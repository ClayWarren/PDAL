//! `readers.pts` -- Leica PTS ASCII point format.
//!
//! Port of `io/PtsReader.cpp`. The first line of a PTS file is the expected
//! point count. The field count of the first data line fixes the dimension
//! set:
//!
//! - 3 fields: `X Y Z`
//! - 4 fields: `X Y Z Intensity`
//! - 7 fields: `X Y Z Intensity Red Green Blue`
//!
//! PTS intensity is stored `-2048..2047`; it is shifted into `0..4095` on
//! read, matching PDAL. Lines whose field count does not match the detected
//! dimension set are skipped, and no more than the declared point count is
//! read.

use crate::source;
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::rc::Rc;

/// Reader for the Leica PTS ASCII point format.
pub struct PtsReader {
    filename: String,
}

impl PtsReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
        }
    }
}

impl Reader for PtsReader {
    fn name(&self) -> &str {
        "readers.pts"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "PtsReader requires a filename option.".to_string(),
            ));
        }

        let text = source::read_to_string(&self.filename)
            .map_err(|_| StageError(format!("Unable to open file '{}'.", self.filename)))?;
        let lines: Vec<&str> = text.lines().collect();

        // The first line is the expected point count.
        let point_count: u64 = lines
            .first()
            .and_then(|line| line.trim().parse().ok())
            .ok_or_else(|| {
                StageError(format!(
                    "Unable to read expected point count at top of the file '{}'.",
                    self.filename
                ))
            })?;

        // The first data line fixes the dimension set.
        let first_data = lines
            .get(1)
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

        for line in lines.iter().skip(1) {
            if view.len() >= point_count {
                break;
            }
            if line.is_empty() {
                continue;
            }
            let fields = split_fields(line);
            // Lines with an unexpected field count are skipped, as in PDAL.
            if fields.len() != dims.len() {
                continue;
            }

            let point = view.add_point();
            for (idx, (field, dim)) in fields.iter().zip(&dims).enumerate() {
                let mut value = field.parse::<f64>().unwrap_or(0.0);
                // PTS intensity (field 3) is -2048..2047; map to 0..4095.
                if idx == 3 {
                    value += 2048.0;
                }
                view.set_f64(point, dim, value);
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.pts")
    }
}

/// The fixed PTS dimension set for a given field count.
fn dims_for(field_count: usize) -> Option<Vec<DimId>> {
    match field_count {
        3 => Some(vec![DimId::X, DimId::Y, DimId::Z]),
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

/// PDAL's default storage type for each PTS dimension.
fn dim_type(dim: &DimId) -> DimType {
    match dim {
        DimId::X | DimId::Y | DimId::Z => DimType::F64,
        _ => DimType::U16,
    }
}

/// Split a PTS line on spaces, dropping empty tokens (PDAL's `split2`).
fn split_fields(line: &str) -> Vec<&str> {
    line.split(' ')
        .map(|field| field.trim_end_matches('\r'))
        .filter(|field| !field.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    fn data_path(path: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        root.join("test/data").join(path).display().to_string()
    }

    fn read_pts(path: &str) -> PointView {
        let mut options = Options::new();
        options.add("filename", data_path(path));
        let mut reader = PtsReader::new(&options);
        let mut views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        views.pop().unwrap()
    }

    fn temp_file(name: &str, contents: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("pdal-rust-pts-{}-{name}", std::process::id()));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        path.display().to_string()
    }

    #[test]
    fn reads_seven_dimension_pts() {
        let view = read_pts("pts/test.pts");
        assert_eq!(view.len(), 19);

        assert_eq!(view.get_f64(0, &DimId::X), 3.980972);
        assert_eq!(view.get_f64(0, &DimId::Y), -2.006119);
        assert_eq!(view.get_f64(0, &DimId::Z), -0.010086);
        // Intensity is shifted by +2048: -1035 -> 1013.
        assert_eq!(view.get_f64(0, &DimId::Intensity), 1013.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("Red")), 97.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("Green")), 59.0);
        assert_eq!(view.get_f64(0, &DimId::from_name("Blue")), 38.0);
    }

    #[test]
    fn reads_three_dimension_pts_and_honors_the_point_count() {
        // bunny_8.pts declares 8 points but holds 9 data lines; only the
        // declared count is read.
        let view = read_pts("pts/bunny_8.pts");
        assert_eq!(view.len(), 8);

        assert_eq!(view.get_f64(0, &DimId::X), -0.037829);
        assert_eq!(view.get_f64(0, &DimId::Y), 0.12794);
        assert_eq!(view.get_f64(0, &DimId::Z), 0.004474);
    }

    #[test]
    fn reads_four_dimension_pts() {
        let view = read_pts("pts/site_56_8.pts");
        assert_eq!(view.len(), 8);

        assert_eq!(view.get_f64(0, &DimId::X), 6691.797611);
        assert_eq!(view.get_f64(0, &DimId::Y), 17.347517);
        assert_eq!(view.get_f64(0, &DimId::Z), 1203.033447);
        // -255 + 2048.
        assert_eq!(view.get_f64(0, &DimId::Intensity), 1793.0);
    }

    #[test]
    fn skips_lines_with_an_unexpected_field_count() {
        let path = temp_file("badrows.pts", "3\n1 2 3\n4 5\n6 7 8 9\n10 11 12\n");
        let mut options = Options::new();
        options.add("filename", path);
        let view = PtsReader::new(&options).read().unwrap().pop().unwrap();

        // The 2-field and 4-field lines are dropped.
        assert_eq!(view.len(), 2);
        assert_eq!(view.get_f64(1, &DimId::X), 10.0);
    }

    #[test]
    fn rejects_a_missing_point_count() {
        let path = temp_file("nocount.pts", "x y z\n1 2 3\n");
        let mut options = Options::new();
        options.add("filename", path);
        assert!(PtsReader::new(&options).read().is_err());
    }

    #[test]
    fn rejects_an_invalid_first_record() {
        let path = temp_file("badfields.pts", "1\n1 2 3 4 5\n");
        let mut options = Options::new();
        options.add("filename", path);
        assert!(PtsReader::new(&options).read().is_err());
    }
}
