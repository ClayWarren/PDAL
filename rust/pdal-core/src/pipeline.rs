//! Pipeline management.
//!
//! A pipeline is a directed acyclic graph of stages (readers, filters, writers).
//! This is the Rust analog of PDAL's `PipelineManager`.

use crate::metadata::MetadataNode;
use crate::options::Options;
use crate::point::PointView;
use crate::stage::{Filter, StageError, Streamable};
use std::collections::{HashMap, HashSet};

/// The kind of stage in a pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageKind {
    Reader,
    Filter,
    Writer,
}

/// A node in the pipeline graph.
pub struct StageNode {
    pub name: String,
    pub stage: Box<dyn StageWrapper>,
    pub inputs: Vec<usize>,
    pub options: Options,
    pub tag: Option<String>,
    pub kind: StageKind,
}

/// Result of pipeline execution.
pub struct ExecResult {
    pub point_count: u64,
    pub view_count: usize,
}

/// Unified stage wrapper that can be a reader, filter, or writer.
pub trait StageWrapper {
    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError>;
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

/// A pipeline of stages represented as a DAG.
pub struct Pipeline {
    nodes: Vec<StageNode>,
    tags: HashMap<String, usize>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            tags: HashMap::new(),
        }
    }

    /// Add a filter stage and return its index.
    pub fn add_stage(
        &mut self,
        name: &str,
        stage: Box<dyn StageWrapper>,
        options: Options,
    ) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(StageNode {
            name: name.to_string(),
            stage,
            inputs: Vec::new(),
            options,
            tag: None,
            kind: StageKind::Filter,
        });
        idx
    }

    /// Add a reader stage and return its index.
    pub fn add_reader(&mut self, name: &str, reader: Box<dyn Reader>, options: Options) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(StageNode {
            name: name.to_string(),
            stage: Box::new(ReaderAdapter::new(reader)),
            inputs: Vec::new(),
            options,
            tag: None,
            kind: StageKind::Reader,
        });
        idx
    }

    /// Add a writer stage and return its index.
    pub fn add_writer(&mut self, name: &str, writer: Box<dyn Writer>, options: Options) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(StageNode {
            name: name.to_string(),
            stage: Box::new(WriterAdapter::new(writer)),
            inputs: Vec::new(),
            options,
            tag: None,
            kind: StageKind::Writer,
        });
        idx
    }

    /// Add a stage with a tag for later reference.
    pub fn add_stage_tagged(
        &mut self,
        name: &str,
        stage: Box<dyn StageWrapper>,
        options: Options,
        tag: &str,
    ) -> Result<usize, StageError> {
        if self.tags.contains_key(tag) {
            return Err(StageError(format!("duplicate pipeline tag: {tag}")));
        }
        let idx = self.add_stage(name, stage, options);
        self.tags.insert(tag.to_string(), idx);
        Ok(idx)
    }

    /// Set the tag for an existing stage index.
    pub fn set_tag(&mut self, idx: usize, tag: &str) -> Result<(), StageError> {
        let node = self
            .nodes
            .get_mut(idx)
            .ok_or_else(|| StageError(format!("stage index {idx} out of range")))?;
        if let Some(existing) = &node.tag {
            self.tags.remove(existing);
        }
        if self.tags.contains_key(tag) {
            return Err(StageError(format!("duplicate pipeline tag: {tag}")));
        }
        node.tag = Some(tag.to_string());
        self.tags.insert(tag.to_string(), idx);
        Ok(())
    }

    /// Declare that `target` depends on `input` (input flows into target).
    pub fn add_dependency(&mut self, target: usize, input: usize) -> Result<(), StageError> {
        if target >= self.nodes.len() || input >= self.nodes.len() {
            return Err(StageError(format!(
                "dependency indices out of range: target={target}, input={input}, nodes={}",
                self.nodes.len()
            )));
        }
        if let Some(node) = self.nodes.get_mut(target) {
            node.inputs.push(input);
        }
        Ok(())
    }

    /// Find a stage index by its tag.
    pub fn find_by_tag(&self, tag: &str) -> Option<usize> {
        self.tags.get(tag).copied()
    }

    /// Return indices of root nodes (no inputs).
    pub fn roots(&self) -> Vec<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.inputs.is_empty())
            .map(|(i, _)| i)
            .collect()
    }

    /// Return indices of leaf nodes (no other node depends on them).
    pub fn leaves(&self) -> Vec<usize> {
        let mut has_dependents = HashSet::new();
        for node in &self.nodes {
            for &input_idx in &node.inputs {
                has_dependents.insert(input_idx);
            }
        }
        self.nodes
            .iter()
            .enumerate()
            .filter(|(i, _)| !has_dependents.contains(i))
            .map(|(i, _)| i)
            .collect()
    }

    /// Topological sort of the DAG. Returns execution order (roots first).
    /// Errors on cycles.
    fn topological_order(&self) -> Result<Vec<usize>, StageError> {
        let n = self.nodes.len();
        if n == 0 {
            return Ok(Vec::new());
        }

        let mut in_degree = vec![0usize; n];
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, node) in self.nodes.iter().enumerate() {
            for &input_idx in &node.inputs {
                adj[input_idx].push(i);
                in_degree[i] += 1;
            }
        }

        let mut queue: Vec<usize> = in_degree
            .iter()
            .enumerate()
            .filter(|(_, &d)| d == 0)
            .map(|(i, _)| i)
            .collect();
        let mut order = Vec::with_capacity(n);

        while let Some(node_idx) = queue.pop() {
            order.push(node_idx);
            for &child in &adj[node_idx] {
                in_degree[child] -= 1;
                if in_degree[child] == 0 {
                    queue.push(child);
                }
            }
        }

        if order.len() != n {
            return Err(StageError(
                "pipeline contains a cycle; cannot determine execution order".into(),
            ));
        }

        Ok(order)
    }

    /// Execute the pipeline with initial input views.
    ///
    /// The pipeline is executed in topological order:
    /// - Reader stages (no inputs) produce views via `read()`
    /// - Filter stages transform views via `run()`
    /// - Writer stages consume views via `write()`
    ///
    /// Root nodes that are filters receive the initial `input_views`.
    /// Root nodes that are readers ignore `input_views` and produce their own.
    ///
    /// Returns the combined outputs of all leaf nodes (excluding writers).
    pub fn execute(&mut self, input_views: Vec<PointView>) -> Result<Vec<PointView>, StageError> {
        if self.nodes.is_empty() {
            return Ok(input_views);
        }

        let order = self.topological_order()?;

        for node in &mut self.nodes {
            node.stage.reset();
        }

        let mut outputs: HashMap<usize, Vec<PointView>> = HashMap::new();

        for &node_idx in &order {
            let node = &mut self.nodes[node_idx];

            match node.kind {
                StageKind::Reader => {
                    let views = node.stage.read()?;
                    outputs.insert(node_idx, views);
                }
                StageKind::Filter => {
                    let inputs_for_node: Vec<PointView> = if node.inputs.is_empty() {
                        input_views.clone()
                    } else {
                        let mut merged = Vec::new();
                        for &input_idx in &node.inputs {
                            if let Some(views) = outputs.get(&input_idx) {
                                merged.extend(views.iter().cloned());
                            }
                        }
                        merged
                    };

                    if inputs_for_node.is_empty() {
                        outputs.insert(node_idx, Vec::new());
                        continue;
                    }

                    let mut node_outputs = Vec::new();
                    for view in &inputs_for_node {
                        let out = node.stage.run(view)?;
                        node_outputs.extend(out);
                    }
                    outputs.insert(node_idx, node_outputs);
                }
                StageKind::Writer => {
                    let inputs_for_node: Vec<PointView> = if node.inputs.is_empty() {
                        input_views.clone()
                    } else {
                        let mut merged = Vec::new();
                        for &input_idx in &node.inputs {
                            if let Some(views) = outputs.get(&input_idx) {
                                merged.extend(views.iter().cloned());
                            }
                        }
                        merged
                    };

                    node.stage.write(&inputs_for_node)?;
                    outputs.insert(node_idx, Vec::new());
                }
            }
        }

        let leaf_indices = self.leaves();
        let mut result = Vec::new();
        for &leaf_idx in &leaf_indices {
            if let Some(views) = outputs.remove(&leaf_idx) {
                result.extend(views);
            }
        }

        Ok(result)
    }

    /// Execute and return a summary result.
    pub fn execute_with_result(
        &mut self,
        input_views: Vec<PointView>,
    ) -> Result<ExecResult, StageError> {
        let views = self.execute(input_views)?;
        let point_count: u64 = views.iter().map(|v| v.len()).sum();
        Ok(ExecResult {
            point_count,
            view_count: views.len(),
        })
    }

    /// Aggregate metadata from all stages.
    pub fn metadata(&self) -> MetadataNode {
        let mut root = MetadataNode::new("pipeline");
        for node in &self.nodes {
            let stage_meta = node.stage.metadata();
            root.add_child(stage_meta);
        }
        root
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn stage(&self, idx: usize) -> Option<&StageNode> {
        self.nodes.get(idx)
    }

    pub fn stage_mut(&mut self, idx: usize) -> Option<&mut StageNode> {
        self.nodes.get_mut(idx)
    }
}

