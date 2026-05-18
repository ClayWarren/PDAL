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
        let mut reader = BufReader::new(file);

        let mut layout = PointLayout::new();
        for (dim, ty) in Self::file_dimensions() {
            layout.register(dim.clone(), *ty);
        }

        let mut view = PointView::new(Rc::new(layout));
        let dims = Self::file_dimensions();
        let mut buf = [0u8; 8];
        let rad_to_deg = 180.0 / std::f64::consts::PI;

        loop {
            let mut point_ok = true;
            let id = view.add_point();
            for (dim, _ty) in dims {
                if reader.read_exact(&mut buf).is_err() {
                    point_ok = false;
                    break;
                }
                let mut val = (&buf[..]).read_f64::<LittleEndian>().unwrap();
                if self.angles_as_degrees && Self::is_angular(dim) {
                    val *= rad_to_deg;
                }
                view.set_f64(id, dim, val);
            }
            if !point_ok {
                let current_len = view.len();
                if current_len > 0 {
                    view.truncate(current_len - 1);
                }
                break;
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.sbet")
    }
}
