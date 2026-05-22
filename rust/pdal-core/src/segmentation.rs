//! Point segmentation helpers, ported from `filters/private/Segmentation.cpp`.
//!
//! Mirrors `pdal::Segmentation::extractClusters` (Euclidean region growing) and
//! `pdal::Segmentation::segmentReturns` (return-number classification).

/// Extract clusters of points by region growing within a distance tolerance.
///
/// Coordinates are interleaved `[x, y, z, x, y, z, ...]`. `is_3d` selects a 3D
/// distance; otherwise only the XY plane is used. Clusters whose size falls
/// outside `[min_points, max_points]` are dropped.
pub fn extract_clusters(
    xyz: &[f64],
    count: usize,
    min_points: u64,
    max_points: u64,
    tolerance: f64,
    is_3d: bool,
) -> Vec<Vec<usize>> {
    let tol_sq = tolerance * tolerance;
    let within = |a: usize, b: usize| -> bool {
        let dx = xyz[3 * a] - xyz[3 * b];
        let dy = xyz[3 * a + 1] - xyz[3 * b + 1];
        let mut dist = dx * dx + dy * dy;
        if is_3d {
            let dz = xyz[3 * a + 2] - xyz[3 * b + 2];
            dist += dz * dz;
        }
        dist <= tol_sq
    };

    let mut processed = vec![false; count];
    let mut clusters: Vec<Vec<usize>> = Vec::new();

    for i in 0..count {
        // Points can only belong to a single cluster.
        if processed[i] {
            continue;
        }

        let mut seed_queue = vec![i];
        processed[i] = true;

        // Region-grow: the queue can grow as neighbors are appended.
        let mut sq_idx = 0;
        while sq_idx < seed_queue.len() {
            let j = seed_queue[sq_idx];
            let neighbors: Vec<usize> = (0..count).filter(|&k| within(j, k)).collect();

            // The only neighbor is the query point itself.
            if neighbors.len() == 1 {
                sq_idx += 1;
                continue;
            }

            for k in neighbors {
                if !processed[k] {
                    seed_queue.push(k);
                    processed[k] = true;
                }
            }
            sq_idx += 1;
        }

        let len = seed_queue.len() as u64;
        if len >= min_points && len <= max_points {
            clusters.push(seed_queue);
        }
    }

    clusters
}

/// Classify points into the "first" output of `segmentReturns`.
///
/// For each point, returns `true` when its return type (derived from return
/// number and number of returns) is among the requested return classes.
pub fn segment_returns(
    return_number: &[u8],
    number_of_returns: &[u8],
    want_first: bool,
    want_intermediate: bool,
    want_last: bool,
    want_only: bool,
) -> Vec<bool> {
    return_number
        .iter()
        .zip(number_of_returns.iter())
        .map(|(&rn, &nr)| {
            (rn == 1 && nr > 1 && want_first)
                || (rn > 1 && rn < nr && want_intermediate)
                || (rn == nr && nr > 1 && want_last)
                || (nr == 1 && want_only)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clusters_split_by_distance() {
        // Two points near the origin, one far away.
        let xyz = [0.0, 0.0, 0.0, 10.0, 10.0, 10.0, 0.5, 0.5, 0.5];
        let clusters = extract_clusters(&xyz, 3, 1, 10, 1.0, true);
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].len(), 2);
        assert_eq!(clusters[1].len(), 1);
    }

    #[test]
    fn cluster_size_bounds_are_applied() {
        let xyz = [0.0, 0.0, 0.0, 10.0, 10.0, 10.0, 0.5, 0.5, 0.5];
        // min_points = 2 drops the lone far point.
        let clusters = extract_clusters(&xyz, 3, 2, 10, 1.0, true);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].len(), 2);
        // max_points = 1 drops the two-point cluster.
        let clusters = extract_clusters(&xyz, 3, 1, 1, 1.0, true);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].len(), 1);
    }

    #[test]
    fn two_dimensional_clustering_ignores_z() {
        // Points 0 and 2 share the same XY but differ in Z.
        let xyz = [0.0, 0.0, 0.0, 10.0, 10.0, 10.0, 0.0, 0.0, 10.0];
        let clusters_2d = extract_clusters(&xyz, 3, 1, 10, 1.0, false);
        assert_eq!(clusters_2d.len(), 2);
        assert_eq!(clusters_2d[0].len(), 2);
        // In 3D the Z separation yields three clusters.
        let clusters_3d = extract_clusters(&xyz, 3, 1, 10, 1.0, true);
        assert_eq!(clusters_3d.len(), 3);
    }

    #[test]
    fn segment_returns_classifies_return_types() {
        // (rn, nr): only, first, last, intermediate.
        let rn = [1u8, 1, 3, 2];
        let nr = [1u8, 3, 3, 3];

        let only = segment_returns(&rn, &nr, false, false, false, true);
        assert_eq!(only, [true, false, false, false]);

        let last_only = segment_returns(&rn, &nr, false, false, true, true);
        assert_eq!(last_only, [true, false, true, false]);

        let first = segment_returns(&rn, &nr, true, false, false, false);
        assert_eq!(first, [false, true, false, false]);

        let intermediate = segment_returns(&rn, &nr, false, true, false, false);
        assert_eq!(intermediate, [false, false, false, true]);
    }
}
