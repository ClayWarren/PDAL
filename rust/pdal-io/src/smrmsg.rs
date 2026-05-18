//! `readers.smrmsg` -- SBET RMS message format.
//!
//! Port of `io/SbetSmrmsgReader.cpp`. SMRMSG is a binary format consisting of
//! 10 double-precision (8-byte) floating point values per point.
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

pub struct SmrmsgReader {
    filename: String,
}

impl SmrmsgReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
        }
    }

    fn file_dimensions() -> &'static [DimId] {
        &[
            DimId::GpsTime,
            DimId::NorthPositionRMS,
            DimId::EastPositionRMS,
            DimId::DownPositionRMS,
            DimId::NorthVelocityRMS,
            DimId::EastVelocityRMS,
            DimId::DownVelocityRMS,
            DimId::RollRMS,
            DimId::PitchRMS,
            DimId::HeadingRMS,
        ]
    }
}

impl Reader for SmrmsgReader {
    fn name(&self) -> &str {
        "readers.smrmsg"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "SmrmsgReader requires a filename option.".to_string(),
            ));
        }
        let file = File::open(Path::new(&self.filename))
            .map_err(|_| StageError(format!("Couldn't open '{}'.", self.filename)))?;
        let mut reader = BufReader::new(file);

        let mut layout = PointLayout::new();
        for dim in Self::file_dimensions() {
            layout.register(dim.clone(), DimType::F64);
        }

        let mut view = PointView::new(Rc::new(layout));
        let dims = Self::file_dimensions();
        let mut buf = [0u8; 8];

        loop {
            let mut point_ok = true;
            let id = view.add_point();
            for dim in dims {
                if reader.read_exact(&mut buf).is_err() {
                    point_ok = false;
                    break;
                }
                let val = (&buf[..]).read_f64::<LittleEndian>().unwrap();
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
        MetadataNode::new("readers.smrmsg")
    }
}
