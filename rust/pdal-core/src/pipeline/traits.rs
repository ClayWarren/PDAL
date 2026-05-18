use crate::metadata::MetadataNode;
use crate::point::PointView;
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

    fn read(&mut self) -> Result<Vec<PointView>, StageError>;
    fn write(&mut self, views: &[PointView]) -> Result<(), StageError>;
    fn process_one(&mut self, view: &mut PointView, idx: crate::point::PointId) -> bool;
    fn reset(&mut self);
    fn metadata(&self) -> MetadataNode;
    fn name(&self) -> &str;
    fn kind(&self) -> StageKind;
}

/// A reader produces point views from a source.
pub trait Reader {
    fn name(&self) -> &str;
    fn read(&mut self) -> Result<Vec<PointView>, StageError>;
    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("metadata")
    }
}

/// A writer consumes point views and produces side effects (files, etc.).
pub trait Writer {
    fn name(&self) -> &str;
    fn write(&mut self, views: &[PointView]) -> Result<(), StageError>;
    fn metadata(&self) -> MetadataNode {
        MetadataNode::new("metadata")
    }
}
