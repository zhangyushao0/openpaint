//! What a recipe may take and stay one the engine draws (S46): each number the brush editor
//! sets has a range that depends on the others, because a pixel takes at most `MAX_STAMPS`
//! stamps of one span (`core::Shape::fits`: the size's largest factor times the stamp's reach
//! over the spacing, plus 2). The editor's controls stop at these ranges, so a change is never
//! refused with an error. Every range keeps the value the recipe has now.

use std::f32::consts::SQRT_2;

use openpaint_core::{
    MAX_BRUSH_SIZE, MAX_FLOW, MAX_SCATTER, MAX_SPACING, MAX_STAMPS, MIN_ASPECT, MIN_SPACING,
    RecipeParts, ShapeParts, Stamp,
};

use crate::brush_knobs::{Knob, KnobRange};
use crate::brushes::SizeRange;

/// The stamps a span may lay within the reach either way: `MAX_STAMPS` less one at each end.
const BUDGET: f32 = MAX_STAMPS as f32 - 2.0;
/// Ranges stop this share short of the engine's edge, so rounding never crosses it.
const MARGIN: f32 = 1.0e-3;
/// Longest taper the editor's sliders give, in diameters (files may set up to `MAX_TAPER`).
pub const EDITOR_TAPER: f32 = 6.0;
/// Least flow and maximum density the sliders give: the engine needs them above 0.
pub const MIN_FLOW: f32 = 0.05;
pub const MIN_DENSITY: f32 = 0.01;
/// Least diameter the size range's sliders give, in canvas pixels.
pub const MIN_SIZE: f32 = 1.0;

/// The largest factor of a curve.
pub(crate) fn highest(curve: &[[f32; 2]]) -> f32 {
    curve.iter().map(|[_, y]| *y).fold(0.0, f32::max)
}

/// The largest factor the size dynamics give: pressure times speed times tilt at their highest.
pub(crate) fn peak(parts: &RecipeParts) -> f32 {
    highest(&parts.size_pressure) * highest(&parts.size_speed) * highest(&parts.size_tilt)
}

/// How far a stamp's shape reaches at size 1, without its jitter and scatter: a disc's or an
/// ellipse's radius, a square's or a texture's corner (`core::Shape::reach`).
fn extent(shape: &ShapeParts) -> f32 {
    match shape.stamp {
        Stamp::Disc | Stamp::Ellipse => 1.0,
        Stamp::Square | Stamp::Texture(_) => shape.aspect.hypot(1.0),
    }
}

/// A stamp's reach in radii: its shape at its largest size, plus its scatter.
pub(crate) fn reach(shape: &ShapeParts) -> f32 {
    (1.0 + shape.size_jitter) * extent(shape) + SQRT_2 * shape.scatter
}

/// The least spacing at which `shape`'s stamps fit with the size dynamics of `parts`.
pub(crate) fn least_spacing(parts: &RecipeParts, shape: &ShapeParts) -> f32 {
    let needed = peak(parts) * reach(shape) / BUDGET * (1.0 + MARGIN);
    needed.max(MIN_SPACING)
}

/// What the stamps' spacing leaves for their reach: `BUDGET` spacings over the size's peak.
fn reach_allowed(parts: &RecipeParts) -> f32 {
    BUDGET * parts.shape.spacing / peak(parts).max(f32::MIN_POSITIVE) * (1.0 - MARGIN)
}

