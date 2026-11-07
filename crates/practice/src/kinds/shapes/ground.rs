//! A single shape's ground faded towards the paper (S99, D103): outside its outline the picture
//! keeps only part of its strength, so the object stands whole against its frame and is read by
//! its negative space, as V大 means, the ground still faint for context. The picture's alpha is
//! scaled, so the paper under it shows through; along the outline by how much of each pixel lies
//! inside it, so the edge stays smooth.

use crate::maths::path::Point;

/// Rows sampled in each pixel row along the outline (across, a pixel's share is exact).
const ROWS: usize = 4;

/// `rgba` (8-bit RGBA with straight alpha, `size` pixels) with what lies outside `outline`
/// (closed lines in the picture's pixels; a line inside another is a hole) at `kept` of its
/// strength.
#[must_use]
pub fn faded_ground(size: [u32; 2], rgba: &[u8], outline: &[Vec<Point>], kept: f64) -> Vec<u8> {
    let inside = coverage(size, outline);
    let mut faded = rgba.to_vec();
    for (pixel, share) in faded.as_chunks_mut::<4>().0.iter_mut().zip(inside) {
        let strength = kept + (1.0 - kept) * f64::from(share.min(1.0));
        pixel[3] = (f64::from(pixel[3]) * strength).round() as u8;
    }
    faded
}

/// How much of each pixel of a picture of `size` lies inside `outline` (even-odd), 0 to 1, rows
/// top first.
fn coverage(size: [u32; 2], outline: &[Vec<Point>]) -> Vec<f32> {
    let [width, height] = size.map(|side| side as usize);
    let mut crossings: Vec<Vec<f64>> = vec![Vec::new(); height * ROWS];
    for line in outline.iter().filter(|line| line.len() >= 3) {
        for (at, a) in line.iter().enumerate() {
            let b = line[(at + 1) % line.len()];
            cross(&mut crossings, *a, b);
        }
    }
    let mut inside = vec![0.0_f32; width * height];
    let weight = 1.0 / ROWS as f32;
    for (sub, row) in crossings.iter_mut().enumerate() {
        row.sort_by(f64::total_cmp);
        let pixels = &mut inside[(sub / ROWS) * width..(sub / ROWS + 1) * width];
        for span in row.as_chunks::<2>().0 {
            add_span(pixels, *span, weight);
        }
    }
    inside
}

/// Where the edge from `a` to `b` crosses each sampled row's middle it passes, a row taken from
/// its edge's top inclusive to its bottom exclusive.
fn cross(crossings: &mut [Vec<f64>], a: Point, b: Point) {
    if a[1] == b[1] {
        return;
    }
    let (low, high) = if a[1] < b[1] { (a, b) } else { (b, a) };
    let rows = ROWS as f64;
    let first = (low[1] * rows - 0.5).ceil().max(0.0) as usize;
    let end = ((high[1] * rows - 0.5).ceil().max(0.0) as usize).min(crossings.len());
    for (sub, row) in crossings.iter_mut().enumerate().take(end).skip(first) {
        let y = (sub as f64 + 0.5) / rows;
        let x = low[0] + (y - low[1]) * (high[0] - low[0]) / (high[1] - low[1]);
        row.push(x);
    }
}

/// `weight` added to the pixels of `row` the span `[from, to)` covers, a pixel at its ends by its
/// share.
fn add_span(row: &mut [f32], [from, to]: [f64; 2], weight: f32) {
    let width = row.len() as f64;
    let (from, to) = (from.clamp(0.0, width), to.clamp(0.0, width));
    if to <= from {
        return;
    }
    let (first, last) = (from.floor() as usize, to.floor() as usize);
    if first == last {
        row[first.min(row.len() - 1)] += (to - from) as f32 * weight;
        return;
    }
    row[first] += (first as f64 + 1.0 - from) as f32 * weight;
    for pixel in &mut row[first + 1..last] {
        *pixel += weight;
    }
    if let Some(pixel) = row.get_mut(last) {
        *pixel += (to - last as f64) as f32 * weight;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(low: f64, high: f64) -> Vec<Point> {
        vec![[low, low], [high, low], [high, high], [low, high]]
    }

    #[test]
    fn a_square_covers_its_pixels_and_half_of_those_its_edge_halves() {
        let inside = coverage([10, 10], &[square(2.5, 7.5)]);
        assert_eq!(inside[5 * 10 + 5], 1.0);
        assert_eq!(inside[0], 0.0);
        assert!((inside[5 * 10 + 2] - 0.5).abs() < 1e-6, "{}", inside[52]);
        assert!((inside[2 * 10 + 2] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn a_hole_is_ground() {
        let inside = coverage([20, 20], &[square(2.0, 18.0), square(8.0, 12.0)]);
        assert_eq!(inside[10 * 20 + 10], 0.0);
        assert_eq!(inside[5 * 20 + 5], 1.0);
    }

    #[test]
    fn only_the_ground_fades() {
        let rgba: Vec<u8> = [90, 120, 60, 255].repeat(100);
        let faded = faded_ground([10, 10], &rgba, &[square(2.0, 8.0)], 0.3);
        assert_eq!(faded[4 * (5 * 10 + 5)..][..4], [90, 120, 60, 255]);
        assert_eq!(faded[..4], [90, 120, 60, 77]);
    }
}
