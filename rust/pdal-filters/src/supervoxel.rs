use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub struct SupervoxelFilter {
    knn: usize,
    resolution: f64,
}

impl SupervoxelFilter {
    pub fn new(knn: usize, resolution: f64) -> Self {
        Self { knn, resolution }
    }
}

impl Filter for SupervoxelFilter {
    fn name(&self) -> &str {
        "filters.supervoxel"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        if !has_normals(view) {
            return Err(StageError("No normals found.".to_string()));
        }

        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let n_points = output.len() as usize;
        if n_points == 0 {
            return Ok(vec![output]);
        }

        let ncluster = estimate_cluster_count(&output, self.resolution);
        let index = SpatialIndex3d::new(&output);
        let mut neighbors = (0..output.len())
            .map(|idx| {
                index
                    .knn(idx, self.knn)
                    .into_iter()
                    .map(|(id, _)| id)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let mut disjoint = DisjointSet::new(n_points);
        let mut roots = (0..output.len()).collect::<BTreeSet<_>>();
        let mut visited = vec![false; n_points];
        let mut counts = vec![1_u32; n_points];
        let mut lambda = initial_lambda(&output, &neighbors, self.resolution);

        while roots.len() > ncluster {
            let current_roots = roots.iter().copied().collect::<Vec<_>>();
            for root in current_roots {
                if !roots.contains(&root) || neighbors[root as usize].is_empty() {
                    continue;
                }

                let mut queue = Vec::with_capacity(n_points);
                visited[root as usize] = true;
                queue.push(root);
                let mut front = 1_usize;
                let mut back = 1_usize;

                for candidate in neighbors[root as usize].clone() {
                    let found = disjoint.find(candidate);
                    if !visited[found as usize] {
                        visited[found as usize] = true;
                        queue.push(found);
                        back += 1;
                    }
                }

                let mut retained = Vec::new();
                while front < back {
                    let other = queue[front];
                    front += 1;
                    if lambda
                        - counts[other as usize] as f64
                            * distance(&output, root, other, self.resolution)
                        > 0.0
                    {
                        disjoint.unite(root, other);
                        roots.remove(&other);
                        counts[root as usize] += counts[other as usize];
                        for candidate in neighbors[other as usize].clone() {
                            let found = disjoint.find(candidate);
                            if !visited[found as usize] {
                                visited[found as usize] = true;
                                queue.push(found);
                                back += 1;
                            }
                        }
                        neighbors[other as usize].clear();
                        if roots.len() == ncluster {
                            break;
                        }
                    } else {
                        retained.push(other);
                    }
                }
                neighbors[root as usize] = retained;

                for id in queue.into_iter().take(back) {
                    visited[id as usize] = false;
                }
                if roots.len() == ncluster {
                    break;
                }
            }

            if roots.len() == ncluster {
                break;
            }
            lambda *= 2.0;
            if lambda.is_infinite() {
                break;
            }
        }

        let mut labels = vec![0_u64; n_points];
        let mut distances = vec![0.0; n_points];
        for idx in 0..output.len() {
            let root = disjoint.find(idx);
            labels[idx as usize] = root;
            distances[idx as usize] = if idx == root {
                0.0
            } else {
                distance(&output, idx, root, self.resolution)
            };
            visited[idx as usize] = false;
        }

        for idx in 0..output.len() {
            neighbors[idx as usize] = index
                .knn(idx, self.knn)
                .into_iter()
                .map(|(id, _)| id)
                .collect();
        }

        let mut edge_queue = VecDeque::new();
        for point in 0..output.len() {
            for neighbor in &neighbors[point as usize] {
                if labels[point as usize] == labels[*neighbor as usize] {
                    continue;
                }
                if !visited[point as usize] {
                    edge_queue.push_back(point);
                    visited[point as usize] = true;
                }
                if !visited[*neighbor as usize] {
                    edge_queue.push_back(*neighbor);
                    visited[*neighbor as usize] = true;
                }
            }
        }

        while let Some(point) = edge_queue.pop_front() {
            visited[point as usize] = false;
            let mut changed = false;
            for neighbor in &neighbors[point as usize] {
                let point_root = labels[point as usize];
                let neighbor_root = labels[*neighbor as usize];
                if point_root == neighbor_root {
                    continue;
                }
                let candidate_distance = distance(&output, point, neighbor_root, self.resolution);
                if candidate_distance < distances[point as usize] {
                    labels[point as usize] = neighbor_root;
                    distances[point as usize] = candidate_distance;
                    changed = true;
                }
            }

            if changed {
                for neighbor in &neighbors[point as usize] {
                    if labels[point as usize] != labels[*neighbor as usize]
                        && !visited[*neighbor as usize]
                    {
                        edge_queue.push_back(*neighbor);
                        visited[*neighbor as usize] = true;
                    }
                }
            }
        }

        let label_map = roots
            .iter()
            .copied()
            .enumerate()
            .map(|(idx, root)| (root, idx as f64))
            .collect::<BTreeMap<_, _>>();
        for idx in 0..output.len() {
            output.set_f64(
                idx,
                &DimId::ClusterID,
                *label_map.get(&labels[idx as usize]).unwrap_or(&0.0),
            );
        }

        Ok(vec![output])
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::ClusterID, DimType::F64)]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for SupervoxelFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

fn has_normals(view: &PointView) -> bool {
    view.layout().dim(&DimId::NormalX).is_some()
        && view.layout().dim(&DimId::NormalY).is_some()
        && view.layout().dim(&DimId::NormalZ).is_some()
}

fn estimate_cluster_count(view: &PointView, resolution: f64) -> usize {
    let mut populated = BTreeSet::new();
    let mut origin = None;
    for idx in 0..view.len() {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        let z = view.get_f64(idx, &DimId::Z);
        let (ox, oy, oz) = *origin.get_or_insert((
            x - resolution / 2.0,
            y - resolution / 2.0,
            z - resolution / 2.0,
        ));
        populated.insert((
            ((x - ox) / resolution).floor() as i32,
            ((y - oy) / resolution).floor() as i32,
            ((z - oz) / resolution).floor() as i32,
        ));
    }
    populated.len()
}

fn initial_lambda(view: &PointView, neighbors: &[Vec<PointId>], resolution: f64) -> f64 {
    let mut distances = vec![f64::MAX; neighbors.len()];
    for (idx, ids) in neighbors.iter().enumerate() {
        let point = idx as PointId;
        for neighbor in ids {
            if point == *neighbor {
                continue;
            }
            distances[idx] = distances[idx].min(distance(view, point, *neighbor, resolution));
        }
    }
    let mid = distances.len() / 2;
    distances.select_nth_unstable_by(mid, |a, b| a.total_cmp(b));
    distances[mid].max(f64::EPSILON)
}

fn distance(view: &PointView, a: PointId, b: PointId, resolution: f64) -> f64 {
    let ax = view.get_f64(a, &DimId::X);
    let ay = view.get_f64(a, &DimId::Y);
    let az = view.get_f64(a, &DimId::Z);
    let bx = view.get_f64(b, &DimId::X);
    let by = view.get_f64(b, &DimId::Y);
    let bz = view.get_f64(b, &DimId::Z);
    let anx = view.get_f64(a, &DimId::NormalX);
    let any = view.get_f64(a, &DimId::NormalY);
    let anz = view.get_f64(a, &DimId::NormalZ);
    let bnx = view.get_f64(b, &DimId::NormalX);
    let bny = view.get_f64(b, &DimId::NormalY);
    let bnz = view.get_f64(b, &DimId::NormalZ);

    let normal = 1.0 - (anx * bnx + any * bny + anz * bnz).abs();
    let spatial = ((ax - bx).powi(2) + (ay - by).powi(2) + (az - bz).powi(2)).sqrt();
    normal + 0.4 * spatial / resolution
}

struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
        }
    }

    fn find(&mut self, id: PointId) -> PointId {
        let idx = id as usize;
        if self.parent[idx] != idx {
            self.parent[idx] = self.find(self.parent[idx] as PointId) as usize;
        }
        self.parent[idx] as PointId
    }

    fn unite(&mut self, a: PointId, b: PointId) {
        let root_a = self.find(a) as usize;
        let root_b = self.find(b) as usize;
        if root_a == root_b {
            return;
        }
        self.parent[root_b] = root_a;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{PointLayout, PointView};
    use std::collections::BTreeSet;
    use std::rc::Rc;

    fn fixture_view() -> PointView {
        let mut layout = PointLayout::new();
        for dim in [
            DimId::X,
            DimId::Y,
            DimId::Z,
            DimId::NormalX,
            DimId::NormalY,
            DimId::NormalZ,
            DimId::ClusterID,
        ] {
            layout.register(dim, DimType::F64);
        }
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z, nx, ny, nz) in [
            (1.0, 1.0, 0.0, 0.0, 1.0, 0.0),
            (2.0, 2.0, 0.0, 0.0, 1.0, 0.0),
            (4.0, 2.0, 0.0, 1.0, 1.0, 0.0),
            (5.0, 1.0, 0.0, 1.0, 1.0, 0.0),
            (1.0, 5.0, 0.0, 0.0, -1.0, 0.0),
            (2.0, 4.0, 0.0, 0.0, -1.0, 0.0),
            (4.0, 4.0, 0.0, 0.0, 1.0, 0.0),
            (5.0, 4.0, 0.0, 0.0, 1.0, 0.0),
        ] {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, x);
            view.set_f64(id, &DimId::Y, y);
            view.set_f64(id, &DimId::Z, z);
            view.set_f64(id, &DimId::NormalX, nx);
            view.set_f64(id, &DimId::NormalY, ny);
            view.set_f64(id, &DimId::NormalZ, nz);
        }
        view
    }

    #[test]
    fn matches_cpp_supervoxel_fixture_shape() {
        let view = fixture_view();
        let mut filter = SupervoxelFilter::new(3, 3.0);
        let out = filter.run_one(&view).unwrap().pop().unwrap();

        let clusters = (0..out.len())
            .map(|idx| out.get_f64(idx, &DimId::ClusterID) as u64)
            .collect::<BTreeSet<_>>();
        assert_eq!(clusters.len(), 4);
        for idx in (0..out.len()).step_by(2) {
            assert_eq!(
                out.get_f64(idx, &DimId::ClusterID),
                out.get_f64(idx + 1, &DimId::ClusterID)
            );
        }
    }

    #[test]
    fn rejects_missing_normals() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let view = PointView::new(Rc::new(layout));
        let mut filter = SupervoxelFilter::new(3, 3.0);

        match filter.run_one(&view) {
            Ok(_) => panic!("expected missing-normal error"),
            Err(err) => assert_eq!(err, StageError("No normals found.".to_string())),
        }
    }
}
