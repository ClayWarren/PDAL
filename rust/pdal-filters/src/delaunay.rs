//! Delaunay triangulation backing `filters.delaunay`.
//!
//! The C++ filter uses `private/delaunator.hpp`, a port of the mapbox
//! `delaunator` algorithm; this module uses the canonical Rust port of the
//! same algorithm, so a point set in general position yields the same
//! (unique) triangulation.

use delaunator::{triangulate, Point};
use pdal_core::point::PointId;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct DelaunayFilter;

impl DelaunayFilter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DelaunayFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Filter for DelaunayFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.delaunay"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let triangles = triangulate_xy(input);
        let mut output = input.clone();
        let Some(mesh) = output.create_named_mesh("delaunay2d") else {
            return Err(StageError(
                "Unable to create mesh 'delaunay2d'.".to_string(),
            ));
        };
        for triangle in triangles.chunks_exact(3) {
            mesh.add(triangle[2], triangle[1], triangle[0]);
        }
        Ok(vec![output])
    }
}

impl Streamable for DelaunayFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

/// Compute the 2D Delaunay triangulation of a point view's XY coordinates.
///
/// Returns a flat array of vertex indices, three per triangle, in the native
/// order produced by the delaunator algorithm. Winding-order adjustment (the
/// C++ filter reverses each triangle) is left to the caller. An input with
/// fewer than three points, or one whose points are degenerate (e.g. all
/// collinear), yields an empty result.
pub fn triangulate_xy(view: &PointView) -> Vec<u64> {
    let n = view.len();
    if n < 3 {
        return Vec::new();
    }
    let points: Vec<Point> = (0..n)
        .map(|i| Point {
            x: view.get_f64(i, &DimId::X),
            y: view.get_f64(i, &DimId::Y),
        })
        .collect();
    triangulate(&points)
        .triangles
        .into_iter()
        .map(|i| i as u64)
        .collect()
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

    #[test]
    fn fewer_than_three_points_yields_no_triangles() {
        assert!(triangulate_xy(&view(&[(0.0, 0.0), (1.0, 0.0)])).is_empty());
    }

    #[test]
    fn single_triangle() {
        let tris = triangulate_xy(&view(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]));
        assert_eq!(tris.len(), 3);
        let mut sorted = tris.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![0, 1, 2]);
    }

    #[test]
    fn unit_square_makes_two_triangles() {
        let tris = triangulate_xy(&view(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]));
        assert_eq!(tris.len(), 6);
    }

    #[test]
    fn filter_attaches_named_mesh() {
        let input = view(&[(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (1.0, 1.0)]);
        let mut filter = DelaunayFilter::new();

        let output = filter.run_one(&input).unwrap().pop().unwrap();

        assert_eq!(output.len(), input.len());
        assert!(!output.mesh_named("delaunay2d").unwrap().is_empty());
    }
}
