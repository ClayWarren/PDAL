//! Future plugin SDK surface for the PDAL Rust port.
//!
//! Do not port optional plugins here until the Rust core and first-party stage
//! surface are stable and a versioned plugin boundary has been designed. This
//! crate only owns metadata and discovery helpers that mirror the stable parts
//! of PDAL's existing plugin contract.

mod discovery;
mod info;

pub use discovery::{dynamic_library_extension, plugin_name_from_filename};
pub use info::{PluginInfo, PluginKind};
