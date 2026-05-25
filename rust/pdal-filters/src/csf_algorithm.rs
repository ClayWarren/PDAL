//! Cloth Simulation Filter (Zhang et al. 2016).
//!
//! Pure-Rust port of `filters/private/csf/` -- the cloth-simulation ground
//! classifier used by `filters.csf`. Mirrors the C++ algorithm:
//!
//! 1. Invert points: store `(x, -z, y)` so cloth falls under gravity onto the
//!    underside-projected terrain.
//! 2. Build a flat cloth grid above the inverted point cloud and rasterize
//!    each cloth particle's nearest point height.
//! 3. Apply Verlet integration with gravity, satisfy distance constraints
//!    between particles, and clamp particles to the terrain height when they
//!    would penetrate it (terr-collision).
//! 4. Optional slope post-processing snaps neighboring cloth particles to the
//!    terrain when the height difference is small enough.
//! 5. Classify each original point as ground/non-ground by the vertical
//!    distance between the point and the bilinearly-interpolated cloth.
//!
//! Output indices are 0-based into the original point input.

use std::collections::VecDeque;

const DAMPING: f64 = 0.01;
const MIN_INF: f64 = -9_999_999_999.0;
const GRAVITY: f64 = 0.2;
const MAX_PARTICLES_FOR_POSTPROCESSING: usize = 50;

/// Spring-constraint correction factors when both endpoints are movable.
const DOUBLE_MOVE: [f64; 15] = [
    0.0, 0.3, 0.42, 0.468, 0.4872, 0.4949, 0.498, 0.4992, 0.4997, 0.4999, 0.4999, 0.5, 0.5, 0.5,
    0.5,
];
/// Spring-constraint correction factor when only one endpoint is movable.
const SINGLE_MOVE: [f64; 15] = [
    0.0, 0.3, 0.51, 0.657, 0.7599, 0.83193, 0.88235, 0.91765, 0.94235, 0.95965, 0.97175, 0.98023,
    0.98616, 0.99031, 0.99322,
];

#[derive(Clone, Copy, Debug)]
pub struct CsfParams {
    pub smooth: bool,
    pub time_step: f64,
    pub class_threshold: f64,
    pub height_threshold: f64,
    pub cloth_resolution: f64,
    pub rigidness: i32,
    pub iterations: i32,
}

