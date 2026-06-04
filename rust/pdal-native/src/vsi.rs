//! GDAL VSI byte-range helpers for local and remote object-store style paths.

use std::ffi::CString;
use std::io;
use std::os::raw::c_void;
use std::ptr;

const SEEK_SET: i32 = 0;
const SEEK_CUR: i32 = 1;
const SEEK_END: i32 = 2;

pub fn write_mem_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    let path_c = CString::new(path).map_err(|e| e.to_string())?;
    unsafe {
        let data = gdal_sys::VSIMalloc(bytes.len()) as *mut u8;
        if data.is_null() {
            return Err(format!("Failed to allocate VSI memory file: {path}"));
        }
        ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
        let handle = gdal_sys::VSIFileFromMemBuffer(path_c.as_ptr(), data, bytes.len() as u64, 1);
        if handle.is_null() {
            gdal_sys::VSIFree(data.cast());
            return Err(format!("Failed to create VSI memory file: {path}"));
        }
        if gdal_sys::VSIFCloseL(handle) != 0 {
            return Err(format!("Failed to close VSI memory file: {path}"));
        }
    }
    Ok(())
}

pub fn unlink(path: &str) -> Result<(), String> {
    let path_c = CString::new(path).map_err(|e| e.to_string())?;
    let result = unsafe { gdal_sys::VSIUnlink(path_c.as_ptr()) };
    if result != 0 {
        return Err(format!("Failed to unlink VSI path: {path}"));
    }
    Ok(())
}

#[derive(Debug)]
pub struct VsiFile {
    handle: *mut gdal_sys::VSILFILE,
    _path_options: Option<PathSpecificOptions>,
}

impl VsiFile {
    pub fn open(path: &str) -> Result<Self, String> {
        Self::open_with_headers(path, &[])
    }

    pub fn open_with_headers(path: &str, headers: &[(String, String)]) -> Result<Self, String> {
        let path_options = if headers.is_empty() {
            None
        } else {
            Some(PathSpecificOptions::set_headers(path, headers)?)
        };
        let path_c = CString::new(path).map_err(|e| e.to_string())?;
        let mode_c = CString::new("rb").expect("static mode has no interior NUL");
        unsafe {
            let handle = gdal_sys::VSIFOpenL(path_c.as_ptr(), mode_c.as_ptr());
            if handle.is_null() {
                return Err(format!("Failed to open VSI path: {path}"));
            }
            Ok(Self {
                handle,
                _path_options: path_options,
            })
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

#[derive(Debug)]
struct PathSpecificOptions {
    path: CString,
}

impl PathSpecificOptions {
    fn set_headers(path: &str, headers: &[(String, String)]) -> Result<Self, String> {
        let path = CString::new(path).map_err(|e| e.to_string())?;
        let header_value = header_option_value(headers)?;
        let header_value = CString::new(header_value).map_err(|e| e.to_string())?;
        let http_headers = CString::new("GDAL_HTTP_HEADERS").expect("static key");
        let headers_key = CString::new("HEADERS").expect("static key");
        unsafe {
            gdal_sys::VSISetPathSpecificOption(
                path.as_ptr(),
                http_headers.as_ptr(),
                header_value.as_ptr(),
            );
            gdal_sys::VSISetPathSpecificOption(
                path.as_ptr(),
                headers_key.as_ptr(),
                header_value.as_ptr(),
            );
        }
        Ok(Self { path })
    }
}

impl Drop for PathSpecificOptions {
    fn drop(&mut self) {
        unsafe {
            gdal_sys::VSIClearPathSpecificOptions(self.path.as_ptr());
        }
    }
}

fn header_option_value(headers: &[(String, String)]) -> Result<String, String> {
    headers
        .iter()
        .map(|(key, value)| {
            if key.trim().is_empty() {
                Err("VSI header names must not be empty.".to_string())
            } else {
                Ok(format!("{key}: {value}"))
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|values| values.join(","))
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
    fn vsi_reads_memory_file() {
        let path = format!("/vsimem/pdal-vsi-{}.bin", std::process::id());
        write_mem_file(&path, b"abcdefghijklmnopqrstuvwxyz").unwrap();

        let mut vsi = VsiFile::open(&path).unwrap();
        assert_eq!(vsi.len().unwrap(), 26);
        assert_eq!(vsi.read_exact_at(2, 4).unwrap(), b"cdef");

        unlink(&path).unwrap();
    }

    #[test]
    fn vsi_path_options_are_scoped_to_file_lifetime() {
        let path = format!("/vsimem/pdal-vsi-headers-{}.bin", std::process::id());
        write_mem_file(&path, b"abc").unwrap();

        let key = CString::new("GDAL_HTTP_HEADERS").unwrap();
        let default = CString::new("").unwrap();
        {
            let _vsi = VsiFile::open_with_headers(
                &path,
                &[("Authorization".to_string(), "Bearer token".to_string())],
            )
            .unwrap();
            let path_c = CString::new(path.as_str()).unwrap();
            let value = unsafe {
                gdal_sys::VSIGetPathSpecificOption(path_c.as_ptr(), key.as_ptr(), default.as_ptr())
            };
            let value = unsafe { std::ffi::CStr::from_ptr(value) }.to_string_lossy();
            assert_eq!(value, "Authorization: Bearer token");
        }

        let path_c = CString::new(path.as_str()).unwrap();
        let value = unsafe {
            gdal_sys::VSIGetPathSpecificOption(path_c.as_ptr(), key.as_ptr(), default.as_ptr())
        };
        let value = unsafe { std::ffi::CStr::from_ptr(value) }.to_string_lossy();
        assert!(value.is_empty());

        unlink(&path).unwrap();
    }

    #[test]
    #[ignore = "network smoke for GDAL /vsicurl/ range reads"]
    fn vsi_reads_remote_ranges() {
        let url = "/vsicurl/https://github.com/PDAL/data/raw/refs/heads/main/autzen/autzen-classified.copc.laz";
        let mut vsi = VsiFile::open(url).unwrap();
        assert_eq!(vsi.read_exact_at(0, 4).unwrap(), b"LASF");
    }
}
