use std::fs;
use std::io::{Read, Seek};
use std::path::Path;

use pdal_core::file_spec::ParsedFileSpec;

pub trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

pub fn open_seek(filename: &str) -> Result<Box<dyn ReadSeek>, String> {
    open_seek_with_headers(filename, &[])
}

pub fn open_seek_with_headers(
    filename: &str,
    headers: &[(String, String)],
) -> Result<Box<dyn ReadSeek>, String> {
    if is_vsi_path(filename) {
        let path = vsi_path(filename);
        return pdal_native::vsi::VsiFile::open_with_headers(&path, headers)
            .map(|file| Box::new(file) as Box<dyn ReadSeek>);
    }

    fs::File::open(Path::new(filename))
        .map(|file| Box::new(file) as Box<dyn ReadSeek>)
        .map_err(|err| err.to_string())
}

pub fn open_seek_file_spec(spec: &ParsedFileSpec) -> Result<Box<dyn ReadSeek>, String> {
    let filename = file_spec_path_with_query(spec);
    let headers = header_pairs(spec);
    open_seek_with_headers(&filename, &headers)
}

pub fn open_seek_len(filename: &str) -> Result<(Box<dyn ReadSeek>, u64), String> {
    open_seek_len_with_headers(filename, &[])
}

pub fn open_seek_len_with_headers(
    filename: &str,
    headers: &[(String, String)],
) -> Result<(Box<dyn ReadSeek>, u64), String> {
    if is_vsi_path(filename) {
        let path = vsi_path(filename);
        let mut file = pdal_native::vsi::VsiFile::open_with_headers(&path, headers)?;
        let len = file.len()?;
        return Ok((Box::new(file), len));
    }

    let file = fs::File::open(Path::new(filename)).map_err(|err| err.to_string())?;
    let len = file.metadata().map_err(|err| err.to_string())?.len();
    Ok((Box::new(file), len))
}

pub fn open_seek_len_file_spec(spec: &ParsedFileSpec) -> Result<(Box<dyn ReadSeek>, u64), String> {
    let filename = file_spec_path_with_query(spec);
    let headers = header_pairs(spec);
    open_seek_len_with_headers(&filename, &headers)
}

pub fn read_bytes(filename: &str) -> Result<Vec<u8>, String> {
    read_bytes_with_headers(filename, &[])
}

pub fn read_bytes_with_headers(
    filename: &str,
    headers: &[(String, String)],
) -> Result<Vec<u8>, String> {
    if is_vsi_path(filename) {
        let path = vsi_path(filename);
        let mut file = pdal_native::vsi::VsiFile::open_with_headers(&path, headers)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|err| format!("failed to read '{filename}': {err}"))?;
        return Ok(bytes);
    }

    fs::read(Path::new(filename)).map_err(|err| err.to_string())
}

pub fn read_to_string(filename: &str) -> Result<String, String> {
    read_to_string_with_headers(filename, &[])
}

pub fn read_to_string_with_headers(
    filename: &str,
    headers: &[(String, String)],
) -> Result<String, String> {
    if is_vsi_path(filename) {
        let path = vsi_path(filename);
        let mut file = pdal_native::vsi::VsiFile::open_with_headers(&path, headers)?;
        let mut text = String::new();
        file.read_to_string(&mut text)
            .map_err(|err| format!("failed to read '{filename}': {err}"))?;
        return Ok(text);
    }

    fs::read_to_string(Path::new(filename)).map_err(|err| err.to_string())
}

pub fn is_vsi_path(filename: &str) -> bool {
    filename.starts_with("/vsi")
        || filename.starts_with("http://")
        || filename.starts_with("https://")
        || filename.starts_with("s3://")
        || filename.starts_with("gs://")
        || filename.starts_with("az://")
}

pub fn vsi_path(filename: &str) -> String {
    if filename.starts_with("http://") || filename.starts_with("https://") {
        format!("/vsicurl/{filename}")
    } else if let Some(path) = filename.strip_prefix("s3://") {
        format!("/vsis3/{path}")
    } else if let Some(path) = filename.strip_prefix("gs://") {
        format!("/vsigs/{path}")
    } else if let Some(path) = filename.strip_prefix("az://") {
        format!("/vsiaz/{path}")
    } else {
        filename.to_string()
    }
}

pub fn file_spec_path_with_query(spec: &ParsedFileSpec) -> String {
    if spec.query.is_empty() || !is_vsi_path(&spec.path) {
        return spec.path.clone();
    }
    let separator = if spec.path.contains('?') { '&' } else { '?' };
    let query = spec
        .query
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                percent_encode_query_component(key),
                percent_encode_query_component(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    format!("{}{}{}", spec.path, separator, query)
}

fn header_pairs(spec: &ParsedFileSpec) -> Vec<(String, String)> {
    spec.headers
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn percent_encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push_str("%20"),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vsi_path_helpers_cover_remote_and_vsi_forms() {
        assert!(is_vsi_path("https://example.com/file.txt"));
        assert!(is_vsi_path("http://example.com/file.txt"));
        assert!(is_vsi_path("/vsicurl/https://example.com/file.txt"));
        assert!(is_vsi_path("s3://bucket/key/file.laz"));
        assert!(is_vsi_path("gs://bucket/key/file.laz"));
        assert!(is_vsi_path("az://container/key/file.laz"));
        assert!(!is_vsi_path("/tmp/file.txt"));
        assert_eq!(
            vsi_path("https://example.com/file.txt"),
            "/vsicurl/https://example.com/file.txt"
        );
        assert_eq!(
            vsi_path("/vsicurl/https://example.com/file.txt"),
            "/vsicurl/https://example.com/file.txt"
        );
        assert_eq!(
            vsi_path("s3://bucket/key/file.laz"),
            "/vsis3/bucket/key/file.laz"
        );
        assert_eq!(
            vsi_path("gs://bucket/key/file.laz"),
            "/vsigs/bucket/key/file.laz"
        );
        assert_eq!(
            vsi_path("az://container/key/file.laz"),
            "/vsiaz/container/key/file.laz"
        );
    }

    #[test]
    fn file_spec_path_applies_query_only_to_remote_paths() {
        let mut remote = pdal_core::file_spec::ParsedFileSpec {
            path: "https://example.com/file.laz?existing=1".to_string(),
            headers: Default::default(),
            query: Default::default(),
        };
        remote.query.insert("token".to_string(), "a b".to_string());
        assert_eq!(
            file_spec_path_with_query(&remote),
            "https://example.com/file.laz?existing=1&token=a%20b"
        );

        let mut local = remote.clone();
        local.path = "/tmp/file.laz".to_string();
        assert_eq!(file_spec_path_with_query(&local), "/tmp/file.laz");
    }
}
