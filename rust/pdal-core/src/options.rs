//! Stage options -- the Rust analog of PDAL's `Options` / `ProgramArgs`.
//!
//! PDAL parses option strings into typed arguments; this keeps the same
//! string-keyed model with typed, defaulted getters.

use std::collections::HashMap;

/// A set of named option values for a stage.
#[derive(Debug, Default, Clone)]
pub struct Options {
    map: HashMap<String, String>,
}

impl Options {
    /// An empty option set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace an option. The value is stored as a string, mirroring
    /// PDAL, where options arrive as text and are parsed per-argument.
    pub fn add(&mut self, key: &str, value: impl ToString) -> &mut Self {
        self.map.insert(key.to_string(), value.to_string());
        self
    }

    /// Whether `key` was set.
    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Option parsed as `f64`, or `default` if unset or unparseable.
    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        self.map
            .get(key)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(default)
    }

    /// Option parsed as `u64`, or `default` if unset or unparseable.
    pub fn get_u64(&self, key: &str, default: u64) -> u64 {
        self.map
            .get(key)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(default)
    }

    /// Option as string, or `default` if unset.
    pub fn get_str(&self, key: &str, default: &str) -> String {
        self.map
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    /// Option parsed as `bool`, or `default` if unset or unparseable.
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.map
            .get(key)
            .and_then(|v| match v.trim().to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            })
            .unwrap_or(default)
    }
}
