//! `GreedyProjection` triangulation algorithm state and per-step methods.
//! Split out of `greedyprojection.rs` to keep modules under ~1k LOC; the
//! large `run` driver lives in the sibling `algo_run` module.

use super::*;

impl<'a> Algo<'a> {
    pub(super) fn knn(
        &self,
        idx: PointId,
        k: usize,
        nn_idx: &mut Vec<PointId>,
        sqr: &mut Vec<f64>,
    ) {
        let neighbors = self.spatial.knn(idx, k);
        nn_idx.clear();
        sqr.clear();
        for (id, d) in neighbors {
            nn_idx.push(id);
            sqr.push(d);
        }
        // Match C++ KD3Index behavior: fill out to k with whatever is present.
        while nn_idx.len() < k {
            nn_idx.push(0);
            sqr.push(f64::INFINITY);
        }
    }

    pub(super) fn state_set(&self, idx: PointId) -> bool {
        self.state[idx as usize] != Gp3::None && self.state[idx as usize] != Gp3::Free
    }

    pub(super) fn coord(&self, id: PointId) -> V3 {
        self.coords[id as usize]
    }

    pub(super) fn normal(&self, id: PointId) -> V3 {
        self.normals[id as usize]
    }

    pub(super) fn add_triangle(&mut self, a: PointId, b: PointId, c: PointId) {
        self.triangles.push([a, b, c]);
    }

    pub(super) fn add_fringe_point(&mut self, v: PointId, s: PointId) {
        self.source[v as usize] = s;
        self.part[v as usize] = self.part[s as usize];
        self.fringe_queue.push(v);
    }

    pub(super) fn close_triangle(&mut self) {
        let r = self.r_;
        self.state[r as usize] = Gp3::Completed;
        let a0 = self.angles[0].index;
        let a1 = self.angles[1].index;
        self.add_triangle(a0, a1, r);
        for a_idx in 0..2 {
            let cur = self.angles[a_idx].index as usize;
            let other = self.angles[(a_idx + 1) % 2].index;
            if self.ffn[cur] == r {
                if self.sfn[cur] == other {
                    self.state[cur] = Gp3::Completed;
                } else {
                    self.ffn[cur] = other;
                }
            } else if self.sfn[cur] == r {
                if self.ffn[cur] == other {
                    self.state[cur] = Gp3::Completed;
                } else {
                    self.sfn[cur] = other;
                }
            }
        }
    }

