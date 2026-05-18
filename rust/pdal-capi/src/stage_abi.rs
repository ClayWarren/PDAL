use pdal_core::metadata::MetadataNode;
use pdal_core::pipeline::StageWrapper as PipelineStageWrapper;
use pdal_core::point::PointView;
use pdal_core::stage::{Filter, StageError, Streamable};

// ---------------------------------------------------------------------------
// Stage ABI
// ---------------------------------------------------------------------------

/// Opaque wrapper around a Rust filter that implements both `Filter` and
/// `Streamable`.
pub struct StageWrapper {
    pub(crate) filter: Box<dyn FilterWrapper>,
}

pub(crate) trait FilterWrapper {
    fn process_one(&mut self, view: &mut PointView, idx: u64) -> bool;
    fn reset(&mut self);
    fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError>;
    fn metadata(&self) -> MetadataNode;
    fn as_any(&self) -> &dyn std::any::Any;
    fn name(&self) -> &str;
}

impl<T: Filter + Streamable> FilterWrapper for T {
    fn process_one(&mut self, view: &mut PointView, idx: u64) -> bool {
        Streamable::process_one(self, view, idx)
    }
    fn reset(&mut self) {
        Streamable::reset(self)
    }
    fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError> {
        Filter::run(self, inputs)
    }
    fn metadata(&self) -> MetadataNode {
        Filter::metadata(self)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        Filter::as_any(self)
    }
    fn name(&self) -> &str {
        Filter::name(self)
    }
}

impl PipelineStageWrapper for StageWrapper {
    fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError> {
        self.filter.run(inputs)
    }
    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        Ok(Vec::new())
    }
    fn write(&mut self, _views: &[PointView]) -> Result<(), StageError> {
        Ok(())
    }
    fn process_one(&mut self, view: &mut PointView, idx: u64) -> bool {
        self.filter.process_one(view, idx)
    }
    fn reset(&mut self) {
        self.filter.reset()
    }
    fn metadata(&self) -> MetadataNode {
        self.filter.metadata()
    }
    fn name(&self) -> &str {
        self.filter.name()
    }
    fn kind(&self) -> pdal_core::pipeline::StageKind {
        pdal_core::pipeline::StageKind::Filter
    }
}
