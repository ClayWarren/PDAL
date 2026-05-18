use crate::PluginKind;
use std::path::Path;

/// Platform dynamic library extension without the leading dot.
pub fn dynamic_library_extension() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "dylib"
    }

    #[cfg(target_os = "windows")]
    {
        "dll"
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "so"
    }
}

/// Return the PDAL stage name represented by a dynamic plugin filename.
///
/// This mirrors the existing C++ convention:
/// `libpdal_plugin_<kind>_<name>.<extension>` becomes
/// `<plural-kind>.<name>`.
pub fn plugin_name_from_filename(path: impl AsRef<Path>) -> Option<String> {
    let filename = path.as_ref().file_name()?.to_str()?;
    let stem = filename.strip_prefix("libpdal_plugin_")?;
    let extension = format!(".{}", dynamic_library_extension());
    let stem = stem.strip_suffix(&extension)?;
    let (kind, name) = stem.split_once('_')?;
    if name.is_empty() || !is_valid_plugin_short_name(name) {
        return None;
    }

    let kind: PluginKind = kind.parse().ok()?;
    Some(format!("{}.{}", kind.stage_prefix(), name))
}

fn is_valid_plugin_short_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_name_start(first) {
        return false;
    }
    chars.all(is_name_continue)
}

fn is_name_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
}

fn is_name_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filename(kind: &str, name: &str) -> String {
        format!(
            "/tmp/libpdal_plugin_{}_{}.{}",
            kind,
            name,
            dynamic_library_extension()
        )
    }

    #[test]
    fn plugin_filename_maps_to_stage_name() {
        assert_eq!(
            plugin_name_from_filename(filename("reader", "example")),
            Some("readers.example".to_string())
        );
        assert_eq!(
            plugin_name_from_filename(filename("writer", "text")),
            Some("writers.text".to_string())
        );
        assert_eq!(
            plugin_name_from_filename(filename("filter", "color")),
            Some("filters.color".to_string())
        );
        assert_eq!(
            plugin_name_from_filename(filename("kernel", "faux")),
            Some("kernels.faux".to_string())
        );
    }

    #[test]
    fn plugin_filename_must_use_pdal_prefix() {
        assert_eq!(
            plugin_name_from_filename(format!(
                "/tmp/other_plugin_reader_example.{}",
                dynamic_library_extension()
            )),
            None
        );
    }

    #[test]
    fn plugin_filename_must_use_supported_kind() {
        assert_eq!(
            plugin_name_from_filename(filename("stage", "example")),
            None
        );
    }

    #[test]
    fn plugin_filename_must_use_platform_extension() {
        assert_eq!(
            plugin_name_from_filename("/tmp/libpdal_plugin_reader_example.txt"),
            None
        );
    }

    #[test]
    fn plugin_filename_rejects_invalid_stage_names() {
        assert_eq!(plugin_name_from_filename(filename("reader", "9bad")), None);
        assert_eq!(
            plugin_name_from_filename(filename("reader", "bad.name")),
            None
        );
        assert_eq!(plugin_name_from_filename(filename("reader", "")), None);
    }
}