impl Default for CsfParams {
    fn default() -> Self {
        Self {
            smooth: true,
            time_step: 0.65,
            class_threshold: 0.5,
            height_threshold: 0.3,
            cloth_resolution: 1.0,
            rigidness: 3,
            iterations: 500,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CsfPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Result of CSF classification: indices into the original input slice.
pub struct CsfResult {
    pub ground_indices: Vec<usize>,
    pub off_ground_indices: Vec<usize>,
}

/// Apply CSF to `xyz_points` (in original coordinate order). Returns
/// indices classified as ground/non-ground.
pub fn classify_ground(xyz_points: &[CsfPoint], params: &CsfParams) -> CsfResult {
    if xyz_points.is_empty() {
        return CsfResult {
            ground_indices: Vec::new(),
            off_ground_indices: Vec::new(),
        };
    }

    // CSF inverts the world: (x, -z, y). The cloth falls in the +y direction
    // of the simulated frame, which corresponds to -z in the original world.
    let inverted: Vec<CsfPoint> = xyz_points
        .iter()
        .map(|p| CsfPoint {
            x: p.x,
            y: -p.z,
            z: p.y,
        })
        .collect();

    let (bb_min, bb_max) = bounding_box(&inverted);

    // Cloth setup matches CSF::do_filtering exactly.
    let cloth_buffer_d: i32 = 2;
    let cloth_y_height = 0.05;
    let origin = [
        bb_min.x - (cloth_buffer_d as f64) * params.cloth_resolution,
        bb_max.y + cloth_y_height,
        bb_min.z - (cloth_buffer_d as f64) * params.cloth_resolution,
    ];
    let width_num =
        ((bb_max.x - bb_min.x) / params.cloth_resolution).floor() as i32 + 2 * cloth_buffer_d;
    let height_num =
        ((bb_max.z - bb_min.z) / params.cloth_resolution).floor() as i32 + 2 * cloth_buffer_d;
    let width_num = width_num.max(1) as usize;
    let height_num = height_num.max(1) as usize;

    let mut cloth = Cloth::new(
        origin,
        width_num,
        height_num,
        params.cloth_resolution,
        params.cloth_resolution,
        params.height_threshold,
        params.rigidness,
        params.time_step,
    );

    rasterize_terrain(&inverted, &mut cloth);

    let time_step_sq = params.time_step * params.time_step;
    cloth.add_force([0.0, -GRAVITY, 0.0], time_step_sq);

    for _ in 0..params.iterations {
        let max_diff = cloth.time_step();
        cloth.terr_collision();
        if max_diff != 0.0 && max_diff < 0.005 {
            break;
        }
    }

    if params.smooth {
        cloth.movable_filter();
    }

    classify_points(&inverted, &cloth, params.class_threshold)
}

#[derive(Clone, Copy)]
struct BBox {
    x: f64,
    y: f64,
    z: f64,
}

fn bounding_box(points: &[CsfPoint]) -> (BBox, BBox) {
    let mut min = BBox {
        x: points[0].x,
        y: points[0].y,
        z: points[0].z,
    };
    let mut max = min;
    for p in &points[1..] {
        if p.x < min.x {
            min.x = p.x;
        }
        if p.y < min.y {
            min.y = p.y;
        }
        if p.z < min.z {
            min.z = p.z;
        }
        if p.x > max.x {
            max.x = p.x;
        }
        if p.y > max.y {
            max.y = p.y;
        }
        if p.z > max.z {
            max.z = p.z;
        }
    }
    (min, max)
}

#[derive(Clone)]
struct Particle {
    pos: [f64; 3],
    old_pos: [f64; 3],
    acceleration: [f64; 3],
    movable: bool,
    is_visited: bool,
    c_pos: i32,
    pos_x: i32,
    pos_y: i32,
    nearest_point_height: f64,
    tmp_dist: f64,
    /// Indices into the cloth grid (row * width + col) of immediate + secondary
    /// constraint neighbours. Built once at cloth construction.
    neighbors: Vec<usize>,
}

impl Particle {
    fn new(pos: [f64; 3]) -> Self {
        Self {
            pos,
            old_pos: pos,
            acceleration: [0.0; 3],
            movable: true,
            is_visited: false,
            c_pos: 0,
            pos_x: 0,
            pos_y: 0,
            nearest_point_height: MIN_INF,
            tmp_dist: f64::INFINITY,
            neighbors: Vec::new(),
        }
    }

    fn add_force(&mut self, f: [f64; 3]) {
        self.acceleration[0] += f[0];
        self.acceleration[1] += f[1];
        self.acceleration[2] += f[2];
    }

    fn time_step(&mut self, time_step_sq: f64) {
        if !self.movable {
            return;
        }
        let new_pos = [
            self.pos[0]
                + (self.pos[0] - self.old_pos[0]) * (1.0 - DAMPING)
                + self.acceleration[0] * time_step_sq,
            self.pos[1]
                + (self.pos[1] - self.old_pos[1]) * (1.0 - DAMPING)
                + self.acceleration[1] * time_step_sq,
            self.pos[2]
                + (self.pos[2] - self.old_pos[2]) * (1.0 - DAMPING)
                + self.acceleration[2] * time_step_sq,
        ];
        self.old_pos = self.pos;
        self.pos = new_pos;
    }

    fn offset_y(&mut self, dy: f64) {
        if self.movable {
            self.pos[1] += dy;
        }
    }
}

struct Cloth {
    width: usize,
    height: usize,
    step_x: f64,
    step_y: f64,
    origin: [f64; 3],
    particles: Vec<Particle>,
    height_vals: Vec<f64>,
    constraint_iterations: i32,
    smooth_threshold: f64,
    height_threshold: f64,
}

impl Cloth {
    #[allow(clippy::too_many_arguments)]
    fn new(
        origin: [f64; 3],
        width: usize,
        height: usize,
        step_x: f64,
        step_y: f64,
        height_threshold: f64,
        rigidness: i32,
        _time_step: f64,
    ) -> Self {
        let mut particles = Vec::with_capacity(width * height);
        for j in 0..height {
            for i in 0..width {
                let mut p = Particle::new([
                    origin[0] + (i as f64) * step_x,
                    origin[1],
                    origin[2] + (j as f64) * step_y,
                ]);
                p.pos_x = i as i32;
                p.pos_y = j as i32;
                particles.push(p);
            }
        }

        let neighbor_pairs = build_neighbor_pairs(width, height);
        for (a, b) in neighbor_pairs {
            particles[a].neighbors.push(b);
            particles[b].neighbors.push(a);
        }

        Self {
            width,
            height,
            step_x,
            step_y,
            origin,
            particles,
            height_vals: vec![0.0; width * height],
            // CSF.h's `smoothThreshold` is sourced from the `height_threshold`
            // option (the C++ Cloth constructor receives 9999 for the
            // height_threshold argument and `height_threshold` for the
            // smoothThreshold argument). See CSF.cpp:150.
            smooth_threshold: height_threshold,
            height_threshold: 9999.0,
            constraint_iterations: rigidness,
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn add_force(&mut self, direction: [f64; 3], time_step_sq: f64) {
        let scaled = [
            direction[0] * time_step_sq,
            direction[1] * time_step_sq,
            direction[2] * time_step_sq,
        ];
        for p in &mut self.particles {
            p.add_force(scaled);
        }
    }

    /// One Verlet step plus all spring-constraint passes. Returns the largest
    /// per-particle Y displacement (used as a convergence signal).
    fn time_step(&mut self) -> f64 {
        let time_step_sq = 1.0; // forces were pre-scaled in add_force
        for p in &mut self.particles {
            p.time_step(time_step_sq);
        }
        // Spring constraints: each particle nudges its neighbors toward
        // matching Y heights. The correction-factor table indexes by rigidness
        // (or its loop counter, in the original code) and the table is small
        // enough that we re-run constraint_iterations passes per particle.
        let particle_count = self.particles.len();
        let constraint_iters = self.constraint_iterations;
        for i in 0..particle_count {
            self.satisfy_constraints_for(i, constraint_iters);
        }
        let mut max_diff = 0.0_f64;
        for p in &self.particles {
            if p.movable {
                let diff = (p.old_pos[1] - p.pos[1]).abs();
                if diff > max_diff {
                    max_diff = diff;
                }
            }
        }
        max_diff
    }

    fn satisfy_constraints_for(&mut self, idx: usize, constraint_times: i32) {
        // Snapshot neighbor indices to avoid borrowing through the loop.
        let neighbor_indices = self.particles[idx].neighbors.clone();
        for n in neighbor_indices {
            let p1_y = self.particles[idx].pos[1];
            let p2_y = self.particles[n].pos[1];
            let correction_y = p2_y - p1_y;
            let p1_movable = self.particles[idx].movable;
            let p2_movable = self.particles[n].movable;
            if p1_movable && p2_movable {
                let factor = if constraint_times > 14 {
                    0.5
                } else {
                    DOUBLE_MOVE[constraint_times as usize]
                };
                let dy = correction_y * factor;
                self.particles[idx].offset_y(dy);
                self.particles[n].offset_y(-dy);
            } else if p1_movable && !p2_movable {
                let factor = if constraint_times > 14 {
                    1.0
                } else {
                    SINGLE_MOVE[constraint_times as usize]
                };
                self.particles[idx].offset_y(correction_y * factor);
            } else if !p1_movable && p2_movable {
                let factor = if constraint_times > 14 {
                    1.0
                } else {
                    SINGLE_MOVE[constraint_times as usize]
                };
                self.particles[n].offset_y(-correction_y * factor);
            }
        }
    }

    fn terr_collision(&mut self) {
        for (i, p) in self.particles.iter_mut().enumerate() {
            let v_y = p.pos[1];
            if v_y < self.height_vals[i] {
                p.offset_y(self.height_vals[i] - v_y);
                p.movable = false;
            }
        }
    }

    fn movable_filter(&mut self) {
        // Reset visit flags.
        for p in &mut self.particles {
            p.is_visited = false;
        }
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                if !self.particles[idx].movable || self.particles[idx].is_visited {
                    continue;
                }
                let (connected, neibors) = self.bfs_movable_component(x, y);
                if connected.len() > MAX_PARTICLES_FOR_POSTPROCESSING {
                    let edge_points = self.find_unmovable_points(&connected);
                    self.handle_slope_connected(&edge_points, &connected, &neibors);
                }
            }
        }
    }

    fn bfs_movable_component(&mut self, x0: usize, y0: usize) -> (Vec<(i32, i32)>, Vec<Vec<i32>>) {
        let mut connected: Vec<(i32, i32)> = Vec::new();
        let mut neibors: Vec<Vec<i32>> = Vec::new();
        let mut queue: VecDeque<usize> = VecDeque::new();

        connected.push((x0 as i32, y0 as i32));
        let init_idx = self.idx(x0, y0);
        self.particles[init_idx].is_visited = true;
        queue.push_back(init_idx);
        let mut sum = 1i32;
        while let Some(curr) = queue.pop_front() {
            let cur_x = self.particles[curr].pos_x;
            let cur_y = self.particles[curr].pos_y;
            let mut neibor: Vec<i32> = Vec::new();

            let candidates: [(i32, i32); 4] = [
                (cur_x - 1, cur_y),
                (cur_x + 1, cur_y),
                (cur_x, cur_y - 1),
                (cur_x, cur_y + 1),
            ];
            for (nx, ny) in candidates {
                if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
                    continue;
                }
                let nidx = self.idx(nx as usize, ny as usize);
                if !self.particles[nidx].movable {
                    continue;
                }
                if !self.particles[nidx].is_visited {
                    sum += 1;
                    self.particles[nidx].is_visited = true;
                    connected.push((nx, ny));
                    queue.push_back(nidx);
                    neibor.push(sum - 1);
                    self.particles[nidx].c_pos = sum - 1;
                } else {
                    neibor.push(self.particles[nidx].c_pos);
                }
            }
            neibors.push(neibor);
        }
        (connected, neibors)
    }

    fn find_unmovable_points(&mut self, connected: &[(i32, i32)]) -> Vec<usize> {
        let mut edge_points = Vec::new();
        for (i, (x, y)) in connected.iter().enumerate() {
            let x = *x;
            let y = *y;
            let index = self.idx(x as usize, y as usize);

            let candidates: [(i32, i32); 4] = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
            for (nx, ny) in candidates {
                if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
                    continue;
                }
                let nidx = self.idx(nx as usize, ny as usize);
                if self.particles[nidx].movable {
                    continue;
                }
                if (self.height_vals[index] - self.height_vals[nidx]).abs() < self.smooth_threshold
                    && self.particles[index].pos[1] - self.height_vals[index]
                        < self.height_threshold
                {
                    let dy = self.height_vals[index] - self.particles[index].pos[1];
                    self.particles[index].offset_y(dy);
                    self.particles[index].movable = false;
                    edge_points.push(i);
                    break;
                }
            }
        }
        edge_points
    }

    fn handle_slope_connected(
        &mut self,
        edge_points: &[usize],
        connected: &[(i32, i32)],
        neibors: &[Vec<i32>],
    ) {
        let mut visited = vec![false; connected.len()];
        let mut queue: VecDeque<usize> = VecDeque::new();
        for &e in edge_points {
            queue.push_back(e);
            visited[e] = true;
        }
        while let Some(idx) = queue.pop_front() {
            let (cx, cy) = connected[idx];
            let index_center = self.idx(cx as usize, cy as usize);
            for &n in &neibors[idx] {
                let n_usize = n as usize;
                let (nx, ny) = connected[n_usize];
                let index_neibor = self.idx(nx as usize, ny as usize);
                let center_h = self.height_vals[index_center];
                let neibor_h = self.height_vals[index_neibor];
                if (center_h - neibor_h).abs() < self.smooth_threshold
                    && (self.particles[index_neibor].pos[1] - neibor_h).abs()
                        < self.height_threshold
                {
                    let dy = neibor_h - self.particles[index_neibor].pos[1];
                    self.particles[index_neibor].offset_y(dy);
                    self.particles[index_neibor].movable = false;
                    if !visited[n_usize] {
                        queue.push_back(n_usize);
                        visited[n_usize] = true;
                    }
                }
            }
        }
    }
}

fn build_neighbor_pairs(width: usize, height: usize) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let idx = |x: usize, y: usize| y * width + x;
    // Immediate neighbours.
    for x in 0..width {
        for y in 0..height {
            if x + 1 < width {
                pairs.push((idx(x, y), idx(x + 1, y)));
            }
            if y + 1 < height {
                pairs.push((idx(x, y), idx(x, y + 1)));
            }
            if x + 1 < width && y + 1 < height {
                pairs.push((idx(x, y), idx(x + 1, y + 1)));
            }
            if x + 1 < width && y + 1 < height {
                pairs.push((idx(x + 1, y), idx(x, y + 1)));
            }
        }
    }
    // Secondary neighbours (distance 2).
    for x in 0..width {
        for y in 0..height {
            if x + 2 < width {
                pairs.push((idx(x, y), idx(x + 2, y)));
            }
            if y + 2 < height {
                pairs.push((idx(x, y), idx(x, y + 2)));
            }
            if x + 2 < width && y + 2 < height {
                pairs.push((idx(x, y), idx(x + 2, y + 2)));
            }
            if x + 2 < width && y + 2 < height {
                pairs.push((idx(x + 2, y), idx(x, y + 2)));
            }
        }
    }
    pairs
}

