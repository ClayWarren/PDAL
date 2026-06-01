//! Stage traits -- the Rust analog of PDAL's `Stage` / `Filter` / `Streamable`.

use crate::point::{DimId, DimType, PointId, PointView};

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

    /// Run the filter taking ownership of its inputs.
    ///
    /// The default delegates to the borrowing [`run`](Self::run). Filters that
    /// can transform a view in place (such as `filters.sort`) override this to
    /// avoid allocating a second full copy of the point buffer. Only the
    /// in-process pipeline executor uses this path; the C ABI filter bridge
    /// still goes through `run`, so overriding it must stay observably
    /// identical to `run`.
    fn run_owned(&mut self, inputs: Vec<PointView>) -> Result<Vec<PointView>, StageError> {
        self.run(&inputs)
    }

    /// Whether this filter can run in PDAL streaming mode (point-by-point over
    /// fixed-size chunks). Point-wise filters (assign, range, decimation,
    /// ferry) are streamable; filters that need all points at once (sort,
    /// stats, voxel) are not. Default: not streamable.
    fn streamable(&self) -> bool {
        false
    }

    /// Transform one streaming chunk in place: mutate points and/or compact the
    /// view down to the kept points. Only called when
    /// [`streamable`](Self::streamable) returns true, and must produce the same
    /// per-point result as the materializing [`run`](Self::run).
    fn stream_chunk(&mut self, _chunk: &mut PointView) -> Result<(), StageError> {
        Err(StageError("filter does not support streaming".into()))
    }

    /// Run the filter over a single input view.
    ///
    /// This is the primary implementation point for most filters.
    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError>;

    /// Dimensions this filter writes and therefore needs prepared on its
    /// input layout before execution.
    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        Vec::new()
    }

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
