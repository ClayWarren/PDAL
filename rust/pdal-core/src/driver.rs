//! Reader/writer driver inference from filenames.
//!
//! Port of `StageFactory::inferReaderDriver` / `inferWriterDriver`. Given a
//! filename (or a few special path forms), this returns the PDAL driver name
//! that handles it -- the first step a command takes when building a pipeline
//! from `input`/`output` paths rather than an explicit stage type.
//!
//! The driver name is returned regardless of whether the Rust port currently
//! implements it; constructing an unimplemented driver is a separate, later
//! failure (a registry concern).

/// Reader driver names keyed by lowercase extension (no leading dot).
///
/// Combines the extensions declared by static reader stages with the
/// dynamic-stage table in PDAL's `StageExtensions`.
const READER_EXTENSIONS: &[(&str, &str)] = &[
    ("txt", "readers.text"),
    ("csv", "readers.text"),
    ("pcd", "readers.pcd"),
    ("pts", "readers.pts"),
    ("ptx", "readers.ptx"),
    ("ply", "readers.ply"),
    ("las", "readers.las"),
    ("laz", "readers.las"),
    ("sbet", "readers.sbet"),
    ("copc", "readers.copc"),
    ("ept", "readers.ept"),
    ("bin", "readers.fbi"),
    ("fbi", "readers.fbi"),
    ("fbx", "readers.fbx"),
    ("tif", "readers.gdal"),
    ("tiff", "readers.gdal"),
    ("jpeg", "readers.gdal"),
    ("jpg", "readers.gdal"),
    ("png", "readers.gdal"),
    ("feather", "readers.arrow"),
    ("parquet", "readers.arrow"),
    ("drc", "readers.draco"),
    ("h5", "readers.icebridge"),
    ("icebridge", "readers.icebridge"),
    ("mat", "readers.matlab"),
    ("nitf", "readers.nitf"),
    ("nsf", "readers.nitf"),
    ("ntf", "readers.nitf"),
    ("rdbx", "readers.rdb"),
    ("sid", "readers.mrsid"),
    ("rxp", "readers.rxp"),
    ("slpk", "readers.slpk"),
    ("i3s", "readers.i3s"),
    ("obj", "readers.obj"),
    ("vpc", "readers.stac"),
    ("e57", "readers.e57"),
];

/// Writer driver names keyed by lowercase extension (no leading dot).
///
/// The empty-string key handles output paths with no extension, which PDAL's
/// `writers.text` registers.
const WRITER_EXTENSIONS: &[(&str, &str)] = &[
    ("csv", "writers.text"),
    ("txt", "writers.text"),
    ("json", "writers.text"),
    ("xyz", "writers.text"),
    ("", "writers.text"),
    ("pcd", "writers.pcd"),
    ("ply", "writers.ply"),
    ("las", "writers.las"),
    ("laz", "writers.las"),
    ("fbi", "writers.fbi"),
    ("feather", "writers.arrow"),
    ("parquet", "writers.arrow"),
    ("drc", "writers.draco"),
    ("mat", "writers.matlab"),
    ("nitf", "writers.nitf"),
    ("nsf", "writers.nitf"),
    ("ntf", "writers.nitf"),
    ("e57", "writers.e57"),
    ("fbx", "writers.fbx"),
];

/// Infer the reader driver for `filename` (PDAL's `inferReaderDriver`).
///
/// Returns `None` when no driver is associated with the extension.
pub fn infer_reader_driver(filename: &str) -> Option<&'static str> {
    if filename.ends_with("ept.json") || filename.starts_with("ept://") {
        return Some("readers.ept");
    }
    if filename.starts_with("i3s://") {
        return Some("readers.i3s");
    }
    if filename.ends_with(".copc.laz") {
        return Some("readers.copc");
    }
    lookup(READER_EXTENSIONS, &extension(filename))
}

/// Infer the writer driver for `filename` (PDAL's `inferWriterDriver`).
///
/// Returns `None` when no driver is associated with the extension.
pub fn infer_writer_driver(filename: &str) -> Option<&'static str> {
    let lower = filename.to_lowercase();
    if lower == "devnull" || lower == "/dev/null" {
        return Some("writers.null");
    }
    if filename.ends_with(".copc.laz") {
        return Some("writers.copc");
    }
    // `stdout` is treated as text output.
    let ext = if lower == "stdout" {
        "txt".to_string()
    } else {
        extension(filename)
    };
    lookup(WRITER_EXTENSIONS, &ext)
}