fn rasterize_terrain(points: &[CsfPoint], cloth: &mut Cloth) {
    for (i, p) in points.iter().enumerate() {
        let delta_x = p.x - cloth.origin[0];
        let delta_z = p.z - cloth.origin[2];
        let col = (delta_x / cloth.step_x + 0.5).floor() as i32;
        let row = (delta_z / cloth.step_y + 0.5).floor() as i32;
        if col < 0 || row < 0 || col >= cloth.width as i32 || row >= cloth.height as i32 {
            continue;
        }
        let idx = cloth.idx(col as usize, row as usize);
        let dx = p.x - cloth.particles[idx].pos[0];
        let dz = p.z - cloth.particles[idx].pos[2];
        let dist_sq = dx * dx + dz * dz;
        if dist_sq < cloth.particles[idx].tmp_dist {
            cloth.particles[idx].tmp_dist = dist_sq;
            cloth.particles[idx].nearest_point_height = p.y;
        }
        let _ = i;
    }
    // Fill in heightvals: nearest rasterized height, falling back to a scan-
    // line search then a BFS through the cloth neighbours when a particle
    // didn't catch any input points.
    for i in 0..cloth.particles.len() {
        let raster = cloth.particles[i].nearest_point_height;
        cloth.height_vals[i] = if raster > MIN_INF {
            raster
        } else {
            find_height_by_scanline(cloth, i)
        };
    }
}

