use pdal_core::point::{PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::BTreeMap;

pub struct SplitterFilter {
    length: f64,
    x_origin: f64,
    y_origin: f64,
    buffer: f64,
}

impl SplitterFilter {
    pub fn new(length: f64, x_origin: f64, y_origin: f64, buffer: f64) -> Self {
        Self {
            length,
            x_origin,
            y_origin,
            buffer,
        }
    }
}

impl Filter for SplitterFilter {
    fn name(&self) -> &str {
        "filters.splitter"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut tiles: BTreeMap<(i32, i32), PointView> = BTreeMap::new();
        for idx in 0..view.len() {
            for coord in self.coords_for_point(view, idx) {
                tiles
                    .entry(coord)
                    .or_insert_with(|| view.make_new())
                    .append_point(view, idx);
            }
        }
        Ok(tiles.into_values().collect())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for SplitterFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

impl SplitterFilter {
    fn coords_for_point(&self, view: &PointView, idx: PointId) -> Vec<(i32, i32)> {
        let x = view.get_f64(idx, &pdal_core::point::DimId::X);
        let y = view.get_f64(idx, &pdal_core::point::DimId::Y);
        let xpos = self.position(x, self.x_origin);
        let ypos = self.position(y, self.y_origin);

        let mut coords = vec![(xpos, ypos)];
        if self.buffer > 0.0 {
            if self.square_contains(xpos - 1, ypos, x, y) {
                coords.push((xpos - 1, ypos));
            } else if self.square_contains(xpos + 1, ypos, x, y) {
                coords.push((xpos + 1, ypos));
            }

            if self.square_contains(xpos, ypos - 1, x, y) {
                coords.push((xpos, ypos - 1));
            } else if self.square_contains(xpos, ypos + 1, x, y) {
                coords.push((xpos, ypos + 1));
            }

            if self.square_contains(xpos - 1, ypos - 1, x, y) {
                coords.push((xpos - 1, ypos - 1));
            } else if self.square_contains(xpos - 1, ypos + 1, x, y) {
                coords.push((xpos - 1, ypos + 1));
            } else if self.square_contains(xpos + 1, ypos - 1, x, y) {
                coords.push((xpos + 1, ypos - 1));
            } else if self.square_contains(xpos + 1, ypos + 1, x, y) {
                coords.push((xpos + 1, ypos + 1));
            }
        }
        coords
    }

    fn position(&self, value: f64, origin: f64) -> i32 {
        let delta = value - origin;
        let mut pos = (delta / self.length) as i32;
        if delta < 0.0 {
            pos -= 1;
        }
        pos
    }

    fn square_contains(&self, xpos: i32, ypos: i32, x: f64, y: f64) -> bool {
        let min_x = self.x_origin + xpos as f64 * self.length - self.buffer;
        let max_x = min_x + self.length + 2.0 * self.buffer;
        let min_y = self.y_origin + ypos as f64 * self.length - self.buffer;
        let max_y = min_y + self.length + 2.0 * self.buffer;

        min_x < x && x < max_x && min_y < y && y < max_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout, PointView};
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
    fn splits_points_by_tile_coord() {
        let input = view(&[(0.0, 0.0), (11.0, 0.0), (-1.0, 0.0)]);
        let mut filter = SplitterFilter::new(10.0, 0.0, 0.0, 0.0);
        let outputs = filter.run(&input).unwrap();

        assert_eq!(outputs.len(), 3);
        assert!(outputs.iter().all(|out| out.len() == 1));
    }

    #[test]
    fn includes_buffered_neighbors() {
        let input = view(&[(9.5, 9.5)]);
        let mut filter = SplitterFilter::new(10.0, 0.0, 0.0, 1.0);
        let outputs = filter.run(&input).unwrap();

        assert_eq!(outputs.len(), 4);
        assert!(outputs.iter().all(|out| out.len() == 1));
    }
}
