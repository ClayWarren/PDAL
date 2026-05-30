//! Rust port of `filters/private/hexer/`. Provides a hexagonal tessellation
//! grid that can identify contiguous dense regions and emit a WKT
//! MULTIPOLYGON boundary, mirroring `hexer::HexGrid` / `hexer::BaseGrid`.
//!
//! The output formatting matches the C++ `Path::toWKT` byte layout when
//! callers wrap `format_wkt` with the same precision/locale conventions used
//! by `Utils::OStringStreamClassicLocale`.

use h3o::{CellIndex, CoordIJ, LatLng, LocalIJ, Resolution};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

pub const SQRT_3: f64 = 1.732_050_808;

/// Axial coordinate (i, j) for a hexagon, matching `hexer::HexId`. Uses a
/// custom ordering that puts even-i hexes before odd-i hexes within a row,
/// so iteration over a sorted set picks the same boundary roots the C++
/// implementation does.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct HexId {
    pub i: i32,
    pub j: i32,
}

impl HexId {
    pub fn new(i: i32, j: i32) -> Self {
        Self { i, j }
    }
    pub fn iodd(&self) -> bool {
        self.i.rem_euclid(2) != 0
    }
    pub fn ieven(&self) -> bool {
        !self.iodd()
    }
}

impl Ord for HexId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        match self.j.cmp(&other.j) {
            Equal => {}
            non_eq => return non_eq,
        }
        if self.ieven() && other.iodd() {
            return Less;
        }
        if self.iodd() && other.ieven() {
            return Greater;
        }
        self.i.cmp(&other.i)
    }
}