fn find_height_by_scanline(cloth: &Cloth, idx: usize) -> f64 {
    let xpos = cloth.particles[idx].pos_x;
    let ypos = cloth.particles[idx].pos_y;
    for i in (xpos + 1)..(cloth.width as i32) {
        let h = cloth.particles[cloth.idx(i as usize, ypos as usize)].nearest_point_height;
        if h > MIN_INF {
            return h;
        }
    }
    for i in (0..xpos).rev() {
        let h = cloth.particles[cloth.idx(i as usize, ypos as usize)].nearest_point_height;
        if h > MIN_INF {
            return h;
        }
    }
    for j in (0..ypos).rev() {
        let h = cloth.particles[cloth.idx(xpos as usize, j as usize)].nearest_point_height;
        if h > MIN_INF {
            return h;
        }
    }
    for j in (ypos + 1)..(cloth.height as i32) {
        let h = cloth.particles[cloth.idx(xpos as usize, j as usize)].nearest_point_height;
        if h > MIN_INF {
            return h;
        }
    }
    find_height_by_neighbor(cloth, idx)
}

fn find_height_by_neighbor(cloth: &Cloth, start: usize) -> f64 {
    let mut visited = vec![false; cloth.particles.len()];
    let mut queue: VecDeque<usize> = VecDeque::new();
    visited[start] = true;
    for &n in &cloth.particles[start].neighbors {
        if !visited[n] {
            visited[n] = true;
            queue.push_back(n);
        }
    }
    while let Some(n) = queue.pop_front() {
        if cloth.particles[n].nearest_point_height > MIN_INF {
            return cloth.particles[n].nearest_point_height;
        }
        for &nn in &cloth.particles[n].neighbors {
            if !visited[nn] {
                visited[nn] = true;
                queue.push_back(nn);
            }
        }
    }
    MIN_INF
}

