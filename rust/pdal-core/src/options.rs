//! Stage options -- the Rust analog of PDAL's `Options` / `ProgramArgs`.
//!
//! PDAL parses option strings into typed arguments; this keeps the same
//! string-keyed model with typed, defaulted getters.

use std::collections::BTreeMap;

/// A set of named option values for a stage.
#[derive(Debug, Default, Clone)]
pub struct Options {
    map: BTreeMap<String, Vec<String>>,
}

impl Options {
    /// An empty option set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an option. The value is stored as a string, mirroring
    /// PDAL, where options arrive as text and are parsed per-argument.
    pub fn add(&mut self, key: &str, value: impl ToString) -> &mut Self {
        self.map
            .entry(key.to_string())
            .or_default()
            .push(value.to_string());
        self
    }

    /// Add an option only if no value with this key already exists.
    pub fn add_conditional(&mut self, key: &str, value: impl ToString) -> &mut Self {
        if !self.has(key) {
            self.add(key, value);
        }
        self
    }

    /// Append all options from another set, preserving duplicate-key order.
    pub fn extend(&mut self, other: &Options) -> &mut Self {
        for (key, values) in &other.map {
            for value in values {
                self.add(key, value);
            }
        }
        self
    }

    /// Append options from another set only for keys missing from this set.
    pub fn extend_conditional(&mut self, other: &Options) -> &mut Self {
        for (key, values) in &other.map {
            if !self.has(key) {
                for value in values {
                    self.add(key, value);
                }
            }
        }
        self
    }

    /// Remove every value for an option key.
    pub fn remove(&mut self, key: &str) -> &mut Self {
        self.map.remove(key);
        self
    }

    /// Replace every value for an option key with a single new value.
    pub fn replace(&mut self, key: &str, value: impl ToString) -> &mut Self {
        self.remove(key).add(key, value)
    }

    /// Whether `key` was set.
    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Number of option entries, including duplicate keys.
    pub fn len(&self) -> usize {
        self.map.values().map(Vec::len).sum()
    }

    /// Whether no options are set.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the key/value pair at a stable sorted-key index.
    pub fn entry(&self, index: usize) -> Option<(&str, &str)> {
        let mut pos = 0;
        for (key, values) in &self.map {
            for value in values {
                if pos == index {
                    return Some((key.as_str(), value.as_str()));
                }
                pos += 1;
            }
        }
        None
    }

    /// Return all options as `--key=value` arguments in stable key order.
    pub fn to_command_line(&self) -> Vec<String> {
        self.map
            .iter()
            .flat_map(|(key, values)| values.iter().map(move |value| format!("--{key}={value}")))
            .collect()
    }

    fn last_value(&self, key: &str) -> Option<&str> {
        self.map
            .get(key)
            .and_then(|values| values.last())
            .map(String::as_str)
    }

    /// Option parsed as `f64`, or `default` if unset or unparseable.
    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        self.map
            .get(key)
            .and_then(|values| values.last())
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(default)
    }

    /// Option parsed as `f64`, or `default` if unset.
    ///
    /// Unlike `get_f64`, this reports malformed values. Use this at user input
    /// boundaries where silently falling back to defaults would hide bad
    /// pipeline JSON or CLI options.
    pub fn try_get_f64(&self, key: &str, default: f64) -> Result<f64, String> {
        self.map
            .get(key)
            .and_then(|values| values.last())
            .map(|value| {
                value
                    .trim()
                    .parse()
                    .map_err(|_| format!("Option '{key}' must be a floating-point value."))
            })
            .unwrap_or(Ok(default))
    }

    /// Option parsed as `u64`, or `default` if unset or unparseable.
    pub fn get_u64(&self, key: &str, default: u64) -> u64 {
        self.map
            .get(key)
            .and_then(|values| values.last())
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(default)
    }

    /// Option parsed as `u64`, or `default` if unset.
    ///
    /// Unlike `get_u64`, this reports malformed or negative values.
    pub fn try_get_u64(&self, key: &str, default: u64) -> Result<u64, String> {
        self.map
            .get(key)
            .and_then(|values| values.last())
            .map(|value| {
                value
                    .trim()
                    .parse()
                    .map_err(|_| format!("Option '{key}' must be an unsigned integer."))
            })
            .unwrap_or(Ok(default))
    }

    /// Option as string, or `default` if unset.
    pub fn get_str(&self, key: &str, default: &str) -> String {
        self.last_value(key)
            .map(str::to_string)
            .unwrap_or_else(|| default.to_string())
    }

    /// Option as string, or `None` if unset.
    pub fn value(&self, key: &str) -> Option<&str> {
        self.last_value(key)
    }

    /// All values for an option key, preserving insertion order.
    pub fn values(&self, key: &str) -> &[String] {
        self.map.get(key).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Option parsed as `bool`, or `default` if unset or unparseable.
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.map
            .get(key)
            .and_then(|values| values.last())
            .and_then(|v| match v.trim().to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            })
            .unwrap_or(default)
    }

    /// Option parsed as `bool`, or `default` if unset.
    ///
    /// Accepts the same PDAL-style boolean spellings as `get_bool`, but reports
    /// malformed values instead of silently using the default.
    pub fn try_get_bool(&self, key: &str, default: bool) -> Result<bool, String> {
        self.map
            .get(key)
            .and_then(|values| values.last())
            .map(|value| match value.trim().to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Ok(true),
                "false" | "0" | "no" | "off" => Ok(false),
                _ => Err(format!("Option '{key}' must be a boolean value.")),
            })
            .unwrap_or(Ok(default))
    }
}

