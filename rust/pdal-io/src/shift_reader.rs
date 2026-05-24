//! `Read + Seek` adapter that adds a fixed offset to all stream positions.
//!
//! Used by `readers.nitf` so the LAS data embedded inside a NITF file
//! appears to the LAS reader as if it starts at position 0.

use std::io::{self, Read, Seek, SeekFrom};

pub struct ShiftReader<R> {
    inner: R,
    shift: u64,
    length: Option<u64>,
}

impl<R: Read + Seek> ShiftReader<R> {
    pub fn new(mut inner: R, shift: u64) -> io::Result<Self> {
        inner.seek(SeekFrom::Start(shift))?;
        Ok(Self {
            inner,
            shift,
            length: None,
        })
    }

    pub fn with_length(mut inner: R, shift: u64, length: u64) -> io::Result<Self> {
        inner.seek(SeekFrom::Start(shift))?;
        Ok(Self {
            inner,
            shift,
            length: Some(length),
        })
    }
}

impl<R: Read + Seek> Read for ShiftReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Some(length) = self.length else {
            return self.inner.read(buf);
        };

        let position = self.stream_position()?;
        if position >= length {
            return Ok(0);
        }

        let limit = buf.len().min((length - position) as usize);
        self.inner.read(&mut buf[..limit])
    }
}

impl<R: Read + Seek> Seek for ShiftReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let mapped = match pos {
            SeekFrom::Start(p) => SeekFrom::Start(p + self.shift),
            SeekFrom::Current(o) => SeekFrom::Current(o),
            SeekFrom::End(o) => {
                if let Some(length) = self.length {
                    let end = self.shift.checked_add(length).ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "shifted stream end overflow")
                    })?;
                    let mapped = if o >= 0 {
                        end.checked_add(o as u64)
                    } else {
                        end.checked_sub(o.unsigned_abs())
                    }
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "invalid shifted seek")
                    })?;
                    SeekFrom::Start(mapped)
                } else {
                    SeekFrom::End(o)
                }
            }
        };
        let actual = self.inner.seek(mapped)?;
        Ok(actual.saturating_sub(self.shift))
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        let actual = self.inner.stream_position()?;
        Ok(actual.saturating_sub(self.shift))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn cursor(data: &[u8]) -> Cursor<Vec<u8>> {
        Cursor::new(data.to_vec())
    }

    #[test]
    fn reads_starting_after_shift() {
        let mut r = ShiftReader::new(cursor(b"PREFIX_DATA"), 7).unwrap();
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"DATA");
    }

    #[test]
    fn seek_from_start_is_relative_to_shift() {
        let mut r = ShiftReader::new(cursor(b"01234ABCDEF"), 5).unwrap();
        r.seek(SeekFrom::Start(2)).unwrap();
        let mut buf = [0u8; 3];
        r.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"CDE");
        assert_eq!(r.stream_position().unwrap(), 5);
    }

    #[test]
    fn seek_from_current_is_relative_to_current() {
        let mut r = ShiftReader::new(cursor(b"01234ABCDEF"), 5).unwrap();
        r.seek(SeekFrom::Current(2)).unwrap();
        let mut buf = [0u8; 2];
        r.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"CD");
    }

    #[test]
    fn seek_from_end_is_relative_to_payload_when_length_is_known() {
        let mut r = ShiftReader::with_length(cursor(b"01234ABCDEFTRAILER"), 5, 6).unwrap();
        r.seek(SeekFrom::End(-2)).unwrap();
        let mut buf = [0u8; 4];
        let count = r.read(&mut buf).unwrap();
        assert_eq!(count, 2);
        assert_eq!(&buf[..count], b"EF");
        assert_eq!(r.stream_position().unwrap(), 6);
    }
}
