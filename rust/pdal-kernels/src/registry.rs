use std::collections::BTreeMap;
use std::fmt;

/// Static user-visible metadata for a CLI kernel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelSpec {
    pub name: &'static str,
    pub description: &'static str,
}

/// Parsed command arguments passed to a Rust-backed kernel.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KernelArgs {
    values: Vec<String>,
}

impl KernelArgs {
    pub fn new<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            values: values.into_iter().map(Into::into).collect(),
        }
    }

    pub fn as_slice(&self) -> &[String] {
        &self.values
    }

    pub fn is_help_request(&self) -> bool {
        self.values.iter().any(|arg| arg == "--help" || arg == "-h")
    }
}

/// Error returned by Rust-backed kernels and registry dispatch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelError {
    message: String,
}

impl KernelError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for KernelError {}

/// Rust-native CLI command implementation.
pub trait Kernel {
    fn spec(&self) -> KernelSpec;

    fn run(&mut self, args: &KernelArgs) -> Result<i32, KernelError>;
}

/// Registry for Rust-backed kernels.
#[derive(Default)]
pub struct KernelRegistry {
    kernels: BTreeMap<String, Box<dyn Kernel>>,
}

impl KernelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, kernel: Box<dyn Kernel>) -> Result<(), KernelError> {
        let name = normalize_name(kernel.spec().name);
        if self.kernels.contains_key(&name) {
            return Err(KernelError::new(format!("duplicate kernel: {name}")));
        }
        self.kernels.insert(name, kernel);
        Ok(())
    }

    pub fn specs(&self) -> Vec<KernelSpec> {
        self.kernels.values().map(|kernel| kernel.spec()).collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.kernels.contains_key(&normalize_name(name))
    }

    pub fn run(&mut self, name: &str, args: &KernelArgs) -> Result<i32, KernelError> {
        let normalized = normalize_name(name);
        let kernel = self
            .kernels
            .get_mut(&normalized)
            .ok_or_else(|| KernelError::new(format!("unknown kernel: {normalized}")))?;
        kernel.run(args)
    }
}

fn normalize_name(name: &str) -> String {
    name.strip_prefix("kernels.").unwrap_or(name).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingKernel {
        args_seen: Vec<String>,
    }

    impl RecordingKernel {
        fn new() -> Self {
            Self {
                args_seen: Vec::new(),
            }
        }
    }

    impl Kernel for RecordingKernel {
        fn spec(&self) -> KernelSpec {
            KernelSpec {
                name: "kernels.record",
                description: "records arguments",
            }
        }

        fn run(&mut self, args: &KernelArgs) -> Result<i32, KernelError> {
            self.args_seen = args.as_slice().to_vec();
            Ok(args.as_slice().len() as i32)
        }
    }

    #[test]
    fn args_detect_help_requests() {
        assert!(KernelArgs::new(["--help"]).is_help_request());
        assert!(KernelArgs::new(["-h"]).is_help_request());
        assert!(!KernelArgs::new(["--input", "in.las"]).is_help_request());
    }

    #[test]
    fn registry_normalizes_names_for_lookup_and_dispatch() {
        let mut registry = KernelRegistry::new();
        registry.insert(Box::new(RecordingKernel::new())).unwrap();

        assert!(registry.contains("record"));
        assert!(registry.contains("kernels.record"));
        assert_eq!(
            registry
                .run("kernels.record", &KernelArgs::new(["--flag", "value"]))
                .unwrap(),
            2
        );
    }

    #[test]
    fn registry_rejects_duplicate_kernel_names() {
        let mut registry = KernelRegistry::new();
        registry.insert(Box::new(RecordingKernel::new())).unwrap();

        let err = registry
            .insert(Box::new(RecordingKernel::new()))
            .unwrap_err();
        assert_eq!(err.message(), "duplicate kernel: record");
    }

    #[test]
    fn registry_lists_specs_in_stable_order() {
        struct NamedKernel(&'static str);

        impl Kernel for NamedKernel {
            fn spec(&self) -> KernelSpec {
                KernelSpec {
                    name: self.0,
                    description: "",
                }
            }

            fn run(&mut self, _args: &KernelArgs) -> Result<i32, KernelError> {
                Ok(0)
            }
        }

        let mut registry = KernelRegistry::new();
        registry
            .insert(Box::new(NamedKernel("kernels.translate")))
            .unwrap();
        registry
            .insert(Box::new(NamedKernel("kernels.info")))
            .unwrap();

        let names: Vec<_> = registry.specs().iter().map(|spec| spec.name).collect();
        assert_eq!(names, vec!["kernels.info", "kernels.translate"]);
    }

    #[test]
    fn unknown_kernel_returns_error() {
        let mut registry = KernelRegistry::new();
        let err = registry.run("missing", &KernelArgs::default()).unwrap_err();
        assert_eq!(err.message(), "unknown kernel: missing");
    }
}