/// Adapter that wraps a `Reader` as a `StageWrapper`.
struct ReaderAdapter {
    reader: Box<dyn Reader>,
}

impl ReaderAdapter {
    fn new(reader: Box<dyn Reader>) -> Self {
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
    fn process_one(&mut self, _view: &mut PointView, _idx: crate::point::PointId) -> bool {
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
struct WriterAdapter {
    writer: Box<dyn Writer>,
}

impl WriterAdapter {
    fn new(writer: Box<dyn Writer>) -> Self {
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
    fn process_one(&mut self, _view: &mut PointView, _idx: crate::point::PointId) -> bool {
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
    fn process_one(&mut self, view: &mut PointView, idx: crate::point::PointId) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::Options;
    use crate::point::{DimId, DimType, PointLayout, PointView};
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

        fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
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
        fn process_one(&mut self, _view: &mut PointView, _idx: crate::point::PointId) -> bool {
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

        fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
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
        fn process_one(&mut self, _view: &mut PointView, _idx: crate::point::PointId) -> bool {
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
            node.add_value("count", crate::metadata::MetadataValue::U64(self.count));
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
            node.add_value(
                "view_count",
                crate::metadata::MetadataValue::U64(self.view_count as u64),
            );
            node.add_value(
                "point_count",
                crate::metadata::MetadataValue::U64(self.point_count),
            );
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

            fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
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

            fn process_one(&mut self, _view: &mut PointView, _idx: crate::point::PointId) -> bool {
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

        let writer =
            pipeline.add_writer("writers.test", Box::new(TestWriter::new()), Options::new());

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
}
