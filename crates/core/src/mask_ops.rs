//! Building selections as region masks: a closed polygon (a lasso or a rectangle) rasterised at
//! pixel centres, and the intersection and inverse of masks. Every result is trimmed to its
//! tight bounding rectangle; an empty result is `None`, since a mask is never empty.

use std::sync::Arc;

use crate::region::RegionMask;
use crate::value::CanvasSize;

impl RegionMask {
    /// The canvas pixels whose centres lie inside the closed polygon through `points` (canvas
    /// pixels, the last point joined back to the first), by the even-odd rule, as draweave's
    /// selections fill; points may lie off the canvas. `None` when no pixel centre is inside or
    /// the polygon has fewer than three finite points.
    #[must_use]
    pub fn polygon(canvas: CanvasSize, points: &[[f32; 2]]) -> Option<Self> {
        let points: Vec<[f32; 2]> = points
            .iter()
            .copied()
            .filter(|p| p[0].is_finite() && p[1].is_finite())
            .collect();
        if points.len() < 3 {
            return None;
        }
        let (width, height) = (canvas.width(), canvas.height());
        let low = points.iter().fold(f32::INFINITY, |low, p| low.min(p[1]));
        let high = points
            .iter()
            .fold(f32::NEG_INFINITY, |high, p| high.max(p[1]));
        // Rows whose centres can lie inside.
        let top = first_centre(low).clamp(0, i64::from(height)) as u32;
        let bottom = first_centre(high).clamp(0, i64::from(height)) as u32;
        if top >= bottom {
            return None;
        }
        let stride = width.div_ceil(32) as usize;
        let mut words = vec![0u32; stride * (bottom - top) as usize];
        let mut crossings = Vec::new();
        for y in top..bottom {
            let centre = y as f32 + 0.5;
            crossings.clear();
            for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
                // Half-open: a vertex on the centre line counts once.
                if (a[1] > centre) != (b[1] > centre) {
                    let t = (centre - a[1]) / (b[1] - a[1]);
                    crossings.push(a[0] + t * (b[0] - a[0]));
                }
            }
            crossings.sort_unstable_by(f32::total_cmp);
            let row = &mut words[(y - top) as usize * stride..][..stride];
            for [from, to] in crossings.as_chunks::<2>().0 {
                let start = first_centre(*from).clamp(0, i64::from(width)) as u32;
                let end = first_centre(*to).clamp(0, i64::from(width)) as u32;
                set_range(row, start, end);
            }
        }
        let rows = Self {
            origin: [0, top],
            size: [width, bottom - top],
            words: Arc::from(words),
        };
        rows.trimmed()
    }

    /// The pixels in both masks.
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let start = [0, 1].map(|axis| self.origin[axis].max(other.origin[axis]));
        let end = [0, 1].map(|axis| {
            (self.origin[axis] + self.size[axis]).min(other.origin[axis] + other.size[axis])
        });
        if start[0] >= end[0] || start[1] >= end[1] {
            return None;
        }
        let size = [end[0] - start[0], end[1] - start[1]];
        gather(start, size, |x, y| self.word_at(x, y) & other.word_at(x, y))
    }

    /// The pixels in either mask.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let start = [0, 1].map(|axis| self.origin[axis].min(other.origin[axis]));
        let end = [0, 1].map(|axis| {
            (self.origin[axis] + self.size[axis]).max(other.origin[axis] + other.size[axis])
        });
        let size = [end[0] - start[0], end[1] - start[1]];
        let both = gather(start, size, |x, y| self.word_at(x, y) | other.word_at(x, y));
        // Never empty: both masks hold pixels.
        both.unwrap_or_else(|| self.clone())
    }

    /// The pixels of a `canvas`-sized picture outside the mask.
    #[must_use]
    pub fn inverted(&self, canvas: CanvasSize) -> Option<Self> {
        let size = [canvas.width(), canvas.height()];
        gather([0, 0], size, |x, y| !self.word_at(x, y))
    }

    /// Every pixel of a `canvas`-sized picture.
    #[must_use]
    pub fn whole(canvas: CanvasSize) -> Self {
        let size = [canvas.width(), canvas.height()];
        let stride = size[0].div_ceil(32) as usize;
        let mut words = vec![0u32; stride * size[1] as usize];
        for row in words.chunks_mut(stride) {
            set_range(row, 0, size[0]);
        }
        Self {
            origin: [0, 0],
            size,
            words: Arc::from(words),
        }
    }

    /// Pixels `x .. x + 32` of canvas row `y` as bits (bit `i` is pixel `x + i`), 0 outside.
    fn word_at(&self, x: u32, y: u32) -> u32 {
        let [ox, oy] = self.origin;
        if y < oy || y - oy >= self.size[1] {
            return 0;
        }
        let stride = self.stride() as usize;
        let row = &self.words[(y - oy) as usize * stride..][..stride];
        let local = i64::from(x) - i64::from(ox);
        if local <= -32 || local >= i64::from(self.size[0]) {
            return 0;
        }
        if local < 0 {
            return row[0] << (-local);
        }
        let (index, shift) = ((local / 32) as usize, (local % 32) as u32);
        let low = row[index] >> shift;
        let high = match row.get(index + 1) {
            Some(next) if shift > 0 => next << (32 - shift),
            _ => 0,
        };
        low | high
    }

    /// The same pixels in their tight rectangle; `None` when there are none.
    fn trimmed(&self) -> Option<Self> {
        let stride = self.stride() as usize;
        let (mut min, mut max) = ([u32::MAX; 2], [0u32; 2]);
        for (y, row) in self.words.chunks(stride).enumerate() {
            for (index, &word) in row.iter().enumerate() {
                if word != 0 {
                    let base = index as u32 * 32;
                    let first = base + word.trailing_zeros();
                    let last = base + 31 - word.leading_zeros();
                    min = [min[0].min(first), min[1].min(y as u32)];
                    max = [max[0].max(last + 1), y as u32 + 1];
                }
            }
        }
        if min[0] == u32::MAX {
            return None;
        }
        let origin = [self.origin[0] + min[0], self.origin[1] + min[1]];
        let size = [max[0] - min[0], max[1] - min[1]];
        let stride = size[0].div_ceil(32) as usize;
        let mut words = vec![0u32; stride * size[1] as usize];
        for (y, row) in words.chunks_mut(stride).enumerate() {
            for (index, word) in row.iter_mut().enumerate() {
                *word = self.word_at(origin[0] + index as u32 * 32, origin[1] + y as u32);
            }
            clear_past(row, size[0]);
        }
        Some(Self {
            origin,
            size,
            words: Arc::from(words),
        })
    }
}

