use crate::metadata::MetadataNode;
use crate::point::{DimId, DimType, PointView};
use crate::stage::StageError;

/// The kind of stage in a pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageKind {
    Reader,
    Filter,
    Writer,
}

/// Unified stage wrapper that can be a reader, filter, or writer.
pub trait StageWrapper {
    /// Run the filter over `inputs`.
    fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError>;

    /// Run the filter, taking ownership of `inputs` so in-place filters (e.g.
    /// `filters.sort`) can mutate and return a view without allocating a second
    /// copy. Defaults to the borrowing [`run`](Self::run).
    fn run_owned(&mut self, inputs: Vec<PointView>) -> Result<Vec<PointView>, StageError> {
        self.run(&inputs)
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError>;
    fn write(&mut self, views: &[PointView]) -> Result<(), StageError>;
    fn process_one(&mut self, view: &mut PointView, idx: crate::point::PointId) -> bool;
    fn reset(&mut self);
    fn metadata(&self) -> MetadataNode;
    fn name(&self) -> &str;
    fn kind(&self) -> StageKind;
    fn output_dimensions(&self) -> Vec<(DimId, DimType)>;

    // --- Streaming (PDAL streaming mode). Default: unsupported, so the
    // executor's streaming gate excludes the stage and falls back to the
    // materializing `read`/`run`/`write` path. ---

    /// Whether this stage can take part in chunked streaming execution.
    fn streamable(&self) -> bool {
        false
    }
    /// Reader: produce the next chunk of up to `capacity` points, or `None` at
    /// end of input. Only called when [`streamable`](Self::streamable) is true.
    fn stream_next(&mut self, _capacity: usize) -> Result<Option<PointView>, StageError> {
        Err(StageError("stage does not support streaming reads".into()))
    }
    /// Filter: transform a chunk in place (mutating and/or compacting kept
    /// points). Only called when [`streamable`](Self::streamable) is true.
    fn stream_chunk(&mut self, _chunk: &mut PointView) -> Result<(), StageError> {
        Err(StageError("stage does not support streaming".into()))
    }
    /// Writer: write one chunk incrementally.
    fn stream_write(&mut self, _chunk: &PointView) -> Result<(), StageError> {
        Err(StageError("stage does not support streaming writes".into()))
    }
    /// Writer: finalize after the last chunk (flush headers, close files).
    fn stream_finish(&mut self) -> Result<(), StageError> {
        Ok(())
    }
}

/// A reader produces point views from a source.
pub trait Reader {
    fn name(&self) -> &str;
    fn read(&mut self) -> Result<Vec<PointView>, StageError>;
    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("metadata")
    }
    /// Reset streaming cursor/state before a run. Default no-op.
    fn reset(&mut self) {}
    /// Whether this reader can produce points in chunks via [`stream_next`].
    fn streamable(&self) -> bool {
        false
    }
    /// Produce the next chunk of up to `capacity` points, or `None` at end.
    fn stream_next(&mut self, _capacity: usize) -> Result<Option<PointView>, StageError> {
        Err(StageError("reader does not support streaming".into()))
    }
}

/// A writer consumes point views and produces side effects (files, etc.).
pub trait Writer {
    fn name(&self) -> &str;
    fn write(&mut self, views: &[PointView]) -> Result<(), StageError>;
    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("metadata")
    }
    /// Whether this writer can consume points chunk by chunk.
    fn streamable(&self) -> bool {
        false
    }
    /// Write one chunk incrementally.
    fn stream_write(&mut self, _chunk: &PointView) -> Result<(), StageError> {
        Err(StageError("writer does not support streaming".into()))
    }
    /// Finalize after the last chunk.
    fn stream_finish(&mut self) -> Result<(), StageError> {
        Ok(())
    }
}
