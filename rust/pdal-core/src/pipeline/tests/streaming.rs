use super::*;

/// A test reader producing `count` points with X = 0..count, either in one pass
/// or chunks, so streaming and standard execution can be compared for parity.
struct StreamingTestReader {
    count: u64,
    cursor: u64,
}

impl StreamingTestReader {
    fn new(count: u64) -> Self {
        Self { count, cursor: 0 }
    }

    fn build(start: u64, end: u64) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for i in start..end {
            let row = view.add_point();
            view.set_f64(row, &DimId::X, i as f64);
        }
        view
    }
}

impl Reader for StreamingTestReader {
    fn name(&self) -> &str {
        "readers.streamtest"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        Ok(vec![Self::build(0, self.count)])
    }

    fn reset(&mut self) {
        self.cursor = 0;
    }

    fn streamable(&self) -> bool {
        true
    }

    fn stream_next(&mut self, capacity: usize) -> Result<Option<PointView>, StageError> {
        if self.cursor >= self.count {
            return Ok(None);
        }
        let start = self.cursor;
        let end = (start + capacity.max(1) as u64).min(self.count);
        self.cursor = end;
        Ok(Some(Self::build(start, end)))
    }
}

/// A streamable filter that keeps only points with even X.
struct KeepEvenFilter;

impl KeepEvenFilter {
    fn keep(view: &PointView, idx: u64) -> bool {
        (view.get_f64(idx, &DimId::X) as i64) % 2 == 0
    }
}

impl Filter for KeepEvenFilter {
    fn name(&self) -> &str {
        "filters.keepeven"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();
        for i in 0..input.len() {
            if Self::keep(input, i) {
                out.append_point(input, i);
            }
        }
        Ok(vec![out])
    }

    fn streamable(&self) -> bool {
        true
    }

    fn stream_chunk(&mut self, chunk: &mut PointView) -> Result<(), StageError> {
        let n = chunk.len();
        let mut write = 0u64;
        for read in 0..n {
            if Self::keep(chunk, read) {
                if write != read {
                    chunk.copy_point_within(read, write);
                }
                write += 1;
            }
        }
        chunk.truncate(write);
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for KeepEvenFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        true
    }
}

/// A streamable writer that records every X it receives into a shared buffer.
struct CollectingWriter {
    sink: std::rc::Rc<std::cell::RefCell<Vec<f64>>>,
}

impl CollectingWriter {
    fn new(sink: std::rc::Rc<std::cell::RefCell<Vec<f64>>>) -> Self {
        Self { sink }
    }
}

impl Writer for CollectingWriter {
    fn name(&self) -> &str {
        "writers.collect"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        for v in views {
            for i in 0..v.len() {
                self.sink.borrow_mut().push(v.get_f64(i, &DimId::X));
            }
        }
        Ok(())
    }

    fn streamable(&self) -> bool {
        true
    }

    fn stream_write(&mut self, chunk: &PointView) -> Result<(), StageError> {
        for i in 0..chunk.len() {
            self.sink.borrow_mut().push(chunk.get_f64(i, &DimId::X));
        }
        Ok(())
    }
}

fn build_collecting_chain(
    count: u64,
    streamable_filter: bool,
) -> (Pipeline, std::rc::Rc<std::cell::RefCell<Vec<f64>>>) {
    let sink = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut pipeline = Pipeline::new();
    let r = pipeline.add_reader(
        "readers.streamtest",
        Box::new(StreamingTestReader::new(count)),
        Options::new(),
    );
    let f = if streamable_filter {
        pipeline.add_stage(
            "filters.keepeven",
            Box::new(FilterWrapper::new(KeepEvenFilter)),
            Options::new(),
        )
    } else {
        pipeline.add_stage(
            "filters.passthrough",
            Box::new(FilterWrapper::new(PassThroughFilter::new())),
            Options::new(),
        )
    };
    let w = pipeline.add_writer(
        "writers.collect",
        Box::new(CollectingWriter::new(sink.clone())),
        Options::new(),
    );
    pipeline.add_dependency(f, r).unwrap();
    pipeline.add_dependency(w, f).unwrap();
    (pipeline, sink)
}

#[test]
fn execution_matches_standard_output() {
    let (mut std_pipe, std_sink) = build_collecting_chain(25, true);
    std_pipe.execute(Vec::new()).unwrap();
    let standard = std_sink.borrow().clone();

    let (mut stream_pipe, stream_sink) = build_collecting_chain(25, true);
    let streamed = stream_pipe.execute_streaming().unwrap();
    assert_eq!(streamed, Some(13));
    assert_eq!(*stream_sink.borrow(), standard);
    assert_eq!(
        *stream_sink.borrow(),
        (0..25)
            .filter(|x| x % 2 == 0)
            .map(|x| x as f64)
            .collect::<Vec<_>>()
    );
}

#[test]
fn returns_none_for_non_streamable_pipeline() {
    let (mut pipe, _sink) = build_collecting_chain(10, false);
    assert_eq!(pipe.execute_streaming().unwrap(), None);
}
