//! The point-buffer model -- the Rust analog of PDAL's `Dimension`,
//! `PointLayout`, `PointTable` and `PointView`.
//!
//! A [`PointLayout`] fixes the set of dimensions and their byte offsets within
//! a fixed-size point record. A [`PointView`] is a contiguous buffer of such
//! records. PDAL shares one `PointTable` across many views; this spike folds
//! the storage into the view for simplicity -- the shared table is a planned
//! follow-up, not a behavioural difference for a single filter.

use crate::raster::{RasterData, RasterLimits};
use crate::srs::SpatialReference;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "point_dimensions.rs"]
mod point_dimensions;
pub use point_dimensions::{
    fix_dimension_name, pdal_dimension_interpretation_name, pdal_dimension_type_from_base_and_size,
    pdal_dimension_type_from_name, resolve_pdal_dimension_type, DimId, DimType,
};

/// Index of a point within a view.
pub type PointId = u64;

static NEXT_POINT_VIEW_ID: AtomicU64 = AtomicU64::new(0);

fn next_point_view_id() -> u64 {
    NEXT_POINT_VIEW_ID.fetch_add(1, Ordering::Relaxed) + 1
}

#[derive(Clone)]
struct DimEntry {
    id: DimId,
    ty: DimType,
    offset: usize,
}

/// The ordered set of registered dimensions, fixing each one's byte offset
/// within a point record. Immutable once a [`PointView`] is built from it.
#[derive(Clone, Default)]
pub struct PointLayout {
    dims: Vec<DimEntry>,
    point_size: usize,
}

impl PointLayout {
    /// An empty layout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a dimension. Re-registering an existing dimension is a no-op
    /// (PDAL likewise keeps the first registration).
    pub fn register(&mut self, id: DimId, ty: DimType) {
        if self.dims.iter().any(|d| d.id == id) {
            return;
        }
        let offset = self.point_size;
        self.point_size += ty.size();
        self.dims.push(DimEntry { id, ty, offset });
    }

    /// Size in bytes of one point record.
    pub fn point_size(&self) -> usize {
        self.point_size
    }

    /// The `(byte offset, type)` of a dimension, if registered.
    pub fn dim(&self, id: &DimId) -> Option<(usize, DimType)> {
        self.dims
            .iter()
            .find(|d| &d.id == id)
            .map(|d| (d.offset, d.ty))
    }

    /// Number of dimensions in registration order.
    pub fn dim_count(&self) -> usize {
        self.dims.len()
    }

    /// Dimension id and type at registration index `idx`.
    pub fn dim_at(&self, idx: usize) -> Option<(&DimId, DimType)> {
        self.dims.get(idx).map(|d| (&d.id, d.ty))
    }
}

/// A buffer of points sharing one layout.
#[derive(Clone)]
pub struct Triangle {
    pub a: PointId,
    pub b: PointId,
    pub c: PointId,
}

#[derive(Clone, Default)]
pub struct TriangularMesh {
    triangles: Vec<Triangle>,
}

impl TriangularMesh {
    pub fn add(&mut self, a: PointId, b: PointId, c: PointId) {
        self.triangles.push(Triangle { a, b, c });
    }

