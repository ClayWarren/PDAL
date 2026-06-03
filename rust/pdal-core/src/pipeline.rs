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
    pub output_views: Vec<PointView>,
}

/// A pipeline of stages represented as a DAG.
pub struct Pipeline {
    nodes: Vec<StageNode>,
    tags: HashMap<String, usize>,
    last_writer_point_count: u64,
    last_writer_view_count: usize,
    last_output_views: Vec<PointView>,
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
            last_output_views: Vec::new(),
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
        self.last_output_views.clear();

        let mut outputs: HashMap<usize, Vec<PointView>> = HashMap::new();

        // How many downstream nodes still need each producer's output. The last
        // consumer moves the views out of `outputs` instead of cloning, so a
        // linear pipeline holds one copy of a view at a time rather than keeping
        // every producer's output alive until the run ends.
        let mut consumers_remaining: HashMap<usize, usize> = HashMap::new();
        for node in &self.nodes {
            for &input_idx in &node.inputs {
                *consumers_remaining.entry(input_idx).or_insert(0) += 1;
            }
        }

        for &node_idx in &order {
            let node = &mut self.nodes[node_idx];

            match node.kind {
                StageKind::Reader => {
                    let views = node.stage.read()?;
                    outputs.insert(node_idx, views);
                }
                StageKind::Filter => {
                    let inputs_for_node = take_node_inputs(
                        &node.inputs,
                        &mut outputs,
                        &mut consumers_remaining,
                        &input_views,
                    );

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
                    let inputs_for_node = take_node_inputs(
                        &node.inputs,
                        &mut outputs,
                        &mut consumers_remaining,
                        &input_views,
                    );

                    let writer_inputs = apply_writer_where(node, inputs_for_node)?;
                    self.last_writer_point_count +=
                        writer_inputs.iter().map(PointView::len).sum::<u64>();
                    self.last_writer_view_count += writer_inputs.len();
                    self.last_output_views.extend(writer_inputs.iter().cloned());
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
        let output_views = if views.is_empty() {
            self.last_output_views.clone()
        } else {
            views.clone()
        };
        let point_count: u64 =
            views.iter().map(|v| v.len()).sum::<u64>() + self.last_writer_point_count;
        Ok(ExecResult {
            point_count,
            view_count: views.len() + self.last_writer_view_count,
            bounds_2d: aggregate_bounds_2d(&output_views),
            bounds_3d: aggregate_bounds_3d(&output_views),
            dimension_summaries: aggregate_dimension_summaries(&output_views),
            output_views,
        })
    }

    /// If this pipeline is a single linear reader -> filters... -> writer chain
    /// where every stage is streamable and no stage has a `where` clause, return
    /// `(reader_idx, filter_idxs_in_order, writer_idx)`. Otherwise `None`, and
    /// the caller should use the materializing [`execute`](Self::execute).
    fn streaming_chain(&self) -> Option<(usize, Vec<usize>, usize)> {
        let n = self.nodes.len();
        if n < 2 {
            return None;
        }
        let order = self.topological_order().ok()?;
        if order.len() != n {
            return None;
        }
        // Number of downstream consumers of each node; a linear chain has one
        // for every node except the terminal writer.
        let mut consumers = vec![0usize; n];
        for node in &self.nodes {
            for &input in &node.inputs {
                if input < n {
                    consumers[input] += 1;
                }
            }
        }
        for (pos, &idx) in order.iter().enumerate() {
            let node = &self.nodes[idx];
            if !node.stage.streamable() {
                return None;
            }
            if !node.options.get_str("where", "").trim().is_empty() {
                return None;
            }
            let is_last = pos == order.len() - 1;
            if pos == 0 {
                if node.kind != StageKind::Reader || !node.inputs.is_empty() {
                    return None;
                }
            } else if node.inputs.len() != 1 || node.inputs[0] != order[pos - 1] {
                // Each non-reader must take exactly its predecessor as input.
                return None;
            }
            if is_last {
                if node.kind != StageKind::Writer || consumers[idx] != 0 {
                    return None;
                }
            } else {
                if consumers[idx] != 1 {
                    return None; // fan-out is not a linear stream
                }
                if pos > 0 && node.kind != StageKind::Filter {
                    return None;
                }
            }
        }
        Some((
            order[0],
            order[1..order.len() - 1].to_vec(),
            order[order.len() - 1],
        ))
    }

    /// Return whether this pipeline is eligible for chunked streaming
    /// execution.
    pub fn streamable(&self) -> bool {
        self.streaming_chain().is_some()
    }

    /// Execute the pipeline in chunked streaming mode when it is a fully
    /// streamable linear chain, keeping peak memory bounded by the chunk size
    /// instead of materializing every point. Returns `Ok(Some(point_count))`
    /// when it streamed, or `Ok(None)` when the pipeline is not streaming-
    /// eligible (the caller should fall back to [`execute`](Self::execute)).
    pub fn execute_streaming(&mut self) -> Result<Option<u64>, StageError> {
        const STREAM_CHUNK_CAPACITY: usize = 10_000;

        let Some((reader, filters, writer)) = self.streaming_chain() else {
            return Ok(None);
        };

        for node in &mut self.nodes {
            node.stage.reset();
        }
        self.last_writer_point_count = 0;
        self.last_writer_view_count = 0;
        self.last_output_views.clear();

        let mut total_points = 0u64;
        while let Some(mut chunk) = self.nodes[reader]
            .stage
            .stream_next(STREAM_CHUNK_CAPACITY)?
        {
            for &filter in &filters {
                self.nodes[filter].stage.stream_chunk(&mut chunk)?;
            }
            total_points += chunk.len();
            self.nodes[writer].stage.stream_write(&chunk)?;
        }
        self.nodes[writer].stage.stream_finish()?;

        self.last_writer_point_count = total_points;
        self.last_writer_view_count = 1;
        Ok(Some(total_points))
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

    pub fn roots_are_readers(&self) -> bool {
        let roots = self.roots();
        !roots.is_empty()
            && roots
                .iter()
                .all(|&idx| self.nodes[idx].kind == StageKind::Reader)
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
        return node.stage.run_owned(inputs);
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

/// Gather a node's input views. A producer's output is moved into its final
/// consumer (cloned only for earlier consumers in a multi-consumer/diamond
/// DAG), and removed from `outputs` once fully consumed, so peak memory tracks
/// the live working set rather than every stage's output for the whole run.
/// Leaf outputs are never gathered as input, so they survive for the caller.
fn take_node_inputs(
    node_inputs: &[usize],
    outputs: &mut HashMap<usize, Vec<PointView>>,
    consumers_remaining: &mut HashMap<usize, usize>,
    external_inputs: &[PointView],
) -> Vec<PointView> {
    if node_inputs.is_empty() {
        return external_inputs.to_vec();
    }
    let mut merged = Vec::new();
    for &input_idx in node_inputs {
        let is_last_consumer = match consumers_remaining.get_mut(&input_idx) {
            Some(count) => {
                *count = count.saturating_sub(1);
                *count == 0
            }
            None => true,
        };
        if is_last_consumer {
            if let Some(views) = outputs.remove(&input_idx) {
                merged.extend(views);
            }
        } else if let Some(views) = outputs.get(&input_idx) {
            merged.extend(views.iter().cloned());
        }
    }
    merged
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