/// The range of `knob` for a recipe of `parts` and a size range `size`; `None` when the knob
/// does nothing for it now (its control is grey).
pub(crate) fn range(parts: &RecipeParts, size: SizeRange, knob: Knob) -> Option<KnobRange> {
    let stamped = parts.shape.spacing > 0.0;
    let deposit = parts.deposit.as_ref();
    let paper = deposit.is_some_and(|deposit| deposit.paper.is_some());
    let shape = &parts.shape;
    let (low, high, curve) = match knob {
        Knob::Spacing if paper => return None,
        Knob::Spacing => (0.0, MAX_SPACING, Curve::Squared),
        Knob::TaperStart | Knob::TaperEnd => (0.0, EDITOR_TAPER, Curve::Linear),
        Knob::TaperTip | Knob::GrainStrength => (0.0, 1.0, Curve::Linear),
        _ if !stamped && knob.stamps_only() => return None,
        Knob::Aspect if shape.stamp == Stamp::Disc => return None,
        Knob::Aspect => (MIN_ASPECT, aspect_most(parts), Curve::Linear),
        Knob::Angle => (-180.0, 180.0, Curve::Linear),
        Knob::RotationJitter => (0.0, 180.0, Curve::Linear),
        Knob::SizeJitter => {
            let most = (reach_allowed(parts) - SQRT_2 * shape.scatter) / extent(shape) - 1.0;
            (0.0, most.clamp(0.0, 1.0), Curve::Linear)
        }
        Knob::Scatter => {
            let most = (reach_allowed(parts) - (1.0 + shape.size_jitter) * extent(shape)) / SQRT_2;
            (0.0, most.clamp(0.0, MAX_SCATTER), Curve::Linear)
        }
        Knob::Softness if deposit.is_some() => return None,
        Knob::Softness => (0.0, 1.0, Curve::Linear),
        Knob::GrainScale => (1.0, 64.0, Curve::Whole),
        Knob::Flow | Knob::MaxDensity if deposit.is_none() => return None,
        Knob::Flow => (MIN_FLOW, MAX_FLOW, Curve::Squared),
        Knob::MaxDensity => (MIN_DENSITY, 1.0, Curve::Linear),
        Knob::SizeMin => (MIN_SIZE, size.max, Curve::Squared),
        Knob::SizeMax => (size.min, MAX_BRUSH_SIZE, Curve::Squared),
    };
    let now = value(parts, size, knob);
    Some(KnobRange::new(low.min(now), high.max(now), curve))
}

/// How a control's position maps to the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    Linear,
    /// The square of the position: small values get more of the control.
    Squared,
    /// Linear in whole numbers.
    Whole,
}

/// The widest aspect a square or textured stamp may take.
fn aspect_most(parts: &RecipeParts) -> f32 {
    let shape = &parts.shape;
    let corner = (reach_allowed(parts) - SQRT_2 * shape.scatter) / (1.0 + shape.size_jitter);
    let most = (corner * corner - 1.0).max(0.0).sqrt();
    most.clamp(MIN_ASPECT, 1.0)
}

/// The value of `knob` in the recipe `parts` with the size range `size`, in the units its
/// control shows (`Knob`).
pub(crate) fn value(parts: &RecipeParts, size: SizeRange, knob: Knob) -> f32 {
    let deposit = parts.deposit.as_ref();
    let shape = &parts.shape;
    match knob {
        Knob::Spacing => shape.spacing,
        Knob::TaperStart => parts.taper[0],
        Knob::TaperEnd => parts.taper[1],
        Knob::TaperTip => parts.taper[2],
        Knob::Aspect => shape.aspect,
        Knob::Angle => shape.angle,
        Knob::RotationJitter => shape.rotation_jitter,
        Knob::SizeJitter => shape.size_jitter,
        Knob::Scatter => shape.scatter,
        Knob::Softness => parts.softness,
        Knob::GrainStrength => parts.grain_strength,
        Knob::GrainScale => parts.grain_scale as f32,
        Knob::Flow => deposit.map_or(0.0, |deposit| deposit.flow),
        Knob::MaxDensity => deposit.map_or(0.0, |deposit| deposit.max_density),
        Knob::SizeMin => size.min,
        Knob::SizeMax => size.max,
    }
}

/// The highest factor a size curve's points may take while the stamps still fit: the curve's
/// own top (`top`) for a continuous stroke.
pub(crate) fn curve_limit(parts: &RecipeParts, own: &[[f32; 2]], top: f32) -> f32 {
    if parts.shape.spacing <= 0.0 {
        return top;
    }
    let others = peak(parts) / highest(own).max(f32::MIN_POSITIVE);
    let allowed = BUDGET * parts.shape.spacing / reach(&parts.shape) * (1.0 - MARGIN);
    (allowed / others.max(f32::MIN_POSITIVE)).clamp(highest(own).min(top), top)
}
