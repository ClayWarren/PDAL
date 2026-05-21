use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::BTreeMap;

/// `((xpos, ypos), view)` tiles keyed by grid cell, as produced by
/// [`SplitterFilter::split`].
pub type Tiles = Vec<((i64, i64), PointView)>;

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

    /// Partition the input into `((xpos, ypos), view)` tiles keyed by grid
    /// cell. The `tile` kernel uses the cell coordinates to name output files;
    /// the `Filter` interface discards them.
    pub fn split(&mut self, input: &PointView) -> Result<Tiles, StageError> {
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

        Ok(views.into_iter().collect())
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
        Ok(self
            .split(input)?
            .into_iter()
            .map(|(_, view)| view)
            .collect())
    }
}

impl Streamable for SplitterFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view(points: &[(f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y) in points {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, *x);
            view.set_f64(id, &DimId::Y, *y);
        }
        view
    }

    #[test]
    fn splits_points_into_grid_cells_with_implicit_origin() {
        let input = view(&[(10.0, 10.0), (11.1, 10.2), (9.2, 9.2)]);
        let mut filter = SplitterFilter::new(1.0, f64::NAN, f64::NAN, 0.0);

        let tiles = filter.split(&input).unwrap();

        let keys: Vec<_> = tiles.iter().map(|(key, _)| *key).collect();
        assert_eq!(keys, vec![(-1, -1), (0, 0), (1, 0)]);
        assert!(tiles.iter().all(|(_, tile)| tile.len() == 1));
    }

    #[test]
    fn buffer_duplicates_points_into_neighboring_tiles() {
        let input = view(&[(0.95, 0.95)]);
        let mut filter = SplitterFilter::new(1.0, 0.0, 0.0, 0.1);

        let tiles = filter.split(&input).unwrap();

        let keys: Vec<_> = tiles.iter().map(|(key, _)| *key).collect();
        assert_eq!(keys, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
    }

    #[test]
    fn rejects_too_large_buffer_and_is_not_streamable() {
        let input = view(&[(0.0, 0.0)]);
        let mut filter = SplitterFilter::new(1.0, 0.0, 0.0, 0.5);

        let err = match filter.split(&input) {
            Ok(_) => panic!("expected oversized buffer to fail"),
            Err(err) => err,
        };
        assert!(err.0.contains("Buffer"));

        let mut stream_view = view(&[(0.0, 0.0)]);
        assert!(!filter.process_one(&mut stream_view, 0));
        filter.reset();
    }
}