    pub(super) fn connect_point(
        &mut self,
        prev_index: PointId,
        next_index: PointId,
        next_next_index: PointId,
        uvn_current: V2,
        uvn_prev: V2,
        uvn_next: V2,
    ) {
        let r = self.r_;
        let ci = self.current_index;
        let cu = ci as usize;
        if self.is_current_free {
            self.ffn[cu] = prev_index;
            self.sfn[cu] = next_index;
        } else if (self.prev_is_ffn && self.next_is_sfn) || (self.prev_is_sfn && self.next_is_ffn) {
            self.state[cu] = Gp3::Completed;
        } else if self.prev_is_ffn && !self.next_is_sfn {
            self.ffn[cu] = next_index;
        } else if self.next_is_ffn && !self.prev_is_sfn {
            self.ffn[cu] = prev_index;
        } else if self.prev_is_sfn && !self.next_is_ffn {
            self.sfn[cu] = next_index;
        } else if self.next_is_sfn && !self.prev_is_ffn {
            self.sfn[cu] = prev_index;
        } else {
            let mut found_triangle = false;
            if prev_index != r {
                let pi = prev_index as usize;
                if self.ffn[cu] == self.ffn[pi] || self.ffn[cu] == self.sfn[pi] {
                    found_triangle = true;
                    let fcu = self.ffn[cu];
                    self.add_triangle(ci, fcu, prev_index);
                    self.state[pi] = Gp3::Completed;
                    self.state[fcu as usize] = Gp3::Completed;
                    self.ffn[cu] = next_index;
                } else if self.sfn[cu] == self.ffn[pi] || self.sfn[cu] == self.sfn[pi] {
                    found_triangle = true;
                    let scu = self.sfn[cu];
                    self.add_triangle(ci, scu, prev_index);
                    self.state[pi] = Gp3::Completed;
                    self.state[scu as usize] = Gp3::Completed;
                    self.sfn[cu] = next_index;
                }
            }
            if !found_triangle && self.state_set(next_index) {
                let ni = next_index as usize;
                if self.ffn[cu] == self.ffn[ni] || self.ffn[cu] == self.sfn[ni] {
                    found_triangle = true;
                    let fcu = self.ffn[cu];
                    self.add_triangle(ci, fcu, next_index);
                    if self.ffn[cu] == self.ffn[ni] {
                        self.ffn[ni] = ci;
                    } else {
                        self.sfn[ni] = ci;
                    }
                    self.state[fcu as usize] = Gp3::Completed;
                    self.ffn[cu] = prev_index;
                } else if self.sfn[cu] == self.ffn[ni] || self.sfn[cu] == self.sfn[ni] {
                    found_triangle = true;
                    let scu = self.sfn[cu];
                    self.add_triangle(ci, scu, next_index);
                    if self.sfn[cu] == self.ffn[ni] {
                        self.ffn[ni] = ci;
                    } else {
                        self.sfn[ni] = ci;
                    }
                    self.state[scu as usize] = Gp3::Completed;
                    self.sfn[cu] = prev_index;
                }
            }
            if found_triangle {
                return;
            }
            // Pre-compute uvn for current's ffn / sfn.
            let fcu = self.ffn[cu];
            let scu = self.sfn[cu];
            let tmp_f = sub3(self.coord(fcu), self.proj_qp);
            self.uvn_ffn = [dot3(tmp_f, self.u_vec), dot3(tmp_f, self.v_vec)];
            let tmp_s = sub3(self.coord(scu), self.proj_qp);
            self.uvn_sfn = [dot3(tmp_s, self.u_vec), dot3(tmp_s, self.v_vec)];

            let prev_ffn = is_visible(uvn_prev, uvn_next, uvn_current, self.uvn_ffn)
                && is_visible(uvn_prev, self.uvn_sfn, uvn_current, self.uvn_ffn);
            let prev_sfn = is_visible(uvn_prev, uvn_next, uvn_current, self.uvn_sfn)
                && is_visible(uvn_prev, self.uvn_ffn, uvn_current, self.uvn_sfn);
            let next_ffn = is_visible(uvn_next, uvn_prev, uvn_current, self.uvn_ffn)
                && is_visible(uvn_next, self.uvn_sfn, uvn_current, self.uvn_ffn);
            let next_sfn = is_visible(uvn_next, uvn_prev, uvn_current, self.uvn_sfn)
                && is_visible(uvn_next, self.uvn_ffn, uvn_current, self.uvn_sfn);

            let mut min_dist: i32 = -1;
            if prev_ffn && next_sfn && prev_sfn && next_ffn {
                let prev2f = sqr_norm3(sub3(self.coord(fcu), self.coord(prev_index)));
                let next2s = sqr_norm3(sub3(self.coord(scu), self.coord(next_index)));
                let prev2s = sqr_norm3(sub3(self.coord(scu), self.coord(prev_index)));
                let next2f = sqr_norm3(sub3(self.coord(fcu), self.coord(next_index)));
                if prev2f < prev2s {
                    if prev2f < next2f {
                        min_dist = if prev2f < next2s { 0 } else { 3 };
                    } else {
                        min_dist = if next2f < next2s { 2 } else { 3 };
                    }
                } else if prev2s < next2f {
                    min_dist = if prev2s < next2s { 1 } else { 3 };
                } else {
                    min_dist = if next2f < next2s { 2 } else { 3 };
                }
            } else if prev_ffn && next_sfn {
                let prev2f = sqr_norm3(sub3(self.coord(fcu), self.coord(prev_index)));
                let next2s = sqr_norm3(sub3(self.coord(scu), self.coord(next_index)));
                min_dist = if prev2f < next2s { 0 } else { 3 };
            } else if prev_sfn && next_ffn {
                let prev2s = sqr_norm3(sub3(self.coord(scu), self.coord(prev_index)));
                let next2f = sqr_norm3(sub3(self.coord(fcu), self.coord(next_index)));
                min_dist = if prev2s < next2f { 1 } else { 2 };
            } else if prev_ffn && !next_sfn && !prev_sfn && !next_ffn {
                min_dist = 0;
            } else if !prev_ffn && !next_sfn && prev_sfn && !next_ffn {
                min_dist = 1;
            } else if !prev_ffn && !next_sfn && !prev_sfn && next_ffn {
                min_dist = 2;
            } else if !prev_ffn && next_sfn && !prev_sfn && !next_ffn {
                min_dist = 3;
            } else if prev_ffn {
                let prev2f = sqr_norm3(sub3(self.coord(fcu), self.coord(prev_index)));
                if prev_sfn {
                    let prev2s = sqr_norm3(sub3(self.coord(scu), self.coord(prev_index)));
                    min_dist = if prev2s < prev2f { 1 } else { 0 };
                } else if next_ffn {
                    let next2f = sqr_norm3(sub3(self.coord(fcu), self.coord(next_index)));
                    min_dist = if next2f < prev2f { 2 } else { 0 };
                }
            } else if next_sfn {
                let next2s = sqr_norm3(sub3(self.coord(scu), self.coord(next_index)));
                if prev_sfn {
                    let prev2s = sqr_norm3(sub3(self.coord(scu), self.coord(prev_index)));
                    min_dist = if prev2s < next2s { 1 } else { 3 };
                } else if next_ffn {
                    let next2f = sqr_norm3(sub3(self.coord(fcu), self.coord(next_index)));
                    min_dist = if next2f < next2s { 2 } else { 3 };
                }
            }

            self.dispatch_min_dist(
                min_dist,
                prev_index,
                next_index,
                next_next_index,
                uvn_current,
                uvn_next,
            );
        }
    }

