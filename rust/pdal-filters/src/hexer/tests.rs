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
fn fixed_origin_bins_shifted_tindex_squares_into_three_hexes() {
    for shift in [1.0, 2.0, 3.0] {
        let mut grid = HexGrid::with_height(SQRT_3, 1);
        grid.set_origin(shift - 1.0, shift - 1.0);
        for (x, y) in [
            (shift, shift),
            (shift, shift + 1.0),
            (shift + 1.0, shift),
            (shift + 1.0, shift + 1.0),
        ] {
            grid.add_xy(x, y);
        }

        assert_eq!(grid.counts().len(), 3);
        assert!(grid.is_dense(HexId::new(0, 0)));
        assert!(grid.is_dense(HexId::new(0, 1)));
        assert!(grid.is_dense(HexId::new(1, 0)));
    }
}

#[test]
fn trim_trailing_zeros_removes_fractional_padding_but_keeps_integers() {
    assert_eq!(trim_trailing_zeros("4.90748000"), "4.90748");
    assert_eq!(trim_trailing_zeros("5.00000000"), "5");
    assert_eq!(trim_trailing_zeros("0.00000000"), "0");
    assert_eq!(trim_trailing_zeros("-0.28867500"), "-0.288675");
}
