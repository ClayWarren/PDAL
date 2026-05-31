//! Remote LAS header/VLR/EVLR extraction used by `pdal info`.

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use pdal_native::vsi::VsiFile;
use std::io::{Cursor, Write};
use std::path::PathBuf;

const MAX_HEADER_SIZE: usize = 375;
const MINOR_VERSION_POS: u64 = 25;
const HEADER_SIZE_POS: u64 = 94;
const POINT_OFFSET_POS: u64 = 96;
const LEGACY_POINT_COUNT_POS: u64 = 107;
const EVLR_OFFSET_POS: u64 = 235;
const EVLR_NUMBER_POS: u64 = EVLR_OFFSET_POS + 8;
const POINT_COUNT_POS: u64 = 247;

#[derive(Debug, Clone)]
pub struct PointlessLas {
    pub point_count: u64,
    pub path: PathBuf,
}

pub fn create(path: &str) -> Result<PointlessLas, String> {
    pdal_native::gdal::register_drivers();
    let mut file = VsiFile::open(&vsi_path(path))?;
    let mut header = file.read_at(0, MAX_HEADER_SIZE)?;

    if header.get(0..4) != Some(b"LASF") {
        return Err("Invalid file signature for .las or .laz file: must be LASF".to_string());
    }

    let minor_version = read_u8(&header, MINOR_VERSION_POS)?;
    let header_size = read_u16(&header, HEADER_SIZE_POS)?;
    let point_offset = read_u32(&header, POINT_OFFSET_POS)?;
    let legacy_point_count = read_u32(&header, LEGACY_POINT_COUNT_POS)?;
    let mut point_count = legacy_point_count as u64;

    write_u32(&mut header, LEGACY_POINT_COUNT_POS, 0)?;

    let mut evlr_offset = 0;
    let mut evlr_number = 0;
    if minor_version >= 4 {
        evlr_offset = read_u64(&header, EVLR_OFFSET_POS)?;
        evlr_number = read_u32(&header, EVLR_NUMBER_POS)?;
        point_count = read_u64(&header, POINT_COUNT_POS)?;
        write_u64(&mut header, EVLR_OFFSET_POS, point_offset as u64)?;
        write_u64(&mut header, POINT_COUNT_POS, 0)?;
    }

    let header_size = header_size as usize;
    if header_size > header.len() {
        return Err(format!(
            "LAS header size {header_size} exceeds supported maximum {MAX_HEADER_SIZE}."
        ));
    }

    let mut data = header[..header_size].to_vec();
    if header_size < point_offset as usize {
        data.extend(file.read_exact_at(header_size as u64, point_offset as usize - header_size)?);
    }
    if evlr_number != 0 && evlr_offset != 0 {
        let file_len = file.len()?;
        if evlr_offset > file_len {
            return Err("LAS EVLR offset is beyond end of file.".to_string());
        }
        data.extend(file.read_exact_at(evlr_offset, (file_len - evlr_offset) as usize)?);
    }

    let temp = tempfile::Builder::new()
        .prefix("pdal-pointless-")
        .suffix(&path_suffix(path))
        .tempfile()
        .map_err(|err| err.to_string())?;
    let (_, temp_path) = temp.keep().map_err(|err| err.error.to_string())?;
    let mut out = std::fs::File::create(&temp_path).map_err(|err| err.to_string())?;
    out.write_all(&data).map_err(|err| err.to_string())?;
    Ok(PointlessLas {
        point_count,
        path: temp_path,
    })
}

fn vsi_path(path: &str) -> String {
    if path.starts_with("/vsi") {
        path.to_string()
    } else if path.starts_with("http://") || path.starts_with("https://") {
        format!("/vsicurl/{path}")
    } else {
        path.to_string()
    }
}

fn path_suffix(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default()
}

fn read_u8(buf: &[u8], offset: u64) -> Result<u8, String> {
    buf.get(offset as usize)
        .copied()
        .ok_or_else(|| format!("Short LAS header at byte {offset}."))
}

fn read_u16(buf: &[u8], offset: u64) -> Result<u16, String> {
    Cursor::new(slice_at(buf, offset, 2)?)
        .read_u16::<LittleEndian>()
        .map_err(|err| err.to_string())
}

fn read_u32(buf: &[u8], offset: u64) -> Result<u32, String> {
    Cursor::new(slice_at(buf, offset, 4)?)
        .read_u32::<LittleEndian>()
        .map_err(|err| err.to_string())
}

fn read_u64(buf: &[u8], offset: u64) -> Result<u64, String> {
    Cursor::new(slice_at(buf, offset, 8)?)
        .read_u64::<LittleEndian>()
        .map_err(|err| err.to_string())
}

fn write_u32(buf: &mut [u8], offset: u64, value: u32) -> Result<(), String> {
    Cursor::new(slice_at_mut(buf, offset, 4)?)
        .write_u32::<LittleEndian>(value)
        .map_err(|err| err.to_string())
}

fn write_u64(buf: &mut [u8], offset: u64, value: u64) -> Result<(), String> {
    Cursor::new(slice_at_mut(buf, offset, 8)?)
        .write_u64::<LittleEndian>(value)
        .map_err(|err| err.to_string())
}

fn slice_at(buf: &[u8], offset: u64, len: usize) -> Result<&[u8], String> {
    let start = offset as usize;
    buf.get(start..start + len)
        .ok_or_else(|| format!("Short LAS header at byte {offset}."))
}

fn slice_at_mut(buf: &mut [u8], offset: u64, len: usize) -> Result<&mut [u8], String> {
    let start = offset as usize;
    buf.get_mut(start..start + len)
        .ok_or_else(|| format!("Short LAS header at byte {offset}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn las_12_header(point_count: u32) -> Vec<u8> {
        let mut header = vec![0; 227];
        header[0..4].copy_from_slice(b"LASF");
        header[MINOR_VERSION_POS as usize] = 2;
        Cursor::new(&mut header[HEADER_SIZE_POS as usize..])
            .write_u16::<LittleEndian>(227)
            .unwrap();
        Cursor::new(&mut header[POINT_OFFSET_POS as usize..])
            .write_u32::<LittleEndian>(227)
            .unwrap();
        Cursor::new(&mut header[LEGACY_POINT_COUNT_POS as usize..])
            .write_u32::<LittleEndian>(point_count)
            .unwrap();
        header
    }

    #[test]
    fn local_las_header_is_copied_without_points() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.las");
        let mut bytes = las_12_header(42);
        bytes.extend([1, 2, 3, 4]);
        std::fs::write(&input, bytes).unwrap();

        let pointless = create(input.to_str().unwrap()).unwrap();

        assert_eq!(pointless.point_count, 42);
        let output = std::fs::read(&pointless.path).unwrap();
        assert_eq!(output.len(), 227);
        assert_eq!(read_u32(&output, LEGACY_POINT_COUNT_POS).unwrap(), 0);
        let _ = std::fs::remove_file(pointless.path);
    }

    #[test]
    fn rejects_non_las_signature() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.las");
        std::fs::write(&input, vec![0; MAX_HEADER_SIZE]).unwrap();
        assert!(create(input.to_str().unwrap())
            .unwrap_err()
            .contains("Invalid file signature"));
    }
}