    pub(super) fn dispatch_min_dist(
        &mut self,
        min_dist: i32,
        prev_index: PointId,
        next_index: PointId,
        next_next_index: PointId,
        uvn_current: V2,
        uvn_next: V2,
    ) {
        let r = self.r_;
        let ci = self.current_index;
        let cu = ci as usize;
        match min_dist {
            0 => {
                // prev2f
                let fcu = self.ffn[cu];
                self.add_triangle(ci, fcu, prev_index);
                let pi = prev_index as usize;
                if self.ffn[pi] == ci {
                    self.ffn[pi] = fcu;
                } else if self.sfn[pi] == ci {
                    self.sfn[pi] = fcu;
                } else if self.ffn[pi] == r {
                    self.changed_1st_fn = true;
                    self.ffn[pi] = fcu;
                } else if self.sfn[pi] == r {
                    self.changed_1st_fn = true;
                    self.sfn[pi] = fcu;
                } else if prev_index == r {
                    self.new2boundary = fcu;
                }
                if self.ffn[fcu as usize] == ci {
                    self.ffn[fcu as usize] = prev_index;
                } else if self.sfn[fcu as usize] == ci {
                    self.sfn[fcu as usize] = prev_index;
                }
                self.ffn[cu] = next_index;
            }
            1 => {
                let scu = self.sfn[cu];
                self.add_triangle(ci, scu, prev_index);
                let pi = prev_index as usize;
                if self.ffn[pi] == ci {
                    self.ffn[pi] = scu;
                } else if self.sfn[pi] == ci {
                    self.sfn[pi] = scu;
                } else if self.ffn[pi] == r {
                    self.changed_1st_fn = true;
                    self.ffn[pi] = scu;
                } else if self.sfn[pi] == r {
                    self.changed_1st_fn = true;
                    self.sfn[pi] = scu;
                } else if prev_index == r {
                    self.new2boundary = scu;
                }
                if self.ffn[scu as usize] == ci {
                    self.ffn[scu as usize] = prev_index;
                } else if self.sfn[scu as usize] == ci {
                    self.sfn[scu as usize] = prev_index;
                }
                self.sfn[cu] = next_index;
            }
            2 => self.case_next2f(
                prev_index,
                next_index,
                next_next_index,
                uvn_current,
                uvn_next,
            ),
            3 => self.case_next2s(
                prev_index,
                next_index,
                next_next_index,
                uvn_current,
                uvn_next,
            ),
            _ => {}
        }
    }

