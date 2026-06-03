use super::*;

pub(super) enum QueryBounds {
    Two(Bounds2D),
    Three(Bounds3D),
}

pub(super) struct BoundsFilter {
    pub(super) query: QueryBounds,
    pub(super) transform: Option<GdalSrsTransform>,
    conservative_hierarchy_overlap: bool,
}

impl BoundsFilter {
    pub(super) fn new(
        query: QueryBounds,
        target_srs: &str,
        info: &Value,
    ) -> Result<Self, StageError> {
        let mut conservative_hierarchy_overlap = false;
        let transform = if target_srs.is_empty() {
            None
        } else {
            let source_srs = info["srs"]["wkt"].as_str().unwrap_or("");
            let source = user_input_to_wkt(source_srs).map_err(StageError)?;
            let target = user_input_to_wkt(target_srs).map_err(StageError)?;
            conservative_hierarchy_overlap =
                pdal_native::srs::is_geocentric(&source.wkt, source.epoch)
                    && pdal_native::srs::is_geographic(&target.wkt, target.epoch);
            Some(
                GdalSrsTransform::new(
                    &source.wkt2,
                    source.epoch,
                    &target.wkt2,
                    target.epoch,
                    &[],
                    &[],
                )
                .map_err(StageError)?,
            )
        };
        Ok(Self {
            query,
            transform,
            conservative_hierarchy_overlap,
        })
    }

    fn contains(&self, view: &PointView, idx: PointId) -> bool {
        let mut x = view.get_f64(idx, &DimId::X);
        let mut y = view.get_f64(idx, &DimId::Y);
        let mut z = view.get_f64(idx, &DimId::Z);
        if let Some(transform) = &self.transform {
            if !transform.transform_xyz(&mut x, &mut y, &mut z) {
                return false;
            }
        }
        self.query.contains_point(x, y, z)
    }

    pub(super) fn overlaps_box(&self, bounds: &Bounds3D) -> bool {
        if self.conservative_hierarchy_overlap {
            return true;
        }
        match &self.transform {
            None => self.query.overlaps_box(bounds),
            Some(transform) => transform_bounds_via_corners(bounds, transform)
                .is_some_and(|bounds| self.query.overlaps_box(&bounds)),
        }
    }

    pub(super) fn preview_clip_bounds(&self) -> Option<Bounds3D> {
        if self.transform.is_some() {
            return None;
        }
        Some(self.query.to_bounds3d())
    }
}

pub(super) fn transform_bounds_via_corners(
    bounds: &Bounds3D,
    transform: &GdalSrsTransform,
) -> Option<Bounds3D> {
    let mut output = Bounds3D::empty();
    for x in [bounds.minx, bounds.maxx] {
        for y in [bounds.miny, bounds.maxy] {
            for z in [bounds.minz, bounds.maxz] {
                let mut x = x;
                let mut y = y;
                let mut z = z;
                if !transform.transform_xyz(&mut x, &mut y, &mut z) {
                    return None;
                }
                output.grow_point(x, y, z);
            }
        }
    }
    Some(output)
}

impl QueryBounds {
    fn contains_point(&self, x: f64, y: f64, z: f64) -> bool {
        match self {
            QueryBounds::Two(bounds) => bounds.contains_point(x, y),
            QueryBounds::Three(bounds) => bounds.contains_point(x, y, z),
        }
    }

    pub(super) fn overlaps_box(&self, bounds: &Bounds3D) -> bool {
        match self {
            QueryBounds::Two(query) => query.overlaps(&Bounds2D {
                minx: bounds.minx,
                maxx: bounds.maxx,
                miny: bounds.miny,
                maxy: bounds.maxy,
            }),
            QueryBounds::Three(query) => query.overlaps(bounds),
        }
    }

    fn to_bounds3d(&self) -> Bounds3D {
        match self {
            QueryBounds::Two(bounds) => Bounds3D {
                minx: bounds.minx,
                maxx: bounds.maxx,
                miny: bounds.miny,
                maxy: bounds.maxy,
                minz: -f64::MAX,
                maxz: f64::MAX,
            },
            QueryBounds::Three(bounds) => *bounds,
        }
    }
}

pub(super) fn parsed_bounds_srs(input: &str, pos: usize) -> String {
    input
        .get(pos..)
        .unwrap_or("")
        .trim()
        .strip_prefix('/')
        .map(str::trim)
        .unwrap_or("")
        .to_string()
}

pub(super) fn apply_bounds(view: PointView, bounds: Option<&BoundsFilter>) -> PointView {
    let Some(bounds) = bounds else {
        return view;
    };
    let mut output = view.make_new();
    for idx in 0..view.len() {
        if bounds.contains(&view, idx) {
            output.append_point(&view, idx);
        }
    }
    output
}

pub(super) struct PolygonFilter {
    pub(super) geometry: Geometry,
    pub(super) transform: Option<GdalSrsTransform>,
}

pub(super) fn polygon_transform(
    source_srs: &str,
    polygon_srs: &str,
) -> Result<Option<GdalSrsTransform>, StageError> {
    if source_srs.trim().is_empty() || polygon_srs.trim().is_empty() {
        return Ok(None);
    }
    let source = user_input_to_wkt(source_srs).map_err(StageError)?;
    let target = user_input_to_wkt(polygon_srs).map_err(StageError)?;
    if source.wkt == target.wkt {
        return Ok(None);
    }
    Ok(Some(
        GdalSrsTransform::new(
            &source.wkt2,
            source.epoch,
            &target.wkt2,
            target.epoch,
            &[],
            &[],
        )
        .map_err(StageError)?,
    ))
}

pub(super) fn apply_polygons(view: PointView, polygons: &[PolygonFilter]) -> PointView {
    if polygons.is_empty() {
        return view;
    }
    let mut output = view.make_new();
    for idx in 0..view.len() {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        if polygons
            .iter()
            .any(|polygon| polygon_contains(polygon, x, y))
        {
            output.append_point(&view, idx);
        }
    }
    output
}

fn polygon_contains(polygon: &PolygonFilter, mut x: f64, mut y: f64) -> bool {
    let mut z = 0.0;
    if let Some(transform) = &polygon.transform {
        if !transform.transform_xyz(&mut x, &mut y, &mut z) {
            return false;
        }
    }
    polygon.geometry.contains(x, y)
}

pub(super) fn apply_origin(view: PointView, origin: Option<u64>) -> PointView {
    let Some(origin) = origin else {
        return view;
    };
    let origin_dim = DimId::from_name("OriginId");
    let mut output = view.make_new();
    for idx in 0..view.len() {
        if view.get_f64(idx, &origin_dim) as u64 == origin {
            output.append_point(&view, idx);
        }
    }
    output
}
