//! Raw LAS dumper used by the `lasdump` tool.

use byteorder::{LittleEndian, ReadBytesExt};
use las::point::Format;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use uuid::Uuid;

const LEGACY_RETURN_COUNT: usize = 5;
const RETURN_COUNT: usize = 15;

#[derive(Debug)]
struct Header {
    file_sig: String,
    source_id: u16,
    global_encoding: u16,
    project_guid: Uuid,
    version_minor: u8,
    system_id: String,
    software_id: String,
    create_doy: u16,
    create_year: u16,
    vlr_offset: u16,
    point_offset: u32,
    vlr_count: u32,
    point_format: u8,
    point_len: u16,
    point_count: u64,
    point_count_by_return: [u64; RETURN_COUNT],
    scales: [f64; 3],
    offsets: [f64; 3],
    min: [f64; 3],
    max: [f64; 3],
    compressed: bool,
    evlr_offset: u64,
    evlr_count: u32,
}

#[derive(Debug)]
struct Vlr {
    record_sig: u16,
    user_id: String,
    record_id: u16,
    description: String,
    data: Vec<u8>,
}

/// Dump a LAS file using the same text format as PDAL's C++ `lasdump` tool.
pub fn dump_las(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|_| format!("Couldn't open file \"{}\".", path.display()))?;
    let header = read_header(&mut file)?;
    let mut output = String::new();

    write_header(&mut output, &header);
    file.seek(SeekFrom::Start(header.vlr_offset as u64))
        .map_err(|e| format!("Failed to seek to VLRs: {e}"))?;
    for _ in 0..header.vlr_count {
        write_vlr(&mut output, &read_vlr(&mut file, false)?);
    }
    if header.version_minor == 0 {
        let data_start_sig = file
            .read_u16::<LittleEndian>()
            .map_err(|e| format!("Failed to read data start signature: {e}"))?;
        writeln!(&mut output, "Data start signature: {data_start_sig}").unwrap();
    }

    if header.compressed {
        write_compressed_point_checksums(&mut output, path, &header)?;
    } else {
        write_point_checksums(&mut output, &mut file, &header)?;
    }

    if header.version_minor >= 4 && header.evlr_count > 0 {
        file.seek(SeekFrom::Start(header.evlr_offset))
            .map_err(|e| format!("Failed to seek to EVLRs: {e}"))?;
        for _ in 0..header.evlr_count {
            write_vlr(&mut output, &read_vlr(&mut file, true)?);
        }
    }

    Ok(output)
}

/// Checksum bytes the same way the C++ lasdump helper does.
pub fn checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_le_bytes(word))
    })
}

