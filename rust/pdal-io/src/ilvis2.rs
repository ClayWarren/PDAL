//! `readers.ilvis2` -- NASA LVIS Level-2 (ILVIS2) ASCII lidar format.
//!
//! Port of `io/Ilvis2Reader.cpp`. An ILVIS2 file has two header lines followed
//! by data lines of twelve space-separated fields:
//!
//! ```text
//! LVIS_LFID SHOTNUMBER TIME
//! LONGITUDE_CENTROID LATITUDE_CENTROID ELEVATION_CENTROID
//! LONGITUDE_LOW LATITUDE_LOW ELEVATION_LOW
//! LONGITUDE_HIGH LATITUDE_HIGH ELEVATION_HIGH
//! ```
//!
//! Each shot carries a low and a high elevation. The `mapping` option chooses
//! which to emit: `low`, `high`, or `all` (the default). For `all`, a shot
//! yields a second point at the high elevation when it differs from the low.
//! `X`/`Y`/`Z` are set from the chosen mapping's longitude/latitude/elevation;
//! longitudes are normalized to `(-180, 180]`.
//!
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::fs;
use std::path::Path;
use std::rc::Rc;

/// Which elevation(s) of each waveform shot to emit as points.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mapping {
    Low,
    High,
    All,
    Invalid,
}

/// The twelve ILVIS2 columns, in file order.
const COLUMNS: [&str; 12] = [
    "LvisLfid",
    "ShotNumber",
    "GpsTime",
    "LongitudeCentroid",
    "LatitudeCentroid",
    "ElevationCentroid",
    "LongitudeLow",
    "LatitudeLow",
    "ElevationLow",
    "LongitudeHigh",
    "LatitudeHigh",
    "ElevationHigh",
];

/// Reader for the NASA LVIS Level-2 (ILVIS2) ASCII format.
pub struct Ilvis2Reader {
    filename: String,
    metadata_filename: String,
    mapping: Mapping,
    metadata: MetadataNode,
}

impl Ilvis2Reader {
    pub fn new(options: &Options) -> Self {
        // An absent option keeps PDAL's `all` default; an unrecognized value
        // maps to `invalid`, matching PDAL's enum stream operator.
        let mapping = match options.get_str("mapping", "") {
            text if text.is_empty() => Mapping::All,
            text => match text.to_uppercase().as_str() {
                "LOW" => Mapping::Low,
                "HIGH" => Mapping::High,
                "ALL" => Mapping::All,
                _ => Mapping::Invalid,
            },
        };
        Self {
            filename: options.get_str("filename", ""),
            metadata_filename: options.get_str("metadata", ""),
            mapping,
            metadata: MetadataNode::new("readers.ilvis2"),
        }
    }
}

impl Reader for Ilvis2Reader {
    fn name(&self) -> &str {
        "readers.ilvis2"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "Ilvis2Reader requires a filename option.".to_string(),
            ));
        }
        let text = fs::read_to_string(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Unable to open file '{}'.", self.filename)))?;
        let lines: Vec<&str> = text.lines().collect();

        let mut layout = PointLayout::new();
        for name in COLUMNS {
            layout.register(DimId::from_name(name), DimType::F64);
        }
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));

        // The first two lines are the file header.
        for (idx, line) in lines.iter().enumerate().skip(2) {
            let fields = split_fields(line);
            if fields.len() != 12 {
                return Err(StageError(format!(
                    "Invalid format for line {}.  Expected 12 fields, got {}.",
                    idx + 1,
                    fields.len()
                )));
            }

            let mut values = [0.0f64; 12];
            for (i, field) in fields.iter().enumerate() {
                values[i] = field.parse::<f64>().map_err(|_| {
                    StageError(format!(
                        "Unable to convert {}, {}, to double",
                        COLUMNS[i], field
                    ))
                })?;
            }
            // The three longitude columns are normalized to (-180, 180].
            for i in [3, 6, 9] {
                values[i] = normalize_longitude(values[i]);
            }

            match self.mapping {
                Mapping::Low => emit(&mut view, &values, Mapping::Low),
                Mapping::High => emit(&mut view, &values, Mapping::High),
                Mapping::All => {
                    emit(&mut view, &values, Mapping::Low);
                    // A second point at the high elevation when it differs.
                    if values[8] != values[11] {
                        emit(&mut view, &values, Mapping::High);
                    }
                }
                // PDAL still emits a (data-less) point for an invalid mapping.
                Mapping::Invalid => {
                    view.add_point();
                }
            }
        }

        if !self.metadata_filename.is_empty() {
            self.metadata =
                crate::ilvis2_metadata::read_metadata_file(Path::new(&self.metadata_filename))?;
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        self.metadata.clone()
    }
}

/// Append one point, taking `X`/`Y`/`Z` from the low or high triple.
fn emit(view: &mut PointView, values: &[f64; 12], which: Mapping) {
    let point = view.add_point();
    for (i, name) in COLUMNS.iter().enumerate() {
        view.set_f64(point, &DimId::from_name(name), values[i]);
    }
    // Low columns start at index 6, high columns at index 9.
    let base = if which == Mapping::High { 9 } else { 6 };
    view.set_f64(point, &DimId::X, values[base]);
    view.set_f64(point, &DimId::Y, values[base + 1]);
    view.set_f64(point, &DimId::Z, values[base + 2]);
}

