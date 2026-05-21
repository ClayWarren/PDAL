//! CLI kernels for the PDAL Rust port.
//!
//! Kernels are a late migration phase because they sit above the core,
//! pipeline, filters, and I/O layers. This crate owns the Rust-native command
//! contract while most concrete commands still delegate to the existing C++
//! implementation.

mod fauxplugin;
mod registry;
mod text;

pub use fauxplugin::FauxPluginKernel;
pub use registry::{Kernel, KernelArgs, KernelError, KernelRegistry, KernelSpec};
pub use text::word_wrap;