/// Return whether `name` is a valid PDAL option name.
///
/// Valid names start with a lowercase ASCII letter and then contain only
/// lowercase ASCII letters, digits, or `_`, matching `Option::nameValid`.
pub fn option_name_valid(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_getters_parse_text_and_fall_back_to_defaults() {
        let mut options = Options::new();
        options
            .add("float", " 12.5 ")
            .add("uint", "42")
            .add("bad_float", "nope")
            .add("bad_uint", "-1")
            .add("text", "value");

        assert!(options.has("float"));
        assert!(!options.has("missing"));
        assert_eq!(options.get_f64("float", 0.0), 12.5);
        assert_eq!(options.get_f64("bad_float", 9.0), 9.0);
        assert_eq!(options.get_u64("uint", 0), 42);
        assert_eq!(options.get_u64("bad_uint", 9), 9);
        assert_eq!(options.get_str("text", "fallback"), "value");
        assert_eq!(options.get_str("missing", "fallback"), "fallback");

        assert_eq!(options.try_get_f64("float", 0.0).unwrap(), 12.5);
        assert!(options.try_get_f64("bad_float", 9.0).is_err());
        assert_eq!(options.try_get_u64("uint", 0).unwrap(), 42);
        assert!(options.try_get_u64("bad_uint", 9).is_err());
    }

    #[test]
    fn bool_getter_accepts_pdal_style_boolean_spellings() {
        let truthy = ["true", "TRUE", "1", "yes", "on"];
        let falsy = ["false", "FALSE", "0", "no", "off"];

        for value in truthy {
            let mut options = Options::new();
            options.add("flag", value);
            assert!(options.get_bool("flag", false));
        }

        for value in falsy {
            let mut options = Options::new();
            options.add("flag", value);
            assert!(!options.get_bool("flag", true));
        }

        let mut options = Options::new();
        options.add("flag", "not-bool");
        assert!(options.get_bool("flag", true));
        assert!(!options.get_bool("missing", false));
        assert!(options.try_get_bool("flag", true).is_err());
        assert!(options.try_get_bool("missing", true).unwrap());
    }

    #[test]
    fn adding_same_key_preserves_entries_and_typed_getters_use_last_value() {
        let mut options = Options::new();
        options.add("count", "1").add("count", "2");

        assert_eq!(options.len(), 2);
        assert_eq!(options.entry(0), Some(("count", "1")));
        assert_eq!(options.entry(1), Some(("count", "2")));
        assert_eq!(options.get_u64("count", 0), 2);
    }

    #[test]
    fn conditional_add_and_extend_match_pdal_key_semantics() {
        let mut base = Options::new();
        base.add("count", "1")
            .add_conditional("count", "ignored")
            .add_conditional("mode", "base");

        let mut other = Options::new();
        other.add("count", "2").add("other", "a").add("other", "b");
        base.extend_conditional(&other);

        assert_eq!(base.values("count"), &["1".to_string()]);
        assert_eq!(base.values("mode"), &["base".to_string()]);
        assert_eq!(base.values("other"), &["a".to_string(), "b".to_string()]);

        base.extend(&other);
        assert_eq!(base.values("count"), &["1".to_string(), "2".to_string()]);
        assert_eq!(
            base.values("other"),
            &[
                "a".to_string(),
                "b".to_string(),
                "a".to_string(),
                "b".to_string()
            ]
        );
    }

    #[test]
    fn remove_and_replace_operate_on_all_values_for_key() {
        let mut options = Options::new();
        options
            .add("count", "1")
            .add("count", "2")
            .add("mode", "before");

        options.replace("count", "3");
        assert_eq!(options.values("count"), &["3".to_string()]);
        assert_eq!(options.get_u64("count", 0), 3);

        options.remove("mode");
        assert!(!options.has("mode"));
        assert_eq!(options.len(), 1);
    }

    #[test]
    fn entries_and_command_line_are_sorted_by_key() {
        let mut options = Options::new();
        options.add("zeta", "last").add("alpha", "first");

        assert_eq!(options.len(), 2);
        assert_eq!(options.entry(0), Some(("alpha", "first")));
        assert_eq!(options.entry(1), Some(("zeta", "last")));
        assert_eq!(options.entry(2), None);
        assert_eq!(
            options.to_command_line(),
            vec!["--alpha=first".to_string(), "--zeta=last".to_string()]
        );
    }

    #[test]
    fn validates_option_names_like_pdal() {
        assert!(option_name_valid("foo_123_bar_baz"));
        assert!(!option_name_valid(""));
        assert!(!option_name_valid("foo_123_bar-baz"));
        assert!(!option_name_valid("Afoo_123_bar_baz"));
        assert!(!option_name_valid("1foo_123_bar_baz"));
    }
}
