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
        for dim in Self::file_dimensions() {
            layout.register(dim.clone(), DimType::F64);
        }

        let mut view = PointView::new(Rc::new(layout));
        let dims = Self::file_dimensions();
        let mut buf = [0u8; 8];

        for _ in 0..(file_size / point_size) {
            let id = view.add_point();
            for dim in dims {
                reader
                    .read_exact(&mut buf)
                    .map_err(|e| StageError(e.to_string()))?;
                let val = (&buf[..]).read_f64::<LittleEndian>().unwrap();
                view.set_f64(id, dim, val);
            }
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.smrmsg")
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
    fn reads_existing_smrmsg_fixture() {
        let mut options = Options::new();
        options.add("filename", data_path("smrmsg/smrmsg.smrmsg"));
        let views = SmrmsgReader::new(&options).read().unwrap();

        assert_eq!(views.len(), 1);
        let view = &views[0];
        assert_eq!(view.len(), 21902);
        assert_near(view.get_f64(0, &DimId::GpsTime), 536258.0);
        assert_near(view.get_f64(0, &DimId::NorthPositionRMS), 0.056279);
        assert_near(view.get_f64(0, &DimId::EastPositionRMS), 0.057791);
        assert_near(view.get_f64(0, &DimId::DownPositionRMS), 0.070774);
        assert_near(view.get_f64(0, &DimId::RollRMS), 0.236985);
        assert_near(view.get_f64(1, &DimId::GpsTime), 536259.0);
        assert_near(view.get_f64(2, &DimId::HeadingRMS), 3.010406);
    }

    #[test]
    fn rejects_empty_or_missing_smrmsg_input() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().to_string());

        assert!(SmrmsgReader::new(&options).read().is_err());
        assert!(SmrmsgReader::new(&Options::new()).read().is_err());
    }
}
