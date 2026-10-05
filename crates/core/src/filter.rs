//! Gaussian Blur and Sharpen (S33, Procreate's, on a layer): adjustments that read neighbouring
//! pixels. They are adjustment records like hue, saturation and brightness (`AdjustEdit`): a
//! value, the selection they were made inside and their place among the layer's strokes; the
//! renderer derives the layer's pixels just before them at canvas resolution and filters those
//! (DECISIONS D59). Their sizes are canvas pixels, so a record looks the same at every output
//! scale.

/// A Gaussian blur of the layer's pixels, alpha too (Procreate's Layer mode), with a standard
/// deviation of `radius` canvas pixels in `0..=MAX_RADIUS`; 0 changes nothing. Procreate's
/// percentage is `radius / MAX_RADIUS`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blur {
    pub radius: f32,
}

impl Blur {
    pub const NONE: Self = Self { radius: 0.0 };
    /// The radius at 100 %.
    pub const MAX_RADIUS: f32 = 32.0;

    /// `radius` clamped to `0..=MAX_RADIUS`; one that is not a number changes nothing.
    #[must_use]
    pub fn new(radius: f32) -> Self {
        let radius = if radius.is_nan() {
            0.0
        } else {
            radius.clamp(0.0, Self::MAX_RADIUS)
        };
        Self { radius }
    }
}

/// Unsharp masking: the layer's pixels plus `amount x STRENGTH` times their difference from
/// their Gaussian blur of standard deviation `RADIUS` canvas pixels, kept to valid colour;
/// `amount` in `0..=1` is Procreate's percentage, 0 changes nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sharpen {
    pub amount: f32,
}

impl Sharpen {
    pub const NONE: Self = Self { amount: 0.0 };
    /// The blur the difference is taken from, in canvas pixels.
    pub const RADIUS: f32 = 1.0;
    /// How much of the difference 100 % adds.
    pub const STRENGTH: f32 = 2.0;

    /// `amount` clamped to `0..=1`; one that is not a number changes nothing.
    #[must_use]
    pub fn new(amount: f32) -> Self {
        let amount = if amount.is_nan() {
            0.0
        } else {
            amount.clamp(0.0, 1.0)
        };
        Self { amount }
    }
}
