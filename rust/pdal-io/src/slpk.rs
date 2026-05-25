use byteorder::{LittleEndian, ReadBytesExt};
use flate2::read::GzDecoder;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlpkSummary {
    pub point_count: usize,
    pub dimensions: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct Location {
    pos: u64,
    len: usize,
}

pub fn summarize_slpk(path: &Path, dimensions: &[String]) -> Result<SlpkSummary, String> {
    let mut file = File::open(path).map_err(|err| format!("failed to open SLPK: {err}"))?;
    let locations = local_file_locations(&mut file)?;
    let layer = read_gzip_json(&mut file, &locations, "3dSceneLayer.json.gz")?;
    let nodepage = read_gzip_json(&mut file, &locations, "nodepages/0.json.gz")?;
    let point_count = nodepage["nodes"]
        .as_array()
        .ok_or_else(|| "SLPK nodepage does not contain nodes".to_string())?
        .iter()
        .map(|node| node["vertexCount"].as_u64().unwrap_or(0) as usize)
        .sum();
    Ok(SlpkSummary {
        point_count,
        dimensions: selected_dimensions(&layer, dimensions),
    })
}

fn local_file_locations(file: &mut File) -> Result<BTreeMap<String, Location>, String> {
    let mut out = BTreeMap::new();
    file.seek(SeekFrom::Start(0))
        .map_err(|err| format!("failed to seek SLPK: {err}"))?;
    while let Ok(magic) = file.read_u32::<LittleEndian>() {
        if magic != 0x0403_4b50 {
            break;
        }
        let _version = file.read_u16::<LittleEndian>().map_err(read_err)?;
        let _purpose = file.read_u16::<LittleEndian>().map_err(read_err)?;
        let compression = file.read_u16::<LittleEndian>().map_err(read_err)?;
        let _time = file.read_u32::<LittleEndian>().map_err(read_err)?;
        let _crc = file.read_u32::<LittleEndian>().map_err(read_err)?;
        let compressed_size = file.read_u32::<LittleEndian>().map_err(read_err)?;
        let uncompressed_size = file.read_u32::<LittleEndian>().map_err(read_err)?;
        let name_len = file.read_u16::<LittleEndian>().map_err(read_err)?;
        let extra_len = file.read_u16::<LittleEndian>().map_err(read_err)?;
        if compression != 0 || compressed_size != uncompressed_size {
            return Err("compressed SLPK zip entries are not supported".to_string());
        }
        let mut name = vec![0; name_len as usize];
        file.read_exact(&mut name).map_err(read_err)?;
        if extra_len > 0 {
            file.seek(SeekFrom::Current(extra_len as i64))
                .map_err(|err| format!("failed to skip SLPK zip extra data: {err}"))?;
        }
        let pos = file
            .stream_position()
            .map_err(|err| format!("failed to locate SLPK entry: {err}"))?;
        let name = String::from_utf8(name).map_err(|err| format!("invalid SLPK path: {err}"))?;
        out.insert(
            name,
            Location {
                pos,
                len: compressed_size as usize,
            },
        );
        file.seek(SeekFrom::Current(compressed_size as i64))
            .map_err(|err| format!("failed to skip SLPK entry: {err}"))?;
    }
    Ok(out)
}

fn read_gzip_json(
    file: &mut File,
    locations: &BTreeMap<String, Location>,
    name: &str,
) -> Result<Value, String> {
    let location = locations
        .get(name)
        .ok_or_else(|| format!("SLPK entry '{name}' not found"))?;
    file.seek(SeekFrom::Start(location.pos))
        .map_err(|err| format!("failed to seek SLPK entry '{name}': {err}"))?;
    let mut compressed = vec![0; location.len];
    file.read_exact(&mut compressed).map_err(read_err)?;
    let mut decoder = GzDecoder::new(Cursor::new(compressed));
    let mut json = String::new();
    decoder
        .read_to_string(&mut json)
        .map_err(|err| format!("failed to decompress SLPK entry '{name}': {err}"))?;
    serde_json::from_str(&json).map_err(|err| format!("invalid JSON in SLPK entry '{name}': {err}"))
}

fn selected_dimensions(layer: &Value, requested: &[String]) -> Vec<String> {
    let requested: Vec<String> = requested
        .iter()
        .map(|dim| dim.trim().to_ascii_uppercase())
        .filter(|dim| !dim.is_empty())
        .collect();
    let mut out = vec!["X".to_string(), "Y".to_string(), "Z".to_string()];
    let Some(attributes) = layer["attributeStorageInfo"].as_array() else {
        return out;
    };
    for attribute in attributes {
        let Some(name) = attribute["name"].as_str() else {
            continue;
        };
        if !requested.is_empty() && !requested.iter().any(|dim| dim == name) {
            continue;
        }
        if let Some(dim) = pdal_dimension_name(name) {
            out.push(dim.to_string());
        }
    }
    out
}

fn pdal_dimension_name(i3s_name: &str) -> Option<&'static str> {
    match i3s_name {
        "INTENSITY" => Some("Intensity"),
        "RETURNS" => Some("NumberOfReturns"),
        "CLASS_CODE" => Some("Classification"),
        "FLAGS" => Some("Flag"),
        "USER_DATA" => Some("UserData"),
        "POINT_SRC_ID" => Some("PointSourceId"),
        "GPS_TIME" => Some("GpsTime"),
        "SCAN_ANGLE" => Some("ScanAngleRank"),
        _ => None,
    }
}

fn read_err(err: std::io::Error) -> String {
    format!("failed to read SLPK: {err}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_small_fixture_summary() {
        let path = Path::new("../../test/data/i3s/SMALL_AUTZEN_LAS_All.slpk");
        let summary = summarize_slpk(path, &["intensity".to_string(), "returns".to_string()])
            .expect("SLPK summary");
        assert_eq!(summary.point_count, 106);
        assert!(summary.dimensions.iter().any(|dim| dim == "Intensity"));
        assert!(summary
            .dimensions
            .iter()
            .any(|dim| dim == "NumberOfReturns"));
        assert!(!summary.dimensions.iter().any(|dim| dim == "GpsTime"));
    }
}