    pub(super) fn case_next2f(
        &mut self,
        prev_index: PointId,
        next_index: PointId,
        next_next_index: PointId,
        uvn_current: V2,
        uvn_next: V2,
    ) {
        let r = self.r_;
        let ci = self.current_index;
        let cu = ci as usize;
        let fcu = self.ffn[cu];
        self.add_triangle(ci, fcu, next_index);
        let mut neighbor_update = next_index;
        if !self.state_set(next_index) {
            let ni = next_index as usize;
            self.state[ni] = Gp3::Fringe;
            self.ffn[ni] = ci;
            self.sfn[ni] = fcu;
        } else {
            let ni = next_index as usize;
            if self.ffn[ni] == r {
                self.changed_2nd_fn = true;
                self.ffn[ni] = fcu;
            } else if self.sfn[ni] == r {
                self.changed_2nd_fn = true;
                self.sfn[ni] = fcu;
            } else if next_index == r {
                self.new2boundary = fcu;
                if next_next_index == self.new2boundary {
                    self.already_connected = true;
                }
            } else if self.ffn[ni] == next_next_index {
                self.already_connected = true;
                self.ffn[ni] = fcu;
            } else if self.sfn[ni] == next_next_index {
                self.already_connected = true;
                self.sfn[ni] = fcu;
            } else {
                let tmp_f = sub3(self.coord(self.ffn[ni]), self.proj_qp);
                self.uvn_next_ffn = [dot3(tmp_f, self.u_vec), dot3(tmp_f, self.v_vec)];
                let tmp_s = sub3(self.coord(self.sfn[ni]), self.proj_qp);
                self.uvn_next_sfn = [dot3(tmp_s, self.u_vec), dot3(tmp_s, self.v_vec)];

                let ffn_next_ffn =
                    is_visible(self.uvn_next_ffn, uvn_next, uvn_current, self.uvn_ffn)
                        && is_visible(self.uvn_next_ffn, uvn_next, self.uvn_next_sfn, self.uvn_ffn);
                let sfn_next_ffn =
                    is_visible(self.uvn_next_sfn, uvn_next, uvn_current, self.uvn_ffn)
                        && is_visible(self.uvn_next_sfn, uvn_next, self.uvn_next_ffn, self.uvn_ffn);

                let mut connect2ffn: i32 = -1;
                if ffn_next_ffn && sfn_next_ffn {
                    let fn2f = sqr_norm3(sub3(self.coord(fcu), self.coord(self.ffn[ni])));
                    let sn2f = sqr_norm3(sub3(self.coord(fcu), self.coord(self.sfn[ni])));
                    connect2ffn = if fn2f < sn2f { 0 } else { 1 };
                } else if ffn_next_ffn {
                    connect2ffn = 0;
                } else if sfn_next_ffn {
                    connect2ffn = 1;
                }
                match connect2ffn {
                    0 => {
                        let target = self.ffn[ni];
                        self.add_triangle(next_index, fcu, target);
                        neighbor_update = target;
                        let ti = target as usize;
                        if self.ffn[ti] == fcu || self.sfn[ti] == fcu {
                            self.state[ti] = Gp3::Completed;
                        } else if self.ffn[ti] == next_index {
                            self.ffn[ti] = fcu;
                        } else if self.sfn[ti] == next_index {
                            self.sfn[ti] = fcu;
                        }
                        self.ffn[ni] = ci;
                    }
                    1 => {
                        let target = self.sfn[ni];
                        self.add_triangle(next_index, fcu, target);
                        neighbor_update = target;
                        let ti = target as usize;
                        // NOTE: faithful port of C++ `(ffn_[sfn_[next]] =
                        // ffn_[current])` assignment-as-condition bug.
                        self.ffn[ti] = fcu;
                        if (self.ffn[ti] == fcu) || self.sfn[ti] == fcu {
                            self.state[ti] = Gp3::Completed;
                        } else if self.ffn[ti] == next_index {
                            self.ffn[ti] = fcu;
                        } else if self.sfn[ti] == next_index {
                            self.sfn[ti] = fcu;
                        }
                        self.sfn[ni] = ci;
                    }
                    _ => {}
                }
            }
        }

        // updating ffn
        let fci = self.ffn[cu] as usize;
        if self.ffn[fci] == neighbor_update || self.sfn[fci] == neighbor_update {
            self.state[fci] = Gp3::Completed;
        } else if self.ffn[fci] == ci {
            self.ffn[fci] = neighbor_update;
        } else if self.sfn[fci] == ci {
            self.sfn[fci] = neighbor_update;
        }
        self.ffn[cu] = prev_index;
    }

