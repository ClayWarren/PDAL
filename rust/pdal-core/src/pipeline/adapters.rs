use super::{Reader, StageKind, StageWrapper, Writer};
use crate::metadata::MetadataNode;
use crate::point::{DimId, DimType, PointId, PointView};
use crate::stage::{Filter, StageError, Streamable};

/// Adapter that wraps a `Reader` as a `StageWrapper`.
pub struct ReaderAdapter(pub Box<dyn Reader>);

impl ReaderAdapter {
    pub fn new(reader: Box<dyn Reader>) -> Self {
        Self(reader)
    }
}

impl StageWrapper for ReaderAdapter {
    fn run(&mut self, _inputs: &[PointView]) -> Result<Vec<PointView>, StageError> {
        self.0.read()
    }
    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        self.0.read()
    }
    fn write(&mut self, _views: &[PointView]) -> Result<(), StageError> {
        Err(StageError("cannot write to a reader".into()))
    }
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
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
        StageKind::Reader
    }
    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        Vec::new()
    }
    fn streamable(&self) -> bool {
        self.0.streamable()
    }
    fn stream_next(&mut self, capacity: usize) -> Result<Option<PointView>, StageError> {
        self.0.stream_next(capacity)
    }
}

/// Adapter that wraps a `Writer` as a `StageWrapper`.
pub struct WriterAdapter(pub Box<dyn Writer>);

impl WriterAdapter {
    pub fn new(writer: Box<dyn Writer>) -> Self {
        Self(writer)
    }
}

impl StageWrapper for WriterAdapter {
    fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError> {
        self.0.write(inputs).map(|_| Vec::new())
    }
    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        Err(StageError("cannot read from a writer".into()))
    }
    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        self.0.write(views)
    }
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
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
        StageKind::Writer
    }
    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        Vec::new()
    }
    fn streamable(&self) -> bool {
        self.0.streamable()
    }
    fn stream_write(&mut self, chunk: &PointView) -> Result<(), StageError> {
        self.0.stream_write(chunk)
    }
    fn stream_finish(&mut self) -> Result<(), StageError> {
        self.0.stream_finish()
    }
}

/// Wrapper that holds a filter and implements `StageWrapper`.
pub struct FilterWrapper<T: Filter + Streamable>(pub T);

impl<T: Filter + Streamable> FilterWrapper<T> {
    pub fn new(filter: T) -> Self {
        Self(filter)
    }
}

impl<T: Filter + Streamable> StageWrapper for FilterWrapper<T> {
    fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError> {
        self.0.run(inputs)
    }
    fn run_owned(&mut self, inputs: Vec<PointView>) -> Result<Vec<PointView>, StageError> {
        self.0.run_owned(inputs)
    }
    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        Err(StageError("cannot read from a filter".into()))
    }
    fn write(&mut self, _views: &[PointView]) -> Result<(), StageError> {
        Err(StageError("cannot write to a filter".into()))
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
    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        self.0.output_dimensions()
    }
    fn streamable(&self) -> bool {
        self.0.streamable()
    }
    fn stream_chunk(&mut self, chunk: &mut PointView) -> Result<(), StageError> {
        self.0.stream_chunk(chunk)
    }
}
