#[derive(Clone, Copy, Debug, Default)]
pub struct M3c2Stats {
    pub distance: f64,
    pub uncertainty: f64,
    pub significant: bool,
    pub std_dev1: f64,
    pub std_dev2: f64,
    pub n1: u64,
    pub n2: u64,
}

pub struct M3c2Config {
    pub skip_first1: bool,
    pub skip_first2: bool,
    pub center: [f64; 3],
    pub normal: [f64; 3],
    pub cyl_radius2: f64,
    pub cyl_half_len: f64,
    pub min_points: usize,
    pub reg_error: f64,
}

pub fn compute_stats(pts1: &[[f64; 3]], pts2: &[[f64; 3]], cfg: M3c2Config) -> Option<M3c2Stats> {
    let dists1 = filter_points(
        pts1,
        cfg.skip_first1,
        cfg.center,
        cfg.normal,
        cfg.cyl_radius2,
        cfg.cyl_half_len,
    );
    let dists2 = filter_points(
        pts2,
        cfg.skip_first2,
        cfg.center,
        cfg.normal,
        cfg.cyl_radius2,
        cfg.cyl_half_len,
    );
    if dists1.len() < cfg.min_points || dists2.len() < cfg.min_points {
        return None;
    }

    let (mean1, var1) = mean_variance(&dists1);
    let (mean2, var2) = mean_variance(&dists2);
    let lod_var = var1 / dists1.len() as f64 + var2 / dists2.len() as f64;
    let lod = 1.96 * (lod_var.sqrt() + cfg.reg_error);
    let distance = mean2 - mean1;

    Some(M3c2Stats {
        distance,
        uncertainty: lod,
        significant: distance.abs() > lod,
        std_dev1: var1.sqrt(),
        std_dev2: var2.sqrt(),
        n1: dists1.len() as u64,
        n2: dists2.len() as u64,
    })
}

fn filter_points(
    pts: &[[f64; 3]],
    skip_first: bool,
    center: [f64; 3],
    normal: [f64; 3],
    cyl_radius2: f64,
    cyl_half_len: f64,
) -> Vec<f64> {
    let start = usize::from(skip_first && !pts.is_empty());
    let mut dists = Vec::with_capacity(pts.len().saturating_sub(start));
    for point in &pts[start..] {
        if let Some(dist) = point_passes(*point, center, normal, cyl_radius2, cyl_half_len) {
            dists.push(dist);
        }
    }
    dists
}

fn point_passes(
    point: [f64; 3],
    center: [f64; 3],
    normal: [f64; 3],
    cyl_radius2: f64,
    cyl_half_len: f64,
) -> Option<f64> {
    let delta = [
        point[0] - center[0],
        point[1] - center[1],
        point[2] - center[2],
    ];
    let along = dot(delta, normal);
    let projection = [
        center[0] + along * normal[0],
        center[1] + along * normal[1],
        center[2] + along * normal[2],
    ];
    if squared_distance(point, projection) > cyl_radius2 {
        return None;
    }
    if along.abs() > cyl_half_len {
        return None;
    }
    Some(along)
}

fn mean_variance(values: &[f64]) -> (f64, f64) {
    let mut sum = 0.0;
    let mut sum2 = 0.0;
    for value in values {
        sum += *value;
        sum2 += value * value;
    }
    let mean = sum / values.len() as f64;
    (mean, sum2 / values.len() as f64 - mean * mean)
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn squared_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    let dz = left[2] - right[2];
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_distance_along_cylinder_normal() {
        let pts1 = [[0.0, 0.0, -1.0], [0.1, 0.0, -0.5]];
        let pts2 = [[0.0, 0.0, 1.0], [0.1, 0.0, 1.5]];
        let stats = compute_stats(
            &pts1,
            &pts2,
            M3c2Config {
                skip_first1: false,
                skip_first2: false,
                center: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                cyl_radius2: 1.0,
                cyl_half_len: 2.0,
                min_points: 1,
                reg_error: 0.0,
            },
        )
        .unwrap();

        assert!((stats.distance - 2.0).abs() < 1e-12);
        assert_eq!(stats.n1, 2);
        assert_eq!(stats.n2, 2);
    }
}
