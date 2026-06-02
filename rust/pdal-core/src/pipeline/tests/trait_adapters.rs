use super::*;

struct MinimalReader;

impl Reader for MinimalReader {
    fn name(&self) -> &str {
        "readers.minimal"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        Ok(Vec::new())
    }
}

struct MinimalWriter;

impl Writer for MinimalWriter {
    fn name(&self) -> &str {
        "writers.minimal"
    }

    fn write(&mut self, _views: &[PointView]) -> Result<(), StageError> {
        Ok(())
    }
}

#[test]
fn metadata_defaults_and_adapters_work() {
    let min_reader = MinimalReader;
    assert_eq!(min_reader.metadata().name(), "metadata");

    let min_writer = MinimalWriter;
    assert_eq!(min_writer.metadata().name(), "metadata");

    let mut reader_adapter = ReaderAdapter::new(Box::new(TestReader::new(5)));
    assert_eq!(reader_adapter.name(), "readers.test");
    assert_eq!(reader_adapter.kind(), StageKind::Reader);
    assert!(reader_adapter.output_dimensions().is_empty());
    assert_eq!(reader_adapter.metadata().name(), "readers.test");

    let layout = PointLayout::new();
    let mut view = PointView::new(Rc::new(layout));
    assert!(!reader_adapter.process_one(&mut view, 0));
    reader_adapter.reset();

    let views = reader_adapter.read().unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 5);

    let views_run = reader_adapter.run(&[]).unwrap();
    assert_eq!(views_run.len(), 1);
    assert_eq!(views_run[0].len(), 5);
    assert!(reader_adapter.write(&[]).is_err());

    let mut writer_adapter = WriterAdapter::new(Box::new(TestWriter::new()));
    assert_eq!(writer_adapter.name(), "writers.test");
    assert_eq!(writer_adapter.kind(), StageKind::Writer);
    assert!(writer_adapter.output_dimensions().is_empty());
    assert_eq!(writer_adapter.metadata().name(), "writers.test");
    assert!(!writer_adapter.process_one(&mut view, 0));
    writer_adapter.reset();

    writer_adapter.write(&views).unwrap();
    assert_eq!(
        writer_adapter
            .metadata()
            .find_child("point_count")
            .unwrap()
            .value()
            .unwrap()
            .as_u64(),
        5
    );

    let run_res = writer_adapter.run(&views).unwrap();
    assert!(run_res.is_empty());
    assert!(writer_adapter.read().is_err());

    let mut filter_wrapper = FilterWrapper::new(PassThroughFilter::new());
    assert_eq!(filter_wrapper.kind(), StageKind::Filter);
    assert!(filter_wrapper.read().is_err());
    assert!(filter_wrapper.write(&[]).is_err());
}