fn read_header<R: Read>(reader: &mut R) -> Result<Header, String> {
    let file_sig = read_fixed_string(reader, 4)?;
    if file_sig != "LASF" {
        return Err("Not a LAS/LAZ file.  Invalid file signature.".to_string());
    }

    let source_id = reader.read_u16::<LittleEndian>().map_err(read_err)?;
    let global_encoding = reader.read_u16::<LittleEndian>().map_err(read_err)?;
    let project_guid = read_uuid(reader)?;
    let version_major = reader.read_u8().map_err(read_err)?;
    let version_minor = reader.read_u8().map_err(read_err)?;
    let system_id = read_fixed_string(reader, 32)?;
    let software_id = read_fixed_string(reader, 32)?;
    let create_doy = reader.read_u16::<LittleEndian>().map_err(read_err)?;
    let create_year = reader.read_u16::<LittleEndian>().map_err(read_err)?;
    let vlr_offset = reader.read_u16::<LittleEndian>().map_err(read_err)?;
    let point_offset = reader.read_u32::<LittleEndian>().map_err(read_err)?;
    let vlr_count = reader.read_u32::<LittleEndian>().map_err(read_err)?;
    let raw_point_format = reader.read_u8().map_err(read_err)?;
    let point_len = reader.read_u16::<LittleEndian>().map_err(read_err)?;
    let legacy_point_count = reader.read_u32::<LittleEndian>().map_err(read_err)?;
    let compressed = raw_point_format & 0x80 != 0;
    let point_format = raw_point_format & !0xC0;

    let mut point_count_by_return = [0u64; RETURN_COUNT];
    for count in point_count_by_return.iter_mut().take(LEGACY_RETURN_COUNT) {
        *count = reader.read_u32::<LittleEndian>().map_err(read_err)? as u64;
    }

    let scales = read_triple(reader)?;
    let offsets = read_triple(reader)?;
    let max_x = reader.read_f64::<LittleEndian>().map_err(read_err)?;
    let min_x = reader.read_f64::<LittleEndian>().map_err(read_err)?;
    let max_y = reader.read_f64::<LittleEndian>().map_err(read_err)?;
    let min_y = reader.read_f64::<LittleEndian>().map_err(read_err)?;
    let max_z = reader.read_f64::<LittleEndian>().map_err(read_err)?;
    let min_z = reader.read_f64::<LittleEndian>().map_err(read_err)?;

    let mut point_count = legacy_point_count as u64;
    let mut evlr_offset = 0;
    let mut evlr_count = 0;
    if version_major == 1 && version_minor >= 3 {
        let _waveform_offset = reader.read_u64::<LittleEndian>().map_err(read_err)?;
    }
    if version_major == 1 && version_minor >= 4 {
        let legacy_count = point_count;
        evlr_offset = reader.read_u64::<LittleEndian>().map_err(read_err)?;
        evlr_count = reader.read_u32::<LittleEndian>().map_err(read_err)?;
        point_count = reader.read_u64::<LittleEndian>().map_err(read_err)?;
        for count in &mut point_count_by_return {
            *count = reader.read_u64::<LittleEndian>().map_err(read_err)?;
        }
        if legacy_count != 0 && legacy_count != point_count {
            return Err(format!(
                "1.4 point count ({point_count}) doesn't match legacy point count ({legacy_count})."
            ));
        }
    }

    Ok(Header {
        file_sig,
        source_id,
        global_encoding,
        project_guid,
        version_minor,
        system_id,
        software_id,
        create_doy,
        create_year,
        vlr_offset,
        point_offset,
        vlr_count,
        point_format,
        point_len,
        point_count,
        point_count_by_return,
        scales,
        offsets,
        min: [min_x, min_y, min_z],
        max: [max_x, max_y, max_z],
        compressed,
        evlr_offset,
        evlr_count,
    })
}

fn read_vlr<R: Read>(reader: &mut R, extended: bool) -> Result<Vlr, String> {
    let record_sig = reader.read_u16::<LittleEndian>().map_err(read_err)?;
    let user_id = read_fixed_string(reader, 16)?;
    let record_id = reader.read_u16::<LittleEndian>().map_err(read_err)?;
    let data_len = if extended {
        reader.read_u64::<LittleEndian>().map_err(read_err)?
    } else {
        reader.read_u16::<LittleEndian>().map_err(read_err)? as u64
    };
    let description = read_fixed_string(reader, 32)?;
    let mut data = vec![0; data_len as usize];
    reader.read_exact(&mut data).map_err(read_err)?;
    Ok(Vlr {
        record_sig,
        user_id,
        record_id,
        description,
        data,
    })
}

fn write_header(output: &mut String, header: &Header) {
    writeln!(output, "File version: 1.{}", header.version_minor).unwrap();
    writeln!(output, "File signature: {}", header.file_sig).unwrap();
    writeln!(output, "File source ID: {}", header.source_id).unwrap();
    writeln!(output, "Global encoding: {}", header.global_encoding).unwrap();
    writeln!(output, "Project GUID: {}", header.project_guid.hyphenated()).unwrap();
    writeln!(output, "System ID: {}", header.system_id).unwrap();
    writeln!(output, "Software ID: {}", header.software_id).unwrap();
    writeln!(output, "Creation DOY: {}", header.create_doy).unwrap();
    writeln!(output, "Creation Year: {}", header.create_year).unwrap();
    writeln!(output, "VLR offset (header size): {}", header.vlr_offset).unwrap();
    writeln!(output, "VLR Count: {}", header.vlr_count).unwrap();
    writeln!(output, "Point format: {}", header.point_format).unwrap();
    writeln!(output, "Point offset: {}", header.point_offset).unwrap();
    writeln!(output, "Point count: {}", header.point_count).unwrap();
    for (i, count) in header.point_count_by_return.iter().enumerate() {
        writeln!(output, "Point count by return[{i}]: {count}").unwrap();
    }
    writeln!(
        output,
        "Scales X/Y/Z: {}/{}/{}",
        format_default_float(header.scales[0]),
        format_default_float(header.scales[1]),
        format_default_float(header.scales[2])
    )
    .unwrap();
    writeln!(
        output,
        "Offsets X/Y/Z: {}/{}/{}",
        format_default_float(header.offsets[0]),
        format_default_float(header.offsets[1]),
        format_default_float(header.offsets[2])
    )
    .unwrap();
    writeln!(
        output,
        "Max X/Y/Z: {}/{}/{}",
        format_default_float(header.max[0]),
        format_default_float(header.max[1]),
        format_default_float(header.max[2])
    )
    .unwrap();
    writeln!(
        output,
        "Min X/Y/Z: {}/{}/{}",
        format_default_float(header.min[0]),
        format_default_float(header.min[1]),
        format_default_float(header.min[2])
    )
    .unwrap();
    if header.version_minor >= 4 {
        writeln!(output, "Ext. VLR offset: {}", header.evlr_offset).unwrap();
        writeln!(output, "Ext. VLR count: {}", header.evlr_count).unwrap();
    }
    writeln!(
        output,
        "Compressed: {}",
        if header.compressed { "true" } else { "false" }
    )
    .unwrap();
}

