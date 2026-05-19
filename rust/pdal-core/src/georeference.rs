//! Georeference math shared by readers and filters.
//!
//! Port of `pdal/util/Georeference.cpp` plus the Optech rotation helper from
//! `io/OptechCommon.hpp`.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xyz {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotationMatrix {
    pub m00: f64,
    pub m01: f64,
    pub m02: f64,
    pub m10: f64,
    pub m11: f64,
    pub m12: f64,
    pub m20: f64,
    pub m21: f64,
    pub m22: f64,
}

pub fn identity_matrix() -> RotationMatrix {
    RotationMatrix {
        m00: 1.0,
        m01: 0.0,
        m02: 0.0,
        m10: 0.0,
        m11: 1.0,
        m12: 0.0,
        m20: 0.0,
        m21: 0.0,
        m22: 1.0,
    }
}

pub fn create_optech_rotation_matrix(roll: f64, pitch: f64, heading: f64) -> RotationMatrix {
    RotationMatrix {
        m00: roll.cos() * heading.cos() + pitch.sin() * roll.sin() * heading.sin(),
        m01: pitch.cos() * heading.sin(),
        m02: heading.cos() * roll.sin() - roll.cos() * pitch.sin() * heading.sin(),
        m10: heading.cos() * pitch.sin() * roll.sin() - roll.cos() * heading.sin(),
        m11: pitch.cos() * heading.cos(),
        m12: -roll.sin() * heading.sin() - roll.cos() * heading.cos() * pitch.sin(),
        m20: -pitch.cos() * roll.sin(),
        m21: pitch.sin(),
        m22: pitch.cos() * roll.cos(),
    }
}

pub fn georeference_wgs84(
    range: f64,
    scan_angle: f64,
    boresight: RotationMatrix,
    imu: RotationMatrix,
    gps_point: Xyz,
) -> Xyz {
    let sensor = Xyz {
        x: range * scan_angle.sin(),
        y: 0.0,
        z: -range * scan_angle.cos(),
    };
    let aligned = rotate(sensor, boresight);
    let local_level = rotate(aligned, imu);
    let curvilinear = cartesian_to_curvilinear(local_level, gps_point.y);
    Xyz {
        x: gps_point.x + curvilinear.x,
        y: gps_point.y + curvilinear.y,
        z: gps_point.z + curvilinear.z,
    }
}

fn rotate(point: Xyz, matrix: RotationMatrix) -> Xyz {
    Xyz {
        x: matrix.m00 * point.x + matrix.m01 * point.y + matrix.m02 * point.z,
        y: matrix.m10 * point.x + matrix.m11 * point.y + matrix.m12 * point.z,
        z: matrix.m20 * point.x + matrix.m21 * point.y + matrix.m22 * point.z,
    }
}

fn cartesian_to_curvilinear(point: Xyz, latitude: f64) -> Xyz {
    let a = 6378137.0;
    let f = 1.0 / 298.257223563;
    let e2 = 2.0 * f - f * f;
    let w = (1.0 - e2 * latitude.sin() * latitude.sin()).sqrt();
    let n = a / w;
    let m = a * (1.0 - e2) / (w * w * w);
    Xyz {
        x: point.x / (n * latitude.cos()),
        y: point.y / m,
        z: point.z,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_rotation_leaves_point_unchanged() {
        let point = Xyz {
            x: 1.0,
            y: -2.0,
            z: 3.0,
        };
        assert_eq!(rotate(point, identity_matrix()), point);
    }

    #[test]
    fn optech_rotation_matrix_matches_cpp_formula() {
        let matrix = create_optech_rotation_matrix(0.1, -0.2, 0.3);
        assert!((matrix.m00 - 0.9447024859948943).abs() < 1e-15);
        assert!((matrix.m01 - 0.28962947762551555).abs() < 1e-15);
        assert!((matrix.m12 - 0.1593450793079779).abs() < 1e-15);
        assert!((matrix.m22 - 0.975170327201816).abs() < 1e-15);
    }

    #[test]
    fn georeference_wgs84_applies_range_at_identity_orientation() {
        let boresight = identity_matrix();
        let imu = identity_matrix();
        let gps = Xyz {
            x: 1.0,
            y: 0.5,
            z: 100.0,
        };
        let point = georeference_wgs84(10.0, 0.0, boresight, imu, gps);

        assert_eq!(point.x, 1.0);
        assert_eq!(point.y, 0.5);
        assert_eq!(point.z, 90.0);
    }
}
