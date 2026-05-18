use super::{Reader, StageKind, StageWrapper, Writer};
use crate::metadata::MetadataNode;
use crate::point::{PointId, PointView};
use crate::stage::{Filter, StageError, Streamable};

/// Adapter that wraps a `Reader` as a `StageWrapper`.
pub(super) struct ReaderAdapter {
    reader: Box<dyn Reader>,
}

impl ReaderAdapter {
    pub(super) fn new(reader: Box<dyn Reader>) -> Self {
        Self { reader }
    }
}

impl StageWrapper for ReaderAdapter {
    fn run(&mut self, _input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.reader.read()
    }
    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        self.reader.read()
    }
    fn write(&mut self, _views: &[PointView]) -> Result<(), StageError> {
        Ok(())
    }
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
    fn reset(&mut self) {}
    fn metadata(&self) -> MetadataNode {
        self.reader.metadata()
    }
    fn name(&self) -> &str {
        self.reader.name()
    }
    fn kind(&self) -> StageKind {
        StageKind::Reader
    }
}

/// Adapter that wraps a `Writer` as a `StageWrapper`.
pub(super) struct WriterAdapter {
    writer: Box<dyn Writer>,
}

impl WriterAdapter {
    pub(super) fn new(writer: Box<dyn Writer>) -> Self {
        Self { writer }
    }
}

impl StageWrapper for WriterAdapter {
    fn run(&mut self, _input: &PointView) -> Result<Vec<PointView>, StageError> {
        Ok(Vec::new())
    }
    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        Ok(Vec::new())
    }
    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        self.writer.write(views)
    }
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        true
    }
    fn reset(&mut self) {}
    fn metadata(&self) -> MetadataNode {
        self.writer.metadata()
    }
    fn name(&self) -> &str {
        self.writer.name()
    }
    fn kind(&self) -> StageKind {
        StageKind::Writer
    }
}

/// A filter wrapper that also implements `StageWrapper` for pipeline use.
pub struct FilterWrapper<F: Filter + Streamable>(F);

impl<F: Filter + Streamable> FilterWrapper<F> {
    pub fn new(filter: F) -> Self {
        Self(filter)
    }
}

impl<F: Filter + Streamable> StageWrapper for FilterWrapper<F> {
    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.0.run(input)
    }
    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        Ok(Vec::new())
    }
    fn write(&mut self, _views: &[PointView]) -> Result<(), StageError> {
        Ok(())
    }
    fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {
        self.0.process_one(view, idx)
    }
    fn reset(&mut self) {
        self.0.reset();
    }
    fn metadata(&self) -> MetadataNode {
        self.0.metadata()
    }
    fn name(&self) -> &str {
        self.0.name()
    }
    fn kind(&self) -> StageKind {
        StageKind::Filter
    }
}
