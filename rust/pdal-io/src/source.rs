use std::fs;
use std::io::Read;
use std::path::Path;

pub fn read_bytes(filename: &str) -> Result<Vec<u8>, String> {
    if is_vsi_path(filename) {
        let path = vsi_path(filename);
        let mut file = pdal_native::vsi::VsiFile::open(&path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|err| format!("failed to read '{filename}': {err}"))?;
        return Ok(bytes);
    }

    fs::read(Path::new(filename)).map_err(|err| err.to_string())
}

pub fn read_to_string(filename: &str) -> Result<String, String> {
    if is_vsi_path(filename) {
        let path = vsi_path(filename);
        let mut file = pdal_native::vsi::VsiFile::open(&path)?;
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
}

pub fn vsi_path(filename: &str) -> String {
    if filename.starts_with("http://") || filename.starts_with("https://") {
        format!("/vsicurl/{filename}")
    } else {
        filename.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vsi_path_helpers_cover_remote_and_vsi_forms() {
        assert!(is_vsi_path("https://example.com/file.txt"));
        assert!(is_vsi_path("http://example.com/file.txt"));
        assert!(is_vsi_path("/vsicurl/https://example.com/file.txt"));
        assert!(!is_vsi_path("/tmp/file.txt"));
        assert_eq!(
            vsi_path("https://example.com/file.txt"),
            "/vsicurl/https://example.com/file.txt"
        );
        assert_eq!(
            vsi_path("/vsicurl/https://example.com/file.txt"),
            "/vsicurl/https://example.com/file.txt"
        );
    }
}
