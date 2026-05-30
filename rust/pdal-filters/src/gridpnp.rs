//! Grid-accelerated point-in-polygon test, ported from
//! `filters/private/pnp/GridPnp` (+ `Grid`, `VoxelRayTrace`, and the
//! `Comparison::closeEnough` ULP helper). Used by `filters.crop` for polygon
//! cropping: build once from a polygon's rings, then query `inside(x, y)`
//! per point in roughly O(1).
//!
//! The C++ implementation seeds per-cell reference points randomly to dodge
//! edge collinearity, but the `inside()` result is invariant to which valid
//! (non-collinear) reference point is chosen, so this port uses a small
//! deterministic generator and still matches the C++ output.

const MAX_ULPS: u64 = 4;

/// Sign-and-magnitude -> biased, matching gtest `FloatingPoint`.
fn sam_to_biased(sam: u64) -> u64 {
    const SIGN: u64 = 1u64 << 63;
    if sam & SIGN != 0 {
        (!sam).wrapping_add(1)
    } else {
        SIGN | sam
    }
}

/// `Comparison::closeEnough(double, double)` — within 4 ULPs (NaN never equal).
fn close_enough(d1: f64, d2: f64) -> bool {
    if d1.is_nan() || d2.is_nan() {
        return false;
    }
    let b1 = sam_to_biased(d1.to_bits());
    let b2 = sam_to_biased(d2.to_bits());
    let dist = if b1 >= b2 { b1 - b2 } else { b2 - b1 };
    dist <= MAX_ULPS
}

type Point = (f64, f64);

#[derive(Clone, Copy, PartialEq, Eq)]
enum IntersectType {
    Cross,
    On,
    None,
}

/// Amanatides & Woo voxel traversal: cells a segment passes through.
struct VoxelRayTrace {
    cell_width: f64,
    cell_height: f64,
    x_cell_origin: f64,
    y_cell_origin: f64,
    xstart: f64,
    ystart: f64,
    xend: f64,
    yend: f64,
    step_x: i32,
    step_y: i32,
    t_max_x: f64,
    t_max_y: f64,
    t_delta_x: f64,
    t_delta_y: f64,
}

impl VoxelRayTrace {
    #[allow(clippy::too_many_arguments)]
    fn new(
        cell_width: f64,
        cell_height: f64,
        x_cell_origin: f64,
        y_cell_origin: f64,
        xstart: f64,
        ystart: f64,
        xend: f64,
        yend: f64,
    ) -> Self {
        let mut v = VoxelRayTrace {
            cell_width,
            cell_height,
            x_cell_origin,
            y_cell_origin,
            xstart,
            ystart,
            xend,
            yend,
            step_x: 0,
            step_y: 0,
            t_max_x: 0.0,
            t_max_y: 0.0,
            t_delta_x: 0.0,
            t_delta_y: 0.0,
        };
        v.initialize();
        v
    }

    fn xcell(&self, xpos: f64) -> i32 {
        let p = (xpos - self.x_cell_origin) / self.cell_width;
        if close_enough(p, p.ceil()) {
            p.ceil() as i32
        } else {
            p.floor() as i32
        }
    }

    fn ycell(&self, ypos: f64) -> i32 {
        let p = (ypos - self.y_cell_origin) / self.cell_height;
        if close_enough(p, p.ceil()) {
            p.ceil() as i32
        } else {
            p.floor() as i32
        }
    }

    fn initialize(&mut self) {
        let xvec = self.xend - self.xstart;
        let yvec = self.yend - self.ystart;
        let grid_x = self.xcell(self.xstart);
        let grid_y = self.ycell(self.ystart);
        self.step_x = if xvec >= 0.0 { 1 } else { -1 };
        self.step_y = if yvec >= 0.0 { 1 } else { -1 };
        let grid_next_x = if self.step_x > 0 { grid_x + 1 } else { grid_x };
        let grid_next_y = if self.step_y > 0 { grid_y + 1 } else { grid_y };
        let x_next_cell = self.x_cell_origin + (grid_next_x as f64) * self.cell_width;
        let y_next_cell = self.y_cell_origin + (grid_next_y as f64) * self.cell_height;
        self.t_max_x = if xvec != 0.0 {
            (x_next_cell - self.xstart) / xvec
        } else {
            f64::MAX
        };
        self.t_max_y = if yvec != 0.0 {
            (y_next_cell - self.ystart) / yvec
        } else {
            f64::MAX
        };
        self.t_delta_x = if xvec != 0.0 {
            (self.cell_width / xvec).abs()
        } else {
            f64::MAX
        };
        self.t_delta_y = if yvec != 0.0 {
            (self.cell_height / yvec).abs()
        } else {
            f64::MAX
        };
    }

