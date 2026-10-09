//! The brush editor's changes (S46): every setting the engine has, set on the current tool's
//! brush. A change is applied to the recipe's plain data and checked by the engine
//! (`BrushRecipe::from_parts`); its values are first held inside their ranges (`brush_fit.rs`),
//! so the engine never refuses one. Some settings follow from others: a shape chosen for a
//! continuous stroke lays stamps; Builds up takes the edge's softness; the paper needs a flat
//! tip and a continuous stroke.

use std::sync::Arc;

use openpaint_core::{
    AccumulationParts, BrushRecipe, MAX_SPACING, PaperParts, PaperTexture, RecipeParts, Rotation,
    ShapeParts, Stamp, Tip,
};

use crate::brush_fit::{self, least_spacing};
use crate::brush_knobs::{Axis, Dynamic, Knob, curve_top};
use crate::brushes::{Brush, SizeRange};

/// One change of the brush the editor shows.
#[derive(Debug, Clone, PartialEq)]
pub enum BrushChange {
    /// A number, held inside its range.
    Set(Knob, f32),
    /// The stamp's shape; on a continuous stroke it lays stamps.
    Stamp(Stamp),
    Rotation(Rotation),
    /// Builds up (`true`) or covers.
    BuildsUp(bool),
    Tip(Tip),
    /// The sketch paper under a flat tip, or none.
    Paper(bool),
    /// A dynamics curve's points `[x, y]` in the axis's units.
    Curve {
        dynamic: Dynamic,
        axis: Axis,
        points: Vec<[f32; 2]>,
    },
    /// A dynamics curve as the built-in the brush descends from has it.
    ResetCurve {
        dynamic: Dynamic,
        axis: Axis,
    },
}

/// Spacing a shape chosen for a continuous stroke starts with, in diameters.
const FIRST_SPACING: f32 = 0.1;
/// Flow and tip Builds up starts with.
const FIRST_FLOW: f32 = 2.0;

/// The paper Paper Sketch lays: the built-in Pencil's (draweave's tuned values).
fn sketch_paper() -> PaperParts {
    PaperParts {
        texture: PaperTexture::Sketch,
        contact: vec![[0.0, 0.4], [1.0, 0.95]],
        scale: 4.5,
        depth: 0.3,
        dust: 0.3,
    }
}

/// `brush` with `change` applied, or `None` when it changes nothing (or a value the engine
/// refuses got through, a bug). `kept` is the stamped shape a stroke made continuous had, which
/// stamps take back; `origin` the recipe of the built-in the brush descends from.
pub(crate) fn edited(
    brush: &Brush,
    change: &BrushChange,
    (kept, origin): (&mut Option<ShapeParts>, &RecipeParts),
) -> Option<Brush> {
    let mut parts = brush.recipe.to_parts();
    let mut size = brush.size;
    match change {
        BrushChange::Set(knob, value) => set(&mut parts, &mut size, (*knob, *value), kept)?,
        BrushChange::Stamp(stamp) => choose_stamp(&mut parts, *stamp, kept)?,
        BrushChange::Rotation(rotation) if parts.shape.spacing > 0.0 => {
            parts.shape.rotation = *rotation;
        }
        BrushChange::BuildsUp(on) => builds_up(&mut parts, *on),
        BrushChange::Tip(tip) => tip_of(&mut parts, *tip)?,
        BrushChange::Paper(on) => paper(&mut parts, *on)?,
        BrushChange::Curve {
            dynamic,
            axis,
            points,
        } => curve(&mut parts, (*dynamic, *axis), points)?,
        BrushChange::ResetCurve { dynamic, axis } => {
            *curve_of(&mut parts, *dynamic, *axis) = curve_of_ref(origin, *dynamic, *axis).to_vec();
        }
        BrushChange::Rotation(_) => return None,
    }
    if parts.shape.spacing > 0.0 {
        let shape = parts.shape;
        parts.shape.spacing = parts.shape.spacing.max(least_spacing(&parts, &shape));
    }
    let recipe = BrushRecipe::from_parts(&parts).ok()?;
    let changed = Brush {
        recipe: Arc::new(recipe),
        size,
        ..brush.clone()
    };
    (changed != *brush).then_some(changed)
}

