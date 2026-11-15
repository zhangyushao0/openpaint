use super::*;

#[test]
fn the_tier_is_the_nearest_power_of_two_above_the_square_root_of_two() {
    let tiers: Vec<Option<u32>> = [
        0.5, 1.0, 1.41, 1.42, 2.0, 2.82, 2.84, 4.0, 5.6, 5.7, 8.0, 30.0,
    ]
    .map(tier_for)
    .into();
    let expected = [
        None,
        None,
        None,
        Some(2),
        Some(2),
        Some(2),
        Some(4),
        Some(4),
        Some(4),
    ]
    .into_iter()
    .chain([Some(8), Some(8), Some(8)]);
    assert!(tiers.into_iter().eq(expected));
    assert_eq!(zoom(&[0.25, 0.0, 3.0, 0.0, 0.25, 1.0]), 4.0);
    let turned = std::f32::consts::FRAC_1_SQRT_2 / 2.0;
    let rotated = zoom(&[turned, -turned, 0.0, turned, turned, 0.0]);
    assert!((rotated - 2.0).abs() < 1e-5, "{rotated}");
}
