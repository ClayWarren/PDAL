//! The `GreedyProjection` main triangulation loop (`Algo::run`).
//! Split out of `greedyprojection.rs` to keep modules under ~1k LOC.

use super::*;

impl<'a> Algo<'a> {
    pub(super) fn run(&mut self) {
        let n = self.coords.len();
        if n == 0 {
            return;
        }
        let sqr_mu = self.params.mu * self.params.mu;
        let sqr_max_edge = self.params.search_radius * self.params.search_radius;
        let eps_angle = self.params.eps_angle;
        let max_angle = self.params.max_angle;
        let min_angle = self.params.min_angle;
        let consistent = self.params.consistent;

        self.nnn = self.params.nnn.min(n);
        self.angles = vec![
            NnAngle {
                angle: 0.0,
                index: 0,
                nn_index: 0,
                visible: false,
            };
            self.nnn
        ];

        self.state.clear();
        self.state.resize(n, Gp3::Free);
        self.source.clear();
        self.source.resize(n, 0);
        self.ffn.clear();
        self.ffn.resize(n, 0);
        self.sfn.clear();
        self.sfn.resize(n, 0);
        self.part.clear();
        self.part.resize(n, 0);
        self.fringe_queue.clear();
        let mut fq_idx: usize = 0;

        let mut nn_idx: Vec<PointId> = vec![0; self.nnn];
        let mut sqr_dists: Vec<f64> = vec![0.0; self.nnn];
        let mut uvn_nn: Vec<V2> = vec![[0.0; 2]; self.nnn];
        let mut uvn_s: V2 = [0.0; 2];
        let uvn_nn_qp_zero: V2 = [0.0; 2];

        let mut is_free: PointId = 0;
        let mut done = false;
        let mut part_index: PointId = 0;
        let mut nr_parts: i64 = 0;
        let mut increase_nnn4fn: i64 = 0;
        let mut increase_nnn4s: i64 = 0;
        let mut _increase_dist: i64 = 0;
        self.already_connected = false;

        while !done {
            self.r_ = is_free;
            if self.state[self.r_ as usize] == Gp3::Free {
                self.state[self.r_ as usize] = Gp3::None;
                self.part[self.r_ as usize] = part_index;
                part_index += 1;

                self.knn(self.r_, self.nnn, &mut nn_idx, &mut sqr_dists);
                let sqr_dist_threshold = sqr_max_edge.min(sqr_mu * sqr_dists[1]);

                let nc = self.normal(self.r_);
                self.v_vec = unit_orthogonal3(nc);
                self.u_vec = cross3(nc, self.v_vec);
                let coord = self.coord(self.r_);
                self.proj_qp = sub3(
                    coord,
                    [
                        dot3(nc, coord) * nc[0],
                        dot3(nc, coord) * nc[1],
                        dot3(nc, coord) * nc[2],
                    ],
                );

                let mut nr_edge = 0;
                let mut double_edges: Vec<DoubleEdge> = Vec::new();
                for i in 1..self.nnn {
                    let tmp = sub3(self.coord(nn_idx[i]), self.proj_qp);
                    uvn_nn[i] = [dot3(tmp, self.u_vec), dot3(tmp, self.v_vec)];
                    self.angles[i].angle = uvn_nn[i][1].atan2(uvn_nn[i][0]);
                    self.angles[i].index = nn_idx[i];
                    let st = self.state[nn_idx[i] as usize];
                    self.angles[i].visible = !(st == Gp3::Completed
                        || st == Gp3::Boundary
                        || st == Gp3::None
                        || sqr_dists[i] > sqr_dist_threshold);
                    if st == Gp3::Fringe || st == Gp3::Boundary {
                        let mut e = DoubleEdge {
                            index: i,
                            first: [0.0; 2],
                            second: [0.0; 2],
                        };
                        nr_edge += 1;
                        let t1 = sub3(self.coord(self.ffn[nn_idx[i] as usize]), self.proj_qp);
                        e.first = [dot3(t1, self.u_vec), dot3(t1, self.v_vec)];
                        let t2 = sub3(self.coord(self.sfn[nn_idx[i] as usize]), self.proj_qp);
                        e.second = [dot3(t2, self.u_vec), dot3(t2, self.v_vec)];
                        double_edges.push(e);
                    }
                }
                self.angles[0].visible = false;

                for i in 1..self.nnn {
                    if self.angles[i].visible
                        && self.ffn[self.r_ as usize] != nn_idx[i]
                        && self.sfn[self.r_ as usize] != nn_idx[i]
                    {
                        let mut visibility = true;
                        for j in 0..nr_edge {
                            let de = &double_edges[j];
                            if self.ffn[nn_idx[de.index] as usize] != nn_idx[i] {
                                visibility =
                                    is_visible(uvn_nn[i], uvn_nn[de.index], de.first, [0.0; 2]);
                            }
                            if !visibility {
                                break;
                            }
                            if self.sfn[nn_idx[de.index] as usize] != nn_idx[i] {
                                visibility =
                                    is_visible(uvn_nn[i], uvn_nn[de.index], de.second, [0.0; 2]);
                            }
                            // C++ has `if (!visibility == false) break;` which is
                            // `if (visibility) break;` due to operator precedence.
                            if visibility {
                                break;
                            }
                        }
                        self.angles[i].visible = visibility;
                    }
                }

                let mut not_found = true;
                let mut left: usize = 1;
                loop {
                    while left < self.nnn
                        && (!self.angles[left].visible || self.state_set(nn_idx[left]))
                    {
                        left += 1;
                    }
                    if left >= self.nnn {
                        break;
                    }
                    let mut right = left + 1;
                    let mut placed = false;
                    loop {
                        while right < self.nnn
                            && (!self.angles[right].visible || self.state_set(nn_idx[right]))
                        {
                            right += 1;
                        }
                        if right >= self.nnn {
                            break;
                        }
                        let diff = sub3(self.coord(nn_idx[left]), self.coord(nn_idx[right]));
                        if sqr_norm3(diff) > sqr_max_edge {
                            right += 1;
                        } else {
                            let r = self.r_;
                            let nl = nn_idx[left];
                            let nr = nn_idx[right];
                            self.add_fringe_point(nr, r);
                            self.add_fringe_point(nl, nr);
                            self.add_fringe_point(r, nl);
                            self.state[r as usize] = Gp3::Fringe;
                            self.state[nl as usize] = Gp3::Fringe;
                            self.state[nr as usize] = Gp3::Fringe;
                            self.ffn[r as usize] = nl;
                            self.sfn[r as usize] = nr;
                            self.ffn[nl as usize] = nr;
                            self.sfn[nl as usize] = r;
                            self.ffn[nr as usize] = r;
                            self.sfn[nr as usize] = nl;
                            self.add_triangle(r, nl, nr);
                            nr_parts += 1;
                            not_found = false;
                            placed = true;
                            break;
                        }
                    }
                    if placed || !not_found {
                        break;
                    }
                    left += 1;
                }
                let _ = not_found;
            }

            // Find next free.
            let next_free = self.state.iter().position(|s| *s == Gp3::Free);
            match next_free {
                None => done = true,
                Some(idx) => {
                    done = false;
                    is_free = idx as PointId;
                }
            }

            let mut is_fringe = true;
            while is_fringe {
                is_fringe = false;
                let fq_size = self.fringe_queue.len();
                while fq_idx < fq_size
                    && self.state[self.fringe_queue[fq_idx] as usize] != Gp3::Fringe
                {
                    fq_idx += 1;
                }
                if fq_idx >= fq_size {
                    continue;
                }
                self.r_ = self.fringe_queue[fq_idx];
                is_fringe = true;
                if self.ffn[self.r_ as usize] == self.sfn[self.r_ as usize] {
                    self.state[self.r_ as usize] = Gp3::Completed;
                    continue;
                }
                self.knn(self.r_, self.nnn, &mut nn_idx, &mut sqr_dists);

                let coord = self.coord(self.r_);
                let sqr_source_dist =
                    sqr_norm3(sub3(coord, self.coord(self.source[self.r_ as usize])));
                let sqr_ffn_dist = sqr_norm3(sub3(coord, self.coord(self.ffn[self.r_ as usize])));
                let sqr_sfn_dist = sqr_norm3(sub3(coord, self.coord(self.sfn[self.r_ as usize])));
                let max_sqr_fn_dist = sqr_ffn_dist.max(sqr_sfn_dist);
                let sqr_dist_threshold = sqr_max_edge.min(sqr_mu * sqr_dists[1]);
                if max_sqr_fn_dist > sqr_dists[self.nnn - 1] {
                    increase_nnn4fn += 1;
                    self.state[self.r_ as usize] = Gp3::Boundary;
                    continue;
                }
                let max_sqr_fns_dist = sqr_source_dist.max(max_sqr_fn_dist);
                if max_sqr_fns_dist > sqr_dists[self.nnn - 1] {
                    increase_nnn4s += 1;
                }

                let nc = self.normal(self.r_);
                self.v_vec = unit_orthogonal3(nc);
                self.u_vec = cross3(nc, self.v_vec);
                let c = self.coord(self.r_);
                self.proj_qp = sub3(
                    c,
                    [
                        dot3(nc, c) * nc[0],
                        dot3(nc, c) * nc[1],
                        dot3(nc, c) * nc[2],
                    ],
                );

                let mut nr_edge = 0;
                let mut double_edges: Vec<DoubleEdge> = Vec::new();
                for i in 1..self.nnn {
                    let tmp = sub3(self.coord(nn_idx[i]), self.proj_qp);
                    uvn_nn[i] = [dot3(tmp, self.u_vec), dot3(tmp, self.v_vec)];
                    self.angles[i].angle = uvn_nn[i][1].atan2(uvn_nn[i][0]);
                    self.angles[i].index = nn_idx[i];
                    self.angles[i].nn_index = i as i32;
                    let st = self.state[nn_idx[i] as usize];
                    self.angles[i].visible = !(st == Gp3::Completed
                        || st == Gp3::Boundary
                        || st == Gp3::None
                        || sqr_dists[i] > sqr_dist_threshold);
                    if self.ffn[self.r_ as usize] == nn_idx[i]
                        || self.sfn[self.r_ as usize] == nn_idx[i]
                    {
                        self.angles[i].visible = true;
                    }
                    let mut same_side = true;
                    let neighbor_normal = self.normal(nn_idx[i]);
                    let mut cosine = dot3(nc, neighbor_normal);
                    if cosine > 1.0 {
                        cosine = 1.0;
                    }
                    if cosine < -1.0 {
                        cosine = -1.0;
                    }
                    let mut angle = cosine.acos();
                    if !consistent && angle > PI / 2.0 {
                        angle = PI - angle;
                    }
                    if angle > eps_angle {
                        self.angles[i].visible = false;
                        same_side = false;
                    }
                    if i != 0 && same_side && (st == Gp3::Fringe || st == Gp3::Boundary) {
                        let mut e = DoubleEdge {
                            index: i,
                            first: [0.0; 2],
                            second: [0.0; 2],
                        };
                        nr_edge += 1;
                        let t1 = sub3(self.coord(self.ffn[nn_idx[i] as usize]), self.proj_qp);
                        e.first = [dot3(t1, self.u_vec), dot3(t1, self.v_vec)];
                        let t2 = sub3(self.coord(self.sfn[nn_idx[i] as usize]), self.proj_qp);
                        e.second = [dot3(t2, self.u_vec), dot3(t2, self.v_vec)];
                        double_edges.push(e);

                        if st == Gp3::Fringe
                            && self.ffn[self.r_ as usize] != nn_idx[i]
                            && self.sfn[self.r_ as usize] != nn_idx[i]
                        {
                            let angle1 =
                                (e.first[1] - uvn_nn[i][1]).atan2(e.first[0] - uvn_nn[i][0]);
                            let angle2 =
                                (e.second[1] - uvn_nn[i][1]).atan2(e.second[0] - uvn_nn[i][0]);
                            let (angle_min, angle_max) = if angle1 < angle2 {
                                (angle1, angle2)
                            } else {
                                (angle2, angle1)
                            };
                            let mut angle_r = self.angles[i].angle + PI;
                            if angle_r >= 2.0 * PI {
                                angle_r -= 2.0 * PI;
                            }
                            if self.source[nn_idx[i] as usize] == self.ffn[nn_idx[i] as usize]
                                || self.source[nn_idx[i] as usize] == self.sfn[nn_idx[i] as usize]
                            {
                                if angle_max - angle_min < PI {
                                    if angle_min < angle_r && angle_r < angle_max {
                                        self.angles[i].visible = false;
                                    }
                                } else if angle_r < angle_min || angle_max < angle_r {
                                    self.angles[i].visible = false;
                                }
                            } else {
                                let t =
                                    sub3(self.coord(self.source[nn_idx[i] as usize]), self.proj_qp);
                                uvn_s = [dot3(t, self.u_vec), dot3(t, self.v_vec)];
                                let angle_s =
                                    (uvn_s[1] - uvn_nn[i][1]).atan2(uvn_s[0] - uvn_nn[i][0]);
                                if angle_min < angle_s && angle_s < angle_max {
                                    if angle_min < angle_r && angle_r < angle_max {
                                        self.angles[i].visible = false;
                                    }
                                } else if angle_r < angle_min || angle_max < angle_r {
                                    self.angles[i].visible = false;
                                }
                            }
                        }
                    }
                }
                self.angles[0].visible = false;

                for i in 1..self.nnn {
                    if self.angles[i].visible
                        && self.ffn[self.r_ as usize] != nn_idx[i]
                        && self.sfn[self.r_ as usize] != nn_idx[i]
                    {
                        let mut visibility = true;
                        for j in 0..nr_edge {
                            let de = &double_edges[j];
                            if de.index != i {
                                let f = self.ffn[nn_idx[de.index] as usize];
                                if f != nn_idx[i] && f != self.r_ {
                                    visibility =
                                        is_visible(uvn_nn[i], uvn_nn[de.index], de.first, [0.0; 2]);
                                }
                                if !visibility {
                                    break;
                                }
                                let s = self.sfn[nn_idx[de.index] as usize];
                                if s != nn_idx[i] && s != self.r_ {
                                    visibility = is_visible(
                                        uvn_nn[i],
                                        uvn_nn[de.index],
                                        de.second,
                                        [0.0; 2],
                                    );
                                }
                                if !visibility {
                                    break;
                                }
                            }
                        }
                        self.angles[i].visible = visibility;
                    }
                }

                self.angles.sort_by(|a1, a2| {
                    if a1.visible == a2.visible {
                        a1.angle
                            .partial_cmp(&a2.angle)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    } else if a1.visible {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                });

                if !self.angles[2].visible {
                    let r = self.r_;
                    if !((self.angles[0].index == self.ffn[r as usize]
                        && self.angles[1].index == self.sfn[r as usize])
                        || (self.angles[0].index == self.sfn[r as usize]
                            && self.angles[1].index == self.ffn[r as usize]))
                    {
                        self.state[r as usize] = Gp3::Boundary;
                    } else if self.source[r as usize] == self.angles[0].index
                        || self.source[r as usize] == self.angles[1].index
                    {
                        self.state[r as usize] = Gp3::Boundary;
                    } else if sqr_max_edge
                        < sqr_norm3(sub3(
                            self.coord(self.ffn[r as usize]),
                            self.coord(self.sfn[r as usize]),
                        ))
                    {
                        self.state[r as usize] = Gp3::Boundary;
                    } else {
                        let t = sub3(self.coord(self.source[r as usize]), self.proj_qp);
                        uvn_s = [dot3(t, self.u_vec), dot3(t, self.v_vec)];
                        let angle_s = uvn_s[1].atan2(uvn_s[0]);
                        let dif = self.angles[1].angle - self.angles[0].angle;
                        if self.angles[0].angle < angle_s && angle_s < self.angles[1].angle {
                            if dif < 2.0 * PI - max_angle {
                                self.state[r as usize] = Gp3::Boundary;
                            } else {
                                self.close_triangle();
                            }
                        } else if dif >= max_angle {
                            self.state[r as usize] = Gp3::Boundary;
                        } else {
                            self.close_triangle();
                        }
                    }
                    continue;
                }

                // Finding FFN and SFN.
                let mut start: i32 = -1;
                let mut end: i32 = -1;
                {
                    let r = self.r_;
                    let mut i: usize = 0;
                    while i < self.nnn {
                        if self.ffn[r as usize] == self.angles[i].index {
                            start = i as i32;
                            if i + 1 < self.nnn && self.sfn[r as usize] == self.angles[i + 1].index
                            {
                                end = (i + 1) as i32;
                            } else {
                                let mut j = i + 2;
                                while j < self.nnn {
                                    if self.sfn[r as usize] == self.angles[j].index {
                                        break;
                                    }
                                    j += 1;
                                }
                                end = j as i32;
                            }
                            break;
                        }
                        if self.sfn[r as usize] == self.angles[i].index {
                            start = i as i32;
                            if i + 1 < self.nnn && self.ffn[r as usize] == self.angles[i + 1].index
                            {
                                end = (i + 1) as i32;
                            } else {
                                let mut j = i + 2;
                                while j < self.nnn {
                                    if self.ffn[r as usize] == self.angles[j].index {
                                        break;
                                    }
                                    j += 1;
                                }
                                end = j as i32;
                            }
                            break;
                        }
                        i += 1;
                    }
                }

                if start < 0
                    || end < 0
                    || end as usize == self.nnn
                    || !self.angles[start as usize].visible
                    || !self.angles[end as usize].visible
                {
                    self.state[self.r_ as usize] = Gp3::Boundary;
                    continue;
                }

                let mut last_visible = end as usize;
                while last_visible + 1 < self.nnn && self.angles[last_visible + 1].visible {
                    last_visible += 1;
                }

                let mut need_invert = false;
                let r = self.r_;
                if self.source[r as usize] == self.ffn[r as usize]
                    || self.source[r as usize] == self.sfn[r as usize]
                {
                    if self.angles[end as usize].angle - self.angles[start as usize].angle < PI {
                        need_invert = true;
                    }
                } else {
                    let mut source_idx = 0usize;
                    while source_idx < self.nnn
                        && self.angles[source_idx].index != self.source[r as usize]
                    {
                        source_idx += 1;
                    }
                    if source_idx == self.nnn {
                        let mut vis_free: i32 = -1;
                        let mut nn_cb: i32 = -1;
                        for i in 1..self.nnn {
                            let st = self.state[nn_idx[i] as usize];
                            if (st == Gp3::Completed || st == Gp3::Boundary) && nn_cb == -1 {
                                nn_cb = i as i32;
                                if vis_free != -1 {
                                    break;
                                }
                            }
                            if !self.state_set(self.angles[i].index) && i <= last_visible {
                                vis_free = i as i32;
                                if nn_cb != -1 {
                                    break;
                                }
                            }
                        }
                        let mut n_cb: i32 = 0;
                        if nn_cb != -1 {
                            while self.angles[n_cb as usize].index != nn_idx[nn_cb as usize] {
                                n_cb += 1;
                            }
                        } else {
                            n_cb = -1;
                        }
                        if vis_free != -1 {
                            if vis_free < start || vis_free > end {
                                need_invert = true;
                            }
                        } else if n_cb != -1 {
                            if n_cb == start || n_cb == end {
                                let mut inside_cb = false;
                                let mut outside_cb = false;
                                for i in 0..self.nnn {
                                    let st = self.state[self.angles[i].index as usize];
                                    if (st == Gp3::Completed || st == Gp3::Boundary)
                                        && i as i32 != start
                                        && i as i32 != end
                                    {
                                        if self.angles[start as usize].angle <= self.angles[i].angle
                                            && self.angles[i].angle
                                                <= self.angles[end as usize].angle
                                        {
                                            inside_cb = true;
                                            if outside_cb {
                                                break;
                                            }
                                        } else {
                                            outside_cb = true;
                                            if inside_cb {
                                                break;
                                            }
                                        }
                                    }
                                }
                                if inside_cb && !outside_cb {
                                    need_invert = true;
                                } else if !(!inside_cb && outside_cb)
                                    && (self.angles[end as usize].angle
                                        - self.angles[start as usize].angle)
                                        < PI
                                {
                                    need_invert = true;
                                }
                            } else if self.angles[n_cb as usize].angle
                                > self.angles[start as usize].angle
                                && self.angles[n_cb as usize].angle
                                    < self.angles[end as usize].angle
                            {
                                need_invert = true;
                            }
                        } else if start == end - 1 {
                            need_invert = true;
                        }
                    } else if self.angles[start as usize].angle < self.angles[source_idx].angle
                        && self.angles[source_idx].angle < self.angles[end as usize].angle
                    {
                        need_invert = true;
                    }
                }

                if need_invert {
                    std::mem::swap(&mut start, &mut end);
                }

                let mut is_boundary = false;
                let mut is_skinny = false;
                let mut gaps = vec![false; self.nnn];
                let mut skinny = vec![false; self.nnn];
                let mut dif = vec![0.0f64; self.nnn];
                let mut angle_idx: Vec<i32> = Vec::with_capacity(self.nnn);

                let push_gap = |j: usize,
                                next_j: usize,
                                dif_v: &mut Vec<f64>,
                                gaps: &mut Vec<bool>,
                                skinny: &mut Vec<bool>,
                                angles: &Vec<NnAngle>,
                                coords: &Vec<V3>,
                                is_boundary: &mut bool,
                                is_skinny: &mut bool| {
                    let d = angles[next_j].angle - angles[j].angle;
                    dif_v[j] = d;
                    if d < min_angle {
                        skinny[j] = true;
                        *is_skinny = true;
                    } else if max_angle <= d {
                        gaps[j] = true;
                        *is_boundary = true;
                    }
                    if !gaps[j]
                        && sqr_max_edge
                            < sqr_norm3(sub3(
                                coords[angles[next_j].index as usize],
                                coords[angles[j].index as usize],
                            ))
                    {
                        gaps[j] = true;
                        *is_boundary = true;
                    }
                };

                if start > end {
                    for j in (start as usize)..last_visible {
                        push_gap(
                            j,
                            j + 1,
                            &mut dif,
                            &mut gaps,
                            &mut skinny,
                            &self.angles,
                            &self.coords,
                            &mut is_boundary,
                            &mut is_skinny,
                        );
                        angle_idx.push(j as i32);
                    }
                    let d = 2.0 * PI + self.angles[0].angle - self.angles[last_visible].angle;
                    dif[last_visible] = d;
                    if d < min_angle {
                        skinny[last_visible] = true;
                        is_skinny = true;
                    } else if max_angle <= d {
                        gaps[last_visible] = true;
                        is_boundary = true;
                    }
                    if !gaps[last_visible]
                        && sqr_max_edge
                            < sqr_norm3(sub3(
                                self.coord(self.angles[0].index),
                                self.coord(self.angles[last_visible].index),
                            ))
                    {
                        gaps[last_visible] = true;
                        is_boundary = true;
                    }
                    angle_idx.push(last_visible as i32);
                    for j in 0..(end as usize) {
                        push_gap(
                            j,
                            j + 1,
                            &mut dif,
                            &mut gaps,
                            &mut skinny,
                            &self.angles,
                            &self.coords,
                            &mut is_boundary,
                            &mut is_skinny,
                        );
                        angle_idx.push(j as i32);
                    }
                    angle_idx.push(end);
                } else {
                    for j in (start as usize)..(end as usize) {
                        push_gap(
                            j,
                            j + 1,
                            &mut dif,
                            &mut gaps,
                            &mut skinny,
                            &self.angles,
                            &self.coords,
                            &mut is_boundary,
                            &mut is_skinny,
                        );
                        angle_idx.push(j as i32);
                    }
                    angle_idx.push(end);
                }

                self.state[self.r_ as usize] = if is_boundary {
                    Gp3::Boundary
                } else {
                    Gp3::Completed
                };

                let mut first_gap_after: Option<usize> = None;
                let mut last_gap_before: usize = 0;
                let mut nr_gaps = 0;
                for (idx, &v) in angle_idx[..angle_idx.len() - 1].iter().enumerate() {
                    if gaps[v as usize] {
                        nr_gaps += 1;
                        if first_gap_after.is_none() {
                            first_gap_after = Some(idx);
                        }
                        last_gap_before = idx + 1;
                    }
                }
                if nr_gaps > 1 {
                    let fga = first_gap_after.unwrap() + 1;
                    if fga < last_gap_before {
                        angle_idx.drain(fga..last_gap_before);
                    }
                }

                if is_skinny {
                    let max_combined_angle = max_angle.min(PI - 2.0 * min_angle);
                    let mut angle_so_far = 0.0;
                    let mut to_erase: Vec<i32> = Vec::new();
                    let mut it = 1usize;
                    while it + 1 < angle_idx.len() {
                        let v_it = angle_idx[it];
                        let v_prev = angle_idx[it - 1];
                        if gaps[v_prev as usize] {
                            angle_so_far = 0.0;
                        } else {
                            angle_so_far += dif[v_prev as usize];
                        }
                        let angle_would_be = if gaps[v_it as usize] {
                            angle_so_far
                        } else {
                            angle_so_far + dif[v_it as usize]
                        };
                        let cond_state = !self.state_set(self.angles[v_it as usize].index)
                            || !self.state_set(self.angles[v_prev as usize].index);
                        let cond_gap1 = !gaps[v_it as usize]
                            || (self.angles[v_it as usize].nn_index
                                > self.angles[v_prev as usize].nn_index);
                        let v_next = angle_idx[it + 1];
                        let cond_gap2 = !gaps[v_prev as usize]
                            || (self.angles[v_it as usize].nn_index
                                > self.angles[v_next as usize].nn_index);
                        if (skinny[v_it as usize] || skinny[v_prev as usize])
                            && cond_state
                            && cond_gap1
                            && cond_gap2
                            && angle_would_be < max_combined_angle
                        {
                            if gaps[v_prev as usize] {
                                gaps[v_it as usize] = true;
                                to_erase.push(v_it);
                            } else if gaps[v_it as usize] {
                                gaps[v_prev as usize] = true;
                                to_erase.push(v_it);
                            } else {
                                // Walk back over already-erased entries.
                                let prev_it = it - 1;
                                let mut erased_idx = to_erase.len() as i32 - 1;
                                while erased_idx >= 0 && it > 0 {
                                    if angle_idx[it] == to_erase[erased_idx as usize] {
                                        erased_idx -= 1;
                                        it -= 1;
                                    } else {
                                        break;
                                    }
                                }
                                let mut can_delete = true;
                                let mut cur_it = prev_it + 1;
                                while cur_it < it + 1 {
                                    let x_tmp = sub3(
                                        self.coord(self.angles[angle_idx[cur_it] as usize].index),
                                        self.proj_qp,
                                    );
                                    let x = [dot3(x_tmp, self.u_vec), dot3(x_tmp, self.v_vec)];
                                    let s1_tmp = sub3(
                                        self.coord(self.angles[angle_idx[prev_it] as usize].index),
                                        self.proj_qp,
                                    );
                                    let s1 = [dot3(s1_tmp, self.u_vec), dot3(s1_tmp, self.v_vec)];
                                    let s2_tmp = sub3(
                                        self.coord(self.angles[angle_idx[it + 1] as usize].index),
                                        self.proj_qp,
                                    );
                                    let s2 = [dot3(s2_tmp, self.u_vec), dot3(s2_tmp, self.v_vec)];
                                    if is_visible(x, s1, s2, [0.0; 2]) {
                                        can_delete = false;
                                        angle_so_far = 0.0;
                                        break;
                                    }
                                    cur_it += 1;
                                }
                                if can_delete {
                                    to_erase.push(angle_idx[it]);
                                }
                            }
                        } else {
                            angle_so_far = 0.0;
                        }
                        it += 1;
                    }
                    for v in to_erase {
                        if let Some(pos) = angle_idx.iter().position(|x| *x == v) {
                            angle_idx.remove(pos);
                        }
                    }
                }

                // Writing edges and updating edge-front.
                self.changed_1st_fn = false;
                self.changed_2nd_fn = false;
                self.new2boundary = NIL;
                let mut it = 1usize;
                while it + 1 < angle_idx.len() {
                    let v_it = angle_idx[it];
                    let v_prev = angle_idx[it - 1];
                    let v_next = angle_idx[it + 1];
                    self.current_index = self.angles[v_it as usize].index;
                    self.is_current_free = false;
                    if !self.state_set(self.current_index) {
                        self.state[self.current_index as usize] = Gp3::Fringe;
                        self.is_current_free = true;
                    } else if !self.already_connected {
                        let ci = self.current_index as usize;
                        self.prev_is_ffn = self.ffn[ci] == self.angles[v_prev as usize].index
                            && !gaps[v_prev as usize];
                        self.prev_is_sfn = self.sfn[ci] == self.angles[v_prev as usize].index
                            && !gaps[v_prev as usize];
                        self.next_is_ffn = self.ffn[ci] == self.angles[v_next as usize].index
                            && !gaps[v_it as usize];
                        self.next_is_sfn = self.sfn[ci] == self.angles[v_next as usize].index
                            && !gaps[v_it as usize];
                    }
                    let r = self.r_;
                    if gaps[v_it as usize] {
                        if gaps[v_prev as usize] {
                            if self.is_current_free {
                                self.state[self.current_index as usize] = Gp3::None;
                            }
                        } else {
                            let ci = self.current_index;
                            self.add_triangle(ci, self.angles[v_prev as usize].index, r);
                            self.add_fringe_point(ci, r);
                            self.new2boundary = ci;
                            if !self.already_connected {
                                let uvn_cur = uvn_nn[self.angles[v_it as usize].nn_index as usize];
                                let uvn_prev =
                                    uvn_nn[self.angles[v_prev as usize].nn_index as usize];
                                self.connect_point(
                                    self.angles[v_prev as usize].index,
                                    r,
                                    self.angles[v_next as usize].index,
                                    uvn_cur,
                                    uvn_prev,
                                    uvn_nn_qp_zero,
                                );
                            } else {
                                self.already_connected = false;
                            }
                            if self.ffn[r as usize] == self.angles[angle_idx[0] as usize].index {
                                self.ffn[r as usize] = self.new2boundary;
                            } else if self.sfn[r as usize]
                                == self.angles[angle_idx[0] as usize].index
                            {
                                self.sfn[r as usize] = self.new2boundary;
                            }
                        }
                    } else if gaps[v_prev as usize] {
                        let ci = self.current_index;
                        self.add_fringe_point(ci, r);
                        self.new2boundary = ci;
                        if !self.already_connected {
                            let uvn_cur = uvn_nn[self.angles[v_it as usize].nn_index as usize];
                            let uvn_next_val =
                                uvn_nn[self.angles[v_next as usize].nn_index as usize];
                            let next_next_index = if it + 2 == angle_idx.len() {
                                NIL
                            } else {
                                self.angles[angle_idx[it + 2] as usize].index
                            };
                            self.connect_point(
                                r,
                                self.angles[v_next as usize].index,
                                next_next_index,
                                uvn_cur,
                                uvn_nn_qp_zero,
                                uvn_next_val,
                            );
                        } else {
                            self.already_connected = false;
                        }
                        let last_idx = angle_idx[angle_idx.len() - 1] as usize;
                        if self.ffn[r as usize] == self.angles[last_idx].index {
                            self.ffn[r as usize] = self.new2boundary;
                        } else if self.sfn[r as usize] == self.angles[last_idx].index {
                            self.sfn[r as usize] = self.new2boundary;
                        }
                    } else {
                        let ci = self.current_index;
                        self.add_triangle(ci, self.angles[v_prev as usize].index, r);
                        self.add_fringe_point(ci, r);
                        if !self.already_connected {
                            let uvn_cur = uvn_nn[self.angles[v_it as usize].nn_index as usize];
                            let uvn_prev_val =
                                uvn_nn[self.angles[v_prev as usize].nn_index as usize];
                            let uvn_next_val =
                                uvn_nn[self.angles[v_next as usize].nn_index as usize];
                            let next_next_index = if it + 2 == angle_idx.len() {
                                NIL
                            } else if gaps[v_next as usize] {
                                r
                            } else {
                                self.angles[angle_idx[it + 2] as usize].index
                            };
                            self.connect_point(
                                self.angles[v_prev as usize].index,
                                self.angles[v_next as usize].index,
                                next_next_index,
                                uvn_cur,
                                uvn_prev_val,
                                uvn_next_val,
                            );
                        } else {
                            self.already_connected = false;
                        }
                    }
                    it += 1;
                }

                // Finishing up R_
                let r = self.r_;
                if self.ffn[r as usize] == self.sfn[r as usize] {
                    self.state[r as usize] = Gp3::Completed;
                }
                let len = angle_idx.len();
                if !gaps[angle_idx[len - 2] as usize] {
                    let aim2 = self.angles[angle_idx[len - 2] as usize].index;
                    let aim1 = self.angles[angle_idx[len - 1] as usize].index;
                    self.add_triangle(aim2, aim1, r);
                    self.add_fringe_point(aim2, r);
                    if r == self.ffn[aim1 as usize] {
                        if aim2 == self.sfn[aim1 as usize] {
                            self.state[aim1 as usize] = Gp3::Completed;
                        } else {
                            self.ffn[aim1 as usize] = aim2;
                        }
                    } else if r == self.sfn[aim1 as usize] {
                        if aim2 == self.ffn[aim1 as usize] {
                            self.state[aim1 as usize] = Gp3::Completed;
                        } else {
                            self.sfn[aim1 as usize] = aim2;
                        }
                    }
                }
                if !gaps[angle_idx[0] as usize] {
                    let a0 = self.angles[angle_idx[0] as usize].index;
                    let a1 = self.angles[angle_idx[1] as usize].index;
                    if r == self.ffn[a0 as usize] {
                        if a1 == self.sfn[a0 as usize] {
                            self.state[a0 as usize] = Gp3::Completed;
                        } else {
                            self.ffn[a0 as usize] = a1;
                        }
                    } else if r == self.sfn[a0 as usize] {
                        if a1 == self.ffn[a0 as usize] {
                            self.state[a0 as usize] = Gp3::Completed;
                        } else {
                            self.sfn[a0 as usize] = a1;
                        }
                    }
                }
            }
        }
        let _ = (nr_parts, increase_nnn4fn, increase_nnn4s, _increase_dist);
    }
}
