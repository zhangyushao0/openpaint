//! A stamped stroke's stamps on the CPU side (S45, DECISIONS D72): stamp `n` lies `n * spacing *
//! size` canvas pixels along the stroke's path (its length along the knots' chords, the same
//! lengths the taper follows, `stroke_path.rs`), and belongs to the one span whose length range
//! holds it: after the span's start up to its end, the first span from its start. So a stamp is
//! evaluated once however its stroke is cut into spans, and an in-progress stroke's settled
//! spans hold exactly the stamps the finished stroke's do. Where in the span it lies, its jitter
//! and its shape are the evaluator's (`stamp.wgsl`).

use openpaint_core::{Deposit, Rotation, Stamp, StrokeStyle};

use crate::protocol::{
    ROTATE_AZIMUTH, ROTATE_DIRECTION, ROTATE_FIXED, ROTATIONS, STAMP_DISC, STAMP_ELLIPSE,
    STAMP_OPERANDS, STAMP_SQUARE, STAMP_TEXTURE, SpanStamps, halves,
};
use crate::shapes::shape_index;
use crate::stroke_path::Path;

/// The last stamp index a float holds exactly: a stroke lays at most this many stamps.
const LAST_STAMP: f64 = 16_777_215.0;
/// Canvas pixels a texture's mip levels may spread a small stamp beyond its square.
const TEXTURE_SPREAD: f32 = 3.0;

/// The canvas pixels between the stamps of a stroke of `style`; `None` for the continuous
/// capsule.
pub(crate) fn step(style: &StrokeStyle) -> Option<f32> {
    let shape = style.recipe().shape();
    shape.is_stamped().then(|| shape.spacing() * style.size())
}

/// The stamps span `j` of `path` holds, for a stamped stroke of `style`.
pub(crate) fn span_stamps(style: &StrokeStyle, path: &Path, j: usize) -> Option<SpanStamps> {
    let step = f64::from(step(style)?);
    let last_knot = path.knots.len() - 1;
    let (start, end) = (path.along(j), path.along((j + 1).min(last_knot)));
    let first = if j == 0 {
        0.0
    } else {
        (f64::from(start) / step).floor() + 1.0
    };
    let last = (f64::from(end) / step).floor().min(LAST_STAMP);
    Some(SpanStamps {
        arc: start,
        first: first.min(LAST_STAMP + 1.0) as f32,
        last: last as f32,
    })
}

/// The paint op's stamp operands for a stroke of `style` (`protocol_stamp.rs`).
pub(crate) fn paint_stamp(style: &StrokeStyle) -> [f32; STAMP_OPERANDS] {
    let Some(step) = step(style) else {
        return [0.0; STAMP_OPERANDS];
    };
    let shape = style.recipe().shape();
    let stamp = match shape.stamp() {
        Stamp::Disc => STAMP_DISC,
        Stamp::Square => STAMP_SQUARE,
        Stamp::Ellipse => STAMP_ELLIPSE,
        Stamp::Texture(texture) => STAMP_TEXTURE + shape_index(texture),
    };
    let rotation = match shape.rotation() {
        Rotation::Fixed => ROTATE_FIXED,
        Rotation::Direction => ROTATE_DIRECTION,
        Rotation::Azimuth => ROTATE_AZIMUTH,
    };
    let [high, low] = halves(style.seed());
    [
        step,
        (stamp + ROTATIONS * rotation) as f32,
        shape.aspect(),
        shape.scatter(),
        shape.size_jitter(),
        shape.angle().to_radians(),
        shape.rotation_jitter().to_radians(),
        high,
        low,
    ]
}

/// How far a stroke of `style` reaches from its path: in radii there, and in canvas pixels
/// beyond (a texture's mip levels); the round capsule its radius.
pub(crate) fn reach(style: &StrokeStyle) -> [f32; 2] {
    let shape = style.recipe().shape();
    if !shape.is_stamped() {
        return [1.0, 0.0];
    }
    let spread = match shape.stamp() {
        Stamp::Texture(_) => TEXTURE_SPREAD,
        _ => 0.0,
    };
    [shape.reach(), spread]
}

/// What one stamp of an accumulating stroke of `style` lays for the flow: the density of the
/// radii of path it stands for (`2 * spacing`), so a spacing never changes how much a stroke
/// lays; 1 for a covering or continuous one.
pub(crate) fn stamp_flow(style: &StrokeStyle) -> f32 {
    let shape = style.recipe().shape();
    match style.recipe().deposit() {
        Deposit::Accumulate(_) if shape.is_stamped() => 2.0 * shape.spacing(),
        _ => 1.0,
    }
}
