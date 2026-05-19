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
use crate::point::{Bounds2D, Bounds3D, PointView};
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

                    let node_outputs = node.stage.run(&inputs_for_node)?;
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
            bounds_2d: aggregate_bounds_2d(&views),
            bounds_3d: aggregate_bounds_3d(&views),
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
