//! Pipeline management.
//!
//! A pipeline is a directed acyclic graph of stages (readers, filters, writers).
//! This is the Rust analog of PDAL's `PipelineManager`.

mod adapters;
mod traits;

#[cfg(test)]
mod tests;

pub use adapters::FilterWrapper;
pub use traits::{Reader, StageKind, StageWrapper, Writer};

use crate::metadata::MetadataNode;
use crate::options::Options;
use crate::point::{Bounds2D, Bounds3D, DimensionSummary, PointView};
use crate::stage::StageError;
use adapters::{ReaderAdapter, WriterAdapter};
use std::collections::{HashMap, HashSet};

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
    pub bounds_2d: Option<Bounds2D>,
    pub bounds_3d: Option<Bounds3D>,
    pub dimension_summaries: Vec<DimensionSummary>,
}

/// A pipeline of stages represented as a DAG.
pub struct Pipeline {
    nodes: Vec<StageNode>,
    tags: HashMap<String, usize>,
    last_writer_point_count: u64,
    last_writer_view_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WhereMergeMode {
    Auto,
    True,
    False,
}

pub fn generate_stage_tag(stage_name: &str, explicit_tag: &str, existing_tags: &[&str]) -> String {
    if !explicit_tag.is_empty() {
        return explicit_tag.to_string();
    }

    for index in 1.. {
        let tag = format!("{stage_name}{index}").replace('.', "_");
        if !existing_tags.contains(&tag.as_str()) {
            return tag;
        }
    }
    unreachable!()
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
            last_writer_point_count: 0,
            last_writer_view_count: 0,
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

    /// Replace a stage while preserving graph edges that referenced it.
    pub fn replace_stage(
        &mut self,
        idx: usize,
        name: &str,
        stage: Box<dyn StageWrapper>,
        options: Options,
    ) -> Result<(), StageError> {
        let old_node = self
            .nodes
            .get(idx)
            .ok_or_else(|| StageError(format!("stage index {idx} out of range")))?;
        let inputs = old_node.inputs.clone();
        let tag = old_node.tag.clone();
        let kind = old_node.kind;

        self.nodes[idx] = StageNode {
            name: name.to_string(),
            stage,
            inputs,
            options,
            tag,
            kind,
        };
        Ok(())
    }

    /// Return the number of direct input edges for a stage.
    pub fn input_count(&self, idx: usize) -> Result<usize, StageError> {
        self.nodes
            .get(idx)
            .map(|node| node.inputs.len())
            .ok_or_else(|| StageError(format!("stage index {idx} out of range")))
    }

    /// Return one direct input stage index for a stage.
    pub fn input(&self, idx: usize, input_idx: usize) -> Result<usize, StageError> {
        self.nodes
            .get(idx)
            .and_then(|node| node.inputs.get(input_idx).copied())
            .ok_or_else(|| {
                StageError(format!(
                    "stage input out of range: stage={idx}, input={input_idx}"
                ))
            })
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
        self.last_writer_point_count = 0;
        self.last_writer_view_count = 0;

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

                    let output_dims = node.stage.output_dimensions();
                    let prepared_inputs = prepare_filter_inputs(inputs_for_node, &output_dims);
                    let node_outputs = run_stage_with_where(node, prepared_inputs)?;
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

                    let writer_inputs = apply_writer_where(node, inputs_for_node)?;
                    self.last_writer_point_count +=
                        writer_inputs.iter().map(PointView::len).sum::<u64>();
                    self.last_writer_view_count += writer_inputs.len();
                    node.stage.write(&writer_inputs)?;
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
        let point_count: u64 =
            views.iter().map(|v| v.len()).sum::<u64>() + self.last_writer_point_count;
        Ok(ExecResult {
            point_count,
            view_count: views.len() + self.last_writer_view_count,
            bounds_2d: aggregate_bounds_2d(&views),
            bounds_3d: aggregate_bounds_3d(&views),
            dimension_summaries: aggregate_dimension_summaries(&views),
        })
    }

    /// Aggregate metadata from all stages.
    pub fn metadata(&self) -> MetadataNode {
        let mut root = MetadataNode::new("pipeline");
        for (i, node) in self.nodes.iter().enumerate() {
            let mut stage_meta = node.stage.metadata();
            if let Some(ref tag) = node.tag {
                stage_meta.set_name(tag.clone());
            } else {
                stage_meta.set_name(format!("stage_{}", i));
            }
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

    pub fn has_reader(&self) -> bool {
        self.nodes.iter().any(|node| node.kind == StageKind::Reader)
    }

    pub fn stage(&self, idx: usize) -> Option<&StageNode> {
        self.nodes.get(idx)
    }

    pub fn stage_mut(&mut self, idx: usize) -> Option<&mut StageNode> {
        self.nodes.get_mut(idx)
    }
}

fn run_stage_with_where(
    node: &mut StageNode,
    inputs: Vec<PointView>,
) -> Result<Vec<PointView>, StageError> {
    let where_expr = node.options.get_str("where", "");
    if where_expr.trim().is_empty() {
        return node.stage.run(&inputs);
    }
    let merge_mode = where_merge_mode(&node.options)?;
    let mut outputs = Vec::new();

    for input in inputs {
        let (keeps, skips) = split_where(&input, &where_expr)?;
        let keep_size = keeps.len();
        let mut view_outputs = if keep_size == 0 && node.kind == StageKind::Filter {
            Vec::new()
        } else {
            node.stage.run(&[keeps])?
        };
        merge_where_skips(&mut view_outputs, skips, keep_size, merge_mode);
        outputs.extend(view_outputs);
    }

    Ok(outputs)
}

fn apply_writer_where(
    node: &StageNode,
    inputs: Vec<PointView>,
) -> Result<Vec<PointView>, StageError> {
    let where_expr = node.options.get_str("where", "");
    if where_expr.trim().is_empty() {
        return Ok(inputs);
    }
    let mut outputs = Vec::new();
    for input in inputs {
        let (keeps, _) = split_where(&input, &where_expr)?;
        outputs.push(keeps);
    }
    Ok(outputs)
}

fn split_where(input: &PointView, where_expr: &str) -> Result<(PointView, PointView), StageError> {
    let mut expr = crate::expr::ConditionalExpression::parse(where_expr)
        .map_err(|err| StageError(format!("Invalid 'where': {err}")))?;
    expr.prepare(input.layout().as_ref())
        .map_err(|err| StageError(format!("Invalid 'where': {err}")))?;

    let mut keeps = input.make_new();
    let mut skips = input.make_new();
    for idx in 0..input.len() {
        if expr.eval(input, idx) {
            keeps.append_point(input, idx);
        } else {
            skips.append_point(input, idx);
        }
    }
    Ok((keeps, skips))
}

fn merge_where_skips(
    outputs: &mut Vec<PointView>,
    skips: PointView,
    keep_size: u64,
    merge_mode: WhereMergeMode,
) {
    if skips.is_empty() {
        return;
    }

    match merge_mode {
        WhereMergeMode::True => {
            if let Some(first) = outputs.first_mut() {
                append_view(first, &skips);
                return;
            }
        }
        WhereMergeMode::Auto => {
            if outputs.len() == 1 && outputs[0].len() == keep_size {
                append_view(&mut outputs[0], &skips);
                return;
            }
        }
        WhereMergeMode::False => {}
    }

    outputs.push(skips);
}

fn append_view(dst: &mut PointView, src: &PointView) {
    for idx in 0..src.len() {
        dst.append_point(src, idx);
    }
}

fn where_merge_mode(options: &Options) -> Result<WhereMergeMode, StageError> {
    match options
        .get_str("where_merge", "auto")
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "auto" => Ok(WhereMergeMode::Auto),
        "true" => Ok(WhereMergeMode::True),
        "false" => Ok(WhereMergeMode::False),
        value => Err(StageError(format!("Invalid 'where_merge': {value}"))),
    }
}

fn aggregate_bounds_2d(views: &[PointView]) -> Option<Bounds2D> {
    views
        .iter()
        .filter_map(PointView::calculate_bounds_2d)
        .fold(None, |acc, bounds| {
            Some(match acc {
                Some(existing) => Bounds2D {
                    minx: existing.minx.min(bounds.minx),
                    maxx: existing.maxx.max(bounds.maxx),
                    miny: existing.miny.min(bounds.miny),
                    maxy: existing.maxy.max(bounds.maxy),
                },
                None => bounds,
            })
        })
}

fn prepare_filter_inputs(
    inputs: Vec<PointView>,
    output_dims: &[(crate::point::DimId, crate::point::DimType)],
) -> Vec<PointView> {
    if output_dims.is_empty() {
        inputs
    } else {
        inputs
            .into_iter()
            .map(|view| view.with_dimensions(output_dims))
            .collect()
    }
}

fn aggregate_bounds_3d(views: &[PointView]) -> Option<Bounds3D> {
    views
        .iter()
        .filter_map(PointView::calculate_bounds_3d)
        .fold(None, |acc, bounds| {
            Some(match acc {
                Some(existing) => Bounds3D {
                    minx: existing.minx.min(bounds.minx),
                    maxx: existing.maxx.max(bounds.maxx),
                    miny: existing.miny.min(bounds.miny),
                    maxy: existing.maxy.max(bounds.maxy),
                    minz: existing.minz.min(bounds.minz),
                    maxz: existing.maxz.max(bounds.maxz),
                },
                None => bounds,
            })
        })
}

fn aggregate_dimension_summaries(views: &[PointView]) -> Vec<DimensionSummary> {
    let mut summaries = Vec::new();
    let mut by_name = HashMap::new();

    for view in views {
        for summary in view.summarize_dimensions() {
            if let Some(&idx) = by_name.get(&summary.name) {
                merge_dimension_summary(&mut summaries[idx], summary);
            } else {
                by_name.insert(summary.name.clone(), summaries.len());
                summaries.push(summary);
            }
        }
    }

    summaries
}

fn merge_dimension_summary(existing: &mut DimensionSummary, incoming: DimensionSummary) {
    let total_count = existing.count + incoming.count;
    if total_count == 0 {
        return;
    }

    existing.minimum = existing.minimum.min(incoming.minimum);
    existing.maximum = existing.maximum.max(incoming.maximum);
    existing.mean = ((existing.mean * existing.count as f64)
        + (incoming.mean * incoming.count as f64))
        / total_count as f64;
    existing.count = total_count;
}

#[cfg(test)]
mod tag_tests {
    use super::*;

    #[test]
    fn generates_unique_stage_tags() {
        assert_eq!(generate_stage_tag("readers.las", "", &[]), "readers_las1");
        assert_eq!(
            generate_stage_tag("readers.las", "", &["readers_las1"]),
            "readers_las2"
        );
        assert_eq!(
            generate_stage_tag("readers.las", "input", &["input"]),
            "input"
        );
    }
}
