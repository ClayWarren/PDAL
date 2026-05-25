//! GDAL VSI byte-range helpers for local and remote object-store style paths.

use std::ffi::CString;
use std::io;
use std::os::raw::c_void;
use std::ptr;

const SEEK_SET: i32 = 0;
const SEEK_CUR: i32 = 1;
const SEEK_END: i32 = 2;

#[derive(Debug)]
pub struct VsiFile {
    handle: *mut gdal_sys::VSILFILE,
}

impl VsiFile {
    pub fn open(path: &str) -> Result<Self, String> {
        let path_c = CString::new(path).map_err(|e| e.to_string())?;
        let mode_c = CString::new("rb").expect("static mode has no interior NUL");
        unsafe {
            let handle = gdal_sys::VSIFOpenL(path_c.as_ptr(), mode_c.as_ptr());
            if handle.is_null() {
                return Err(format!("Failed to open VSI path: {path}"));
            }
            Ok(Self { handle })
        }
    }

    pub fn len(&mut self) -> Result<u64, String> {
        self.seek(0, SEEK_END)?;
        let len = unsafe { gdal_sys::VSIFTellL(self.handle) };
        self.seek(0, SEEK_SET)?;
        Ok(len as u64)
    }

    pub fn is_empty(&mut self) -> Result<bool, String> {
        Ok(self.len()? == 0)
    }

    pub fn read_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>, String> {
        self.seek(offset, SEEK_SET)?;
        let mut data = vec![0; len];
        let read =
            unsafe { gdal_sys::VSIFReadL(data.as_mut_ptr().cast::<c_void>(), 1, len, self.handle) };
        data.truncate(read);
        Ok(data)
    }

    pub fn read_exact_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>, String> {
        let data = self.read_at(offset, len)?;
        if data.len() == len {
            Ok(data)
        } else {
            Err(format!(
                "Short VSI read at offset {offset}: requested {len} bytes, got {}",
                data.len()
            ))
        }
    }

    fn seek(&mut self, offset: u64, whence: i32) -> Result<(), String> {
        let result = unsafe { gdal_sys::VSIFSeekL(self.handle, offset, whence) };
        if result == 0 {
            Ok(())
        } else {
            Err(format!("VSI seek failed at offset {offset}"))
        }
    }
}

impl io::Read for VsiFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = buf.len();
        let read =
            unsafe { gdal_sys::VSIFReadL(buf.as_mut_ptr().cast::<c_void>(), 1, len, self.handle) };
        Ok(read)
    }
}

impl io::Seek for VsiFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let (offset, whence) = match pos {
            io::SeekFrom::Start(n) => (n as i64, SEEK_SET),
            io::SeekFrom::Current(n) => (n, SEEK_CUR),
            io::SeekFrom::End(n) => (n, SEEK_END),
        };
        let result = unsafe { gdal_sys::VSIFSeekL(self.handle, offset as u64, whence) };
        if result != 0 {
            return Err(io::Error::other(format!(
                "VSI seek failed (offset={offset}, whence={whence})"
            )));
        }
        let position = unsafe { gdal_sys::VSIFTellL(self.handle) };
        Ok(position as u64)
    }
}

impl Drop for VsiFile {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                gdal_sys::VSIFCloseL(self.handle);
            }
            self.handle = ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn vsi_reads_local_ranges() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"abcdefghijklmnopqrstuvwxyz").unwrap();

        let mut vsi = VsiFile::open(file.path().to_str().unwrap()).unwrap();
        assert_eq!(vsi.len().unwrap(), 26);
        assert_eq!(vsi.read_exact_at(2, 4).unwrap(), b"cdef");
        assert_eq!(vsi.read_at(24, 8).unwrap(), b"yz");
    }

    #[test]
    fn vsi_reports_missing_paths() {
        let err = VsiFile::open("/no/such/pdal-vsi-file").unwrap_err();
        assert!(err.contains("Failed to open VSI path"));
    }

    #[test]
    #[ignore = "network smoke for GDAL /vsicurl/ range reads"]
    fn vsi_reads_remote_ranges() {
        let url = "/vsicurl/https://github.com/PDAL/data/raw/refs/heads/main/autzen/autzen-classified.copc.laz";
        let mut vsi = VsiFile::open(url).unwrap();
        assert_eq!(vsi.read_exact_at(0, 4).unwrap(), b"LASF");
    }
}
