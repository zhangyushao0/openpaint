use super::*;

#[test]
fn weights_sum_to_one_and_reach_three_deviations() {
    assert_eq!(weights(0.0), [1.0]);
    for sigma in [0.3, 1.0, 4.5, 32.0] {
        let weights = weights(sigma);
        assert_eq!(weights.len(), 2 * reach(sigma) as usize + 1);
        let total: f32 = weights.iter().sum();
        assert!((total - 1.0).abs() < 1e-5, "{sigma}: {total}");
        assert_eq!(weights.first(), weights.last());
    }
    assert_eq!(reach(32.0), MAX_REACH);
    assert_eq!(reach(1.0), 3);
}

#[test]
fn large_deviations_run_on_cells_and_spread_a_little_further() {
    let [small, half, quarter] = [3.9, 4.0, 8.0].map(kernels::factor);
    assert_eq!([small, half, quarter], [1, 2, 4]);
    assert_eq!(extent(3.0), reach(3.0));
    assert_eq!(extent(32.0), MAX_EXTENT);
    let block = PixelRect {
        min: [100, 40],
        max: [300, 90],
    };
    let cells = kernels::Cells::of(block, [400, 200], 32.0);
    assert_eq!(cells.factor, 4);
    assert!(cells.sigma < 8.0 && cells.sigma > 7.9, "{}", cells.sigma);
    assert!(reach(cells.sigma) <= kernels::MAX_CELL_REACH);
    let [x, y, columns, rows] = cells.needed;
    assert!(x * 4 <= 100 && (x + columns) * 4 >= 300 && y * 4 <= 40 && (y + rows) * 4 >= 90);
}
