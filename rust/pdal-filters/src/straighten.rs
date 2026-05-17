use std::f64::consts::FRAC_PI_2;

pub fn straighten_point(
    point: [f64; 3],
    segment: [f64; 5],
    segment_offset: f64,
    offset: f64,
) -> [f64; 3] {
    let (matrix, translation) = transform(
        segment[0],
        segment[1],
        segment[2],
        segment[3],
        0.0,
        FRAC_PI_2 - segment[4],
    );
    let delta = [
        point[0] - translation[0],
        point[1] - translation[1],
        point[2] - translation[2],
    ];
    let straight = transpose_mul(matrix, delta);
    [
        straight[0] + segment_offset + offset,
        straight[1],
        straight[2],
    ]
}

pub fn unstraighten_point(point: [f64; 3], segment: [f64; 5]) -> [f64; 3] {
    let (matrix, translation) = transform(
        segment[0],
        segment[1],
        segment[2],
        segment[3],
        0.0,
        FRAC_PI_2 - segment[4],
    );
    let straight = [0.0, point[1], point[2]];
    let rotated = matrix_mul(matrix, straight);
    [
        rotated[0] + translation[0],
        rotated[1] + translation[1],
        rotated[2] + translation[2],
    ]
}

fn transform(x: f64, y: f64, z: f64, roll: f64, pitch: f64, yaw: f64) -> ([[f64; 3]; 3], [f64; 3]) {
    let a = yaw.cos();
    let b = yaw.sin();
    let c = pitch.cos();
    let d = pitch.sin();
    let e = roll.cos();
    let f = roll.sin();
    let de = d * e;
    let df = d * f;

    (
        [
            [a * c, a * df - b * e, b * f + a * de],
            [b * c, a * e + b * df, b * de - a * f],
            [-d, c * f, c * e],
        ],
        [x, y, z],
    )
}

fn matrix_mul(matrix: [[f64; 3]; 3], value: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * value[0] + matrix[0][1] * value[1] + matrix[0][2] * value[2],
        matrix[1][0] * value[0] + matrix[1][1] * value[1] + matrix[1][2] * value[2],
        matrix[2][0] * value[0] + matrix[2][1] * value[1] + matrix[2][2] * value[2],
    ]
}

fn transpose_mul(matrix: [[f64; 3]; 3], value: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * value[0] + matrix[1][0] * value[1] + matrix[2][0] * value[2],
        matrix[0][1] * value[0] + matrix[1][1] * value[1] + matrix[2][1] * value[2],
        matrix[0][2] * value[0] + matrix[1][2] * value[1] + matrix[2][2] * value[2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straightening_round_trips_on_simple_polyline_segment() {
        let segment = [0.0, 25.0, 0.0, 0.0, 0.0];
        let original = [2.0, 25.0, 1.0];
        let straight = straighten_point(original, segment, 25.0, 0.0);
        let back = unstraighten_point(straight, segment);
        for i in 0..3 {
            assert!((back[i] - original[i]).abs() < 1e-12);
        }
    }
}