/// Normalize a longitude into `(-180, 180]` (PDAL's `normalizeLongitude`).
fn normalize_longitude(longitude: f64) -> f64 {
    let mut longitude = longitude % 360.0;
    if longitude <= -180.0 {
        longitude += 360.0;
    } else if longitude > 180.0 {
        longitude -= 360.0;
    }
    longitude
}

/// Split a line on spaces, dropping empty tokens (PDAL's `split2`).
fn split_fields(line: &str) -> Vec<&str> {
    line.split(' ')
        .map(|field| field.trim_end_matches('\r'))
        .filter(|field| !field.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn data_path(path: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        root.join("test/data").join(path).display().to_string()
    }

    fn read_ilvis2(path: &str, mapping: Option<&str>) -> PointView {
        let mut options = Options::new();
        options.add("filename", data_path(path));
        if let Some(mapping) = mapping {
            options.add("mapping", mapping);
        }
        let mut reader = Ilvis2Reader::new(&options);
        let mut views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        views.pop().unwrap()
    }

    fn gps_time(view: &PointView, idx: u64) -> f64 {
        view.get_f64(idx, &DimId::from_name("GpsTime"))
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9
    }

    fn metadata_value<'a>(
        metadata: &'a MetadataNode,
        name: &str,
    ) -> &'a pdal_core::metadata::MetadataValue {
        metadata
            .find_child(name)
            .and_then(MetadataNode::value)
            .unwrap()
    }

    #[test]
    fn reads_all_mapping_with_resampled_high_point() {
        let view = read_ilvis2("ilvis2/ILVIS2_TEST_FILE.TXT", None);
        // Three shots; the third has differing low/high elevations.
        assert_eq!(view.len(), 4);

        // Point 0: line 1, low elevation.
        assert!(close(view.get_f64(0, &DimId::X), -58.785213));
        assert_eq!(view.get_f64(0, &DimId::Y), 78.307672);
        assert_eq!(view.get_f64(0, &DimId::Z), 1956.777);
        assert_eq!(gps_time(&view, 0), 42504.48313);

        // Point 3: line 3, resampled high elevation.
        assert!(close(view.get_f64(3, &DimId::X), -58.78459));
        assert_eq!(view.get_f64(3, &DimId::Y), 78.307512);
        assert_eq!(view.get_f64(3, &DimId::Z), 2956.667);
        assert_eq!(gps_time(&view, 3), 42504.48712);
    }

    #[test]
    fn reads_high_mapping() {
        let view = read_ilvis2("ilvis2/ILVIS2_TEST_FILE.TXT", Some("high"));
        assert_eq!(view.len(), 3);

        assert!(close(view.get_f64(0, &DimId::X), -58.785213));
        assert_eq!(view.get_f64(0, &DimId::Y), 78.307672);
        assert_eq!(view.get_f64(0, &DimId::Z), 1956.777);
        // The third shot's high elevation.
        assert_eq!(view.get_f64(2, &DimId::Z), 2956.667);
    }

    #[test]
    fn reads_low_mapping() {
        let view = read_ilvis2("ilvis2/ILVIS2_TEST_FILE.TXT", Some("low"));
        assert_eq!(view.len(), 3);
        // The third shot's low elevation (no resampled high point).
        assert_eq!(view.get_f64(2, &DimId::Z), 1956.667);
    }

    #[test]
    fn reads_the_larger_fixture() {
        let view = read_ilvis2("ilvis2/ILVIS2_GL2009_0414_R1401_042504.TXT", None);
        // 998 data lines, each emitting at least one point.
        assert!(view.len() >= 998);
    }

    #[test]
    fn reads_metadata_sidecar_when_requested() {
        let mut options = Options::new();
        options.add("filename", data_path("ilvis2/ILVIS2_TEST_FILE.TXT"));
        options.add("metadata", data_path("ilvis2/ILVIS2_TEST_FILE.TXT.xml"));
        let mut reader = Ilvis2Reader::new(&options);

        let views = reader.read().unwrap();
        let metadata = reader.metadata();

        assert_eq!(views[0].len(), 4);
        assert_eq!(
            metadata_value(&metadata, "GranuleUR").as_string(),
            "SC:ILVIS2.001:51203496"
        );
        assert_eq!(metadata_value(&metadata, "DbID").as_i64(), 51203496);
        assert!(metadata_value(&metadata, "ConvexHull")
            .as_string()
            .starts_with("POLYGON"));
    }

    #[test]
    fn rejects_a_line_without_twelve_fields() {
        let path =
            std::env::temp_dir().join(format!("pdal-rust-ilvis2-{}-bad.txt", std::process::id()));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"# header one\n# header two\n1 2 3 4 5 6 7 8 9 10 11\n")
            .unwrap();

        let mut options = Options::new();
        options.add("filename", path.display().to_string());
        assert!(Ilvis2Reader::new(&options).read().is_err());
    }
}
