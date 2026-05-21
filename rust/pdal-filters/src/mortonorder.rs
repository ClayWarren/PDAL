use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct MortonOrderFilter {
    pub reverse: bool,
}

impl MortonOrderFilter {
    pub fn new(reverse: bool) -> Self {
        MortonOrderFilter { reverse }
    }
}

fn less_msb(x: i32, y: i32) -> bool {
    x < y && x < (x ^ y)
}

fn cmp_z_order(c1: &(f64, f64), c2: &(f64, f64)) -> std::cmp::Ordering {
    let a: [i32; 2] = [
        (c1.0 * i32::MAX as f64) as i32,
        (c1.1 * i32::MAX as f64) as i32,
    ];
    let b: [i32; 2] = [
        (c2.0 * i32::MAX as f64) as i32,
        (c2.1 * i32::MAX as f64) as i32,
    ];

    let mut j = 0;
    let mut x = 0;

    for k in 0..2 {
        let y = a[k] ^ b[k];
        if less_msb(x, y) {
            j = k;
            x = y;
        }
    }
    a[j].cmp(&b[j])
}

fn part1_by1(mut x: u32) -> u32 {
    x &= 0x0000ffff;
    x = (x ^ (x << 8)) & 0x00ff00ff;
    x = (x ^ (x << 4)) & 0x0f0f0f0f;
    x = (x ^ (x << 2)) & 0x33333333;
    x = (x ^ (x << 1)) & 0x55555555;
    x
}

fn encode_morton(x: u32, y: u32) -> u32 {
    (part1_by1(y) << 1) + part1_by1(x)
}

fn reverse_morton(mut index: u32) -> u32 {
    index = ((index >> 1) & 0x55555555) | ((index & 0x55555555) << 1);
    index = ((index >> 2) & 0x33333333) | ((index & 0x33333333) << 2);
    index = ((index >> 4) & 0x0f0f0f0f) | ((index & 0x0f0f0f0f) << 4);
    index = ((index >> 8) & 0x00ff00ff) | ((index & 0x00ff00ff) << 8);
    index = ((index >> 16) & 0xffff) | ((index & 0xffff) << 16);
    index
}

impl Filter for MortonOrderFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.mortonorder"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        if input.is_empty() {
            return Ok(vec![input.make_new()]);
        }

        let x_dim = DimId::from_name("X");
        let y_dim = DimId::from_name("Y");

        let mut minx = f64::MAX;
        let mut maxx = f64::MIN;
        let mut miny = f64::MAX;
        let mut maxy = f64::MIN;

        for i in 0..input.len() {
            let x = input.get_f64(i, &x_dim);
            let y = input.get_f64(i, &y_dim);
            if x < minx {
                minx = x;
            }
            if x > maxx {
                maxx = x;
            }
            if y < miny {
                miny = y;
            }
            if y > maxy {
                maxy = y;
            }
        }

        let xrange = if maxx > minx { maxx - minx } else { 1.0 };
        let yrange = if maxy > miny { maxy - miny } else { 1.0 };

        let mut indices: Vec<u64> = (0..input.len()).collect();

        if self.reverse {
            let cell = (input.len() as f64).sqrt() as i32;
            let cell_width = xrange / cell as f64;
            let cell_height = yrange / cell as f64;

            indices.sort_by_key(|&idx| {
                let x = input.get_f64(idx, &x_dim);
                let y = input.get_f64(idx, &y_dim);
                let xpos = ((x - minx) / cell_width).floor() as u32;
                let ypos = ((y - miny) / cell_height).floor() as u32;
                let code = encode_morton(xpos, ypos);
                reverse_morton(code)
            });
        } else {
            indices.sort_by(|&a, &b| {
                let xa = (input.get_f64(a, &x_dim) - minx) / xrange;
                let ya = (input.get_f64(a, &y_dim) - miny) / yrange;
                let xb = (input.get_f64(b, &x_dim) - minx) / xrange;
                let yb = (input.get_f64(b, &y_dim) - miny) / yrange;
                cmp_z_order(&(xa, ya), &(xb, yb))
            });
        }

        let mut out = input.make_new();
        for idx in indices {
            out.append_point(input, idx);
        }

        Ok(vec![out])
    }
}

impl Streamable for MortonOrderFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
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
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Y, *y);
        }
        view
    }

    fn xy(view: &PointView) -> Vec<(f64, f64)> {
        (0..view.len())
            .map(|idx| (view.get_f64(idx, &DimId::X), view.get_f64(idx, &DimId::Y)))
            .collect()
    }

    #[test]
    fn morton_order_sorts_points_by_z_order() {
        let input = view(&[(1.0, 1.0), (0.0, 1.0), (1.0, 0.0), (0.0, 0.0)]);
        let mut filter = MortonOrderFilter::new(false);

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);
        assert_eq!(
            xy(&out),
            vec![(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)]
        );
    }

    #[test]
    fn reverse_morton_order_uses_reversed_codes() {
        let input = view(&[(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0)]);
        let mut filter = MortonOrderFilter::new(true);

        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);
        assert_eq!(out.len(), 4);
        assert_ne!(xy(&out), xy(&input));
    }

    #[test]
    fn empty_input_returns_empty_view_and_streaming_is_unsupported() {
        let input = view(&[]);
        let mut filter = MortonOrderFilter::new(false);
        let out = filter.run(std::slice::from_ref(&input)).unwrap().remove(0);

        assert_eq!(out.len(), 0);
        assert!(!filter.process_one(&mut input.clone(), 0));
    }
}
