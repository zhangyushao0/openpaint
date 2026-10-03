//! The paper under an accumulating flat tip (DECISIONS D38): a height field fixed to the canvas,
//! each texel's value its rank among all of them (uniform in `0..=1`), wrapping at its edges.
//! The sketch paper is ambientCG's Paper001 (CC0) as draweave packed it, halved to 1024 x 512
//! texels and ranked again (`assets/paper-sketch.bin`, one byte per texel, row major).
//!
//! A canvas pixel covers several texels, so the evaluator takes the mean and the variance of
//! the ranks under it from a mip chain (each level the means of 2 x 2 texels of the one below)
//! and integrates the coverage over them (draweave's five-point Gauss-Hermite rule), rather
//! than averaging heights first, which would fade the grain, or reading one texel, which would
//! be per-pixel noise.

/// The sketch paper's ranks.
const SKETCH: &[u8] = include_bytes!("../assets/paper-sketch.bin");
/// `[width, height]` of the finest level in texels; powers of two, so coordinates wrap by mask.
pub(crate) const PAPER_SIZE: [u32; 2] = [1024, 512];
/// Moments are stored as whole numbers of `1 / MOMENT_ONE`.
pub(crate) const MOMENT_ONE: u32 = 65_535;

/// How many levels the chain has: halving both sides until one side is a single texel.
pub(crate) fn level_count() -> u32 {
    PAPER_SIZE[0].min(PAPER_SIZE[1]).ilog2() + 1
}

/// Every level of the chain, finest first, each row major: per texel the mean of the ranks it
/// covers in the low 16 bits and their mean square in the high 16 bits.
pub(crate) fn paper_levels() -> Vec<u32> {
    let [mut width, mut height] = PAPER_SIZE;
    let mut level: Vec<[f32; 2]> = SKETCH
        .iter()
        .map(|&byte| {
            let rank = f32::from(byte) / 255.0;
            [rank, rank * rank]
        })
        .collect();
    let mut words = Vec::with_capacity(level.len() * 4 / 3 + 1);
    for k in 0..level_count() {
        words.extend(level.iter().map(|&moments| pack(moments)));
        if k + 1 == level_count() {
            break;
        }
        let (next_width, next_height) = (width / 2, height / 2);
        let at = |x: u32, y: u32| level[(y * width + x) as usize];
        level = (0..next_height)
            .flat_map(|y| (0..next_width).map(move |x| (x, y)))
            .map(|(x, y)| {
                let four = [
                    at(2 * x, 2 * y),
                    at(2 * x + 1, 2 * y),
                    at(2 * x, 2 * y + 1),
                    at(2 * x + 1, 2 * y + 1),
                ];
                [0, 1].map(|c| four.iter().map(|texel| texel[c]).sum::<f32>() * 0.25)
            })
            .collect();
        (width, height) = (next_width, next_height);
    }
    words
}

fn pack([mean, square]: [f32; 2]) -> u32 {
    let step = |value: f32| (value.clamp(0.0, 1.0) * MOMENT_ONE as f32).round() as u32;
    step(mean) | (step(square) << 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sketch_ranks_are_uniform_and_the_chain_keeps_their_moments() {
        assert_eq!(SKETCH.len(), (PAPER_SIZE[0] * PAPER_SIZE[1]) as usize);
        let mut counts = [0u32; 256];
        for &byte in SKETCH {
            counts[usize::from(byte)] += 1;
        }
        // Ranked: every value holds the same share of the texels.
        assert!(counts.iter().all(|&count| count == counts[0]), "{counts:?}");
        assert_eq!(level_count(), 10);
        let words = paper_levels();
        let texels: u32 = (0..level_count())
            .map(|k| (PAPER_SIZE[0] >> k).max(1) * (PAPER_SIZE[1] >> k).max(1))
            .sum();
        assert_eq!(words.len(), texels as usize);
        // The coarsest level (2 x 1) averages everything: mean 1/2, variance about 1/12.
        for word in &words[words.len() - 2..] {
            let mean = (word & 0xffff) as f32 / MOMENT_ONE as f32;
            let square = (word >> 16) as f32 / MOMENT_ONE as f32;
            assert!((mean - 0.5).abs() < 0.01, "{mean}");
            assert!((square - mean * mean - 1.0 / 12.0).abs() < 0.01, "{square}");
        }
    }
}
