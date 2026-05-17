//! Stage traits -- the Rust analog of PDAL's `Stage` / `Filter` / `Streamable`.

use crate::point::PointView;

/// An error raised while constructing or running a stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageError(pub String);

impl std::fmt::Display for StageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StageError {}

/// A processing stage that transforms an input view into a set of output
/// views (PDAL's `Filter::run`). A filter may emit one view (the common case)
/// or several (e.g. splitting stages).
pub trait Filter {
    /// The stage name, e.g. `"filters.decimation"`.
    fn name(&self) -> &str;

    /// Run the filter over `input`, producing the output view(s).
    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError>;

    /// Support downcasting to concrete type.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// A stage that can also process points one at a time (PDAL's `Streamable`).
///
/// The implementer keeps its own running point counter, exactly as PDAL's
/// streaming stages do; `reset` clears it before a run.
pub trait Streamable {
    /// Reset streaming state before a run (PDAL's `ready`).
    fn reset(&mut self) {}

    /// Decide whether to keep the next point. Returns `true` to keep it.
    fn process_one(&mut self) -> bool;
}
