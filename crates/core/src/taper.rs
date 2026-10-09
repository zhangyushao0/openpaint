//! `Taper`: how a stroke narrows towards its ends (S44, DECISIONS D71). Along the stroke's path
//! the radius is multiplied by a factor that rises from the tip's share at the very start to 1
//! over the start length, and falls back to the tip's share over the end length before its end,
//! each by smoothstep. Lengths are in diameters of the stroke's size, so a larger brush tapers
//! over a longer stretch. The end taper needs the stroke's end: the renderer applies it once the
//! pen has lifted.

use crate::error::CoreError;
use crate::value::{invalid, is_unit};

/// Longest taper at either end, in diameters of the stroke's size.
pub const MAX_TAPER: f32 = 64.0;

/// A stroke's taper: `start` and `end` lengths in diameters (0: none at that end) and `tip`, the
/// share of the radius left at the very start and end (0: a point).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Taper {
    start: f32,
    end: f32,
    tip: f32,
}

impl Taper {
    /// No taper at either end: the factor is 1 everywhere.
    pub const NONE: Self = Self {
        start: 0.0,
        end: 0.0,
        tip: 0.0,
    };

    /// # Errors
    /// `InvalidValue` when a length is outside `0..=MAX_TAPER` or `tip` outside `0..=1`.
    pub fn new(start: f32, end: f32, tip: f32) -> Result<Self, CoreError> {
        let length = |value: f32| (0.0..=MAX_TAPER).contains(&value);
        if length(start) && length(end) && is_unit(tip) {
            Ok(Self { start, end, tip })
        } else {
            Err(invalid("brush taper"))
        }
    }

    /// Diameters over which the stroke widens from its start.
    #[must_use]
    pub const fn start(self) -> f32 {
        self.start
    }

    /// Diameters over which the stroke narrows to its end.
    #[must_use]
    pub const fn end(self) -> f32 {
        self.end
    }

    /// The share of the radius at the very start and end.
    #[must_use]
    pub const fn tip(self) -> f32 {
        self.tip
    }

    /// True when it narrows nothing.
    #[must_use]
    pub fn is_none(self) -> bool {
        self.start == 0.0 && self.end == 0.0
    }

    /// The radius factor of a stroke of `diameter` canvas pixels at `along` pixels of its path
    /// from its start and, once it has ended, `left` pixels of its path before its end (`None`
    /// while it is drawn: no end taper yet). The smaller of the two ends' factors, so it never
    /// exceeds 1 and is continuous even where a short stroke's tapers overlap; exactly 1 past
    /// both lengths.
    #[must_use]
    pub fn factor(self, along: f32, left: Option<f32>, diameter: f32) -> f32 {
        let start = self.rise(along, self.start * diameter);
        match left {
            Some(left) => start.min(self.rise(left, self.end * diameter)),
            None => start,
        }
    }

    /// From `tip` at `distance` 0 to exactly 1 at `length` and beyond, by smoothstep.
    fn rise(self, distance: f32, length: f32) -> f32 {
        if length <= 0.0 {
            return 1.0;
        }
        let t = (distance / length).clamp(0.0, 1.0);
        let eased = t * t * (3.0 - 2.0 * t);
        1.0 - (1.0 - self.tip) * (1.0 - eased)
    }
}