fn write_vlr(output: &mut String, vlr: &Vlr) {
    writeln!(output, "Record Signature: {}", vlr.record_sig).unwrap();
    writeln!(output, "User ID: {}", vlr.user_id).unwrap();
    writeln!(output, "Record ID: {}", vlr.record_id).unwrap();
    writeln!(output, "Description: {}", vlr.description).unwrap();
    writeln!(output, "Data checksum: {}", checksum(&vlr.data)).unwrap();
}

fn write_point_checksums<R: Read + Seek>(
    output: &mut String,
    reader: &mut R,
    header: &Header,
) -> Result<(), String> {
    reader
        .seek(SeekFrom::Start(header.point_offset as u64))
        .map_err(|e| format!("Failed to seek to points: {e}"))?;
    let mut buf = vec![0; header.point_len as usize];
    for i in 0..header.point_count {
        reader.read_exact(&mut buf).map_err(read_err)?;
        writeln!(output, "{i} {}", checksum(&buf)).unwrap();
    }
    Ok(())
}

fn write_compressed_point_checksums(
    output: &mut String,
    path: &Path,
    header: &Header,
) -> Result<(), String> {
    let mut reader =
        las::Reader::from_path(path).map_err(|e| format!("Failed to open LAZ points: {e}"))?;
    let mut format =
        Format::new(header.point_format).map_err(|e| format!("Invalid point format: {e}"))?;
    let base_len = format.len();
    if header.point_len < base_len {
        return Err(format!(
            "Point record length {} is smaller than base format length {}.",
            header.point_len, base_len
        ));
    }
    format.extra_bytes = header.point_len - base_len;

    for i in 0..header.point_count {
        let point = reader
            .read_point()
            .map_err(|e| format!("Failed to read LAZ point: {e}"))?
            .ok_or_else(|| format!("Expected point {i}, but LAZ stream ended."))?;
        let raw = point
            .into_raw(reader.header().transforms())
            .map_err(|e| format!("Failed to encode LAZ point: {e}"))?;
        let mut buf = Vec::with_capacity(header.point_len as usize);
        raw.write_to(&mut buf, &format)
            .map_err(|e| format!("Failed to encode LAZ point: {e}"))?;
        writeln!(output, "{i} {}", checksum(&buf)).unwrap();
    }
    Ok(())
}

fn read_uuid<R: Read>(reader: &mut R) -> Result<Uuid, String> {
    let data1 = reader.read_u32::<LittleEndian>().map_err(read_err)?;
    let data2 = reader.read_u16::<LittleEndian>().map_err(read_err)?;
    let data3 = reader.read_u16::<LittleEndian>().map_err(read_err)?;
    let mut data4 = [0u8; 8];
    reader.read_exact(&mut data4).map_err(read_err)?;
    Ok(Uuid::from_fields(data1, data2, data3, &data4))
}

fn read_triple<R: Read>(reader: &mut R) -> Result<[f64; 3], String> {
    Ok([
        reader.read_f64::<LittleEndian>().map_err(read_err)?,
        reader.read_f64::<LittleEndian>().map_err(read_err)?,
        reader.read_f64::<LittleEndian>().map_err(read_err)?,
    ])
}

fn read_fixed_string<R: Read>(reader: &mut R, len: usize) -> Result<String, String> {
    let mut buf = vec![0; len];
    reader.read_exact(&mut buf).map_err(read_err)?;
    let nul = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    Ok(String::from_utf8_lossy(&buf[..nul]).into_owned())
}

