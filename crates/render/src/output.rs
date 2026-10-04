//! Where an evaluation's target lies in the output (S29): output pixels per canvas pixel and the
//! output pixel at the target's top-left. Every op is evaluated at the output pixel's centre
//! mapped back to canvas pixels, so the screen's canvas (scale 1, at the origin) and an export at
//! 2x or 4x in bands are one evaluation over different grids. Ops keep their bounds in canvas
//! pixels; the plan maps them here.

use crate::error::RenderError;
use crate::geom::PixelRect;

/// The largest output scale: an export at 4x is 16 output pixels per canvas pixel.
pub const MAX_SCALE: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Output {
    /// Output pixels per canvas pixel along each side: 1, 2 or 4.
    pub(crate) scale: u32,
    /// The output pixel the target's top-left pixel holds.
    pub(crate) origin: [u32; 2],
}

impl Output {
    /// The canvas itself: one target pixel per canvas pixel, from its top-left.
    pub(crate) const CANVAS: Self = Self {
        scale: 1,
        origin: [0, 0],
    };

    /// A target at `origin` of the output at `scale`.
    ///
    /// # Errors
    /// `Invalid` for a scale that is not 1, 2 or 4.
    pub(crate) fn at(scale: u32, origin: [u32; 2]) -> Result<Self, RenderError> {
        if !scale.is_power_of_two() || scale > MAX_SCALE {
            return Err(RenderError::Invalid("an output scale is 1, 2 or 4"));
        }
        Ok(Self { scale, origin })
    }

    /// The target pixels holding the output pixels whose centres lie in the canvas pixels
    /// `rect`. Above scale 1 it is widened by a canvas pixel on every side: a bilinear read of a
    /// source on the canvas grid (a smudge's residual) reaches that far past its nodes. For the
    /// canvas itself it is `rect`.
    pub(crate) fn place(&self, rect: &PixelRect) -> PixelRect {
        let scale = i64::from(self.scale);
        let widen = i64::from(self.scale > 1);
        let side = |value: i32, origin: u32, out: i64| {
            let placed = (i64::from(value) + out) * scale - i64::from(origin);
            placed.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
        };
        PixelRect {
            min: [
                side(rect.min[0], self.origin[0], -widen),
                side(rect.min[1], self.origin[1], -widen),
            ],
            max: [
                side(rect.max[0], self.origin[0], widen),
                side(rect.max[1], self.origin[1], widen),
            ],
        }
    }

    /// The operands the evaluator reads: the origin, the scale and its inverse (exact: a power
    /// of two).
    pub(crate) fn operands(&self) -> ([u32; 2], f32, f32) {
        let scale = self.scale as f32;
        (self.origin, scale, 1.0 / scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_canvas_places_every_rect_as_it_is() {
        let rect = PixelRect {
            min: [-3, 4],
            max: [17, 9],
        };
        assert_eq!(Output::CANVAS.place(&rect), rect);
    }

    #[test]
    fn a_scaled_target_places_canvas_pixels_widened_by_one() {
        let output = Output::at(4, [8, 400]).expect("scale");
        let rect = PixelRect {
            min: [2, 100],
            max: [5, 103],
        };
        let placed = output.place(&rect);
        // Canvas pixels 1..6 and 99..104 (widened by one), times 4, less the origin.
        assert_eq!(placed.min, [4 - 8, 396 - 400]);
        assert_eq!(placed.max, [24 - 8, 416 - 400]);
        let unbounded = PixelRect {
            min: [i32::MIN, i32::MIN],
            max: [i32::MAX, i32::MAX],
        };
        let far = output.place(&unbounded);
        assert_eq!((far.min[0], far.max[0]), (i32::MIN, i32::MAX), "clamped");
        assert!(Output::at(3, [0, 0]).is_err() && Output::at(8, [0, 0]).is_err());
        assert_eq!(Output::at(2, [0, 0]).expect("2x").operands().2, 0.5);
    }
}
