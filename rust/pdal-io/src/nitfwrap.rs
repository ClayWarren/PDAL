//! `nitfwrap` support built on the Nitro native adapter.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const LAS_SIGNATURE: &[u8; 4] = b"LASF";
const BPF_SIGNATURE: &[u8; 4] = b"BPF!";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WrappedKind {
    Las,
    Laz,
    Bpf,
}

pub fn wrap(input: &Path, output: Option<&Path>) -> Result<PathBuf, String> {
    if !input.exists() {
        return Err(format!("Input file '{}' doesn't exist.", input.display()));
    }
    let kind = identify_input(input)?;
    let output = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| input.with_extension("ntf"));
    let title = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("pdal.ntf");
    let bounds = match kind {
        WrappedKind::Las | WrappedKind::Laz => las_bounds_or_default(input),
        WrappedKind::Bpf => [0.0, 0.0, 1.0, 1.0],
    };
    pdal_native::nitf::wrap(
        &path_text(input),
        &path_text(&output),
        title,
        normalized_bounds(bounds),
    )?;
    Ok(output)
}

pub fn unwrap(input: &Path, output: Option<&Path>) -> Result<PathBuf, String> {
    if !input.exists() {
        return Err(format!("Input file '{}' doesn't exist.", input.display()));
    }
    let (offset, length) = pdal_native::nitf::lidar_segment(&path_text(input))?;
    let kind = identify_wrapped(input, offset)?;
    let output = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| input.with_extension(kind.extension()));
    copy_range(input, &output, offset, length)?;
    Ok(output)
}

fn identify_input(path: &Path) -> Result<WrappedKind, String> {
    let mut file =
        File::open(path).map_err(|_| format!("Couldn't open input file '{}'.", path.display()))?;
    let mut signature = [0; 4];
    file.read_exact(&mut signature)
        .map_err(|_| "Input file must be LAS/LAZ or BPF.".to_string())?;
    if &signature == LAS_SIGNATURE {
        return Ok(if las_compressed(path) {
            WrappedKind::Laz
        } else {
            WrappedKind::Las
        });
    }
    if &signature == BPF_SIGNATURE {
        return Ok(WrappedKind::Bpf);
    }
    Err("Input file must be LAS/LAZ or BPF.".to_string())
}

fn identify_wrapped(path: &Path, offset: u64) -> Result<WrappedKind, String> {
    let mut file =
        File::open(path).map_err(|_| format!("Couldn't open input file '{}'.", path.display()))?;
    file.seek(SeekFrom::Start(offset)).map_err(io_error)?;
    let mut signature = [0; 4];
    file.read_exact(&mut signature)
        .map_err(|_| "Wrapped file isn't BPF or LAS.".to_string())?;
    if &signature == LAS_SIGNATURE {
        return Ok(if las_compressed_at(path, offset) {
            WrappedKind::Laz
        } else {
            WrappedKind::Las
        });
    }
    if &signature == BPF_SIGNATURE {
        return Ok(WrappedKind::Bpf);
    }
    Err("Wrapped file isn't BPF or LAS.".to_string())
}

fn copy_range(input: &Path, output: &Path, offset: u64, length: u64) -> Result<(), String> {
    let mut reader = File::open(input)
        .map_err(|_| format!("Couldn't open input file '{}'.", input.display()))?;
    reader.seek(SeekFrom::Start(offset)).map_err(io_error)?;
    let mut writer = File::create(output)
        .map_err(|_| format!("Couldn't create output file '{}'.", output.display()))?;
    let mut remaining = length;
    let mut buffer = [0; 8192];
    while remaining > 0 {
        let to_read = remaining.min(buffer.len() as u64) as usize;
        reader
            .read_exact(&mut buffer[..to_read])
            .map_err(io_error)?;
        writer.write_all(&buffer[..to_read]).map_err(io_error)?;
        remaining -= to_read as u64;
    }
    Ok(())
}

fn las_compressed(path: &Path) -> bool {
    las_compressed_at(path, 0)
}

fn las_compressed_at(path: &Path, offset: u64) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    if file.seek(SeekFrom::Start(offset + 104)).is_err() {
        return false;
    }
    let mut format = [0];
    if file.read_exact(&mut format).is_err() {
        return false;
    }
    format[0] & 0x80 != 0
}

