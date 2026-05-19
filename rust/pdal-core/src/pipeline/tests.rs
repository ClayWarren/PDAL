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
}

#[test]
fn test_execute_result_aggregates_bounds_across_views() {
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
