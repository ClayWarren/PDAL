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
use std::io::{BufReader, Read};
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
        let point_size = (Self::file_dimensions().len() * std::mem::size_of::<f64>()) as u64;
        let (mut reader, file_size) = open_smrmsg_reader(&self.filename)?;
        if file_size == 0 || file_size % point_size != 0 {
            return Err(StageError("Invalid file size.".to_string()));
        }

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

fn open_smrmsg_reader(
    filename: &str,
) -> Result<(BufReader<Box<dyn crate::source::ReadSeek>>, u64), StageError> {
    let (file, len) = crate::source::open_seek_len(filename)
        .map_err(|err| StageError(format!("Couldn't open '{filename}': {err}")))?;
    Ok((BufReader::new(file), len))
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::WriteBytesExt;
    use pdal_core::pipeline::Reader;
    use std::io::Write;

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

    #[test]
    fn rejects_partial_record_smrmsg_input() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(&[0, 1, 2, 3]).unwrap();
        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().to_string());

        let err = match SmrmsgReader::new(&options).read() {
            Ok(_) => panic!("partial SMRMSG record should fail"),
            Err(err) => err,
        };
        assert_eq!(err.0, "Invalid file size.");
    }

    #[test]
    fn reads_single_record_and_reports_metadata() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        let values = [1.0, 0.1, 0.2, 0.3, 4.0, 5.0, 6.0, 0.01, 0.02, 0.03];
        for value in values {
            temp.write_f64::<LittleEndian>(value).unwrap();
        }

        let mut options = Options::new();
        options.add("filename", temp.path().to_string_lossy().to_string());
        let mut reader = SmrmsgReader::new(&options);

        assert_eq!(reader.name(), "readers.smrmsg");
        assert_eq!(reader.metadata().name(), "readers.smrmsg");

        let views = reader.read().unwrap();
        assert_eq!(views.len(), 1);
        let view = &views[0];
        assert_eq!(view.len(), 1);
        assert_eq!(view.get_f64(0, &DimId::GpsTime), 1.0);
        assert_eq!(view.get_f64(0, &DimId::NorthPositionRMS), 0.1);
        assert_eq!(view.get_f64(0, &DimId::HeadingRMS), 0.03);
    }
}
