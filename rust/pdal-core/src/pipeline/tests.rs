use super::*;
use crate::metadata::MetadataValue;
use crate::options::Options;
use crate::point::{DimId, DimType, PointId, PointLayout, PointView};
use crate::stage::{Filter, StageError, Streamable};
use std::rc::Rc;

struct PassThroughFilter {
    run_count: usize,
}

impl PassThroughFilter {
    fn new() -> Self {
        Self { run_count: 0 }
    }
}

impl Filter for PassThroughFilter {
    fn name(&self) -> &str {
        "filters.passthrough"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.run_count += 1;
        let mut out = input.make_new();
        for i in 0..input.len() {
            out.append_point(input, i);
        }
        Ok(vec![out])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for PassThroughFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        true
    }
}

struct DuplicateFilter {
    run_count: usize,
}

impl DuplicateFilter {
    fn new() -> Self {
        Self { run_count: 0 }
    }
}

impl Filter for DuplicateFilter {
    fn name(&self) -> &str {
        "filters.duplicate"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.run_count += 1;
        let mut out1 = input.make_new();
        let mut out2 = input.make_new();
        for i in 0..input.len() {
            out1.append_point(input, i);
            out2.append_point(input, i);
        }
        Ok(vec![out1, out2])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for DuplicateFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        true
    }
}

struct AppendOneFilter;

impl Filter for AppendOneFilter {
    fn name(&self) -> &str {
        "filters.appendone"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();
        for i in 0..input.len() {
            out.append_point(input, i);
        }
        let idx = out.add_point();
        out.set_f64(idx, &DimId::X, 999.0);
        Ok(vec![out])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for AppendOneFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        true
    }
}

struct ErrorFilter;

impl Filter for ErrorFilter {
    fn name(&self) -> &str {
        "filters.error"
    }

    fn run_one(&mut self, _input: &PointView) -> Result<Vec<PointView>, StageError> {
        Err(StageError("filter failed".to_string()))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for ErrorFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        true
    }
}

/// Borrow path is identity; owned path reverses the view in place. A reversed
/// result proves the executor took the `run_owned` path.
struct ReverseInPlaceFilter;

impl Filter for ReverseInPlaceFilter {
    fn name(&self) -> &str {
        "filters.reverse_inplace"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();
        for i in 0..input.len() {
            out.append_point(input, i);
        }
        Ok(vec![out])
    }

    fn run_owned(&mut self, inputs: Vec<PointView>) -> Result<Vec<PointView>, StageError> {
        let mut outputs = Vec::with_capacity(inputs.len());
        for mut view in inputs {
            let order: Vec<PointId> = (0..view.len()).rev().collect();
            view.reorder(&order);
            outputs.push(view);
        }
        Ok(outputs)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for ReverseInPlaceFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        true
    }
}

/// A test reader that generates N points with Z = 1..N.
struct TestReader {
    count: u64,
}

impl TestReader {
    fn new(count: u64) -> Self {
        Self { count }
    }
}

impl Reader for TestReader {
    fn name(&self) -> &str {
        "readers.test"
    }

    fn read(&mut self) -> Result<Vec<PointView>, StageError> {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);
        for i in 0..self.count {
            view.add_point();
            view.set_f64(i, &DimId::Z, (i + 1) as f64);
        }
        Ok(vec![view])
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("readers.test");
        node.add_value("count", MetadataValue::U64(self.count));
        node
    }
}

/// A test writer that counts how many views and points it receives.
struct TestWriter {
    view_count: usize,
    point_count: u64,
}

impl TestWriter {
    fn new() -> Self {
        Self {
            view_count: 0,
            point_count: 0,
        }
    }
}

impl Writer for TestWriter {
    fn name(&self) -> &str {
        "writers.test"
    }

    fn write(&mut self, views: &[PointView]) -> Result<(), StageError> {
        self.view_count += views.len();
        for v in views {
            self.point_count += v.len();
        }
        Ok(())
    }

    fn metadata(&self) -> MetadataNode {
        let mut node = MetadataNode::new("writers.test");
        node.add_value("view_count", MetadataValue::U64(self.view_count as u64));
        node.add_value("point_count", MetadataValue::U64(self.point_count));
        node
    }
}

fn make_test_view(count: u64) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let layout = Rc::new(layout);
    let mut view = PointView::new(layout);
    for _ in 0..count {
        view.add_point();
    }
    view
}