impl PartialOrd for HexId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::ops::Add for HexId {
    type Output = HexId;
    fn add(self, rhs: HexId) -> HexId {
        HexId::new(self.i + rhs.i, self.j + rhs.j)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Segment {
    pub hex: HexId,
    pub edge: i32,
}

impl Segment {
    pub fn new(hex: HexId, edge: i32) -> Self {
        Self { hex, edge }
    }
    pub fn horizontal(&self) -> bool {
        self.edge == 0 || self.edge == 3
    }
}

/// A closed boundary on the hex grid; outer rings are clockwise, holes are
/// anti-clockwise. Path nesting is tracked by index because parent/child
/// edges form a DAG that's awkward to encode with Rust references.
#[derive(Debug)]
struct Path {
    root_hex: HexId,
    parent: Option<usize>,
    children: Vec<usize>,
    /// `false` = clockwise (outer), `true` = anti-clockwise (hole).
    anticlockwise: bool,
    points: Vec<Point>,
}

impl Path {
    fn new(root_hex: HexId) -> Self {
        Self {
            root_hex,
            parent: None,
            children: Vec::new(),
            anticlockwise: false,
            points: Vec::new(),
        }
    }
}

/// A regular hexagonal grid, oriented with one side parallel to the X axis.
pub struct HexGrid {
    height: f64,
    width: f64,
    offsets: [Point; 6],
    origin: Point,
    counts: HashMap<HexId, i32>,
    dense_limit: i32,
    /// Cells with non-dense neighbors at edge 0, sorted for deterministic
    /// root selection. Each `findShape` peels off the leading entry.
    possible_roots: BTreeSet<HexId>,
    /// Path index keyed by the hex whose top-or-bottom edge a path touches.
    /// Used by `parent_or_child` to walk vertically through nesting.
    hex_paths: HashMap<HexId, usize>,
    paths: Vec<Path>,
    roots: Vec<usize>,
    /// Minimum j coordinate seen for any horizontal segment, used as the
    /// stop condition when walking down through nested paths.
    min_y: i32,
}

pub struct H3Grid {
    dense_limit: i32,
    origin: CellIndex,
    counts: HashMap<HexId, i32>,
    possible_roots: BTreeSet<HexId>,
    hex_paths: HashMap<HexId, usize>,
    paths: Vec<Path>,
    roots: Vec<usize>,
    min_i: i32,
}

impl HexGrid {
    /// Construct a grid with a fixed hex height (twice the apothem).
    pub fn with_height(height: f64, dense_limit: i32) -> Self {
        let mut g = Self::empty(dense_limit);
        g.process_height(height);
        g
    }

    fn empty(dense_limit: i32) -> Self {
        Self {
            height: -1.0,
            width: -1.0,
            offsets: [Point::new(0.0, 0.0); 6],
            origin: Point::new(0.0, 0.0),
            counts: HashMap::new(),
            dense_limit,
            possible_roots: BTreeSet::new(),
            hex_paths: HashMap::new(),
            paths: Vec::new(),
            roots: Vec::new(),
            min_y: i32::MAX,
        }
    }

    fn process_height(&mut self, height: f64) {
        self.height = height;
        self.width = (3.0 / (2.0 * SQRT_3)) * height;
        self.offsets = [
            Point::new(0.0, 0.0),
            Point::new(-self.width / 3.0, height / 2.0),
            Point::new(0.0, height),
            Point::new(2.0 * self.width / 3.0, height),
            Point::new(self.width, height / 2.0),
            Point::new(2.0 * self.width / 3.0, 0.0),
        ];
    }

    /// Increment the count for the hexagon containing `(x, y)`, expanding
    /// the `possible_roots` set when a hexagon first becomes dense.
    pub fn add_xy(&mut self, x: f64, y: f64) {
        let h = self.find_hexagon(Point::new(x, y));
        let count = {
            let entry = self.counts.entry(h).or_insert(0);
            *entry += 1;
            *entry
        };
        if count == self.dense_limit {
            let above = self.edge_hex(h, 0);
            let below = self.edge_hex(h, 3);
            if !self.is_dense(above) {
                self.possible_roots.insert(h);
            }
            self.possible_roots.remove(&below);
        }
    }

    /// Force-load hexes for tests; matches `BaseGrid::setHexes`.
    pub fn set_hexes(&mut self, hexes: &[HexId]) {
        for &h in hexes {
            self.counts.insert(h, self.dense_limit + 1);
            let above = self.edge_hex(h, 0);
            let below = self.edge_hex(h, 3);
            if !self.is_dense(above) {
                self.possible_roots.insert(h);
            }
            self.possible_roots.remove(&below);
        }
    }

    pub fn is_dense(&self, h: HexId) -> bool {
        self.counts.get(&h).copied().unwrap_or(0) >= self.dense_limit
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn offsets(&self) -> &[Point; 6] {
        &self.offsets
    }

    pub fn counts(&self) -> &HashMap<HexId, i32> {
        &self.counts
    }

    /// Vertices for a hexagon, walking edges 0..6 (closing with edge 0).
    /// Useful for emitting hex polygons in density output.
    pub fn hex_vertices(&self, hex: HexId) -> [Point; 7] {
        let mut out = [Point::new(0.0, 0.0); 7];
        for edge in 0..=6 {
            out[edge as usize] = self.find_point(Segment::new(hex, edge % 6));
        }
        out
    }

    /// C++ `hexer::HexGrid::getID` — packs (i, j) into a stable u64.
    pub fn hex_id_u64(hex: HexId) -> u64 {
        ((hex.i as u64) << 32) | (hex.j as u32 as u64)
    }

    /// Map a point to the hexagon that contains it, ported from
    /// `hexer::HexGrid::findHexagon`. The first point added defines the
    /// grid origin and lands at hex (0, 0).
    fn find_hexagon(&mut self, p: Point) -> HexId {
        if self.counts.is_empty() {
            self.origin = p;
            return HexId::new(0, 0);
        }

        let px = p.x - self.origin.x;
        let py = p.y - self.origin.y;
        let col = px / self.width;
        let mut x = col.floor() as i32;
        let mut y = if x.rem_euclid(2) == 0 {
            (py / self.height).floor() as i32
        } else {
            ((py - self.height / 2.0) / self.height).floor() as i32
        };

        let mut xcol_offset = col - col.floor();
        if xcol_offset > 2.0 / 3.0 {
            xcol_offset -= 2.0 / 3.0;
            xcol_offset *= 3.0;
            let halfrow = py / (self.height / 2.0);
            let halfy = halfrow as i32;
            let yrow_offset = halfrow - halfrow.floor();
            let x_even = x.rem_euclid(2) == 0;
            let halfy_even = halfy.rem_euclid(2) == 0;
            if (halfy_even && x_even) || (!x_even && !halfy_even) {
                if xcol_offset > yrow_offset {
                    if x_even {
                        y -= 1;
                    }
                    x += 1;
                }
            } else if yrow_offset > xcol_offset {
                if !x_even {
                    y += 1;
                }
                x += 1;
            }
        }
        HexId::new(x, y)
    }

    fn edge_hex(&self, hex: HexId, edge: i32) -> HexId {
        // Indexed counter-clockwise from edge 0 (bottom).
        const EVEN: [HexId; 6] = [
            HexId { i: 0, j: -1 },
            HexId { i: -1, j: -1 },
            HexId { i: -1, j: 0 },
            HexId { i: 0, j: 1 },
            HexId { i: 1, j: 0 },
            HexId { i: 1, j: -1 },
        ];
        const ODD: [HexId; 6] = [
            HexId { i: 0, j: -1 },
            HexId { i: -1, j: 0 },
            HexId { i: -1, j: 1 },
            HexId { i: 0, j: 1 },
            HexId { i: 1, j: 1 },
            HexId { i: 1, j: 0 },
        ];
        let table = if hex.iodd() { ODD } else { EVEN };
        hex + table[edge as usize]
    }

    /// Counter-clockwise advance along the boundary, ported from
    /// `hexer::HexGrid::nextSegment`. Picks the right-turn segment when the
    /// neighbor across the current edge is dense, otherwise the left turn.
    fn next_segment(&self, s: Segment) -> Segment {
        const NEXT: [i32; 6] = [5, 0, 1, 2, 3, 4];
        const PREV: [i32; 6] = [1, 2, 3, 4, 5, 0];
        let left = Segment::new(s.hex, NEXT[s.edge as usize]);
        let right = Segment::new(self.edge_hex(s.hex, left.edge), PREV[s.edge as usize]);
        if self.is_dense(right.hex) {
            right
        } else {
            left
        }
    }

    /// One vertex of `s.hex` chosen by `s.edge`; matches `findPoint`.
    fn find_point(&self, s: Segment) -> Point {
        let side = if s.edge - 1 < 0 {
            5usize
        } else {
            (s.edge - 1) as usize
        };
        let mut pos_y = s.hex.j as f64 * self.height;
        if s.hex.iodd() {
            pos_y += self.height / 2.0;
        }
        let pos_x = s.hex.i as f64 * self.width;
        let off = self.offsets[side];
        Point::new(pos_x + off.x + self.origin.x, pos_y + off.y + self.origin.y)
    }

    /// Discover every boundary path around the dense region(s).
    pub fn find_shapes(&mut self) -> Result<(), String> {
        if self.possible_roots.is_empty() {
            return Err("No areas of sufficient density - no shapes. \
                 Decrease density or area size."
                .to_string());
        }
        while let Some(&root) = self.possible_roots.iter().next() {
            self.find_shape(root);
        }
        Ok(())
    }

    fn find_shape(&mut self, root: HexId) {
        let path_idx = self.paths.len();
        self.paths.push(Path::new(root));

        let first = Segment::new(root, 0);
        let mut cur = first;
        loop {
            if cur.horizontal() {
                if cur.edge == 0 {
                    self.possible_roots.remove(&cur.hex);
                }
                let path_hex = if cur.edge == 0 {
                    cur.hex
                } else {
                    self.edge_hex(cur.hex, 3)
                };
                self.hex_paths.entry(path_hex).or_insert(path_idx);
                if path_hex.j < self.min_y {
                    self.min_y = path_hex.j;
                }
            }
            let pt = self.find_point(cur);
            self.paths[path_idx].points.push(pt);
            cur = self.next_segment(cur);
            if cur == first {
                break;
            }
        }
        let close = self.find_point(cur);
        self.paths[path_idx].points.push(close);
    }

    /// Resolve which paths are holes inside which outer rings.
    pub fn find_parent_paths(&mut self) {
        for idx in 0..self.paths.len() {
            self.parent_or_child(idx);
        }
        for idx in 0..self.paths.len() {
            if self.paths[idx].parent.is_none() {
                self.roots.push(idx);
            } else if let Some(parent) = self.paths[idx].parent {
                self.paths[parent].children.push(idx);
            }
        }
        for &root in &self.roots.clone() {
            self.finalize(root, false);
        }
    }

    /// Walk straight down from a path's root hex, toggling the parent each
    /// time we cross another path. Matches `BaseGrid::parentOrChild` /
    /// `HexGrid::inGrid` / `moveCoord`.
    fn parent_or_child(&mut self, idx: usize) {
        let mut hex = self.paths[idx].root_hex;
        while hex.j >= self.min_y {
            if let Some(&parent_idx) = self.hex_paths.get(&hex) {
                let current_parent = self.paths[idx].parent;
                if current_parent == Some(parent_idx) {
                    self.paths[idx].parent = None;
                } else if current_parent.is_none() && parent_idx != idx {
                    self.paths[idx].parent = Some(parent_idx);
                }
            }
            hex = HexId::new(hex.i, hex.j - 1);
        }
    }

    fn finalize(&mut self, idx: usize, anticlockwise: bool) {
        self.paths[idx].anticlockwise = anticlockwise;
        let children = self.paths[idx].children.clone();
        for child in children {
            self.finalize(child, !anticlockwise);
        }
        if anticlockwise {
            self.paths[idx].points.reverse();
        }
    }

    /// Sort roots/children for deterministic test output, matching
    /// `BaseGrid::sortPaths`.
    pub fn sort_paths(&mut self) {
        self.roots
            .sort_by(|&a, &b| self.paths[a].root_hex.cmp(&self.paths[b].root_hex));
        for idx in 0..self.paths.len() {
            let mut children = self.paths[idx].children.clone();
            children.sort_by(|&a, &b| self.paths[a].root_hex.cmp(&self.paths[b].root_hex));
            self.paths[idx].children = children;
        }
    }

    /// Format the boundary as `MULTIPOLYGON (...)` with the given numeric
    /// precision, matching the C++ classic-locale stream output.
    pub fn to_wkt(&self, precision: usize) -> String {
        self.format_wkt(precision, false)
    }

    /// Like [`to_wkt`], but with `std::fixed`-style precision (exactly
    /// `precision` decimals). Used for `hex_boundary_raw`, which `HexBinFilter`
    /// re-parses and smooths through GEOS — it must match the C++
    /// `HexGrid::toWKT` fixed output so the simplified boundary is identical.
    pub fn to_wkt_fixed(&self, precision: usize) -> String {
        self.format_wkt(precision, true)
    }

    fn format_wkt(&self, precision: usize, fixed: bool) -> String {
        let mut out = String::from("MULTIPOLYGON (");
        for (i, &root) in self.roots.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            self.write_path_polygon(root, precision, fixed, &mut out);
        }
        out.push(')');
        out
    }

    fn write_path_polygon(&self, idx: usize, precision: usize, fixed: bool, out: &mut String) {
        let islands = self.write_polygon(idx, precision, fixed, out);
        let mut pending = islands;
        while !pending.is_empty() {
            let next = std::mem::take(&mut pending);
            for sub in next {
                out.push_str(", ");
                pending.extend(self.write_polygon(sub, precision, fixed, out));
            }
        }
    }

    fn write_polygon(&self, idx: usize, precision: usize, fixed: bool, out: &mut String) -> Vec<usize> {
        let mut islands = Vec::new();
        out.push('(');
        self.write_ring(idx, precision, fixed, out);
        for &child in &self.paths[idx].children {
            out.push_str(", ");
            self.write_ring(child, precision, fixed, out);
            islands.extend(self.paths[child].children.iter().copied());
        }
        out.push(')');
        islands
    }

    fn write_ring(&self, idx: usize, precision: usize, fixed: bool, out: &mut String) {
        let pts = &self.paths[idx].points;
        debug_assert!(pts.len() > 2);
        out.push('(');
        write_point(out, pts[0], precision, fixed);
        for pt in &pts[1..] {
            out.push_str(", ");
            write_point(out, *pt, precision, fixed);
        }
        out.push(')');
    }

    #[cfg(test)]
    fn root_count(&self) -> usize {
        self.roots.len()
    }
}

impl H3Grid {
    pub fn new(resolution: u8, dense_limit: i32, origin: CellIndex) -> Result<Self, String> {
        Resolution::try_from(resolution).map_err(|err| format!("Invalid H3 resolution: {err}"))?;
        Ok(Self {
            dense_limit,
            origin,
            counts: HashMap::new(),
            possible_roots: BTreeSet::new(),
            hex_paths: HashMap::new(),
            paths: Vec::new(),
            roots: Vec::new(),
            min_i: i32::MAX,
        })
    }

    pub fn origin_from_degrees(lat: f64, lng: f64, resolution: u8) -> Result<CellIndex, String> {
        let resolution = Resolution::try_from(resolution)
            .map_err(|err| format!("Invalid H3 resolution: {err}"))?;
        LatLng::new(lat, lng)
            .map(|ll| ll.to_cell(resolution))
            .map_err(|err| format!("Invalid H3 origin: {err}"))
    }

    pub fn set_hexes(&mut self, hexes: &[HexId]) {
        for &h in hexes {
            self.counts.insert(h, self.dense_limit + 1);
            let above = self.edge_hex(h, 0);
            let below = self.edge_hex(h, 3);
            if !self.is_dense(above) {
                self.possible_roots.insert(h);
            }
            self.possible_roots.remove(&below);
        }
    }

    /// The H3 resolution this grid bins at (derived from the origin cell).
    pub fn resolution(&self) -> Resolution {
        self.origin.resolution()
    }

    /// Bin a point given in degrees (lng = x, lat = y) into its H3 cell,
    /// mirroring `hexer::H3Grid::addXY` + `BaseGrid::addPoint`. The origin cell
    /// fixes the local IJ frame and the resolution, so this must be called only
    /// after the grid is constructed at the chosen resolution.
    pub fn add_lat_lng(&mut self, lat_deg: f64, lng_deg: f64) -> Result<(), String> {
        let cell = LatLng::new(lat_deg, lng_deg)
            .map_err(|err| format!("Invalid lat/lng ({lat_deg}, {lng_deg}): {err}"))?
            .to_cell(self.resolution());
        let h = self.h3_to_ij(cell)?;
        let count = {
            let entry = self.counts.entry(h).or_insert(0);
            *entry += 1;
            *entry
        };
        if count == self.dense_limit {
            let above = self.edge_hex(h, 0);
            let below = self.edge_hex(h, 3);
            if !self.is_dense(above) {
                self.possible_roots.insert(h);
            }
            self.possible_roots.remove(&below);
        }
        Ok(())
    }

    /// All binned hexagons and their point counts (local IJ keys).
    pub fn counts(&self) -> &HashMap<HexId, i32> {
        &self.counts
    }

    pub fn find_shapes(&mut self) -> Result<(), String> {
        if self.possible_roots.is_empty() {
            return Err("No areas of sufficient density - no shapes. \
                 Decrease density or area size."
                .to_string());
        }
        while let Some(&root) = self.possible_roots.iter().next() {
            self.find_shape(root)?;
        }
        Ok(())
    }

    pub fn find_parent_paths(&mut self) {
        for idx in 0..self.paths.len() {
            self.parent_or_child(idx);
        }
        for idx in 0..self.paths.len() {
            if self.paths[idx].parent.is_none() {
                self.roots.push(idx);
            } else if let Some(parent) = self.paths[idx].parent {
                self.paths[parent].children.push(idx);
            }
        }
        for &root in &self.roots.clone() {
            self.finalize(root, false);
        }
    }

    pub fn sort_paths(&mut self) {
        self.roots
            .sort_by(|&a, &b| self.paths[a].root_hex.cmp(&self.paths[b].root_hex));
        for idx in 0..self.paths.len() {
            let mut children = self.paths[idx].children.clone();
            children.sort_by(|&a, &b| self.paths[a].root_hex.cmp(&self.paths[b].root_hex));
            self.paths[idx].children = children;
        }
    }

    pub fn to_wkt(&self, precision: usize) -> String {
        let mut out = String::from("MULTIPOLYGON (");
        for (i, &root) in self.roots.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            self.write_path_polygon(root, precision, &mut out);
        }
        out.push(')');
        out
    }

    fn is_dense(&self, h: HexId) -> bool {
        self.counts.get(&h).copied().unwrap_or(0) >= self.dense_limit
    }

    fn find_shape(&mut self, root: HexId) -> Result<(), String> {
        let path_idx = self.paths.len();
        self.paths.push(Path::new(root));

        let first = Segment::new(root, 0);
        let mut cur = first;
        loop {
            if cur.horizontal() {
                if cur.edge == 0 {
                    self.possible_roots.remove(&cur.hex);
                }
                let path_hex = if cur.edge == 0 {
                    cur.hex
                } else {
                    self.edge_hex(cur.hex, 3)
                };
                self.hex_paths.entry(path_hex).or_insert(path_idx);
                if path_hex.i < self.min_i {
                    self.min_i = path_hex.i;
                }
            }
            let pt = self.find_point(cur)?;
            self.paths[path_idx].points.push(pt);
            cur = self.next_segment(cur);
            if cur == first {
                break;
            }
        }
        let close = self.find_point(cur)?;
        self.paths[path_idx].points.push(close);
        Ok(())
    }

    fn parent_or_child(&mut self, idx: usize) {
        let mut hex = self.paths[idx].root_hex;
        while hex.i >= self.min_i {
            if let Some(&parent_idx) = self.hex_paths.get(&hex) {
                let current_parent = self.paths[idx].parent;
                if current_parent == Some(parent_idx) {
                    self.paths[idx].parent = None;
                } else if current_parent.is_none() && parent_idx != idx {
                    self.paths[idx].parent = Some(parent_idx);
                }
            }
            hex = HexId::new(hex.i - 1, hex.j);
        }
    }

    fn finalize(&mut self, idx: usize, anticlockwise: bool) {
        self.paths[idx].anticlockwise = anticlockwise;
        let children = self.paths[idx].children.clone();
        for child in children {
            self.finalize(child, !anticlockwise);
        }
        if anticlockwise {
            self.paths[idx].points.reverse();
        }
    }

    fn write_path_polygon(&self, idx: usize, precision: usize, out: &mut String) {
        let islands = self.write_polygon(idx, precision, out);
        let mut pending = islands;
        while !pending.is_empty() {
            let next = std::mem::take(&mut pending);
            for sub in next {
                out.push_str(", ");
                pending.extend(self.write_polygon(sub, precision, out));
            }
        }
    }

    fn write_polygon(&self, idx: usize, precision: usize, out: &mut String) -> Vec<usize> {
        let mut islands = Vec::new();
        out.push('(');
        self.write_ring(idx, precision, out);
        for &child in &self.paths[idx].children {
            out.push_str(", ");
            self.write_ring(child, precision, out);
            islands.extend(self.paths[child].children.iter().copied());
        }
        out.push(')');
        islands
    }

    fn write_ring(&self, idx: usize, precision: usize, out: &mut String) {
        let pts = &self.paths[idx].points;
        debug_assert!(pts.len() > 2);
        out.push('(');
        write_point(out, pts[0], precision, false);
        for pt in &pts[1..] {
            out.push_str(", ");
            write_point(out, *pt, precision, false);
        }
        out.push(')');
    }

    fn next_segment(&self, s: Segment) -> Segment {
        const NEXT: [i32; 6] = [1, 2, 3, 4, 5, 0];
        const PREV: [i32; 6] = [5, 0, 1, 2, 3, 4];
        let right = Segment::new(s.hex, NEXT[s.edge as usize]);
        let left = Segment::new(self.edge_hex(s.hex, right.edge), PREV[s.edge as usize]);
        if self.is_dense(left.hex) {
            left
        } else {
            right
        }
    }

    fn edge_hex(&self, hex: HexId, edge: i32) -> HexId {
        const OFFSETS: [HexId; 6] = [
            HexId { i: 1, j: 0 },
            HexId { i: 0, j: -1 },
            HexId { i: -1, j: -1 },
            HexId { i: -1, j: 0 },
            HexId { i: 0, j: 1 },
            HexId { i: 1, j: 1 },
        ];
        hex + OFFSETS[edge as usize]
    }

    fn find_point(&self, segment: Segment) -> Result<Point, String> {
        let origin = self.ij_to_h3(segment.hex)?;
        let neighbor = self.ij_to_h3(self.edge_hex(segment.hex, segment.edge))?;
        let edge = origin
            .edge(neighbor)
            .ok_or_else(|| "Can't get directed edge between H3 cells.".to_string())?;
        let boundary = edge.boundary();
        let point = boundary
            .get(1)
            .ok_or_else(|| "H3 directed edge boundary is missing endpoint.".to_string())?;
        Ok(Point::new(point.lng(), point.lat()))
    }

    fn ij_to_h3(&self, ij: HexId) -> Result<CellIndex, String> {
        CellIndex::try_from(LocalIJ::new(self.origin, CoordIJ::new(ij.i, ij.j)))
            .map_err(|err| format!("Can't convert IJ ({}, {}) to H3Index: {err}", ij.i, ij.j))
    }

    fn h3_to_ij(&self, cell: CellIndex) -> Result<HexId, String> {
        cell.to_local_ij(self.origin)
            .map(|ij| HexId::new(ij.coord.i, ij.coord.j))
            .map_err(|err| format!("Can't convert H3 index to IJ: {err}"))
    }
}

/// Pick an H3 resolution from an estimated hexagon height **in radians**,
/// mirroring `hexer::H3Grid::processHeight`. The caller must convert the
/// degree-space sample height to radians first (H3Grid::addXY stores radian
/// coordinates before `computeHexSize`). Resolutions 1-7 are skipped, so the
/// table entry index is offset by 8; the largest matching entry wins.
pub fn h3_resolution_from_height(height_rad: f64) -> Result<u8, String> {
    const RES_HEIGHTS: [f64; 7] = [2.0, 2.62e-4, 6.28e-5, 2.09e-5, 8.73e-6, 3.32e-6, 1.4e-6];
    let mut res: i32 = -1;
    for (i, &h) in RES_HEIGHTS.iter().take(6).enumerate() {
        if height_rad < h {
            res = i as i32 + 8;
        }
    }
    if res < 0 {
        return Err("unable to calculate H3 grid size!".to_string());
    }
    Ok(res as u8)
}

/// Format a point with the same trimmed-trailing-zero behavior as
/// `Utils::OStringStreamClassicLocale` after `std::ios_base::fixed` + the
/// precision setting (e.g. `4.90748 0.5`, not `4.907480 0.500000`).
fn write_point(out: &mut String, p: Point, precision: usize, fixed: bool) {
    write_double(out, p.x, precision, fixed);
    out.push(' ');
    write_double(out, p.y, precision, fixed);
}

fn write_double(out: &mut String, v: f64, precision: usize, fixed: bool) {
    // `fixed` mirrors `Utils::OStringStreamClassicLocale` with
    // `std::ios_base::fixed`: exactly `precision` digits after the decimal
    // point, trailing zeros kept. This is what `HexBinFilter` feeds to
    // `pdal::Polygon::simplify`, so the Rust `hex_boundary_raw` must use it to
    // match the C++ smoothed boundary byte-for-byte.
    if fixed {
        let _ = write!(out, "{:.*}", precision, v);
        return;
    }
    // Default ostream behavior: `precision` is the number of significant
    // digits, and trailing zeros after the decimal point are removed. Every
    // coordinate produced by HexGrid has |v| < 1e6, so we stay in the %f-style
    // branch.
    if v == 0.0 {
        out.push('0');
        return;
    }
    let abs = v.abs();
    let magnitude = abs.log10().floor() as i32;
    let fractional = (precision as i32 - 1 - magnitude).max(0) as usize;
    let formatted = format!("{:.*}", fractional, v);
    let trimmed = trim_trailing_zeros(&formatted);
    let _ = write!(out, "{trimmed}");
}

fn trim_trailing_zeros(s: &str) -> &str {
    if !s.contains('.') {
        return s;
    }
    let trimmed = s.trim_end_matches('0');
    trimmed.trim_end_matches('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_id_orders_by_j_then_even_before_odd() {
        let a = HexId::new(2, 0);
        let b = HexId::new(1, 0);
        let c = HexId::new(0, 1);
        assert!(a < b);
        assert!(b < c);
    }

    #[test]
    fn hex_id_negative_i_is_treated_as_odd_or_even_correctly() {
        assert!(HexId::new(-1, 0).iodd());
        assert!(HexId::new(-2, 0).ieven());
    }

    /// Byte-for-byte reproduction of `HexbinFilterTest.HexGrid_issue_2507`
    /// from `test/unit/filters/HexbinFilterTest.cpp`.
    #[test]
    fn hexgrid_issue_2507_matches_cpp_wkt() {
        let mut grid = HexGrid::with_height(1.0, 1);
        let hexes: Vec<HexId> = [
            (0, 3),
            (0, 4),
            (0, 5),
            (0, 6),
            (1, 2),
            (1, 6),
            (2, 2),
            (2, 4),
            (2, 5),
            (2, 7),
            (3, 1),
            (3, 3),
            (3, 5),
            (3, 7),
            (4, 1),
            (4, 2),
            (4, 4),
            (4, 5),
            (4, 8),
            (5, 0),
            (5, 2),
            (5, 6),
            (5, 8),
            (6, 1),
            (6, 3),
            (6, 4),
            (6, 8),
            (7, 1),
            (7, 3),
            (7, 4),
            (7, 5),
            (7, 7),
            (8, 2),
            (8, 3),
            (8, 4),
            (8, 5),
            (8, 6),
            (8, 7),
        ]
        .into_iter()
        .map(|(i, j)| HexId::new(i, j))
        .collect();

        grid.set_hexes(&hexes);
        grid.find_shapes().unwrap();
        grid.find_parent_paths();
        grid.sort_paths();
        // C++ test uses the default ostream precision (6 significant digits).
        let wkt = grid.to_wkt(6);

        let expected = "MULTIPOLYGON (((4.90748 0.5, 5.19615 1, 5.7735 1, 6.06218 1.5, 6.63953 1.5, 6.9282 2, 7.50555 2, 7.79423 2.5, 7.50555 3, 7.79423 3.5, 7.50555 4, 7.79423 4.5, 7.50555 5, 7.79423 5.5, 7.50555 6, 7.79423 6.5, 7.50555 7, 7.79423 7.5, 7.50555 8, 6.9282 8, 6.63953 8.5, 6.06218 8.5, 5.7735 9, 5.19615 9, 4.90748 9.5, 4.33013 9.5, 4.04145 9, 3.4641 9, 3.17543 8.5, 2.59808 8.5, 2.3094 8, 1.73205 8, 1.44338 7.5, 0.866025 7.5, 0.57735 7, 0 7, -0.288675 6.5, 0 6, -0.288675 5.5, 0 5, -0.288675 4.5, 0 4, -0.288675 3.5, 0 3, 0.57735 3, 0.866025 2.5, 1.44338 2.5, 1.73205 2, 2.3094 2, 2.59808 1.5, 3.17543 1.5, 3.4641 1, 4.04145 1, 4.33013 0.5, 4.90748 0.5), (4.90748 2.5, 4.33013 2.5, 4.04145 2, 4.33013 1.5, 4.90748 1.5, 5.19615 2, 5.7735 2, 6.06218 2.5, 6.63953 2.5, 6.9282 3, 6.63953 3.5, 6.06218 3.5, 5.7735 3, 5.19615 3, 4.90748 2.5), (1.44338 6.5, 0.866025 6.5, 0.57735 6, 0.866025 5.5, 0.57735 5, 0.866025 4.5, 0.57735 4, 0.866025 3.5, 1.44338 3.5, 1.73205 3, 2.3094 3, 2.59808 2.5, 3.17543 2.5, 3.4641 3, 4.04145 3, 4.33013 3.5, 4.90748 3.5, 5.19615 4, 4.90748 4.5, 5.19615 5, 5.7735 5, 6.06218 5.5, 5.7735 6, 6.06218 6.5, 6.63953 6.5, 6.9282 7, 6.63953 7.5, 6.06218 7.5, 5.7735 8, 5.19615 8, 4.90748 8.5, 4.33013 8.5, 4.04145 8, 3.4641 8, 3.17543 7.5, 2.59808 7.5, 2.3094 7, 1.73205 7, 1.44338 6.5)), ((3.17543 3.5, 3.4641 4, 4.04145 4, 4.33013 4.5, 4.04145 5, 4.33013 5.5, 4.04145 6, 3.4641 6, 3.17543 6.5, 2.59808 6.5, 2.3094 6, 1.73205 6, 1.44338 5.5, 1.73205 5, 1.44338 4.5, 1.73205 4, 2.3094 4, 2.59808 3.5, 3.17543 3.5), (3.17543 5.5, 2.59808 5.5, 2.3094 5, 2.59808 4.5, 3.17543 4.5, 3.4641 5, 3.17543 5.5)), ((4.90748 6.5, 5.19615 7, 4.90748 7.5, 4.33013 7.5, 4.04145 7, 4.33013 6.5, 4.90748 6.5)))";
        assert_eq!(wkt, expected);
    }

    #[test]
    fn finds_no_shapes_when_grid_is_empty() {
        let mut grid = HexGrid::with_height(1.0, 1);
        assert!(grid.find_shapes().is_err());
    }

    #[test]
    fn single_dense_hex_produces_one_root_polygon() {
        let mut grid = HexGrid::with_height(1.0, 1);
        grid.set_hexes(&[HexId::new(0, 0)]);
        grid.find_shapes().unwrap();
        grid.find_parent_paths();
        assert_eq!(grid.root_count(), 1);
    }

    #[test]
    fn trim_trailing_zeros_removes_fractional_padding_but_keeps_integers() {
        assert_eq!(trim_trailing_zeros("4.90748000"), "4.90748");
        assert_eq!(trim_trailing_zeros("5.00000000"), "5");
        assert_eq!(trim_trailing_zeros("0.00000000"), "0");
        assert_eq!(trim_trailing_zeros("-0.28867500"), "-0.288675");
    }
}