    fn emit(&mut self) -> Vec<(i32, i32)> {
        let mut cells = Vec::new();
        let mut grid_x = self.xcell(self.xstart);
        let mut grid_y = self.ycell(self.ystart);
        let xlast = self.xcell(self.xend);
        let ylast = self.ycell(self.yend);
        cells.push((grid_x, grid_y));
        while grid_x != xlast && grid_y != ylast {
            if self.t_max_x < self.t_max_y {
                self.t_max_x += self.t_delta_x;
                grid_x += self.step_x;
            } else {
                self.t_max_y += self.t_delta_y;
                grid_y += self.step_y;
            }
            cells.push((grid_x, grid_y));
        }
        while grid_x != xlast {
            grid_x += self.step_x;
            cells.push((grid_x, grid_y));
        }
        while grid_y != ylast {
            grid_y += self.step_y;
            cells.push((grid_x, grid_y));
        }
        cells
    }
}

#[derive(Default, Clone)]
struct Cell {
    edges: Vec<usize>,
    inside: bool,
    point: Option<Point>,
}

/// Grid-accelerated point-in-polygon engine.
pub struct GridPnp {
    rings: Vec<Point>,
    cells: Vec<Cell>,
    grid_w: usize,
    grid_h: usize,
    cell_width: f64,
    cell_height: f64,
    x_origin: f64,
    y_origin: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    rng: u64,
}

impl GridPnp {
    /// Build from an exterior ring and optional interior (hole) rings. Each
    /// ring is a closed list of (x, y) with the first point repeated last.
    /// Returns an error string for invalid rings (matching `grid_error`).
    pub fn new(outer: &[Point], inners: &[Vec<Point>]) -> Result<Self, String> {
        validate_ring(outer)?;
        for inner in inners {
            validate_ring(inner)?;
        }
        let mut g = GridPnp {
            rings: Vec::new(),
            cells: Vec::new(),
            grid_w: 0,
            grid_h: 0,
            cell_width: 0.0,
            cell_height: 0.0,
            x_origin: 0.0,
            y_origin: 0.0,
            x_min: 0.0,
            x_max: 0.0,
            y_min: 0.0,
            y_max: 0.0,
            rng: 0x9e3779b97f4a7c15,
        };
        g.calc_bounds(outer);
        g.fill_ring_list(outer, inners);
        g.setup_grid();
        Ok(g)
    }

    fn point1(&self, id: usize) -> Point {
        self.rings[id]
    }
    fn point2(&self, id: usize) -> Point {
        self.rings[id + 1]
    }

    fn calc_bounds(&mut self, outer: &[Point]) {
        let p = outer[0];
        self.x_min = p.0;
        self.x_max = p.0;
        self.y_min = p.1;
        self.y_max = p.1;
        for id in 0..outer.len() - 1 {
            let p1 = outer[id];
            self.x_min = self.x_min.min(p1.0);
            self.x_max = self.x_max.max(p1.0);
            self.y_min = self.y_min.min(p1.1);
            self.y_max = self.y_max.max(p1.1);
        }
    }

    fn fill_ring_list(&mut self, inner: &[Point], outers: &[Vec<Point>]) {
        let nan = f64::NAN;
        for &p in inner {
            self.rings.push(p);
        }
        for r in outers {
            self.rings.push((nan, nan));
            for &p in r {
                self.rings.push(p);
            }
        }
    }

    /// Iterate valid edge ids (skip NaN separators and zero-length edges),
    /// matching the C++ `EdgeIt`.
    fn edge_ids(&self) -> Vec<usize> {
        let n = self.rings.len();
        let mut ids = Vec::new();
        let mut id = 0usize;
        let valid = |i: usize| -> bool {
            !(self.rings[i].0.is_nan()
                || self.rings[i + 1].0.is_nan()
                || self.rings[i] == self.rings[i + 1])
        };
        while id < n - 1 {
            if valid(id) {
                ids.push(id);
            }
            id += 1;
        }
        ids
    }

    fn setup_grid(&mut self) {
        let (x_avg, y_avg) = self.calc_avg_edge_len();
        let (mx, my) = self.calc_grid_size(x_avg, y_avg);
        self.create_grid(mx, my);
        self.assign_edges();
        self.compute_all_cells();
    }

    fn calc_avg_edge_len(&self) -> (f64, f64) {
        let mut xdist = 0.0;
        let mut ydist = 0.0;
        let mut num = 0usize;
        for id in self.edge_ids() {
            let p1 = self.point1(id);
            let p2 = self.point2(id);
            xdist += (p2.0 - p1.0).abs();
            ydist += (p2.1 - p1.1).abs();
            num += 1;
        }
        (xdist / num as f64, ydist / num as f64)
    }