/// A mask over the canvas rectangle at `origin` of `size` from `word` (32 pixels from canvas
/// `x` in row `y`, as `word_at` gives them), trimmed.
fn gather(origin: [u32; 2], size: [u32; 2], word: impl Fn(u32, u32) -> u32) -> Option<RegionMask> {
    let stride = size[0].div_ceil(32) as usize;
    let mut words = vec![0u32; stride * size[1] as usize];
    for (y, row) in words.chunks_mut(stride).enumerate() {
        for (index, value) in row.iter_mut().enumerate() {
            *value = word(origin[0] + index as u32 * 32, origin[1] + y as u32);
        }
        clear_past(row, size[0]);
    }
    RegionMask {
        origin,
        size,
        words: Arc::from(words),
    }
    .trimmed()
}

/// The first pixel whose centre lies at or right of (below) `at`.
fn first_centre(at: f32) -> i64 {
    (at - 0.5).ceil() as i64
}

/// Sets bits `start .. end` of a row of words.
fn set_range(row: &mut [u32], start: u32, end: u32) {
    let mut at = start;
    while at < end {
        let (index, shift) = ((at / 32) as usize, at % 32);
        let count = (32 - shift).min(end - at);
        let bits = if count == 32 {
            u32::MAX
        } else {
            ((1u32 << count) - 1) << shift
        };
        row[index] |= bits;
        at += count;
    }
}

/// Clears the bits of a row past its `width` pixels.
fn clear_past(row: &mut [u32], width: u32) {
    let used = width % 32;
    if used > 0
        && let Some(last) = row.last_mut()
    {
        *last &= (1u32 << used) - 1;
    }
}