    pub fn len(&self) -> usize {
        self.triangles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    pub fn triangles(&self) -> &[Triangle] {
        &self.triangles
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds2D {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
}

impl Bounds2D {
    fn new(x: f64, y: f64) -> Self {
        Self {
            minx: x,
            maxx: x,
            miny: y,
            maxy: y,
        }
    }

    fn grow(&mut self, x: f64, y: f64) {
        self.minx = self.minx.min(x);
        self.maxx = self.maxx.max(x);
        self.miny = self.miny.min(y);
        self.maxy = self.maxy.max(y);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds3D {
    pub minx: f64,
    pub maxx: f64,
    pub miny: f64,
    pub maxy: f64,
    pub minz: f64,
    pub maxz: f64,
}

impl Bounds3D {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            minx: x,
            maxx: x,
            miny: y,
            maxy: y,
            minz: z,
            maxz: z,
        }
    }

    fn grow(&mut self, x: f64, y: f64, z: f64) {
        self.minx = self.minx.min(x);
        self.maxx = self.maxx.max(x);
        self.miny = self.miny.min(y);
        self.maxy = self.maxy.max(y);
        self.minz = self.minz.min(z);
        self.maxz = self.maxz.max(z);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DimensionSummary {
    pub name: String,
    pub count: u64,
    pub minimum: f64,
    pub maximum: f64,
    pub mean: f64,
}

pub struct PointView {
    id: u64,
    layout: Rc<PointLayout>,
    data: Vec<u8>,
    source_indices: Vec<PointId>,
    spatial_reference: SpatialReference,
    meshes: BTreeMap<String, TriangularMesh>,
    rasters: Vec<RasterData>,
}

impl Clone for PointView {
    fn clone(&self) -> Self {
        Self {
            id: next_point_view_id(),
            layout: Rc::clone(&self.layout),
            data: self.data.clone(),
            source_indices: self.source_indices.clone(),
            spatial_reference: self.spatial_reference.clone(),
            meshes: self.meshes.clone(),
            rasters: self.rasters.clone(),
        }
    }
}

impl PointView {
    /// A new, empty view over the given layout.
    pub fn new(layout: Rc<PointLayout>) -> Self {
        PointView {
            id: next_point_view_id(),
            layout,
            data: Vec::new(),
            source_indices: Vec::new(),
            spatial_reference: SpatialReference::default(),
            meshes: BTreeMap::new(),
            rasters: Vec::new(),
        }
    }

    /// A new empty view sharing this view's layout (PDAL's `makeNew`).
    pub fn make_new(&self) -> PointView {
        PointView {
            id: next_point_view_id(),
            layout: Rc::clone(&self.layout),
            data: Vec::new(),
            source_indices: Vec::new(),
            spatial_reference: self.spatial_reference.clone(),
            meshes: BTreeMap::new(),
            rasters: Vec::new(),
        }
    }

    /// Return a copy of this view whose layout includes the requested
    /// dimensions. Existing values are copied through typed accessors.
    pub fn with_dimensions(&self, dims: &[(DimId, DimType)]) -> PointView {
        if dims.iter().all(|(dim, _)| self.layout.dim(dim).is_some()) {
            return self.clone();
        }

        let mut layout = (*self.layout).clone();
        for (dim, ty) in dims {
            layout.register(dim.clone(), *ty);
        }

        let mut output = PointView::new(Rc::new(layout));
        output.spatial_reference = self.spatial_reference.clone();
        output.meshes = self.meshes.clone();
        output.rasters = self.rasters.clone();

        for idx in 0..self.len() {
            let out_idx = output.add_point();
            for dim_idx in 0..self.layout.dim_count() {
                if let Some((dim, _)) = self.layout.dim_at(dim_idx) {
                    output.set_f64(out_idx, dim, self.get_f64(idx, dim));
                }
            }
            if let Some(source_index) = self.source_indices.get(idx as usize) {
                output.source_indices[out_idx as usize] = *source_index;
            }
        }

        output
    }

    /// Return a copy of this view containing only dimensions already present in
    /// this view and named in `dims`, preserving the requested order.
    pub fn select_dimensions(&self, dims: &[DimId]) -> PointView {
        let mut layout = PointLayout::new();
        let mut selected = Vec::new();
        for dim in dims {
            if let Some((_, ty)) = self.layout.dim(dim) {
                layout.register(dim.clone(), ty);
                selected.push(dim.clone());
            }
        }

        let mut output = PointView::new(Rc::new(layout));
        output.spatial_reference = self.spatial_reference.clone();
        output.meshes = self.meshes.clone();
        output.rasters = self.rasters.clone();

        for idx in 0..self.len() {
            let out_idx = output.add_point();
            for dim in &selected {
                let Some((in_off, ty)) = self.layout.dim(dim) else {
                    continue;
                };
                let Some((out_off, _)) = output.layout.dim(dim) else {
                    continue;
                };
                let in_base = (idx as usize) * self.layout.point_size() + in_off;
                let out_base = (out_idx as usize) * output.layout.point_size() + out_off;
                output.data[out_base..out_base + ty.size()]
                    .copy_from_slice(&self.data[in_base..in_base + ty.size()]);
            }
            if let Some(source_index) = self.source_indices.get(idx as usize) {
                output.source_indices[out_idx as usize] = *source_index;
            }
        }

        output
    }

    /// The view's layout.
    pub fn layout(&self) -> &Rc<PointLayout> {
        &self.layout
    }

    /// Stable monotonically increasing identity for this view.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The view's spatial reference.
    pub fn spatial_reference(&self) -> &SpatialReference {
        &self.spatial_reference
    }

    /// Set the view's spatial reference.
    pub fn set_spatial_reference(&mut self, spatial_reference: SpatialReference) {
        self.spatial_reference = spatial_reference;
    }

    pub fn create_mesh(&mut self) -> &mut TriangularMesh {
        self.meshes.entry(String::new()).or_default()
    }

    pub fn create_named_mesh(&mut self, name: &str) -> Option<&mut TriangularMesh> {
        if self.meshes.contains_key(name) {
            return None;
        }
        self.meshes
            .insert(name.to_string(), TriangularMesh::default());
        self.meshes.get_mut(name)
    }

    pub fn mesh(&self) -> Option<&TriangularMesh> {
        self.mesh_named("")
    }

    pub fn mesh_named(&self, name: &str) -> Option<&TriangularMesh> {
        self.meshes.get(name).or_else(|| {
            name.is_empty()
                .then(|| self.meshes.values().next())
                .flatten()
        })
    }

    pub fn mesh_mut_named(&mut self, name: &str) -> Option<&mut TriangularMesh> {
        if self.meshes.contains_key(name) {
            return self.meshes.get_mut(name);
        }
        if name.is_empty() {
            let key = self.meshes.keys().next()?.clone();
            return self.meshes.get_mut(&key);
        }
        None
    }

    pub fn add_raster(&mut self, raster: RasterData) {
        self.rasters.push(raster);
    }

    pub fn create_raster(
        &mut self,
        name: &str,
        limits: RasterLimits,
        initializer: f64,
    ) -> Option<&mut RasterData> {
        if self.raster(name).is_some() {
            return None;
        }
        self.rasters
            .push(RasterData::new(name.to_string(), limits, initializer));
        self.rasters.last_mut()
    }

    pub fn raster(&self, name: &str) -> Option<&RasterData> {
        self.rasters
            .iter()
            .find(|raster| raster.name() == name)
            .or_else(|| name.is_empty().then(|| self.rasters.first()).flatten())
    }

    pub fn raster_mut(&mut self, name: &str) -> Option<&mut RasterData> {
        let idx = self
            .rasters
            .iter()
            .position(|raster| raster.name() == name)
            .or_else(|| {
                name.is_empty()
                    .then_some(0)
                    .filter(|_| !self.rasters.is_empty())
            })?;
        self.rasters.get_mut(idx)
    }

    pub fn rasters(&self) -> &[RasterData] {
        &self.rasters
    }

    pub fn calculate_bounds_2d(&self) -> Option<Bounds2D> {
        if self.is_empty()
            || self.layout.dim(&DimId::X).is_none()
            || self.layout.dim(&DimId::Y).is_none()
        {
            return None;
        }

        let mut bounds = Bounds2D::new(self.get_f64(0, &DimId::X), self.get_f64(0, &DimId::Y));
        for idx in 1..self.len() {
            bounds.grow(self.get_f64(idx, &DimId::X), self.get_f64(idx, &DimId::Y));
        }
        Some(bounds)
    }

    pub fn calculate_bounds_3d(&self) -> Option<Bounds3D> {
        if self.is_empty()
            || self.layout.dim(&DimId::X).is_none()
            || self.layout.dim(&DimId::Y).is_none()
            || self.layout.dim(&DimId::Z).is_none()
        {
            return None;
        }

        let mut bounds = Bounds3D::new(
            self.get_f64(0, &DimId::X),
            self.get_f64(0, &DimId::Y),
            self.get_f64(0, &DimId::Z),
        );
        for idx in 1..self.len() {
            bounds.grow(
                self.get_f64(idx, &DimId::X),
                self.get_f64(idx, &DimId::Y),
                self.get_f64(idx, &DimId::Z),
            );
        }
        Some(bounds)
    }

    pub fn summarize_dimension(&self, dim: &DimId) -> Option<DimensionSummary> {
        if self.is_empty() || self.layout.dim(dim).is_none() {
            return None;
        }

        let mut minimum = self.get_f64(0, dim);
        let mut maximum = minimum;
        let mut sum = minimum;
        for idx in 1..self.len() {
            let value = self.get_f64(idx, dim);
            minimum = minimum.min(value);
            maximum = maximum.max(value);
            sum += value;
        }

        Some(DimensionSummary {
            name: dim.name().to_string(),
            count: self.len(),
            minimum,
            maximum,
            mean: sum / self.len() as f64,
        })
    }

    pub fn summarize_dimensions(&self) -> Vec<DimensionSummary> {
        (0..self.layout.dim_count())
            .filter_map(|idx| {
                self.layout
                    .dim_at(idx)
                    .and_then(|(dim, _)| self.summarize_dimension(dim))
            })
            .collect()
    }

    /// Number of points in the view.
    pub fn len(&self) -> u64 {
        let ps = self.layout.point_size();
        self.data.len().checked_div(ps).unwrap_or(0) as u64
    }

    /// Whether the view holds no points.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Truncate the view to `len` points.
    pub fn truncate(&mut self, len: u64) {
        let ps = self.layout.point_size();
        self.data.truncate((len as usize) * ps);
        self.source_indices.truncate(len as usize);
    }

    /// Append a zero-initialised point and return its index.
    pub fn add_point(&mut self) -> PointId {
        let id = self.len();
        let ps = self.layout.point_size();
        self.data.resize(self.data.len() + ps, 0);
        self.source_indices.push(id);
        id
    }

    /// Copy point `src_idx` from `src` onto the end of this view (PDAL's
    /// `appendPoint`). The two views must share an identical layout.
    pub fn append_point(&mut self, src: &PointView, src_idx: PointId) {
        let ps = self.layout.point_size();
        let start = (src_idx as usize) * ps;
        self.data.extend_from_slice(&src.data[start..start + ps]);
        self.source_indices.push(src.source_index(src_idx));
    }

    /// Append every point of `src` onto the end of this view (PDAL's
    /// `PointView::append`). The two views must share an identical layout; the
    /// raw point buffer and source indices are copied in bulk.
    pub fn append(&mut self, src: &PointView) {
        self.data.extend_from_slice(&src.data);
        self.source_indices.extend_from_slice(&src.source_indices);
    }

    /// Swap the stored data and source indices for two point rows.
    pub fn swap_points(&mut self, a: PointId, b: PointId) -> bool {
        if a >= self.len() || b >= self.len() {
            return false;
        }
        if a == b {
            return true;
        }

        let ps = self.layout.point_size();
        let a_start = (a as usize) * ps;
        let b_start = (b as usize) * ps;
        for offset in 0..ps {
            self.data.swap(a_start + offset, b_start + offset);
        }
        self.source_indices.swap(a as usize, b as usize);
        true
    }

    /// Reorder points in place so output position `i` holds the point currently
    /// at `order[i]` (a gather permutation, as produced by sorting an index
    /// vector). `order` must be a permutation of `0..len()`; if its length,
    /// values, or uniqueness are invalid the view is left unchanged. Done in
    /// place with only auxiliary index vectors, so callers like `filters.sort`
    /// avoid allocating a second full copy of the point buffer.
    pub fn reorder(&mut self, order: &[PointId]) {
        let n = self.len() as usize;
        if order.len() != n {
            return;
        }
        // `inverse[src]` is the destination slot for the point currently at
        // `src`. Applying the inverse permutation with swap cycles realizes the
        // requested gather (each row is moved into place exactly once).
        let mut inverse = vec![0u64; n];
        let mut seen = vec![false; n];
        for (dst, &src) in order.iter().enumerate() {
            let s = src as usize;
            if s >= n || seen[s] {
                return;
            }
            seen[s] = true;
            inverse[s] = dst as u64;
        }
        for i in 0..n {
            while inverse[i] as usize != i {
                let j = inverse[i] as usize;
                self.swap_points(i as PointId, j as PointId);
                inverse.swap(i, j);
            }
        }
    }

    /// Drop mesh and raster attachments. Used by in-place transforms whose
    /// historical (`make_new`-based) output carried neither, so the in-place
    /// path stays observably identical.
    pub fn clear_attachments(&mut self) {
        self.meshes.clear();
        self.rasters.clear();
    }

    /// Copy point row `src` onto row `dst` (data and source index). Used for
    /// in-place left-compaction by streaming filters: iterate points in order,
    /// copying each kept row down to the next write slot, then `truncate`.
    /// Returns false if either index is out of range.
    pub fn copy_point_within(&mut self, src: PointId, dst: PointId) -> bool {
        let n = self.len();
        if src >= n || dst >= n {
            return false;
        }
        if src == dst {
            return true;
        }
        let ps = self.layout.point_size();
        let s = (src as usize) * ps;
        let d = (dst as usize) * ps;
        self.data.copy_within(s..s + ps, d);
        self.source_indices[dst as usize] = self.source_indices[src as usize];
        true
    }

    /// Original source row copied into this point. Used by the C++ bridge to
    /// preserve PDAL PointView table IDs when filters return subsets.
    pub fn source_index(&self, idx: PointId) -> PointId {
        self.source_indices
            .get(idx as usize)
            .copied()
            .unwrap_or(idx)
    }

    /// Set the original source row for point `idx`.
    pub fn set_source_index(&mut self, idx: PointId, source: PointId) -> bool {
        let Some(slot) = self.source_indices.get_mut(idx as usize) else {
            return false;
        };
        *slot = source;
        true
    }

    /// Read a dimension of point `idx` as `f64` (PDAL's `getFieldAs`).
    /// Unregistered dimensions read as `0.0`.
    pub fn get_f64(&self, idx: PointId, dim: &DimId) -> f64 {
        match self.layout.dim(dim) {
            Some((off, ty)) => {
                let base = (idx as usize) * self.layout.point_size() + off;
                read_value(&self.data[base..base + ty.size()], ty)
            }
            None => 0.0,
        }
    }

    /// Write a dimension of point `idx` from an `f64` (PDAL's `setField`),
    /// converting to the dimension's storage type. Unregistered dimensions
    /// are ignored.
    pub fn set_f64(&mut self, idx: PointId, dim: &DimId, value: f64) {
        if let Some((off, ty)) = self.layout.dim(dim) {
            let base = (idx as usize) * self.layout.point_size() + off;
            write_value(&mut self.data[base..base + ty.size()], ty, value);
        }
    }

    /// Read a dimension of point `idx` as an exact `u64`.
    ///
    /// Unlike [`PointView::get_f64`], 64-bit integer dimensions (such as the
    /// uint64 `H3` index) are read from their raw storage, so values above
    /// `2^53` are preserved without `f64` rounding. Unregistered dimensions
    /// read as `0`.
    pub fn get_u64(&self, idx: PointId, dim: &DimId) -> u64 {
        match self.layout.dim(dim) {
            Some((off, ty)) => {
                let base = (idx as usize) * self.layout.point_size() + off;
                read_u64(&self.data[base..base + ty.size()], ty)
            }
            None => 0,
        }
    }

    /// Write a dimension of point `idx` from an exact `u64`.
    ///
    /// 64-bit integer dimensions store the value without an intermediate
    /// `f64` conversion, so large indexes are preserved exactly. Unregistered
    /// dimensions are ignored.
    pub fn set_u64(&mut self, idx: PointId, dim: &DimId, value: u64) {
        if let Some((off, ty)) = self.layout.dim(dim) {
            let base = (idx as usize) * self.layout.point_size() + off;
            write_u64(&mut self.data[base..base + ty.size()], ty, value);
        }
    }

    /// Checked variant of [`PointView::set_f64`]. Returns `false` if the point,
    /// dimension, or target type cannot accept the value.
    pub fn try_set_f64(&mut self, idx: PointId, dim: &DimId, value: f64) -> bool {
        let Some((off, ty)) = self.layout.dim(dim) else {
            return false;
        };
        if idx >= self.len() || !value_fits_type(value, ty) {
            return false;
        }

        let base = (idx as usize) * self.layout.point_size() + off;
        write_value(&mut self.data[base..base + ty.size()], ty, value);
        true
    }
}

#[path = "point/value_io.rs"]
mod value_io;
use value_io::{read_u64, read_value, value_fits_type, write_u64, write_value};

#[cfg(test)]
mod tests;