fn lookup(table: &[(&str, &'static str)], ext: &str) -> Option<&'static str> {
    table
        .iter()
        .find(|(entry, _)| *entry == ext)
        .map(|(_, driver)| *driver)
}

/// The lowercase extension of `filename` without the leading dot, or an empty
/// string when there is none (PDAL's `FileUtils::extension`, post-processed).
///
/// A leading-dot name (e.g. `.bashrc`) has no extension.
fn extension(filename: &str) -> String {
    let name = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
    match name.rfind('.') {
        None | Some(0) => String::new(),
        Some(pos) => name[pos + 1..].to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_ported_reader_drivers_by_extension() {
        assert_eq!(infer_reader_driver("cloud.ply"), Some("readers.ply"));
        assert_eq!(infer_reader_driver("scan.ptx"), Some("readers.ptx"));
        assert_eq!(infer_reader_driver("points.pts"), Some("readers.pts"));
        assert_eq!(infer_reader_driver("grid.pcd"), Some("readers.pcd"));
        assert_eq!(infer_reader_driver("table.txt"), Some("readers.text"));
        assert_eq!(infer_reader_driver("table.csv"), Some("readers.text"));
    }

    #[test]
    fn infers_reader_drivers_for_unimplemented_formats() {
        // Inference names the driver even when the Rust port can't build it.
        assert_eq!(infer_reader_driver("tile.las"), Some("readers.las"));
        assert_eq!(infer_reader_driver("tile.laz"), Some("readers.las"));
        assert_eq!(infer_reader_driver("flight.sbet"), Some("readers.sbet"));
    }

    #[test]
    fn reader_inference_is_case_insensitive_and_path_aware() {
        assert_eq!(infer_reader_driver("CLOUD.PLY"), Some("readers.ply"));
        assert_eq!(
            infer_reader_driver("/data/sub.dir/cloud.Ply"),
            Some("readers.ply")
        );
    }

    #[test]
    fn reader_inference_handles_special_path_forms() {
        assert_eq!(infer_reader_driver("dataset/ept.json"), Some("readers.ept"));
        assert_eq!(
            infer_reader_driver("ept://example.com/x"),
            Some("readers.ept")
        );
        assert_eq!(
            infer_reader_driver("i3s://example.com/x"),
            Some("readers.i3s")
        );
        assert_eq!(infer_reader_driver("tile.copc.laz"), Some("readers.copc"));
    }

    #[test]
    fn reader_inference_returns_none_for_unknown_or_missing_extensions() {
        assert_eq!(infer_reader_driver("mystery.xyzzy"), None);
        assert_eq!(infer_reader_driver("noextension"), None);
        assert_eq!(infer_reader_driver(".bashrc"), None);
    }

    #[test]
    fn infers_writer_drivers_by_extension() {
        assert_eq!(infer_writer_driver("out.ply"), Some("writers.ply"));
        assert_eq!(infer_writer_driver("out.pcd"), Some("writers.pcd"));
        assert_eq!(infer_writer_driver("out.csv"), Some("writers.text"));
        assert_eq!(infer_writer_driver("out.txt"), Some("writers.text"));
        assert_eq!(infer_writer_driver("out.las"), Some("writers.las"));
    }

    #[test]
    fn writer_inference_handles_special_path_forms() {
        assert_eq!(infer_writer_driver("devnull"), Some("writers.null"));
        assert_eq!(infer_writer_driver("/dev/null"), Some("writers.null"));
        assert_eq!(infer_writer_driver("stdout"), Some("writers.text"));
        assert_eq!(infer_writer_driver("tile.copc.laz"), Some("writers.copc"));
        // An output path with no extension is text output.
        assert_eq!(infer_writer_driver("output"), Some("writers.text"));
    }

    #[test]
    fn writer_inference_returns_none_for_unknown_extensions() {
        assert_eq!(infer_writer_driver("out.xyzzy"), None);
    }
}
