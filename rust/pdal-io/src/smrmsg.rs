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
    stream: Option<SmrmsgStreamState>,
}

struct SmrmsgStreamState {
    reader: BufReader<Box<dyn crate::source::ReadSeek>>,
    layout: Rc<PointLayout>,
    remaining: u64,
}

impl SmrmsgReader {
    pub fn new(options: &Options) -> Self {
        Self {
            filename: options.get_str("filename", ""),
            stream: None,
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

    fn point_size() -> u64 {
        (Self::file_dimensions().len() * std::mem::size_of::<f64>()) as u64
    }

    fn layout() -> Rc<PointLayout> {
        let mut layout = PointLayout::new();
        for dim in Self::file_dimensions() {
            layout.register(dim.clone(), DimType::F64);
        }
        Rc::new(layout)
    }

    fn append_record(
        view: &mut PointView,
        reader: &mut BufReader<Box<dyn crate::source::ReadSeek>>,
    ) -> Result<(), StageError> {
        let id = view.add_point();
        let dims = Self::file_dimensions();
        let mut buf = [0u8; 8];
        for dim in dims {
            reader
                .read_exact(&mut buf)
                .map_err(|e| StageError(e.to_string()))?;
            let val = (&buf[..]).read_f64::<LittleEndian>().unwrap();
            view.set_f64(id, dim, val);
        }
        Ok(())
    }

    fn stream_init(&mut self) -> Result<(), StageError> {
        if self.filename.is_empty() {
            return Err(StageError(
                "SmrmsgReader requires a filename option.".to_string(),
            ));
        }
        let point_size = Self::point_size();
        let (reader, file_size) = open_smrmsg_reader(&self.filename)?;
        if file_size == 0 || file_size % point_size != 0 {
            return Err(StageError("Invalid file size.".to_string()));
        }
        self.stream = Some(SmrmsgStreamState {
            reader,
            layout: Self::layout(),
            remaining: file_size / point_size,
        });
        Ok(())
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
        let point_size = Self::point_size();
        let (mut reader, file_size) = open_smrmsg_reader(&self.filename)?;
        if file_size == 0 || file_size % point_size != 0 {
            return Err(StageError("Invalid file size.".to_string()));
        }

        let mut view = PointView::new(Self::layout());
        for _ in 0..(file_size / point_size) {
            Self::append_record(&mut view, &mut reader)?;
        }

        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("readers.smrmsg")
    }

    fn reset(&mut self) {
        self.stream = None;
    }

    fn streamable(&self) -> bool {
        !self.filename.is_empty()
    }

    fn stream_next(&mut self, capacity: usize) -> Result<Option<PointView>, StageError> {
        if self.stream.is_none() {
            self.stream_init()?;
        }
        let state = self.stream.as_mut().expect("stream initialized above");
        if state.remaining == 0 {
            return Ok(None);
        }

        let take = (capacity.max(1) as u64).min(state.remaining);
        let mut view = PointView::new(Rc::clone(&state.layout));
        for _ in 0..take {
            Self::append_record(&mut view, &mut state.reader)?;
        }
        state.remaining -= take;
        Ok(Some(view))
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
    fn streaming_chunks_match_full_read() {
        let mut options = Options::new();
        options.add("filename", data_path("smrmsg/smrmsg.smrmsg"));

        let mut full_reader = SmrmsgReader::new(&options);
        let full = full_reader.read().unwrap().pop().unwrap();

        let mut stream_reader = SmrmsgReader::new(&options);
        assert!(stream_reader.streamable());
        let first = stream_reader.stream_next(10_000).unwrap().unwrap();
        let second = stream_reader.stream_next(10_000).unwrap().unwrap();
        let third = stream_reader.stream_next(10_000).unwrap().unwrap();
        assert!(stream_reader.stream_next(10_000).unwrap().is_none());

        assert_eq!(first.len(), 10_000);
        assert_eq!(second.len(), 10_000);
        assert_eq!(third.len(), full.len() - 20_000);
        assert_near(
            first.get_f64(0, &DimId::GpsTime),
            full.get_f64(0, &DimId::GpsTime),
        );
        assert_near(
            second.get_f64(0, &DimId::NorthPositionRMS),
            full.get_f64(10_000, &DimId::NorthPositionRMS),
        );
        assert_near(
            third.get_f64(third.len() - 1, &DimId::HeadingRMS),
            full.get_f64(full.len() - 1, &DimId::HeadingRMS),
        );
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