/// A streamable test reader producing `count` points with X = 0..count, either
/// in one pass (`read`) or in chunks (`stream_next`), so streaming and standard
/// execution can be compared for parity.
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

/// A streamable filter that keeps only points with even X. `run_one` (standard)
/// and `stream_chunk` (streaming) must agree.
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
        // PassThroughFilter is not streamable -> forces the fallback.
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
fn test_streaming_execution_matches_standard_output() {
    // 25 points across chunks of 10 -> exercises multi-chunk streaming + the
    // even-X compaction, and must match the materializing path exactly.
    let (mut std_pipe, std_sink) = build_collecting_chain(25, true);
    std_pipe.execute(Vec::new()).unwrap();
    let standard = std_sink.borrow().clone();

    let (mut stream_pipe, stream_sink) = build_collecting_chain(25, true);
    let streamed = stream_pipe.execute_streaming().unwrap();
    assert_eq!(streamed, Some(13)); // even X in 0..25: 0,2,...,24 -> 13 points
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
fn test_execute_streaming_returns_none_for_non_streamable_pipeline() {
    // A non-streamable filter in the chain makes the pipeline ineligible; the
    // caller must fall back to execute().
    let (mut pipe, _sink) = build_collecting_chain(10, false);
    assert_eq!(pipe.execute_streaming().unwrap(), None);
}

fn where_options(expr: &str, merge_mode: Option<&str>) -> Options {
    let mut options = Options::new();
    options.add("where", expr);
    if let Some(mode) = merge_mode {
        options.add("where_merge", mode);
    }
    options
}

fn make_xyz_view(points: &[(f64, f64, f64)]) -> PointView {
    let mut layout = PointLayout::new();
    layout.register(DimId::X, DimType::F64);
    layout.register(DimId::Y, DimType::F64);
    layout.register(DimId::Z, DimType::F64);
    let layout = Rc::new(layout);
    let mut view = PointView::new(layout);

    for &(x, y, z) in points {
        let point = view.add_point();
        view.set_f64(point, &DimId::X, x);
        view.set_f64(point, &DimId::Y, y);
        view.set_f64(point, &DimId::Z, z);
    }

    view
}

#[test]
fn test_empty_pipeline_returns_input() {
    let mut pipeline = Pipeline::new();
    let views = vec![make_test_view(5)];
    let result = pipeline.execute(views).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 5);
}

#[test]
fn test_single_stage_pipeline() {
    let mut pipeline = Pipeline::new();
    let filter = Box::new(FilterWrapper::new(PassThroughFilter::new()));
    pipeline.add_stage("filters.passthrough", filter, Options::new());

    let views = vec![make_test_view(10)];
    let result = pipeline.execute(views).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 10);
}

#[test]
fn test_where_auto_merges_skips_when_filter_preserves_kept_view_size() {
    let mut pipeline = Pipeline::new();
    pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        where_options("X < 3", None),
    );

    let result = pipeline
        .execute(vec![make_xyz_view(&[
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (3.0, 0.0, 0.0),
            (4.0, 0.0, 0.0),
        ])])
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 5);
}

#[test]
fn test_where_auto_keeps_skips_separate_when_filter_changes_kept_view_size() {
    let mut pipeline = Pipeline::new();
    pipeline.add_stage(
        "filters.appendone",
        Box::new(FilterWrapper::new(AppendOneFilter)),
        where_options("X < 3", None),
    );

    let result = pipeline
        .execute(vec![make_xyz_view(&[
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (3.0, 0.0, 0.0),
            (4.0, 0.0, 0.0),
        ])])
        .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result.iter().map(PointView::len).sum::<u64>(), 6);
}

#[test]
fn test_executor_uses_run_owned_without_where() {
    let mut pipeline = Pipeline::new();
    pipeline.add_stage(
        "filters.reverse_inplace",
        Box::new(FilterWrapper::new(ReverseInPlaceFilter)),
        Options::new(),
    );

    let result = pipeline
        .execute(vec![make_xyz_view(&[
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
        ])])
        .unwrap();

    assert_eq!(result.len(), 1);
    let xs: Vec<f64> = (0..result[0].len())
        .map(|i| result[0].get_f64(i, &DimId::X))
        .collect();
    // Reversed (2,1,0) only if the executor used run_owned; the borrow path is
    // identity (0,1,2).
    assert_eq!(xs, vec![2.0, 1.0, 0.0]);
}

