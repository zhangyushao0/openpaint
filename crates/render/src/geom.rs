//! Pixel rectangles (what to evaluate) and float bounds (where an op can reach).

/// A rectangle of target pixels: `min` inclusive, `max` exclusive. It may reach outside the
/// target; evaluation clips it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub min: [i32; 2],
    pub max: [i32; 2],
}

impl PixelRect {
    /// No pixels.
    pub(crate) const NONE: Self = Self {
        min: [0, 0],
        max: [0, 0],
    };

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

    /// Its width and height in pixels (0 for an empty side).
    pub(crate) fn size(&self) -> [u32; 2] {
        [0, 1].map(|axis| (self.max[axis] - self.min[axis]).max(0) as u32)
    }

    /// It widened by `by` pixels each way.
    pub(crate) fn widened(&self, by: i32) -> Self {
        Self {
            min: self.min.map(|value| value.saturating_sub(by)),
            max: self.max.map(|value| value.saturating_add(by)),
        }
    }
}

/// Float bounds in target pixels, `[min_x, min_y, max_x, max_y]`: outside them an op adds
/// nothing to any pixel whose centre lies there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bounds(pub(crate) [f32; 4]);

impl Bounds {
    /// Bounds that reach no pixel: an adjustment over nothing.
    pub(crate) const NOTHING: Self = Self([1.0, 1.0, -1.0, -1.0]);

    /// Whether they reach no pixel.
    pub(crate) fn is_empty(&self) -> bool {
        let [x0, y0, x1, y1] = self.0;
        x1 < x0 || y1 < y0
    }

    /// The smallest bounds holding both.
    pub(crate) fn union(self, Self(b): Self) -> Self {
        let a = self.0;
        Self([
            a[0].min(b[0]),
            a[1].min(b[1]),
            a[2].max(b[2]),
            a[3].max(b[3]),
        ])
    }

    /// Their overlap, `None` when they do not meet.
    pub(crate) fn intersect(self, Self(b): Self) -> Option<Self> {
        let a = self.0;
        let overlap = [
            a[0].max(b[0]),
            a[1].max(b[1]),
            a[2].min(b[2]),
            a[3].min(b[3]),
        ];
        (overlap[0] <= overlap[2] && overlap[1] <= overlap[3]).then_some(Self(overlap))
    }

    /// Widened by `by` canvas pixels each way.
    pub(crate) fn widened(self, by: f32) -> Self {
        let [x0, y0, x1, y1] = self.0;
        Self([x0 - by, y0 - by, x1 + by, y1 + by])
    }

    /// The area of the pixels `rect`, from their top-left corner to past their bottom-right one.
    pub(crate) fn of(rect: &PixelRect) -> Self {
        let [x0, y0] = rect.min.map(|v| v as f32);
        let [x1, y1] = rect.max.map(|v| v as f32);
        Self([x0, y0, x1, y1])
    }

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
