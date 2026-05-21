//! `readers.sbet` -- Applanix (Trimble) SBET trajectory format.
//!
//! Port of `io/SbetReader.cpp`. SBET is a binary format consisting of a
//! sequence of 17 double-precision (8-byte) floating point values per point.
//! All values are Little Endian.

use byteorder::{LittleEndian, ReadBytesExt};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Reader;
use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::StageError;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::rc::Rc;

pub struct SbetReader {
    filename: String,
    angles_as_degrees: bool,
}

impl SbetReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            angles_as_degrees: options.get_bool("angles_as_degrees", true),
        }
    }

    fn file_dimensions() -> &'static [(DimId, DimType)] {
        &[
            (DimId::GpsTime, DimType::F64),
            (DimId::Y, DimType::F64),
            (DimId::X, DimType::F64),
            (DimId::Z, DimType::F64),
            (DimId::XVelocity, DimType::F64),
            (DimId::YVelocity, DimType::F64),
            (DimId::ZVelocity, DimType::F64),
            (DimId::Roll, DimType::F32),  // PDAL default for Roll
            (DimId::Pitch, DimType::F32), // PDAL default for Pitch
            (DimId::Azimuth, DimType::F64),
            (DimId::WanderAngle, DimType::F64),
            (DimId::XBodyAccel, DimType::F64),
            (DimId::YBodyAccel, DimType::F64),
            (DimId::ZBodyAccel, DimType::F64),
            (DimId::XBodyAngRate, DimType::F64),
            (DimId::YBodyAngRate, DimType::F64),
            (DimId::ZBodyAngRate, DimType::F64),
        ]
    }

    fn is_angular(dim: &DimId) -> bool {
        matches!(
            dim,
            DimId::X
                | DimId::Y
                | DimId::Roll
                | DimId::Pitch
                | DimId::Azimuth
                | DimId::WanderAngle
                | DimId::XBodyAngRate
                | DimId::YBodyAngRate
                | DimId::ZBodyAngRate
        )
    }
}

impl Reader for SbetReader {
    fn name(&self) -> &str {
        "readers.sbet"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "SbetReader requires a filename option.".to_string(),
            ));
        }
        let file = File::open(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Couldn't open '{}'.", self.filename)))?;
        let point_size = (Self::file_dimensions().len() * std::mem::size_of::<f64>()) as u64;
        let file_size = file
            .metadata()
            .map_err(|e| StageError(e.to_string()))?
            .len();
        if file_size == 0 || file_size % point_size != 0 {
            return Err(StageError("Invalid file size.".to_string()));
        }
        let mut reader = BufReader::new(file);

        let mut layout = PointLayout::new();
        for (dim, ty) in Self::file_dimensions() {
            layout.register(dim.clone(), *ty);
        }

        let mut view = PointView::new(Rc::new(layout));
        let dims = Self::file_dimensions();
        let mut buf = [0u8; 8];
        let rad_to_deg = 180.0 / std::f64::consts::PI;

        for _ in 0..(file_size / point_size) {
            let id = view.add_point();
            for (dim, _ty) in dims {
                reader
                    .read_exact(&mut buf)
                    .map_err(|e| StageError(e.to_string()))?;
                let mut val = (&buf[..]).read_f64::<LittleEndian>().unwrap();
                if self.angles_as_degrees && Self::is_angular(dim) {
                    val *= rad_to_deg;
                }
                view.set_f64(id, dim, val);
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.sbet")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::pipeline::Reader;

    fn data_path(path: &str) -> String {
        format!("{}/../../test/data/{path}", env!("CARGO_MANIFEST_DIR"))
    }

    fn assert_near(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn reads_existing_two_point_fixture_as_degrees() {
        let mut options = Options::new();
        options.add("filename", data_path("sbet/2-points.sbet"));
        let views = SbetReader::new(&options).read().unwrap();

        assert_eq!(views.len(), 1);
        let view = &views[0];
        assert_eq!(view.len(), 2);
        assert_near(view.get_f64(0, &DimId::GpsTime), 1.516_310_028_360_71e5);
        assert_near(view.get_f64(0, &DimId::Y), 32.545149);
        assert_near(view.get_f64(0, &DimId::X), -116.978180);
        assert_near(view.get_f64(0, &DimId::Z), 107.715_295_329_656);
        assert_near(view.get_f64(0, &DimId::Roll), -1.611964);
        assert_near(view.get_f64(0, &DimId::Pitch), -1.392233);
        assert_near(view.get_f64(0, &DimId::Azimuth), 174.567247);
        assert_near(view.get_f64(1, &DimId::GpsTime), 1.516310078318641e5);
        assert_near(view.get_f64(1, &DimId::Y), 32.545216);
        assert_near(view.get_f64(1, &DimId::Azimuth), 174.587752);
    }

    #[test]
    fn can_leave_angular_values_as_radians() {
        let mut options = Options::new();
        options.add("filename", data_path("sbet/2-points.sbet"));
        options.add("angles_as_degrees", false);
        let views = SbetReader::new(&options).read().unwrap();
        let view = &views[0];

        assert_near(view.get_f64(0, &DimId::Y), 0.5680211852972264);
        assert_near(view.get_f64(0, &DimId::X), -2.041_654_392_303_94);
        assert_near(view.get_f64(0, &DimId::Roll), -0.02813407149321339);
        assert_near(view.get_f64(0, &DimId::Azimuth), 3.046773230278662);
    }

    #[test]
    fn rejects_bad_sbet_file_sizes() {
        let mut options = Options::new();
        options.add("filename", data_path("sbet/badfile.sbet"));

        assert!(SbetReader::new(&options).read().is_err());
        assert!(SbetReader::new(&Options::new()).read().is_err());
    }
}
