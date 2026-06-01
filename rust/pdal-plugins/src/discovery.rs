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
    valid_plugin_stem(
        stem,
        &[
            PluginKind::Reader,
            PluginKind::Writer,
            PluginKind::Filter,
            PluginKind::Kernel,
        ],
        &extension,
    )
}

/// Return the PDAL stage name represented by a plugin filename.
///
/// `types` is the set of singular plugin type names accepted by the caller,
/// such as `reader`, `writer`, `filter`, or a test-only custom type.
/// `dynamic_lib_extension` includes the leading dot.
pub fn valid_plugin_name(
    path: impl AsRef<Path>,
    types: &[&str],
    dynamic_lib_extension: &str,
) -> Option<String> {
    let filename = path.as_ref().file_name()?.to_str()?;
    let stem = filename.strip_prefix("libpdal_plugin_")?;
    valid_plugin_stem_for_types(stem, types, dynamic_lib_extension)
}

fn valid_plugin_stem(
    stem: &str,
    kinds: &[PluginKind],
    dynamic_lib_extension: &str,
) -> Option<String> {
    let (kind, name) = plugin_stem_parts(stem, dynamic_lib_extension)?;
    let kind: PluginKind = kind.parse().ok()?;
    if !kinds.contains(&kind) {
        return None;
    }
    Some(format!("{}.{}", kind.stage_prefix(), name))
}

fn valid_plugin_stem_for_types(
    stem: &str,
    types: &[&str],
    dynamic_lib_extension: &str,
) -> Option<String> {
    let (kind, name) = plugin_stem_parts(stem, dynamic_lib_extension)?;
    if !types.iter().any(|ty| ty == &kind) {
        return None;
    }
    Some(format!("{kind}s.{name}"))
}

fn plugin_stem_parts<'a>(stem: &'a str, dynamic_lib_extension: &str) -> Option<(&'a str, &'a str)> {
    let (kind, name) = stem.split_once('_')?;
    let name = name.strip_suffix(dynamic_lib_extension)?;
    if name.is_empty() || name.contains('.') || !is_valid_plugin_short_name(name) {
        return None;
    }
    Some((kind, name))
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
    ch.is_ascii_alphanumeric() || ch == '_'
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
    fn plugin_filename_accepts_cpp_short_name_characters() {
        assert_eq!(
            plugin_name_from_filename(filename("filter", "with_underscore")),
            Some("filters.with_underscore".to_string())
        );
    }

    #[test]
    fn plugin_filename_requires_kind_name_separator() {
        assert_eq!(
            plugin_name_from_filename(format!(
                "/tmp/libpdal_plugin_reader.{}",
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
        assert_eq!(
            plugin_name_from_filename(filename("reader", "bad-name")),
            None
        );
        assert_eq!(plugin_name_from_filename(filename("reader", "")), None);
    }

    #[test]
    fn validates_plugin_filenames_like_cpp_contract() {
        let types = ["reader", "writer"];
        let ext = ".dylib";

        assert_eq!(valid_plugin_name("I'm a plugin", &["foo"], ext), None);
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_ac.dylib.dylib", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_a_b_c.dylib.dylib", &types, ext),
            None
        );
        assert_eq!(valid_plugin_name("reader_a.dylib", &types, ext), None);
        assert_eq!(
            valid_plugin_name("libpdal_plugin_rea_a.dylib", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_a.dylib", &["foo"], ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_writer.dylib", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_writer_.dylib", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_1a_b.dylib", &types, ext),
            None
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_a+_b.dylib", &types, ext),
            None
        );

        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_aP.dylib", &types, ext),
            Some("readers.aP".into())
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_Pa.dylib", &types, ext),
            Some("readers.Pa".into())
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_a.dylib", &types, ext),
            Some("readers.a".into())
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_foo_foo.dylib", &["foo"], ext),
            Some("foos.foo".into())
        );
        assert_eq!(
            valid_plugin_name("libpdal_plugin_reader_a_b.dylib", &types, ext),
            Some("readers.a_b".into())
        );
    }
}