fn format_default_float(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0".to_string()
        } else {
            "0".to_string()
        };
    }
    if !value.is_finite() {
        return value.to_string();
    }

    let abs = value.abs();
    let exponent = abs.log10().floor() as i32;
    let precision = 6;
    if exponent < -4 || exponent >= precision {
        let text = format!("{:.*e}", (precision - 1) as usize, value);
        return trim_scientific_float(&text);
    }

    let decimals = (precision - exponent - 1).max(0);
    let scale = 10_f64.powi(decimals);
    let rounded = (value * scale).round() / scale;
    trim_decimal_float(&format!("{rounded}"))
}

fn trim_decimal_float(text: &str) -> String {
    let trimmed = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text
    };
    if trimmed == "-0" {
        "-0".to_string()
    } else if trimmed.is_empty() || trimmed == "-" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

fn trim_scientific_float(text: &str) -> String {
    let Some((mantissa, exponent)) = text.split_once('e') else {
        return text.to_string();
    };
    let exponent = exponent.trim_start_matches('+');
    format!("{}e{}", trim_decimal_float(mantissa), exponent)
}

fn read_err(err: std::io::Error) -> String {
    format!("Failed to read LAS data: {err}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_matches_lasdump_word_accumulation() {
        assert_eq!(checksum(&[]), 0);
        assert_eq!(checksum(&[1, 0, 0, 0]), 1);
        assert_eq!(checksum(&[1, 2, 3, 4]), 0x0403_0201);
        assert_eq!(checksum(&[1, 2, 3, 4, 5]), 0x0403_0206);
    }

    #[test]
    fn default_float_format_matches_lasdump_precision() {
        assert_eq!(format_default_float(638982.55), "638983");
        assert_eq!(format_default_float(586.38), "586.38");
        assert_eq!(format_default_float(0.01), "0.01");
        assert_eq!(format_default_float(0.0), "0");
        assert_eq!(format_default_float(-0.0), "-0");
        assert_eq!(format_default_float(1.234567e8), "1.23457e8");
        assert_eq!(format_default_float(1.234567e-5), "1.23457e-5");
        assert_eq!(format_default_float(f64::INFINITY), "inf");
        assert_eq!(format_default_float(f64::NEG_INFINITY), "-inf");
        assert_eq!(format_default_float(f64::NAN), "NaN");
        assert_eq!(trim_scientific_float("1.230000e+08"), "1.23e08");
        assert_eq!(trim_scientific_float("1.230000"), "1.230000");
    }

    #[test]
    fn dumps_simple_las_header_and_point_checksums() {
        let text = dump_las(Path::new("../../test/data/las/simple.las")).unwrap();
        assert!(text.contains("File version: 1.2\n"));
        assert!(text.contains("Point format: 3\n"));
        assert!(text.contains("Point count: 1065\n"));
        assert!(text.contains("Compressed: false\n"));
        assert!(text.contains("0 2865236059\n"));
        assert!(text.contains("1064 "));
    }

    #[test]
    fn dumps_vlrs_and_las_14_fields() {
        let text = dump_las(Path::new("../../test/data/las/test1_4.las")).unwrap();
        assert!(text.contains("File version: 1.4\n"));
        assert!(text.contains("Ext. VLR offset: "));
        assert!(text.contains("Ext. VLR count: "));
        assert!(text.contains("Record Signature: "));
        assert!(text.contains("Data checksum: "));
    }

    #[test]
    fn dumps_las_10_data_start_signature() {
        let text = dump_las(Path::new("../../test/data/las/permutations/1.0_0.las")).unwrap();
        assert!(text.contains("File version: 1.0\n"));
        assert!(text.contains("Data start signature: "));
    }

    #[test]
    fn reports_missing_invalid_and_compressed_inputs() {
        let missing = dump_las(Path::new("../../test/data/las/does-not-exist.las")).unwrap_err();
        assert!(missing.contains("Couldn't open file"));

        let invalid = dump_las(Path::new("../../test/data/las/mvk-thin.las.wkt")).unwrap_err();
        assert_eq!(invalid, "Not a LAS/LAZ file.  Invalid file signature.");

        let compressed = dump_las(Path::new("../../test/data/laz/simple.laz")).unwrap();
        assert!(compressed.contains("Compressed: true\n"));
        assert!(compressed.contains("0 "));
    }
}
