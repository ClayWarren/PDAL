//! Scale/offset helpers for PDAL point dimensions.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XFormComponent {
    pub auto: bool,
    pub value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XForm {
    pub offset: XFormComponent,
    pub scale: XFormComponent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scaling {
    pub x: XForm,
    pub y: XForm,
    pub z: XForm,
}

pub fn set_auto_xform(scaling: &mut Scaling, xs: &[f64], ys: &[f64], zs: &[f64]) {
    let count = xs.len().min(ys.len()).min(zs.len());
    if count == 0 {
        return;
    }

    update_axis(&mut scaling.x, &xs[..count]);
    update_axis(&mut scaling.y, &ys[..count]);
    update_axis(&mut scaling.z, &zs[..count]);
}

fn update_axis(xform: &mut XForm, values: &[f64]) {
    if !xform.offset.auto && !xform.scale.auto {
        return;
    }

    let mut min = f64::MAX;
    let mut max = f64::MIN;
    for value in values {
        min = min.min(*value);
        max = max.max(*value);
    }

    if xform.offset.auto {
        xform.offset.value = 0.5 * min + 0.5 * max;
    }
    if xform.scale.auto {
        let range = (max - xform.offset.value)
            .abs()
            .max((min - xform.offset.value).abs());
        xform.scale.value = if range != 0.0 {
            range / f64::from(i32::MAX)
        } else {
            1.0
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis(offset_auto: bool, scale_auto: bool) -> XForm {
        XForm {
            offset: XFormComponent {
                auto: offset_auto,
                value: 0.0,
            },
            scale: XFormComponent {
                auto: scale_auto,
                value: 1.0,
            },
        }
    }

    #[test]
    fn computes_auto_offsets_and_scales() {
        let mut scaling = Scaling {
            x: axis(true, true),
            y: axis(true, true),
            z: axis(true, true),
        };

        set_auto_xform(
            &mut scaling,
            &[-10.0, 30.0, 20.0, 40.0],
            &[100.0, 160.0, 80.0, 200.0],
            &[2.0, 8.0, -4.0, 14.0],
        );

        assert_eq!(scaling.x.offset.value, 15.0);
        assert_eq!(scaling.y.offset.value, 140.0);
        assert_eq!(scaling.z.offset.value, 5.0);
        assert_eq!(scaling.x.scale.value, 25.0 / f64::from(i32::MAX));
        assert_eq!(scaling.y.scale.value, 60.0 / f64::from(i32::MAX));
        assert_eq!(scaling.z.scale.value, 9.0 / f64::from(i32::MAX));
    }

    #[test]
    fn leaves_standard_transform_unchanged() {
        let mut scaling = Scaling {
            x: axis(false, false),
            y: axis(false, false),
            z: axis(false, false),
        };

        set_auto_xform(&mut scaling, &[10.0], &[20.0], &[30.0]);

        assert_eq!(scaling.x.scale.value, 1.0);
        assert_eq!(scaling.y.scale.value, 1.0);
        assert_eq!(scaling.z.scale.value, 1.0);
        assert_eq!(scaling.x.offset.value, 0.0);
        assert_eq!(scaling.y.offset.value, 0.0);
        assert_eq!(scaling.z.offset.value, 0.0);
    }
}