fn set(
    parts: &mut RecipeParts,
    size: &mut SizeRange,
    (knob, value): (Knob, f32),
    kept: &mut Option<ShapeParts>,
) -> Option<()> {
    if !value.is_finite() {
        return None;
    }
    if knob == Knob::Spacing {
        return spacing(parts, value, kept);
    }
    let value = brush_fit::range(parts, *size, knob)?.clamp(value);
    let shape = &mut parts.shape;
    let deposit = parts.deposit.as_mut();
    match knob {
        Knob::TaperStart => parts.taper[0] = value,
        Knob::TaperEnd => parts.taper[1] = value,
        Knob::TaperTip => parts.taper[2] = value,
        Knob::Aspect => shape.aspect = value,
        Knob::Angle => shape.angle = value,
        Knob::RotationJitter => shape.rotation_jitter = value,
        Knob::SizeJitter => shape.size_jitter = value,
        Knob::Scatter => shape.scatter = value,
        Knob::Softness => parts.softness = value,
        Knob::GrainStrength => parts.grain_strength = value,
        Knob::GrainScale => parts.grain_scale = value as u32,
        Knob::Flow => deposit?.flow = value,
        Knob::MaxDensity => deposit?.max_density = value,
        Knob::SizeMin | Knob::SizeMax => *size = sized(*size, knob, value)?,
        Knob::Spacing => return None,
    }
    Some(())
}

/// The size range with its least or largest diameter at `value`, its first one moved inside.
fn sized(size: SizeRange, knob: Knob, value: f32) -> Option<SizeRange> {
    let (min, max) = match knob {
        Knob::SizeMin => (value, size.max),
        _ => (size.min, value),
    };
    SizeRange::new(min, size.default.clamp(min, max), max)
}

/// Spacing below half the least the stamps fit at makes the stroke continuous (keeping its
/// stamps' shape for later); above, stamps lie that far apart, the shape kept before coming back.
fn spacing(parts: &mut RecipeParts, value: f32, kept: &mut Option<ShapeParts>) -> Option<()> {
    if parts.deposit.as_ref().is_some_and(|d| d.paper.is_some()) {
        return None;
    }
    let mut shape = if parts.shape.spacing > 0.0 {
        parts.shape
    } else {
        kept.unwrap_or(ShapeParts {
            spacing: FIRST_SPACING,
            ..ShapeParts::CONTINUOUS
        })
    };
    let least = least_spacing(parts, &shape);
    if value < least / 2.0 {
        if parts.shape.spacing > 0.0 {
            *kept = Some(parts.shape);
        }
        parts.shape = ShapeParts::CONTINUOUS;
    } else {
        shape.spacing = value.clamp(least, MAX_SPACING.max(least));
        parts.shape = shape;
    }
    Some(())
}

/// A stamp's shape; a continuous stroke starts laying stamps (the shape it had before, or
/// every `FIRST_SPACING` diameters). A disc is always as wide as long.
fn choose_stamp(parts: &mut RecipeParts, stamp: Stamp, kept: &Option<ShapeParts>) -> Option<()> {
    if parts.shape.spacing <= 0.0 {
        if stamp == Stamp::Disc || parts.deposit.as_ref().is_some_and(|d| d.paper.is_some()) {
            return None;
        }
        parts.shape = kept.unwrap_or(ShapeParts {
            spacing: FIRST_SPACING,
            ..ShapeParts::CONTINUOUS
        });
    }
    parts.shape.stamp = stamp;
    if stamp == Stamp::Disc {
        parts.shape.aspect = 1.0;
    }
    Some(())
}

