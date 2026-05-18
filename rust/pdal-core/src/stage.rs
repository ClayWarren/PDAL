//! Stage traits -- the Rust analog of PDAL's `Stage` / `Filter` / `Streamable`.

use crate::point::{PointId, PointView};

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

    /// Run the filter over `inputs`, producing the output view(s).
    ///
    /// The default implementation loops over all input views and calls
    /// `run_one` for each. Specialized filters like `filters.merge`
    /// should override this to process all inputs at once.
    fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError> {
        let mut outputs = Vec::new();
        for input in inputs {
            outputs.extend(self.run_one(input)?);
        }
        Ok(outputs)
    }

    /// Run the filter over a single input view.
    ///
    /// This is the primary implementation point for most filters.
    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError>;

    /// Export the stage's accumulated metadata, if any.
    fn metadata(&self) -> crate::metadata::MetadataNode {
        crate::metadata::MetadataNode::new("metadata")
    }

    /// Support downcasting to concrete type.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// A stage that can also process points one at a time (PDAL's `Streamable`).
pub trait Streamable {
    /// Reset streaming state before a run (PDAL's `ready`).
    fn reset(&mut self) {}

    /// Decide whether to keep point `idx` of `view`; `true` keeps it.
    ///
    /// Mirrors PDAL's `Streamable::processOne(PointRef&)`: the point is always
    /// passed. Counter-based filters (decimation, head, tail) ignore it, just
    /// as `DecimationFilter::processOne` ignores its `PointRef&` in C++.
    fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool;
}