    fn calc_grid_size(&self, x_avg: f64, y_avg: f64) -> (usize, usize) {
        let m = std::cmp::max(1000usize, self.rings.len());
        let scalex = ((self.x_max - self.x_min) * y_avg) / ((self.y_max - self.y_min) * x_avg);
        let scaley = 1.0 / scalex;
        let mx = (m as f64 * scalex).sqrt() as usize;
        let my = (m as f64 * scaley).sqrt() as usize;
        (mx + 1, my + 1)
    }

    fn create_grid(&mut self, grid_w: usize, grid_h: usize) {
        let box_w = self.x_max - self.x_min;
        let box_h = self.y_max - self.y_min;
        self.cell_width = box_w / (grid_w - 1) as f64;
        self.cell_height = box_h / (grid_h - 1) as f64;
        self.x_origin = self.x_min - self.cell_width / 2.0;
        self.y_origin = self.y_min - self.cell_height / 2.0;
        self.grid_w = grid_w;
        self.grid_h = grid_h;
        self.cells = vec![Cell::default(); grid_w * grid_h];
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.grid_w + x
    }

    fn cell_origin(&self, x: usize, y: usize) -> Point {
        (
            self.x_origin + self.cell_width * x as f64,
            self.y_origin + self.cell_height * y as f64,
        )
    }

    fn cell_pos(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let xr = x - self.x_origin;
        let yr = y - self.y_origin;
        if xr < 0.0 || yr < 0.0 {
            return None;
        }
        let px = (xr / self.cell_width) as usize;
        let py = (yr / self.cell_height) as usize;
        if px < self.grid_w && py < self.grid_h {
            Some((px, py))
        } else {
            None
        }
    }

    fn assign_edges(&mut self) {
        for id in self.edge_ids() {
            let p1 = self.point1(id);
            let p2 = self.point2(id);
            let mut vrt = VoxelRayTrace::new(
                self.cell_width,
                self.cell_height,
                self.x_origin,
                self.y_origin,
                p1.0,
                p1.1,
                p2.0,
                p2.1,
            );
            for c in vrt.emit() {
                // Cells from emit are within grid bounds for polygon edges.
                if c.0 >= 0 && c.1 >= 0 && (c.0 as usize) < self.grid_w && (c.1 as usize) < self.grid_h
                {
                    let i = self.idx(c.0 as usize, c.1 as usize);
                    self.cells[i].edges.push(id);
                }
            }
        }
    }

    fn point_collinear(&self, x: f64, y: f64, edge_id: usize) -> bool {
        let p1 = self.point1(edge_id);
        let p2 = self.point2(edge_id);
        close_enough((x - p2.0) * (y - p1.1), (y - p2.1) * (x - p1.0))
    }

    /// Small deterministic LCG in [0, 1).
    fn next_rand(&mut self) -> f64 {
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        // top 53 bits -> [0,1)
        ((self.rng >> 11) as f64) / ((1u64 << 53) as f64)
    }

    fn generate_ref_point(&mut self, x: usize, y: usize) -> Point {
        let origin = self.cell_origin(x, y);
        let edges: Vec<usize> = self.cells[self.idx(x, y)].edges.clone();
        loop {
            let px = origin.0 + self.next_rand() * self.cell_width;
            let py = origin.1 + self.next_rand() * self.cell_height;
            if edges.iter().all(|&e| !self.point_collinear(px, py, e)) {
                return (px, py);
            }
        }
    }

    fn intersects(&self, e1: (Point, Point), e2: (Point, Point)) -> IntersectType {
        let p = e1.0;
        let r = (e1.1 .0 - e1.0 .0, e1.1 .1 - e1.0 .1);
        let q = e2.0;
        let s = (e2.1 .0 - e2.0 .0, e2.1 .1 - e2.0 .1);
        let r_cross_s = r.0 * s.1 - r.1 * s.0;
        let pq = (q.0 - p.0, q.1 - p.1);
        let pq_cross_s = pq.0 * s.1 - pq.1 * s.0;
        let t = pq_cross_s / r_cross_s;
        let t_close = close_enough(t, 0.0) || close_enough(t, 1.0);
        let intersect = t_close || (t > 0.0 && t < 1.0);
        if !intersect {
            return IntersectType::None;
        }
        let pq_cross_r = pq.0 * r.1 - pq.1 * r.0;
        let u = pq_cross_r / r_cross_s;
        let u_close = close_enough(u, 0.0) || close_enough(u, 1.0);
        let intersect = u_close || (u > 0.0 && u < 1.0);
        if intersect {
            if u_close || t_close {
                IntersectType::On
            } else {
                IntersectType::Cross
            }
        } else {
            IntersectType::None
        }
    }

