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

pub fn is_zero_matrix(matrix: [[f64; 3]; 3]) -> bool {
    matrix.iter().flatten().all(|value| *value == 0.0)
}

pub fn rank(view: &PointView, ids: &[PointId], threshold: f64) -> u8 {
    symmetric_eigenvalues(covariance(view, ids))
        .into_iter()
        .filter(|value| value.abs() > threshold)
        .count() as u8
}

pub fn symmetric_eigenvalues(matrix: [[f64; 3]; 3]) -> [f64; 3] {
    symmetric_eigen_decomposition(matrix).0
}

pub fn symmetric_eigen_decomposition(mut matrix: [[f64; 3]; 3]) -> ([f64; 3], [[f64; 3]; 3]) {
    let mut vectors = [[0.0; 3]; 3];
    for (idx, row) in vectors.iter_mut().enumerate() {
        row[idx] = 1.0;
    }

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

        let mut updates = Vec::new();
        for (r, row) in matrix.iter_mut().enumerate() {
            if r != p && r != q {
                let mrp = row[p];
                let mrq = row[q];
                row[p] = mrp - s * (mrq + tau * mrp);
                row[q] = mrq + s * (mrp - tau * mrq);
                updates.push((r, row[p], row[q]));
            }
        }
        for (r, new_rp, new_rq) in updates {
            matrix[p][r] = new_rp;
            matrix[q][r] = new_rq;
        }

        for row in &mut vectors {
            let vrp = row[p];
            let vrq = row[q];
            row[p] = vrp - s * (vrq + tau * vrp);
            row[q] = vrq + s * (vrp - tau * vrq);
        }
    }

    let mut pairs = [
        (matrix[0][0], [vectors[0][0], vectors[1][0], vectors[2][0]]),
        (matrix[1][1], [vectors[0][1], vectors[1][1], vectors[2][1]]),
        (matrix[2][2], [vectors[0][2], vectors[1][2], vectors[2][2]]),
    ];
    pairs.sort_by(|left, right| left.0.total_cmp(&right.0));

    let values = [pairs[0].0, pairs[1].0, pairs[2].0];
    let sorted_vectors = [
        [pairs[0].1[0], pairs[1].1[0], pairs[2].1[0]],
        [pairs[0].1[1], pairs[1].1[1], pairs[2].1[1]],
        [pairs[0].1[2], pairs[1].1[2], pairs[2].1[2]],
    ];
    (values, sorted_vectors)
}

pub fn centroid(view: &PointView, ids: &[PointId]) -> [f64; 3] {
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

/// Compute the centroid of interleaved `[x, y, z]` points.
///
/// Uses the running-mean update of `pdal::math::computeCentroid` so the result
/// matches the C++ helper bit-for-bit.
pub fn compute_centroid(xyz: &[f64], count: usize) -> [f64; 3] {
    let mut mean = [0.0f64; 3];
    let mut n = 0.0f64;
    for i in 0..count {
        n += 1.0;
        for (k, m) in mean.iter_mut().enumerate() {
            let value = xyz[3 * i + k];
            *m += (value - *m) / n;
        }
    }
    mean
}

fn largest_off_diagonal(matrix: [[f64; 3]; 3]) -> (usize, usize) {
    let pairs = [(0, 1), (0, 2), (1, 2)];
    *pairs
        .iter()
        .max_by(|(ap, aq), (bp, bq)| matrix[*ap][*aq].abs().total_cmp(&matrix[*bp][*bq].abs()))
        .unwrap()
}

/// Compute the numerical gradient in the X direction (column major).
pub fn grad_x(data: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let mut out = vec![0.0; data.len()];
    if cols < 2 {
        return out;
    }

    for r in 0..rows {
        // Edge column 0
        out[r] = data[rows + r] - data[r];
        // Interior columns
        for c in 1..(cols - 1) {
            out[c * rows + r] = 0.5 * (data[(c + 1) * rows + r] - data[(c - 1) * rows + r]);
        }
        // Edge column last
        out[(cols - 1) * rows + r] = data[(cols - 1) * rows + r] - data[(cols - 2) * rows + r];
    }
    out
}

/// Compute the numerical gradient in the Y direction (column major).
pub fn grad_y(data: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let mut out = vec![0.0; data.len()];
    if rows < 2 {
        return out;
    }

    for c in 0..cols {
        let offset = c * rows;
        // Edge row 0
        out[offset] = data[offset + 1] - data[offset];
        // Interior rows
        for r in 1..(rows - 1) {
            out[offset + r] = 0.5 * (data[offset + r + 1] - data[offset + r - 1]);
        }
        // Edge row last
        out[offset + rows - 1] = data[offset + rows - 1] - data[offset + rows - 2];
    }
    out
}

pub fn dilate_diamond(data: &mut [f64], rows: usize, cols: usize, iterations: usize) {
    let mut out = vec![f64::NEG_INFINITY; data.len()];
    for _ in 0..iterations {
        for c in 0..cols {
            let offset = c * rows;
            for r in 0..rows {
                let idx = offset + r;
                let mut max_val = data[idx];
                if r > 0 {
                    max_val = max_val.max(data[idx - 1]);
                }
                if r < rows - 1 {
                    max_val = max_val.max(data[idx + 1]);
                }
                if c > 0 {
                    max_val = max_val.max(data[idx - rows]);
                }
                if c < cols - 1 {
                    max_val = max_val.max(data[idx + rows]);
                }
                out[idx] = max_val;
            }
        }
        data.copy_from_slice(&out);
        out.fill(f64::NEG_INFINITY);
    }
}

pub fn erode_diamond(data: &mut [f64], rows: usize, cols: usize, iterations: usize) {
    let mut out = vec![f64::INFINITY; data.len()];
    for _ in 0..iterations {
        for c in 0..cols {
            let offset = c * rows;
            for r in 0..rows {
                let idx = offset + r;
                let mut min_val = data[idx];
                if r > 0 {
                    min_val = min_val.min(data[idx - 1]);
                }
                if r < rows - 1 {
                    min_val = min_val.min(data[idx + 1]);
                }
                if c > 0 {
                    min_val = min_val.min(data[idx - rows]);
                }
                if c < cols - 1 {
                    min_val = min_val.min(data[idx + rows]);
                }
                out[idx] = min_val;
            }
        }
        data.copy_from_slice(&out);
        out.fill(f64::INFINITY);
    }
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

    #[test]
    fn eigenvectors_follow_sorted_eigenvalues() {
        let (values, vectors) =
            symmetric_eigen_decomposition([[3.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 2.0]]);
        assert_eq!(values, [1.0, 2.0, 3.0]);
        assert_eq!(vectors, [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
    }
}
