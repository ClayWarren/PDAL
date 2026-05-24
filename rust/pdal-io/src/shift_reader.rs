//! `Read + Seek` adapter that adds a fixed offset to all stream positions.
//!
//! Used by `readers.nitf` so the LAS data embedded inside a NITF file
//! appears to the LAS reader as if it starts at position 0.

use std::io::{self, Read, Seek, SeekFrom};

pub struct ShiftReader<R> {
    inner: R,
    shift: u64,
}

impl<R: Read + Seek> ShiftReader<R> {
    pub fn new(mut inner: R, shift: u64) -> io::Result<Self> {
        inner.seek(SeekFrom::Start(shift))?;
        Ok(Self { inner, shift })
    }
}

impl<R: Read> Read for ShiftReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: Read + Seek> Seek for ShiftReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let mapped = match pos {
            SeekFrom::Start(p) => SeekFrom::Start(p + self.shift),
            SeekFrom::Current(o) => SeekFrom::Current(o),
            SeekFrom::End(o) => SeekFrom::End(o),
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
}
