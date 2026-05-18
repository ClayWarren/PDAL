//! Future plugin compatibility surface for the PDAL Rust port.
//!
//! Do not port optional plugins here until the Rust core and first-party stage
//! surface, I/O, apps/tools, and command path are stable. Do not design a Rust
//! plugin loading SDK until a versioned dynamic-library boundary has been
//! designed. This crate only owns metadata and discovery helpers that mirror
//! the stable parts of PDAL's existing plugin contract.

mod discovery;
mod info;

pub use discovery::{dynamic_library_extension, plugin_name_from_filename};
pub use info::{PluginInfo, PluginKind};
