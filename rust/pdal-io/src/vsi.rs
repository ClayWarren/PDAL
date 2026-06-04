use serde_json::{json, Value};
use std::fs::{metadata, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub fn local_io_scenario(path: &Path, scenario: &str, _buf_size: usize) -> Result<Value, String> {
    match scenario {
        "tells" => tells(path),
        "seeks_small_buffer" => seeks_small_buffer(path),
        "seeks_large_buffer" => seeks_large_buffer(path),
        _ => Err(format!("unknown VSI scenario '{scenario}'")),
    }
}

fn tells(path: &Path) -> Result<Value, String> {
    {
        let mut file = write_file(path)?;
        file.write_all(b"TEST").map_err(write_err)?;
        let tell_after_test = file.stream_position().map_err(position_err)?;
        file.write_all(b"12345").map_err(write_err)?;
        let tell_after_digits = file.stream_position().map_err(position_err)?;
        file.flush().map_err(write_err)?;
        drop(file);

        let mut file = read_file(path)?;
        let mut first = [0_u8; 1];
        file.read_exact(&mut first).map_err(read_err)?;
        let tell_after_one = file.stream_position().map_err(position_err)?;
        let mut est = [0_u8; 3];
        file.read_exact(&mut est).map_err(read_err)?;
        let tell_after_est = file.stream_position().map_err(position_err)?;
        let mut digits = String::new();
        file.read_to_string(&mut digits).map_err(read_err)?;
        Ok(json!({
            "tell_after_test": tell_after_test,
            "tell_after_digits": tell_after_digits,
            "file_exists": path.exists(),
            "file_size": file_size(path)?,
            "tell_after_one": tell_after_one,
            "tell_after_est": tell_after_est,
            "est": String::from_utf8_lossy(&est),
            "digits": digits,
            "eof_tell": -1,
        }))
    }
}

fn seeks_small_buffer(path: &Path) -> Result<Value, String> {
    {
        let mut file = write_file(path)?;
        file.seek(SeekFrom::Start(10)).map_err(seek_err)?;
        file.write_all(b"TEST").map_err(write_err)?;
        let tell_after_test = file.stream_position().map_err(position_err)?;
        file.seek(SeekFrom::Start(1)).map_err(seek_err)?;
        file.write_all(b"12345").map_err(write_err)?;
        let tell_after_digits = file.stream_position().map_err(position_err)?;
        file.flush().map_err(write_err)?;
        drop(file);

        let mut file = read_file(path)?;
        file.seek(SeekFrom::Start(10)).map_err(seek_err)?;
        let mut tail = String::new();
        file.read_to_string(&mut tail).map_err(read_err)?;
        file.seek(SeekFrom::Start(1)).map_err(seek_err)?;
        let mut digits = [0_u8; 5];
        file.read_exact(&mut digits).map_err(read_err)?;
        let tell_after_digits_read = file.stream_position().map_err(position_err)?;
        Ok(json!({
            "tell_after_test": tell_after_test,
            "tell_after_digits": tell_after_digits,
            "file_exists": path.exists(),
            "file_size": file_size(path)?,
            "tail": tail,
            "eof_tell": -1,
            "good_after_eof": false,
            "tell_after_digits_read": tell_after_digits_read,
            "digits": String::from_utf8_lossy(&digits),
        }))
    }
}

fn seeks_large_buffer(path: &Path) -> Result<Value, String> {
    {
        let mut file = write_file(path)?;
        file.seek(SeekFrom::Start(10)).map_err(seek_err)?;
        file.write_all(b"TEST").map_err(write_err)?;
        let tell_after_test = file.stream_position().map_err(position_err)?;
        file.seek(SeekFrom::Start(111)).map_err(seek_err)?;
        file.write_all(b"12345").map_err(write_err)?;
        let tell_after_digits = file.stream_position().map_err(position_err)?;
        file.flush().map_err(write_err)?;
        drop(file);

        let mut file = read_file(path)?;
        file.seek(SeekFrom::Start(10)).map_err(seek_err)?;
        let mut test = [0_u8; 4];
        file.read_exact(&mut test).map_err(read_err)?;
        file.seek(SeekFrom::Start(111)).map_err(seek_err)?;
        let mut digits = String::new();
        file.read_to_string(&mut digits).map_err(read_err)?;
        Ok(json!({
            "tell_after_test": tell_after_test,
            "tell_after_digits": tell_after_digits,
            "file_exists": path.exists(),
            "file_size": file_size(path)?,
            "test": String::from_utf8_lossy(&test),
            "digits": digits,
            "eof_tell": -1,
        }))
    }
}

fn write_file(path: &Path) -> Result<std::fs::File, String> {
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|err| format!("failed to open output file '{}': {err}", path.display()))
}

fn read_file(path: &Path) -> Result<std::fs::File, String> {
    OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|err| format!("failed to open input file '{}': {err}", path.display()))
}

fn file_size(path: &Path) -> Result<u64, String> {
    metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|err| format!("failed to stat file '{}': {err}", path.display()))
}

fn read_err(err: std::io::Error) -> String {
    format!("failed to read local VSI test file: {err}")
}

fn write_err(err: std::io::Error) -> String {
    format!("failed to write local VSI test file: {err}")
}

fn seek_err(err: std::io::Error) -> String {
    format!("failed to seek local VSI test file: {err}")
}

fn position_err(err: std::io::Error) -> String {
    format!("failed to tell local VSI test file: {err}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_seek_scenarios() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let summary = local_io_scenario(temp.path(), "seeks_large_buffer", 1024).unwrap();
        assert_eq!(summary["tell_after_test"], 14);
        assert_eq!(summary["tell_after_digits"], 116);
        assert_eq!(summary["file_size"], 116);
        assert_eq!(summary["test"], "TEST");
        assert_eq!(summary["digits"], "12345");
    }

    #[test]
    fn runs_tell_scenario() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let summary = local_io_scenario(temp.path(), "tells", 1024).unwrap();
        assert_eq!(summary["tell_after_test"], 4);
        assert_eq!(summary["tell_after_digits"], 9);
        assert_eq!(summary["file_size"], 9);
        assert_eq!(summary["tell_after_one"], 1);
        assert_eq!(summary["tell_after_est"], 4);
        assert_eq!(summary["est"], "EST");
        assert_eq!(summary["digits"], "12345");
    }

    #[test]
    fn runs_small_seek_scenario() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let summary = local_io_scenario(temp.path(), "seeks_small_buffer", 1024).unwrap();
        assert_eq!(summary["tell_after_test"], 14);
        assert_eq!(summary["tell_after_digits"], 6);
        assert_eq!(summary["file_size"], 14);
        assert_eq!(summary["tail"], "TEST");
        assert_eq!(summary["tell_after_digits_read"], 6);
        assert_eq!(summary["digits"], "12345");
    }

    #[test]
    fn rejects_unknown_scenario() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let err = local_io_scenario(temp.path(), "nope", 1024).unwrap_err();
        assert_eq!(err, "unknown VSI scenario 'nope'");
    }
}
