//! Pixel rectangles (what to evaluate) and float bounds (where an op can reach).

/// A rectangle of target pixels: `min` inclusive, `max` exclusive. It may reach outside the
/// target; evaluation clips it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub min: [i32; 2],
    pub max: [i32; 2],
}

impl PixelRect {
    /// Every pixel of a target of `size` pixels.
    #[must_use]
    pub fn full(size: [u32; 2]) -> Self {
        let side = |value: u32| i32::try_from(value).unwrap_or(i32::MAX);
        Self {
            min: [0, 0],
            max: [side(size[0]), side(size[1])],
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.max[0] <= self.min[0] || self.max[1] <= self.min[1]
    }

    /// The pixels in both; empty (`is_empty`) when they do not meet.
    #[must_use]
    pub(crate) fn intersect(&self, other: &Self) -> Self {
        Self {
            min: [self.min[0].max(other.min[0]), self.min[1].max(other.min[1])],
            max: [self.max[0].min(other.max[0]), self.max[1].min(other.max[1])],
        }
    }

    /// The smallest rectangle holding both.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: [self.min[0].min(other.min[0]), self.min[1].min(other.min[1])],
            max: [self.max[0].max(other.max[0]), self.max[1].max(other.max[1])],
        }
    }
}

/// Float bounds in target pixels, `[min_x, min_y, max_x, max_y]`: outside them an op adds
/// nothing to any pixel whose centre lies there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bounds(pub(crate) [f32; 4]);

impl Bounds {
    /// The pixels whose area meets the bounds.
    pub(crate) fn pixels(&self) -> PixelRect {
        // `as` saturates, and the coordinates stay far inside `i32` (core's `COORD_LIMIT`).
        let [x0, y0, x1, y1] = self.0;
        PixelRect {
            min: [x0.floor() as i32, y0.floor() as i32],
            max: [x1.floor() as i32 + 1, y1.floor() as i32 + 1],
        }
    }
}
