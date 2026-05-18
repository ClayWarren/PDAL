//! `writers.sbet` -- Applanix (Trimble) SBET trajectory format.
//!
//! Port of `io/SbetWriter.cpp`. SBET is a binary format consisting of a
//! sequence of 17 double-precision (8-byte) floating point values per point.
//! All values are Little Endian.

use byteorder::{LittleEndian, WriteBytesExt};
use pdal_core::metadata::MetadataNode;
use pdal_core::options::Options;
use pdal_core::pipeline::Writer;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::StageError;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub struct SbetWriter {
    filename: String,
    angles_are_degrees: bool,
}

impl SbetWriter {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            angles_are_degrees: options.get_bool("angles_are_degrees", true),
        }
    }

    fn file_dimensions() -> &'static [DimId] {
        &[
            DimId::GpsTime,
            DimId::Y,
            DimId::X,
            DimId::Z,
            DimId::XVelocity,
            DimId::YVelocity,
            DimId::ZVelocity,
            DimId::Roll,
            DimId::Pitch,
            DimId::Azimuth,
            DimId::WanderAngle,
            DimId::XBodyAccel,
            DimId::YBodyAccel,
            DimId::ZBodyAccel,
            DimId::XBodyAngRate,
            DimId::YBodyAngRate,
            DimId::ZBodyAngRate,
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

impl Writer for SbetWriter {
    fn name(&self) -> &str {
        "writers.sbet"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "SbetWriter requires a filename option.".to_string(),
            ));
        }
        let file = File::create(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Couldn't open '{}' for writing.", self.filename)))?;
        let mut writer = BufWriter::new(file);

        let dims = Self::file_dimensions();
        let deg_to_rad = std::f64::consts::PI / 180.0;

        for view in views {
            for i in 0..view.len() {
                for dim in dims {
                    let mut val = view.get_f64(i, dim);
                    if self.angles_are_degrees && Self::is_angular(dim) {
                        val *= deg_to_rad;
                    }
                    writer
                        .write_f64::<LittleEndian>(val)
                        .map_err(|e| StageError(e.to_string()))?;
                }
            }
        }

        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("writers.sbet")
    }
}