    pub(super) fn case_next2s(
        &mut self,
        prev_index: PointId,
        next_index: PointId,
        next_next_index: PointId,
        uvn_current: V2,
        uvn_next: V2,
    ) {
        let r = self.r_;
        let ci = self.current_index;
        let cu = ci as usize;
        let scu = self.sfn[cu];
        self.add_triangle(ci, scu, next_index);
        let mut neighbor_update = next_index;
        if !self.state_set(next_index) {
            let ni = next_index as usize;
            self.state[ni] = Gp3::Fringe;
            self.ffn[ni] = ci;
            self.sfn[ni] = scu;
        } else {
            let ni = next_index as usize;
            if self.ffn[ni] == r {
                self.changed_2nd_fn = true;
                self.ffn[ni] = scu;
            } else if self.sfn[ni] == r {
                self.changed_2nd_fn = true;
                self.sfn[ni] = scu;
            } else if next_index == r {
                self.new2boundary = scu;
                if next_next_index == self.new2boundary {
                    self.already_connected = true;
                }
            } else if self.ffn[ni] == next_next_index {
                self.already_connected = true;
                self.ffn[ni] = scu;
            } else if self.sfn[ni] == next_next_index {
                self.already_connected = true;
                self.sfn[ni] = scu;
            } else {
                let tmp_f = sub3(self.coord(self.ffn[ni]), self.proj_qp);
                self.uvn_next_ffn = [dot3(tmp_f, self.u_vec), dot3(tmp_f, self.v_vec)];
                let tmp_s = sub3(self.coord(self.sfn[ni]), self.proj_qp);
                self.uvn_next_sfn = [dot3(tmp_s, self.u_vec), dot3(tmp_s, self.v_vec)];

                let ffn_next_sfn =
                    is_visible(self.uvn_next_ffn, uvn_next, uvn_current, self.uvn_sfn)
                        && is_visible(self.uvn_next_ffn, uvn_next, self.uvn_next_sfn, self.uvn_sfn);
                let sfn_next_sfn =
                    is_visible(self.uvn_next_sfn, uvn_next, uvn_current, self.uvn_sfn)
                        && is_visible(self.uvn_next_sfn, uvn_next, self.uvn_next_ffn, self.uvn_sfn);

                let mut connect2sfn: i32 = -1;
                if ffn_next_sfn && sfn_next_sfn {
                    let fn2s = sqr_norm3(sub3(self.coord(scu), self.coord(self.ffn[ni])));
                    let sn2s = sqr_norm3(sub3(self.coord(scu), self.coord(self.sfn[ni])));
                    connect2sfn = if fn2s < sn2s { 0 } else { 1 };
                } else if ffn_next_sfn {
                    connect2sfn = 0;
                } else if sfn_next_sfn {
                    connect2sfn = 1;
                }
                match connect2sfn {
                    0 => {
                        let target = self.ffn[ni];
                        self.add_triangle(next_index, scu, target);
                        neighbor_update = target;
                        let ti = target as usize;
                        if self.ffn[ti] == scu || self.sfn[ti] == scu {
                            self.state[ti] = Gp3::Completed;
                        } else if self.ffn[ti] == next_index {
                            self.ffn[ti] = scu;
                        } else if self.sfn[ti] == next_index {
                            self.sfn[ti] = scu;
                        }
                        self.ffn[ni] = ci;
                    }
                    1 => {
                        let target = self.sfn[ni];
                        self.add_triangle(next_index, scu, target);
                        neighbor_update = target;
                        let ti = target as usize;
                        if self.ffn[ti] == scu || self.sfn[ti] == scu {
                            self.state[ti] = Gp3::Completed;
                        } else if self.ffn[ti] == next_index {
                            self.ffn[ti] = scu;
                        } else if self.sfn[ti] == next_index {
                            self.sfn[ti] = scu;
                        }
                        self.sfn[ni] = ci;
                    }
                    _ => {}
                }
            }
        }

        // updating sfn
        let sci = self.sfn[cu] as usize;
        if self.ffn[sci] == neighbor_update || self.sfn[sci] == neighbor_update {
            self.state[sci] = Gp3::Completed;
        } else if self.ffn[sci] == ci {
            self.ffn[sci] = neighbor_update;
        } else if self.sfn[sci] == ci {
            self.sfn[sci] = neighbor_update;
        }
        self.sfn[cu] = prev_index;
    }
}
