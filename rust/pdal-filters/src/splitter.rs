use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::BTreeMap;

pub struct SplitterFilter {
    length: f64,
    origin_x: f64,
    origin_y: f64,
    buffer: f64,
}

impl SplitterFilter {
    pub fn new(length: f64, origin_x: f64, origin_y: f64, buffer: f64) -> Self {
        Self {
            length,
            origin_x,
            origin_y,
            buffer,
        }
    }

    fn square_contains(&self, xpos: i64, ypos: i64, x: f64, y: f64) -> bool {
        let minx = self.origin_x + xpos as f64 * self.length - self.buffer;
        let maxx = minx + self.length + 2.0 * self.buffer;
        let miny = self.origin_y + ypos as f64 * self.length - self.buffer;
        let maxy = miny + self.length + 2.0 * self.buffer;

        minx < x && x < maxx && miny < y && y < maxy
    }

    fn cell_position(&self, x: f64, y: f64) -> (i64, i64) {
        let dx = x - self.origin_x;
        let mut xpos = (dx / self.length) as i64;
        if dx < 0.0 {
            xpos -= 1;
        }

        let dy = y - self.origin_y;
        let mut ypos = (dy / self.length) as i64;
        if dy < 0.0 {
            ypos -= 1;
        }

        (xpos, ypos)
    }

    fn add_point(
        views: &mut BTreeMap<(i64, i64), PointView>,
        template: &PointView,
        idx: PointId,
        xpos: i64,
        ypos: i64,
    ) {
        views
            .entry((xpos, ypos))
            .or_insert_with(|| template.make_new())
            .append_point(template, idx);
    }
}

impl Filter for SplitterFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.splitter"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        if self.buffer >= self.length / 2.0 {
            return Err(StageError(format!(
                "Buffer ({}) must be less than half of length ({})",
                self.buffer, self.length
            )));
        }

        if self.origin_x.is_nan() {
            self.origin_x = input.get_f64(0, &DimId::X);
        }
        if self.origin_y.is_nan() {
            self.origin_y = input.get_f64(0, &DimId::Y);
        }

        let mut views = BTreeMap::new();
        for idx in 0..input.len() {
            let x = input.get_f64(idx, &DimId::X);
            let y = input.get_f64(idx, &DimId::Y);
            let (xpos, ypos) = self.cell_position(x, y);

            Self::add_point(&mut views, input, idx, xpos, ypos);

            if self.buffer > 0.0 {
                if self.square_contains(xpos - 1, ypos, x, y) {
                    Self::add_point(&mut views, input, idx, xpos - 1, ypos);
                } else if self.square_contains(xpos + 1, ypos, x, y) {
                    Self::add_point(&mut views, input, idx, xpos + 1, ypos);
                }

                if self.square_contains(xpos, ypos - 1, x, y) {
                    Self::add_point(&mut views, input, idx, xpos, ypos - 1);
                } else if self.square_contains(xpos, ypos + 1, x, y) {
                    Self::add_point(&mut views, input, idx, xpos, ypos + 1);
                }

                if self.square_contains(xpos - 1, ypos - 1, x, y) {
                    Self::add_point(&mut views, input, idx, xpos - 1, ypos - 1);
                } else if self.square_contains(xpos - 1, ypos + 1, x, y) {
                    Self::add_point(&mut views, input, idx, xpos - 1, ypos + 1);
                } else if self.square_contains(xpos + 1, ypos - 1, x, y) {
                    Self::add_point(&mut views, input, idx, xpos + 1, ypos - 1);
                } else if self.square_contains(xpos + 1, ypos + 1, x, y) {
                    Self::add_point(&mut views, input, idx, xpos + 1, ypos + 1);
                }
            }
        }

        Ok(views.into_values().collect())
    }
}

impl Streamable for SplitterFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}