/// Builds up under a flat tip (the edge's softness gone: the tip has its own), or covers.
fn builds_up(parts: &mut RecipeParts, on: bool) {
    match (on, parts.deposit.is_some()) {
        (true, false) => {
            parts.softness = 0.0;
            parts.deposit = Some(AccumulationParts {
                tip: Tip::Flat,
                flow: FIRST_FLOW,
                max_density: 1.0,
                paper: None,
            });
        }
        (false, true) => parts.deposit = None,
        _ => {}
    }
}

/// The tip of a deposit that builds up; a bell takes no paper.
fn tip_of(parts: &mut RecipeParts, tip: Tip) -> Option<()> {
    let deposit = parts.deposit.as_mut()?;
    if tip == Tip::Bell && deposit.paper.is_some() {
        return None;
    }
    deposit.tip = tip;
    Some(())
}

/// The sketch paper under a flat tip building up a continuous stroke, or none.
fn paper(parts: &mut RecipeParts, on: bool) -> Option<()> {
    let stamped = parts.shape.spacing > 0.0;
    let deposit = parts.deposit.as_mut()?;
    if !on {
        deposit.paper = None;
    } else if deposit.paper.is_none() && deposit.tip == Tip::Flat && !stamped {
        deposit.paper = Some(sketch_paper());
    }
    Some(())
}

/// A dynamics curve: up to its axis's largest value and its highest allowed factor, its points'
/// x apart in order (the engine's `Curve`).
fn curve(
    parts: &mut RecipeParts,
    (dynamic, axis): (Dynamic, Axis),
    points: &[[f32; 2]],
) -> Option<()> {
    let top = curve_top(dynamic, axis);
    let own = curve_of_ref(parts, dynamic, axis).to_vec();
    let limit = match dynamic {
        Dynamic::Size => brush_fit::curve_limit(parts, &own, top),
        Dynamic::Opacity => top,
    };
    let held: Vec<[f32; 2]> = points
        .iter()
        .map(|[x, y]| [x.clamp(0.0, axis.most()), y.clamp(0.0, limit)])
        .collect();
    let ordered = held.windows(2).all(|pair| pair[0][0] < pair[1][0]);
    if held.is_empty() || !ordered || points.iter().flatten().any(|v| !v.is_finite()) {
        return None;
    }
    *curve_of(parts, dynamic, axis) = held;
    Some(())
}

pub(crate) fn curve_of(
    parts: &mut RecipeParts,
    dynamic: Dynamic,
    axis: Axis,
) -> &mut Vec<[f32; 2]> {
    match (dynamic, axis) {
        (Dynamic::Size, Axis::Pressure) => &mut parts.size_pressure,
        (Dynamic::Size, Axis::Speed) => &mut parts.size_speed,
        (Dynamic::Size, Axis::Tilt) => &mut parts.size_tilt,
        (Dynamic::Opacity, Axis::Pressure) => &mut parts.opacity_pressure,
        (Dynamic::Opacity, Axis::Speed) => &mut parts.opacity_speed,
        (Dynamic::Opacity, Axis::Tilt) => &mut parts.opacity_tilt,
    }
}

pub(crate) fn curve_of_ref(parts: &RecipeParts, dynamic: Dynamic, axis: Axis) -> &[[f32; 2]] {
    match (dynamic, axis) {
        (Dynamic::Size, Axis::Pressure) => &parts.size_pressure,
        (Dynamic::Size, Axis::Speed) => &parts.size_speed,
        (Dynamic::Size, Axis::Tilt) => &parts.size_tilt,
        (Dynamic::Opacity, Axis::Pressure) => &parts.opacity_pressure,
        (Dynamic::Opacity, Axis::Speed) => &parts.opacity_speed,
        (Dynamic::Opacity, Axis::Tilt) => &parts.opacity_tilt,
    }
}
