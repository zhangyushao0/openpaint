//! Quantiles and medians for `stroke-stats`.

/// The median, the 90th percentile and the largest.
pub fn pick(mut values: Vec<f32>) -> [f32; 3] {
    values.sort_by(f32::total_cmp);
    let at = |share: f32| values[((values.len() - 1) as f32 * share) as usize];
    [at(0.5), at(0.9), at(1.0)]
}

pub fn quantiles(values: Vec<f32>) -> String {
    let mut values = values;
    values.sort_by(f32::total_cmp);
    let at = |share: f32| values[((values.len() - 1) as f32 * share) as usize];
    format!(
        "p10 {:.2}  p50 {:.2}  p90 {:.2}  p99 {:.2}  max {:.2}",
        at(0.1),
        at(0.5),
        at(0.9),
        at(0.99),
        at(1.0)
    )
}

pub fn median_u32(mut values: Vec<u32>) -> u32 {
    values.sort_unstable();
    values.get(values.len() / 2).copied().unwrap_or(1)
}

pub fn median_f32(mut values: Vec<f32>) -> f32 {
    values.sort_by(f32::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(0.0)
}