fn las_bounds_or_default(path: &Path) -> [f64; 4] {
    let Ok(mut file) = File::open(path) else {
        return [0.0, 0.0, 1.0, 1.0];
    };
    if file.seek(SeekFrom::Start(179)).is_err() {
        return [0.0, 0.0, 1.0, 1.0];
    }
    let mut values = [0.0; 6];
    for value in &mut values {
        let mut bytes = [0; 8];
        if file.read_exact(&mut bytes).is_err() {
            return [0.0, 0.0, 1.0, 1.0];
        }
        *value = f64::from_le_bytes(bytes);
    }
    [values[3], values[4], values[2], values[1]]
}

fn normalized_bounds(mut bounds: [f64; 4]) -> [f64; 4] {
    if !(bounds[0].is_finite()
        && bounds[1].is_finite()
        && bounds[2].is_finite()
        && bounds[3].is_finite())
        || bounds[0] >= bounds[2]
        || bounds[1] >= bounds[3]
    {
        bounds = [0.0, 0.0, 1.0, 1.0];
    }
    bounds
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn io_error(err: std::io::Error) -> String {
    err.to_string()
}

impl WrappedKind {
    fn extension(self) -> &'static str {
        match self {
            WrappedKind::Las => "las",
            WrappedKind::Laz => "laz",
            WrappedKind::Bpf => "bpf",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "nitf")]
    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    #[cfg(feature = "nitf")]
    fn unwraps_existing_nitf_fixture() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("autzen.las");
        unwrap(
            &repo().join("test/data/nitf/autzen-utm10.ntf"),
            Some(&output),
        )
        .unwrap();

        assert_eq!(
            std::fs::read(output).unwrap(),
            std::fs::read(repo().join("test/data/nitf/autzen-utm10.las")).unwrap()
        );
    }

    #[test]
    #[cfg(feature = "nitf")]
    fn wraps_and_unwraps_las() {
        let temp = tempfile::tempdir().unwrap();
        let input = repo().join("test/data/las/simple.las");
        let nitf = temp.path().join("simple.ntf");
        let out = temp.path().join("simple.las");

        wrap(&input, Some(&nitf)).unwrap();
        unwrap(&nitf, Some(&out)).unwrap();

        assert_eq!(std::fs::read(out).unwrap(), std::fs::read(input).unwrap());
    }

    #[test]
    #[cfg(feature = "nitf")]
    fn uses_default_output_names() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("simple.las");
        std::fs::copy(repo().join("test/data/las/simple.las"), &input).unwrap();

        let nitf = wrap(&input, None).unwrap();
        assert_eq!(nitf, temp.path().join("simple.ntf"));
        assert!(nitf.exists());

        std::fs::remove_file(&input).unwrap();
        let out = unwrap(&nitf, None).unwrap();
        assert_eq!(out, temp.path().join("simple.las"));
        assert_eq!(
            std::fs::read(out).unwrap(),
            std::fs::read(repo().join("test/data/las/simple.las")).unwrap()
        );
    }

    #[test]
    #[cfg(feature = "nitf")]
    fn wraps_and_unwraps_bpf() {
        let temp = tempfile::tempdir().unwrap();
        let input = repo().join("test/data/bpf/autzen-dd.bpf");
        let nitf = temp.path().join("autzen-dd.ntf");
        let out = temp.path().join("autzen-dd.bpf");

        wrap(&input, Some(&nitf)).unwrap();
        unwrap(&nitf, Some(&out)).unwrap();

        assert_eq!(std::fs::read(out).unwrap(), std::fs::read(input).unwrap());
    }

    #[test]
    fn rejects_non_lidar_input() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("not-lidar.bin");
        std::fs::write(&input, b"nope").unwrap();

        assert_eq!(
            wrap(&input, Some(&temp.path().join("out.ntf"))).unwrap_err(),
            "Input file must be LAS/LAZ or BPF."
        );
    }

    #[test]
    fn rejects_missing_input() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("missing.las");

        assert_eq!(
            wrap(&input, Some(&temp.path().join("out.ntf"))).unwrap_err(),
            format!("Input file '{}' doesn't exist.", input.display())
        );
        assert_eq!(
            unwrap(&input, Some(&temp.path().join("out.las"))).unwrap_err(),
            format!("Input file '{}' doesn't exist.", input.display())
        );
    }

    #[test]
    fn helper_branches_match_pdal_tool_defaults() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing.las");
        assert!(!las_compressed(&missing));
        assert_eq!(las_bounds_or_default(&missing), [0.0, 0.0, 1.0, 1.0]);

        let short = temp.path().join("short.las");
        std::fs::write(&short, b"LASF").unwrap();
        assert!(!las_compressed(&short));
        assert_eq!(las_bounds_or_default(&short), [0.0, 0.0, 1.0, 1.0]);

        let compressed = temp.path().join("compressed.las");
        let mut bytes = vec![0; 105];
        bytes[..4].copy_from_slice(LAS_SIGNATURE);
        bytes[104] = 0x80;
        std::fs::write(&compressed, bytes).unwrap();
        assert_eq!(identify_input(&compressed).unwrap(), WrappedKind::Laz);

        assert_eq!(
            normalized_bounds([f64::NAN, 0.0, 1.0, 1.0]),
            [0.0, 0.0, 1.0, 1.0]
        );
        assert_eq!(
            normalized_bounds([5.0, 0.0, 1.0, 1.0]),
            [0.0, 0.0, 1.0, 1.0]
        );
        assert_eq!(WrappedKind::Laz.extension(), "laz");
        assert_eq!(WrappedKind::Bpf.extension(), "bpf");
    }

    #[test]
    fn wrapped_identification_and_copy_helpers_cover_offsets() {
        let temp = tempfile::tempdir().unwrap();
        let wrapped_bpf = temp.path().join("wrapped-bpf.bin");
        std::fs::write(&wrapped_bpf, b"prefixBPF!payload").unwrap();
        assert_eq!(identify_wrapped(&wrapped_bpf, 6).unwrap(), WrappedKind::Bpf);

        let wrapped_laz = temp.path().join("wrapped-laz.bin");
        let mut bytes = vec![0; 6 + 105];
        bytes[6..10].copy_from_slice(LAS_SIGNATURE);
        bytes[6 + 104] = 0x80;
        std::fs::write(&wrapped_laz, bytes).unwrap();
        assert_eq!(identify_wrapped(&wrapped_laz, 6).unwrap(), WrappedKind::Laz);

        let copied = temp.path().join("copied.bin");
        copy_range(&wrapped_bpf, &copied, 6, 4).unwrap();
        assert_eq!(std::fs::read(copied).unwrap(), b"BPF!");

        let bad_out = temp.path().join("missing-dir/out.bin");
        assert!(!copy_range(&wrapped_bpf, &bad_out, 6, 4)
            .unwrap_err()
            .is_empty());

        let bad_wrapped = temp.path().join("wrapped-bad.bin");
        std::fs::write(&bad_wrapped, b"prefixNOPE").unwrap();
        assert_eq!(
            identify_wrapped(&bad_wrapped, 6).unwrap_err(),
            "Wrapped file isn't BPF or LAS."
        );
    }

    #[test]
    fn las_bounds_reads_valid_header_order() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bounds.las");
        let mut bytes = vec![0; 179];
        for value in [40.0_f64, 10.0, 50.0, 20.0, 30.0, 0.0] {
            bytes.extend(value.to_le_bytes());
        }
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(las_bounds_or_default(&path), [20.0, 30.0, 50.0, 10.0]);
        assert_eq!(
            normalized_bounds([20.0, 30.0, 50.0, 10.0]),
            [0.0, 0.0, 1.0, 1.0]
        );
    }

    #[test]
    fn io_helpers_report_platform_errors() {
        let temp = tempfile::tempdir().unwrap();
        assert!(!las_compressed_at(temp.path(), 0));
        assert_eq!(las_bounds_or_default(temp.path()), [0.0, 0.0, 1.0, 1.0]);

        let message = io_error(std::io::Error::other("nitfwrap helper"));
        assert_eq!(message, "nitfwrap helper");
    }
}
