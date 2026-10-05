//! The CPU reference the filter tests compare with: the renderer's Gaussian weights, a
//! separable blur or unsharp mask of premultiplied 8-bit pixels, and the largest difference.

use openpaint_render::RgbaImage;

/// The Gaussian's weights as the renderer builds them: `exp(-k^2 / 2 sigma^2)` over three
/// deviations each way (at most 96), summing to 1.
pub fn weights(sigma: f32) -> Vec<f32> {
    let reach = ((3.0 * sigma).ceil() as i64).min(96);
    let sigma = f64::from(sigma);
    let raw: Vec<f64> = (-reach..=reach)
        .map(|k| (-((k * k) as f64) / (2.0 * sigma * sigma)).exp())
        .collect();
    let total: f64 = raw.iter().sum();
    raw.iter().map(|weight| (weight / total) as f32).collect()
}

/// `image` (premultiplied) blurred on the CPU, across then down, reads past the edge taking the
/// edge pixel; then for sharpening (`strength` > 0) the pixel plus `strength` times its
/// difference from the blur, kept valid; rounded to 8 bits.
pub fn reference(image: &RgbaImage, sigma: f32, strength: f32) -> Vec<u8> {
    let [width, height] = image.size().map(|side| side as i64);
    let weights = weights(sigma);
    let reach = (weights.len() / 2) as i64;
    let at = |x: i64, y: i64, channel: usize| {
        let i = ((y.clamp(0, height - 1) * width + x.clamp(0, width - 1)) * 4) as usize;
        f32::from(image.pixels()[i + channel]) / 255.0
    };
    let mut across = vec![[0.0f32; 4]; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let sums = &mut across[(y * width + x) as usize];
            for (channel, sum) in sums.iter_mut().enumerate() {
                for k in -reach..=reach {
                    *sum += weights[(k + reach) as usize] * at(x + k, y, channel);
                }
            }
        }
    }
    let mut out = Vec::with_capacity(image.pixels().len());
    for y in 0..height {
        for x in 0..width {
            let mut value = [0.0f32; 4];
            for (channel, slot) in value.iter_mut().enumerate() {
                for k in -reach..=reach {
                    let row = (y + k).clamp(0, height - 1);
                    *slot +=
                        weights[(k + reach) as usize] * across[(row * width + x) as usize][channel];
                }
                if strength > 0.0 {
                    let here = at(x, y, channel);
                    *slot = here + strength * (here - *slot);
                }
            }
            let alpha = value[3].clamp(0.0, 1.0);
            for (channel, v) in value.iter().enumerate() {
                let kept = if channel == 3 {
                    alpha
                } else {
                    v.clamp(0.0, alpha)
                };
                out.push((kept * 255.0 + 0.5).floor() as u8);
            }
        }
    }
    out
}

/// Largest channel difference.
pub fn largest(a: &[u8], b: &[u8]) -> u8 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0)
}
