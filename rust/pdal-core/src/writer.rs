//! Writer filename-template helpers ported from `pdal/Writer.cpp`.
//!
//! These mirror the static helpers `pdal::Writer::handleFilenameTemplate` and
//! `pdal::Writer::replaceTags`, which resolve the `#` placeholder and `#uuid#`
//! tag used by `FlexWriter`-based output filenames.

use crate::uuid::{random_v4_uuid_bytes, unparse_uuid};

/// Result of validating a writer filename template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilenameTemplate {
    /// No `#` placeholder; the writer produces a single output file.
    NoPlaceholder,
    /// A `#` placeholder at the given byte offset within the filename.
    Placeholder(usize),
}

/// Locate and validate the `#` template placeholder in a writer filename.
///
/// Mirrors `pdal::Writer::handleFilenameTemplate`: the placeholder may not
/// appear in the filename suffix, and only a single placeholder is allowed.
pub fn handle_filename_template(filename: &str) -> Result<FilenameTemplate, String> {
    let suffix_pos = filename.rfind('.');
    let Some(hash_pos) = filename.find('#') else {
        return Ok(FilenameTemplate::NoPlaceholder);
    };

    if let Some(suffix_pos) = suffix_pos {
        if hash_pos > suffix_pos {
            return Err("Filename template placeholder ('#') is not \
                        allowed in filename suffix."
                .to_string());
        }
    }

    if filename[hash_pos + 1..].contains('#') {
        return Err("Filename specification can only contain \
                    a single '#' template placeholder."
            .to_string());
    }

    Ok(FilenameTemplate::Placeholder(hash_pos))
}

/// Replace each `#uuid#` tag in `filename` with a fresh lowercase random UUID.
///
/// Mirrors `pdal::Writer::replaceTags`. Each occurrence receives its own UUID.
pub fn replace_filename_tags(filename: &str) -> Result<String, String> {
    const TAG: &str = "#uuid#";
    let mut result = filename.to_string();
    while let Some(pos) = result.find(TAG) {
        let bytes =
            random_v4_uuid_bytes().map_err(|err| format!("failed to generate uuid: {err}"))?;
        let uuid = unparse_uuid(&bytes).to_lowercase();
        result.replace_range(pos..pos + TAG.len(), &uuid);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_placeholder_returns_none() {
        assert_eq!(
            handle_filename_template("output.las").unwrap(),
            FilenameTemplate::NoPlaceholder
        );
    }

    #[test]
    fn single_placeholder_returns_position() {
        assert_eq!(
            handle_filename_template("out_#.las").unwrap(),
            FilenameTemplate::Placeholder(4)
        );
        assert_eq!(
            handle_filename_template("#_foo.txt").unwrap(),
            FilenameTemplate::Placeholder(0)
        );
    }

    #[test]
    fn placeholder_in_suffix_is_rejected() {
        let err = handle_filename_template("output.la#s").unwrap_err();
        assert!(err.contains("filename suffix"));
    }

    #[test]
    fn multiple_placeholders_are_rejected() {
        let err = handle_filename_template("out_#_#.las").unwrap_err();
        assert!(err.contains("single '#'"));
    }

    #[test]
    fn placeholder_without_suffix_is_allowed() {
        assert_eq!(
            handle_filename_template("output_#").unwrap(),
            FilenameTemplate::Placeholder(7)
        );
    }

    #[test]
    fn replace_tags_leaves_untagged_filename_unchanged() {
        assert_eq!(
            replace_filename_tags("output_#.las").unwrap(),
            "output_#.las"
        );
    }

    #[test]
    fn replace_tags_substitutes_lowercase_uuid() {
        let result = replace_filename_tags("#_#uuid#_foo.txt").unwrap();
        // The `#uuid#` tag (6 chars) is replaced by a 36-char canonical UUID.
        assert_eq!(result.len(), "#_#uuid#_foo.txt".len() - 6 + 36);
        assert!(result.starts_with("#_"));
        assert!(result.ends_with("_foo.txt"));
        let uuid = &result["#_".len()..result.len() - "_foo.txt".len()];
        assert_eq!(uuid.len(), 36);
        assert!(uuid
            .chars()
            .all(|c| c == '-' || (c.is_ascii_hexdigit() && !c.is_ascii_uppercase())));
    }

    #[test]
    fn replace_tags_substitutes_each_occurrence_distinctly() {
        let result = replace_filename_tags("#uuid#/#uuid#").unwrap();
        let parts: Vec<&str> = result.split('/').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 36);
        assert_eq!(parts[1].len(), 36);
        assert_ne!(parts[0], parts[1]);
    }
}
