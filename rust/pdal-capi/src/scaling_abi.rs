use pdal_core::scaling::{set_auto_xform, Scaling, XForm, XFormComponent};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_xform_component_t {
    pub is_auto: bool,
    pub value: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_xform_t {
    pub offset: pdal_xform_component_t,
    pub scale: pdal_xform_component_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct pdal_scaling_t {
    pub x: pdal_xform_t,
    pub y: pdal_xform_t,
    pub z: pdal_xform_t,
}

/// Compute PDAL auto scale/offset values in-place.
///
/// # Safety
///
/// `xs`, `ys`, and `zs` must each point to at least `count` doubles.
/// `scaling` must be a valid mutable pointer.
#[no_mangle]
pub unsafe extern "C" fn pdal_scaling_set_auto_xform(
    xs: *const f64,
    ys: *const f64,
    zs: *const f64,
    count: u64,
    scaling: *mut pdal_scaling_t,
) -> bool {
    if xs.is_null() || ys.is_null() || zs.is_null() || scaling.is_null() || count == 0 {
        return false;
    }

    let len = count as usize;
    let xs = std::slice::from_raw_parts(xs, len);
    let ys = std::slice::from_raw_parts(ys, len);
    let zs = std::slice::from_raw_parts(zs, len);
    let mut rust_scaling = (*scaling).into();
    set_auto_xform(&mut rust_scaling, xs, ys, zs);
    *scaling = rust_scaling.into();
    true
}

impl From<pdal_xform_component_t> for XFormComponent {
    fn from(component: pdal_xform_component_t) -> Self {
        Self {
            auto: component.is_auto,
            value: component.value,
        }
    }
}

impl From<XFormComponent> for pdal_xform_component_t {
    fn from(component: XFormComponent) -> Self {
        Self {
            is_auto: component.auto,
            value: component.value,
        }
    }
}

impl From<pdal_xform_t> for XForm {
    fn from(xform: pdal_xform_t) -> Self {
        Self {
            offset: xform.offset.into(),
            scale: xform.scale.into(),
        }
    }
}

impl From<XForm> for pdal_xform_t {
    fn from(xform: XForm) -> Self {
        Self {
            offset: xform.offset.into(),
            scale: xform.scale.into(),
        }
    }
}

impl From<pdal_scaling_t> for Scaling {
    fn from(scaling: pdal_scaling_t) -> Self {
        Self {
            x: scaling.x.into(),
            y: scaling.y.into(),
            z: scaling.z.into(),
        }
    }
}

impl From<Scaling> for pdal_scaling_t {
    fn from(scaling: Scaling) -> Self {
        Self {
            x: scaling.x.into(),
            y: scaling.y.into(),
            z: scaling.z.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(is_auto: bool, value: f64) -> pdal_xform_component_t {
        pdal_xform_component_t { is_auto, value }
    }

    fn xform(offset_auto: bool, scale_auto: bool) -> pdal_xform_t {
        pdal_xform_t {
            offset: component(offset_auto, 0.0),
            scale: component(scale_auto, 1.0),
        }
    }

    #[test]
    fn scaling_c_abi_computes_auto_values() {
        let xs = [-10.0, 30.0, 20.0, 40.0];
        let ys = [100.0, 160.0, 80.0, 200.0];
        let zs = [2.0, 8.0, -4.0, 14.0];
        let mut scaling = pdal_scaling_t {
            x: xform(true, true),
            y: xform(true, true),
            z: xform(true, true),
        };

        let ok = unsafe {
            pdal_scaling_set_auto_xform(
                xs.as_ptr(),
                ys.as_ptr(),
                zs.as_ptr(),
                xs.len() as u64,
                &mut scaling,
            )
        };

        assert!(ok);
        assert_eq!(scaling.x.offset.value, 15.0);
        assert_eq!(scaling.y.offset.value, 140.0);
        assert_eq!(scaling.z.offset.value, 5.0);
    }
}