    fn intersections(&self, e1: (Point, Point), edges: &[usize]) -> usize {
        let mut isect = 0;
        for &edge_id in edges {
            let e2 = (self.point1(edge_id), self.point2(edge_id));
            if self.intersects(e1, e2) != IntersectType::None {
                isect += 1;
            }
        }
        isect
    }

    /// Precompute every cell's reference point and inside/outside status,
    /// column-major (x ascending) so each cell's left neighbor is ready.
    fn compute_all_cells(&mut self) {
        for y in 0..self.grid_h {
            for x in 0..self.grid_w {
                self.compute_cell(x, y);
            }
        }
    }

    fn compute_cell(&mut self, x: usize, y: usize) {
        let i = self.idx(x, y);
        if self.cells[i].point.is_some() {
            return;
        }
        let refpt = self.generate_ref_point(x, y);
        self.cells[i].point = Some(refpt);
        self.determine_point_status(x, y);
    }

    fn determine_point_status(&mut self, x: usize, y: usize) {
        let here = self.idx(x, y);
        let p1 = self.cells[here].point.unwrap();
        let edges_here = self.cells[here].edges.clone();
        let intersect_count;
        if x == 0 {
            let x2 = p1.0 - self.cell_width;
            let edge = (p1, (x2, p1.1));
            intersect_count = self.intersections(edge, &edges_here);
        } else {
            let previ = self.idx(x - 1, y);
            if self.cells[previ].point.is_none() {
                self.compute_cell(x - 1, y);
            }
            let prev = &self.cells[previ];
            let prev_pt = prev.point.unwrap();
            let prev_inside = prev.inside;
            let mut edge_set: Vec<usize> = edges_here.clone();
            edge_set.extend(prev.edges.iter().copied());
            edge_set.sort_unstable();
            edge_set.dedup();
            let edge = (p1, prev_pt);
            let mut count = self.intersections(edge, &edge_set);
            if prev_inside {
                count += 1;
            }
            intersect_count = count;
        }
        self.cells[here].inside = intersect_count % 2 == 1;
    }

    fn test_cell(&self, x: usize, y: usize, qx: f64, qy: f64) -> bool {
        let cell = &self.cells[self.idx(x, y)];
        let refpt = cell.point.unwrap();
        let tester = ((qx, qy), refpt);
        let mut inside = cell.inside;
        for &edge_idx in &cell.edges {
            let other = (self.point1(edge_idx), self.point2(edge_idx));
            match self.intersects(tester, other) {
                IntersectType::On => return true,
                IntersectType::Cross => inside = !inside,
                IntersectType::None => {}
            }
        }
        inside
    }

    /// Point-in-polygon test (true if inside, with edge-on counted as inside).
    pub fn inside(&self, x: f64, y: f64) -> bool {
        let Some((cx, cy)) = self.cell_pos(x, y) else {
            return false;
        };
        let cell = &self.cells[self.idx(cx, cy)];
        if cell.edges.is_empty() {
            return cell.inside;
        }
        self.test_cell(cx, cy, x, y)
    }
}

fn validate_ring(r: &[Point]) -> Result<(), String> {
    if r.len() < 4 {
        return Err("Invalid ring. Ring must consist of at least  four points.".to_string());
    }
    if r[0] != r[r.len() - 1] {
        return Err("Invalid ring. First point is not equal to the last point.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Point> {
        vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0), (0.0, 0.0)]
    }

    #[test]
    fn close_enough_basic() {
        assert!(close_enough(1.0, 1.0));
        assert!(!close_enough(1.0, 1.001));
        assert!(!close_enough(0.0, f64::NAN));
    }

    #[test]
    fn inside_outside_square() {
        let g = GridPnp::new(&square(), &[]).unwrap();
        assert!(g.inside(5.0, 5.0));
        assert!(!g.inside(-1.0, 5.0));
        assert!(!g.inside(11.0, 5.0));
        assert!(!g.inside(5.0, 11.0));
        assert!(g.inside(0.01, 0.01));
        assert!(g.inside(9.99, 9.99));
    }

    #[test]
    fn square_with_hole() {
        let outer = square();
        let hole = vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0), (3.0, 3.0)];
        let g = GridPnp::new(&outer, &[hole]).unwrap();
        assert!(g.inside(1.0, 1.0)); // in outer, outside hole
        assert!(!g.inside(5.0, 5.0)); // in hole
    }

    #[test]
    fn invalid_ring_errors() {
        assert!(GridPnp::new(&[(0.0, 0.0), (1.0, 0.0)], &[]).is_err());
        assert!(GridPnp::new(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (9.0, 9.0)], &[]).is_err());
    }
}