#[test]
fn test_where_merge_modes_match_stage_runner_shape() {
    let view = make_xyz_view(&[
        (0.0, 0.0, 0.0),
        (1.0, 0.0, 0.0),
        (2.0, 0.0, 0.0),
        (3.0, 0.0, 0.0),
        (4.0, 0.0, 0.0),
    ]);

    let mut true_pipeline = Pipeline::new();
    true_pipeline.add_stage(
        "filters.duplicate",
        Box::new(FilterWrapper::new(DuplicateFilter::new())),
        where_options("X < 3", Some("true")),
    );
    let true_result = true_pipeline.execute(vec![view.clone()]).unwrap();
    assert_eq!(true_result.len(), 2);
    assert_eq!(true_result.iter().map(PointView::len).sum::<u64>(), 8);

    let mut false_pipeline = Pipeline::new();
    false_pipeline.add_stage(
        "filters.duplicate",
        Box::new(FilterWrapper::new(DuplicateFilter::new())),
        where_options("X < 3", Some("false")),
    );
    let false_result = false_pipeline.execute(vec![view]).unwrap();
    assert_eq!(false_result.len(), 3);
    assert_eq!(false_result.iter().map(PointView::len).sum::<u64>(), 8);
}

#[test]
fn test_where_filters_writer_inputs() {
    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader("readers.test", Box::new(TestReader::new(5)), Options::new());
    let writer = pipeline.add_writer(
        "writers.test",
        Box::new(TestWriter::new()),
        where_options("Z > 2", None),
    );
    pipeline.add_dependency(writer, reader).unwrap();

    let result = pipeline.execute(Vec::new()).unwrap();
    assert!(result.is_empty());
    let metadata = pipeline.metadata();
    let writer_meta = metadata.find_child("stage_1").unwrap();
    assert_eq!(
        writer_meta
            .find_child("point_count")
            .unwrap()
            .value()
            .unwrap()
            .as_u64(),
        3
    );
}

#[test]
fn test_linear_pipeline() {
    let mut pipeline = Pipeline::new();
    let s0 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    let s1 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    let s2 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    pipeline.add_dependency(s1, s0).unwrap();
    pipeline.add_dependency(s2, s1).unwrap();

    let views = vec![make_test_view(7)];
    let result = pipeline.execute(views).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 7);
}

#[test]
fn test_diamond_pipeline() {
    let mut pipeline = Pipeline::new();
    let s0 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    let s1 = pipeline.add_stage(
        "filters.duplicate",
        Box::new(FilterWrapper::new(DuplicateFilter::new())),
        Options::new(),
    );
    let s2 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    pipeline.add_dependency(s1, s0).unwrap();
    pipeline.add_dependency(s2, s1).unwrap();

    let views = vec![make_test_view(3)];
    let result = pipeline.execute(views).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].len(), 3);
    assert_eq!(result[1].len(), 3);
}

#[test]
fn test_multiple_roots() {
    let mut pipeline = Pipeline::new();
    let s0 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    let s1 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    let s2 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    pipeline.add_dependency(s2, s0).unwrap();
    pipeline.add_dependency(s2, s1).unwrap();

    let views = vec![make_test_view(4)];
    let result = pipeline.execute(views).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].len(), 4);
    assert_eq!(result[1].len(), 4);
}

#[test]
fn test_roots_and_leaves() {
    let mut pipeline = Pipeline::new();
    let s0 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    let s1 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    let s2 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    pipeline.add_dependency(s1, s0).unwrap();
    pipeline.add_dependency(s2, s1).unwrap();

    assert_eq!(pipeline.roots(), vec![0]);
    assert_eq!(pipeline.leaves(), vec![2]);
}

#[test]
fn test_tagging() {
    let mut pipeline = Pipeline::new();
    let s0 = pipeline
        .add_stage_tagged(
            "filters.passthrough",
            Box::new(FilterWrapper::new(PassThroughFilter::new())),
            Options::new(),
            "reader",
        )
        .unwrap();
    let s1 = pipeline
        .add_stage_tagged(
            "filters.passthrough",
            Box::new(FilterWrapper::new(PassThroughFilter::new())),
            Options::new(),
            "filter",
        )
        .unwrap();

    pipeline.add_dependency(s1, s0).unwrap();

    assert_eq!(pipeline.find_by_tag("reader"), Some(s0));
    assert_eq!(pipeline.find_by_tag("filter"), Some(s1));
    assert_eq!(pipeline.find_by_tag("nonexistent"), None);
}

