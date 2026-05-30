//! C ABI for the grid-accelerated point-in-polygon engine
//! (`pdal_filters::gridpnp::GridPnp`), used by the C++ `filters.crop` polygon
//! path. Build once from a polygon's rings, then query `inside` per point.

use crate::error::set_last_error;
use pdal_filters::gridpnp::GridPnp;

#[allow(non_camel_case_types)]
pub struct pdal_gridpnp_t {
    inner: GridPnp,
}

/// Build a point-in-polygon engine. `coords` is all ring vertices flattened as
/// x,y pairs; `ring_sizes[i]` is the vertex count of ring `i`, with ring 0 the
/// exterior and the rest interior (hole) rings. Each ring must be closed (first
/// vertex repeated) with >= 4 vertices. Returns null on error and sets the last
/// error message.
///
/// # Safety
///
/// `coords` must hold `2 * sum(ring_sizes)` f64s; `ring_sizes` must hold
/// `ring_count` usizes. The returned pointer is owned by the caller and must be
/// released with `pdal_gridpnp_destroy`.
#[no_mangle]
pub unsafe extern "C" fn pdal_gridpnp_create(
    coords: *const f64,
    ring_sizes: *const usize,
    ring_count: usize,
) -> *mut pdal_gridpnp_t {
    if coords.is_null() || ring_sizes.is_null() || ring_count == 0 {
        set_last_error("pdal_gridpnp_create: null/empty arguments");
        return std::ptr::null_mut();
    }
    let sizes = std::slice::from_raw_parts(ring_sizes, ring_count);
    let total: usize = sizes.iter().sum();
    let flat = std::slice::from_raw_parts(coords, total * 2);

    // Split the flat coordinate buffer into rings.
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::with_capacity(ring_count);
    let mut pos = 0usize;
    for &n in sizes {
        let mut ring = Vec::with_capacity(n);
        for _ in 0..n {
            ring.push((flat[pos * 2], flat[pos * 2 + 1]));
            pos += 1;
        }
        rings.push(ring);
    }

    let outer = &rings[0];
    let inners: Vec<Vec<(f64, f64)>> = rings[1..].to_vec();
    match GridPnp::new(outer, &inners) {
        Ok(inner) => Box::into_raw(Box::new(pdal_gridpnp_t { inner })),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

/// Free a handle from `pdal_gridpnp_create`. Safe to pass null.
///
/// # Safety
///
/// `handle` must have come from `pdal_gridpnp_create` and not been freed.
#[no_mangle]
pub unsafe extern "C" fn pdal_gridpnp_destroy(handle: *mut pdal_gridpnp_t) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Point-in-polygon test. Returns true if (x, y) is inside the polygon (edge-on
/// counts as inside), false otherwise or on a null handle.
///
/// # Safety
///
/// `handle` must come from `pdal_gridpnp_create`.
#[no_mangle]
pub unsafe extern "C" fn pdal_gridpnp_inside(
    handle: *const pdal_gridpnp_t,
    x: f64,
    y: f64,
) -> bool {
    match handle.as_ref() {
        Some(h) => h.inner.inside(x, y),
        None => false,
    }
}
