use pdal_core::point::{DimId, PointId, PointView};

pub fn covariance(view: &PointView, ids: &[PointId]) -> [[f64; 3]; 3] {
    if ids.len() < 2 {
        return [[0.0; 3]; 3];
    }

    let centroid = centroid(view, ids);
    let mut cov = [[0.0; 3]; 3];
    for id in ids {
        let delta = [
            (view.get_f64(*id, &DimId::X) - centroid[0]) as f32 as f64,
            (view.get_f64(*id, &DimId::Y) - centroid[1]) as f32 as f64,
            (view.get_f64(*id, &DimId::Z) - centroid[2]) as f32 as f64,
        ];
        for row in 0..3 {
            for col in 0..3 {
                cov[row][col] += delta[row] * delta[col];
            }
        }
    }

    let denom = (ids.len() - 1) as f64;
    for row in &mut cov {
        for value in row {
            *value /= denom;
        }
    }
    cov
}

pub fn rank(view: &PointView, ids: &[PointId], threshold: f64) -> u8 {
    symmetric_eigenvalues(covariance(view, ids))
        .into_iter()
        .filter(|value| value.abs() > threshold)
        .count() as u8
}

pub fn symmetric_eigenvalues(mut matrix: [[f64; 3]; 3]) -> [f64; 3] {
    for _ in 0..32 {
        let (p, q) = largest_off_diagonal(matrix);
        if matrix[p][q].abs() < 1e-12 {
            break;
        }

        let theta = 0.5 * (matrix[q][q] - matrix[p][p]) / matrix[p][q];
        let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
        let c = 1.0 / (t * t + 1.0).sqrt();
        let s = t * c;
        let tau = s / (1.0 + c);
        let mpq = matrix[p][q];

        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
        matrix[p][p] -= t * mpq;
        matrix[q][q] += t * mpq;

        for r in 0..3 {
            if r != p && r != q {
                let mrp = matrix[r][p];
                let mrq = matrix[r][q];
                matrix[r][p] = mrp - s * (mrq + tau * mrp);
                matrix[p][r] = matrix[r][p];
                matrix[r][q] = mrq + s * (mrp - tau * mrq);
                matrix[q][r] = matrix[r][q];
            }
        }
    }

    let mut values = [matrix[0][0], matrix[1][1], matrix[2][2]];
    values.sort_by(f64::total_cmp);
    values
}

fn centroid(view: &PointView, ids: &[PointId]) -> [f64; 3] {
    let mut centroid = [0.0; 3];
    for id in ids {
        centroid[0] += view.get_f64(*id, &DimId::X);
        centroid[1] += view.get_f64(*id, &DimId::Y);
        centroid[2] += view.get_f64(*id, &DimId::Z);
    }
    let count = ids.len() as f64;
    centroid[0] /= count;
    centroid[1] /= count;
    centroid[2] /= count;
    centroid
}

fn largest_off_diagonal(matrix: [[f64; 3]; 3]) -> (usize, usize) {
    let pairs = [(0, 1), (0, 2), (1, 2)];
    *pairs
        .iter()
        .max_by(|(ap, aq), (bp, bq)| matrix[*ap][*aq].abs().total_cmp(&matrix[*bp][*bq].abs()))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eigenvalues_of_diagonal_matrix_are_sorted() {
        assert_eq!(
            symmetric_eigenvalues([[3.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 2.0]]),
            [1.0, 2.0, 3.0]
        );
    }
}