#[test]
fn test_duplicate_tag_error() {
    let mut pipeline = Pipeline::new();
    pipeline
        .add_stage_tagged(
            "filters.passthrough",
            Box::new(FilterWrapper::new(PassThroughFilter::new())),
            Options::new(),
            "dup",
        )
        .unwrap();
    let result = pipeline.add_stage_tagged(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
        "dup",
    );
    assert!(result.is_err());
}

#[test]
fn test_set_tag_replaces_existing_tag_mapping() {
    let mut pipeline = Pipeline::new();
    let stage = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    pipeline.set_tag(stage, "before").unwrap();
    assert_eq!(pipeline.find_by_tag("before"), Some(stage));

    pipeline.set_tag(stage, "after").unwrap();
    assert_eq!(pipeline.find_by_tag("before"), None);
    assert_eq!(pipeline.find_by_tag("after"), Some(stage));
}

#[test]
fn test_set_tag_rejects_duplicate_existing_tag() {
    let mut pipeline = Pipeline::new();
    let first = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    let second = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    pipeline.set_tag(first, "used").unwrap();
    let result = pipeline.set_tag(second, "used");

    assert!(result.is_err());
    assert_eq!(pipeline.find_by_tag("used"), Some(first));
}

#[test]
fn test_add_dependency_rejects_out_of_range_indices() {
    let mut pipeline = Pipeline::new();
    let stage = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    assert!(pipeline.add_dependency(stage, stage + 1).is_err());
    assert!(pipeline.add_dependency(stage + 1, stage).is_err());
}

#[test]
fn test_cycle_detection() {
    let mut pipeline = Pipeline::new();
    let s0 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    let s1 = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    pipeline.add_dependency(s0, s1).unwrap();
    pipeline.add_dependency(s1, s0).unwrap();

    let result = pipeline.execute(vec![make_test_view(1)]);
    assert!(result.is_err());
}

#[test]
fn test_filter_error_stops_pipeline_execution() {
    let mut pipeline = Pipeline::new();
    pipeline.add_stage(
        "filters.error",
        Box::new(FilterWrapper::new(ErrorFilter)),
        Options::new(),
    );

    match pipeline.execute(vec![make_test_view(1)]) {
        Ok(_) => panic!("expected filter failure"),
        Err(err) => assert_eq!(err.0, "filter failed"),
    }
}

#[test]
fn test_execute_result() {
    let mut pipeline = Pipeline::new();
    pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    let views = vec![make_test_view(42)];
    let result = pipeline.execute_with_result(views).unwrap();
    assert_eq!(result.point_count, 42);
    assert_eq!(result.view_count, 1);
    assert_eq!(
        result.bounds_2d,
        Some(crate::point::Bounds2D {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
        })
    );
    assert_eq!(
        result.bounds_3d,
        Some(crate::point::Bounds3D {
            minx: 0.0,
            maxx: 0.0,
            miny: 0.0,
            maxy: 0.0,
            minz: 0.0,
            maxz: 0.0,
        })
    );
    assert_eq!(result.dimension_summaries.len(), 3);
    assert_eq!(result.dimension_summaries[0].name, "X");
    assert_eq!(result.dimension_summaries[0].count, 42);
    assert_eq!(result.dimension_summaries[0].minimum, 0.0);
    assert_eq!(result.dimension_summaries[0].maximum, 0.0);
    assert_eq!(result.dimension_summaries[0].mean, 0.0);
}