fn classify_points(points: &[CsfPoint], cloth: &Cloth, class_threshold: f64) -> CsfResult {
    let mut ground = Vec::new();
    let mut off_ground = Vec::new();
    for (i, p) in points.iter().enumerate() {
        let delta_x = p.x - cloth.origin[0];
        let delta_z = p.z - cloth.origin[2];
        let col0 = (delta_x / cloth.step_x).floor() as i32;
        let row0 = (delta_z / cloth.step_y).floor() as i32;
        let col1 = col0 + 1;
        let row1 = row0;
        let col2 = col0 + 1;
        let row2 = row0 + 1;
        let col3 = col0;
        let row3 = row0 + 1;
        let in_bounds =
            col0 >= 0 && row0 >= 0 && col2 < cloth.width as i32 && row2 < cloth.height as i32;
        if !in_bounds {
            off_ground.push(i);
            continue;
        }
        let sub_dx = (delta_x - (col0 as f64) * cloth.step_x) / cloth.step_x;
        let sub_dz = (delta_z - (row0 as f64) * cloth.step_y) / cloth.step_y;
        let h00 = cloth.particles[cloth.idx(col0 as usize, row0 as usize)].pos[1];
        let h33 = cloth.particles[cloth.idx(col3 as usize, row3 as usize)].pos[1];
        let h22 = cloth.particles[cloth.idx(col2 as usize, row2 as usize)].pos[1];
        let h11 = cloth.particles[cloth.idx(col1 as usize, row1 as usize)].pos[1];
        let fxy = h00 * (1.0 - sub_dx) * (1.0 - sub_dz)
            + h33 * (1.0 - sub_dx) * sub_dz
            + h22 * sub_dx * sub_dz
            + h11 * sub_dx * (1.0 - sub_dz);
        let height_var = fxy - p.y;
        if height_var.abs() < class_threshold {
            ground.push(i);
        } else {
            off_ground.push(i);
        }
    }
    CsfResult {
        ground_indices: ground,
        off_ground_indices: off_ground,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_simple_terrain() {
        // Build a synthetic terrain: 10x10 grid at z=0 with a few points
        // raised above (off-ground).
        let mut points = Vec::new();
        for ix in 0..10 {
            for iy in 0..10 {
                points.push(CsfPoint {
                    x: ix as f64,
                    y: iy as f64,
                    z: 0.0,
                });
            }
        }
        // Off-ground points well above the ground.
        for _ in 0..5 {
            points.push(CsfPoint {
                x: 5.0,
                y: 5.0,
                z: 50.0,
            });
        }
        let params = CsfParams {
            cloth_resolution: 1.0,
            iterations: 50,
            ..Default::default()
        };
        let r = classify_ground(&points, &params);
        assert!(
            r.ground_indices.len() >= 80,
            "too few ground points: {}",
            r.ground_indices.len()
        );
        assert!(
            r.off_ground_indices.len() >= 5,
            "off-ground points missing: {}",
            r.off_ground_indices.len()
        );
    }
}