#[test]
fn test_execute_result_aggregates_bounds_and_dimension_summaries_across_views() {
    let mut pipeline = Pipeline::new();
    pipeline.add_stage(
        "filters.duplicate",
        Box::new(FilterWrapper::new(DuplicateFilter::new())),
        Options::new(),
    );

    let views = vec![make_xyz_view(&[(-10.0, 5.0, 100.0), (20.0, -15.0, -50.0)])];
    let result = pipeline.execute_with_result(views).unwrap();

    assert_eq!(result.point_count, 4);
    assert_eq!(result.view_count, 2);
    assert_eq!(
        result.bounds_2d,
        Some(crate::point::Bounds2D {
            minx: -10.0,
            maxx: 20.0,
            miny: -15.0,
            maxy: 5.0,
        })
    );
    assert_eq!(
        result.bounds_3d,
        Some(crate::point::Bounds3D {
            minx: -10.0,
            maxx: 20.0,
            miny: -15.0,
            maxy: 5.0,
            minz: -50.0,
            maxz: 100.0,
        })
    );
    assert_eq!(result.dimension_summaries.len(), 3);
    assert_eq!(result.dimension_summaries[0].name, "X");
    assert_eq!(result.dimension_summaries[0].count, 4);
    assert_eq!(result.dimension_summaries[0].minimum, -10.0);
    assert_eq!(result.dimension_summaries[0].maximum, 20.0);
    assert_eq!(result.dimension_summaries[0].mean, 5.0);
    assert_eq!(result.dimension_summaries[1].name, "Y");
    assert_eq!(result.dimension_summaries[1].minimum, -15.0);
    assert_eq!(result.dimension_summaries[1].maximum, 5.0);
    assert_eq!(result.dimension_summaries[1].mean, -5.0);
    assert_eq!(result.dimension_summaries[2].name, "Z");
    assert_eq!(result.dimension_summaries[2].minimum, -50.0);
    assert_eq!(result.dimension_summaries[2].maximum, 100.0);
    assert_eq!(result.dimension_summaries[2].mean, 25.0);
}

#[test]
fn test_writer_leaf_does_not_hide_other_leaf_outputs() {
    let mut pipeline = Pipeline::new();
    let reader = pipeline.add_reader("readers.test", Box::new(TestReader::new(6)), Options::new());
    let filter = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    let writer = pipeline.add_writer("writers.test", Box::new(TestWriter::new()), Options::new());

    pipeline.add_dependency(filter, reader).unwrap();
    pipeline.add_dependency(writer, reader).unwrap();

    let result = pipeline.execute(Vec::new()).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 6);
}

#[test]
fn test_metadata_aggregation() {
    let mut pipeline = Pipeline::new();
    pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );
    pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    let meta = pipeline.metadata();
    assert_eq!(meta.name(), "pipeline");
    assert_eq!(meta.child_count(), 2);
}

#[test]
fn test_reset_before_execute() {
    struct CountingFilter {
        process_count: usize,
    }

    impl CountingFilter {
        fn new() -> Self {
            Self { process_count: 0 }
        }
    }

    impl Filter for CountingFilter {
        fn name(&self) -> &str {
            "filters.counting"
        }

        fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
            let mut out = input.make_new();
            for i in 0..input.len() {
                out.append_point(input, i);
            }
            Ok(vec![out])
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    impl Streamable for CountingFilter {
        fn reset(&mut self) {
            self.process_count = 0;
        }

        fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
            self.process_count += 1;
            true
        }
    }

    let mut pipeline = Pipeline::new();
    pipeline.add_stage(
        "filters.counting",
        Box::new(FilterWrapper::new(CountingFilter::new())),
        Options::new(),
    );

    pipeline.execute(vec![make_test_view(5)]).unwrap();
    pipeline.execute(vec![make_test_view(3)]).unwrap();
}

#[test]
fn test_reader_filter_writer_pipeline() {
    let mut pipeline = Pipeline::new();

    let reader = pipeline.add_reader(
        "readers.test",
        Box::new(TestReader::new(20)),
        Options::new(),
    );

    let filter = pipeline.add_stage(
        "filters.passthrough",
        Box::new(FilterWrapper::new(PassThroughFilter::new())),
        Options::new(),
    );

    let writer = pipeline.add_writer("writers.test", Box::new(TestWriter::new()), Options::new());

    pipeline.add_dependency(filter, reader).unwrap();
    pipeline.add_dependency(writer, filter).unwrap();

    assert_eq!(pipeline.stage(reader).unwrap().kind, StageKind::Reader);
    assert_eq!(pipeline.stage(filter).unwrap().kind, StageKind::Filter);
    assert_eq!(pipeline.stage(writer).unwrap().kind, StageKind::Writer);

    let result = pipeline.execute(Vec::new()).unwrap();
    // Writer consumes all output, so leaf (writer) returns empty
    assert!(result.is_empty());
}

#[test]
fn test_execute_result_counts_writer_inputs() {
    let mut pipeline = Pipeline::new();

    let reader = pipeline.add_reader(
        "readers.test",
        Box::new(TestReader::new(20)),
        Options::new(),
    );
    let writer = pipeline.add_writer("writers.test", Box::new(TestWriter::new()), Options::new());
    pipeline.add_dependency(writer, reader).unwrap();

    let result = pipeline.execute_with_result(Vec::new()).unwrap();
    assert_eq!(result.point_count, 20);
    assert_eq!(result.view_count, 1);
}

#[test]
fn test_reader_produces_correct_data() {
    let mut pipeline = Pipeline::new();
    pipeline.add_reader(
        "readers.test",
        Box::new(TestReader::new(10)),
        Options::new(),
    );

    let result = pipeline.execute(Vec::new()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 10);
    for i in 0..10u64 {
        let z = result[0].get_f64(i, &DimId::Z);
        assert_eq!(z, (i + 1) as f64);
    }
}

#[test]
fn test_writer_counts_points() {
    let mut pipeline = Pipeline::new();

    let reader = pipeline.add_reader(
        "readers.test",
        Box::new(TestReader::new(42)),
        Options::new(),
    );

    let writer_idx =
        pipeline.add_writer("writers.test", Box::new(TestWriter::new()), Options::new());

    pipeline.add_dependency(writer_idx, reader).unwrap();

    pipeline.execute(Vec::new()).unwrap();

    let writer_node = pipeline.stage(writer_idx).unwrap();
    let meta = writer_node.stage.metadata();
    let pc = meta.find_child("point_count").unwrap();
    assert_eq!(pc.value().unwrap().as_u64(), 42);
}

#[test]
fn test_adapters_and_traits_coverage() {
    // 1. Test traits metadata defaults
    struct MinimalReader;
    impl Reader for MinimalReader {
        fn name(&self) -> &str {
            "readers.minimal"
        }
        fn read(&mut self) -> Result<Vec<PointView>, StageError> {
            Ok(Vec::new())
        }
    }
    let min_reader = MinimalReader;
    assert_eq!(min_reader.metadata().name(), "metadata");

    struct MinimalWriter;
    impl Writer for MinimalWriter {
        fn name(&self) -> &str {
            "writers.minimal"
        }
        fn write(&mut self, _views: &[PointView]) -> Result<(), StageError> {
            Ok(())
        }
    }
    let min_writer = MinimalWriter;
    assert_eq!(min_writer.metadata().name(), "metadata");

    // 2. Test ReaderAdapter
    let mut reader_adapter = ReaderAdapter::new(Box::new(TestReader::new(5)));
    assert_eq!(reader_adapter.name(), "readers.test");
    assert_eq!(reader_adapter.kind(), StageKind::Reader);
    assert!(reader_adapter.output_dimensions().is_empty());
    assert_eq!(reader_adapter.metadata().name(), "readers.test");

    // Test process_one and reset
    let layout = PointLayout::new();
    let mut view = PointView::new(Rc::new(layout));
    assert!(!reader_adapter.process_one(&mut view, 0));
    reader_adapter.reset();

    // run/read should read points
    let views = reader_adapter.read().unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].len(), 5);

    let views_run = reader_adapter.run(&[]).unwrap();
    assert_eq!(views_run.len(), 1);
    assert_eq!(views_run[0].len(), 5);

    // write should fail
    assert!(reader_adapter.write(&[]).is_err());

    // 3. Test WriterAdapter
    let mut writer_adapter = WriterAdapter::new(Box::new(TestWriter::new()));
    assert_eq!(writer_adapter.name(), "writers.test");
    assert_eq!(writer_adapter.kind(), StageKind::Writer);
    assert!(writer_adapter.output_dimensions().is_empty());
    assert_eq!(writer_adapter.metadata().name(), "writers.test");
    assert!(!writer_adapter.process_one(&mut view, 0));
    writer_adapter.reset();

    // write and run
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

    // read should fail
    assert!(writer_adapter.read().is_err());

    // 4. Test FilterWrapper fail paths
    let mut filter_wrapper = FilterWrapper::new(PassThroughFilter::new());
    assert_eq!(filter_wrapper.kind(), StageKind::Filter);
    assert!(filter_wrapper.read().is_err());
    assert!(filter_wrapper.write(&[]).is_err());
}
