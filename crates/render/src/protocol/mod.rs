//! The GPU protocol: the op record a scene's items are on the GPU, shared by the Rust encoder and
//! the raster core's WGSL, which reads its operands (`word`). Defined here once; `prelude`
//! generates the matching WGSL declarations, which are prepended to the raster core's shader, and
//! the layout test proves both sides agree.

/// Number of `f32` operands in one op record.
const OP_DATA: usize = 23;
/// A paint op's grain strength is a whole number in `0..=GRAIN_ONE`, so the shader's grain is
/// exact integer arithmetic on every backend.
pub(crate) const GRAIN_ONE: u32 = 256;

/// Operand slots the shaders read by name, declared to WGSL as constants by `prelude`.
const PAINT_OPACITY: usize = 3;
const PAINT_GRAIN: usize = 4;
const PAINT_GRAIN_SCALE: usize = 5;
/// How the paint combines with the layer: one of `BLENDS`, a whole number.
const PAINT_BLEND: usize = 6;
/// An accumulating stroke's maximum density; 0 for a covering one.
const PAINT_DENSITY: usize = 7;
/// The paper: texels per pixel (0: none), the mip level of one pixel, depth and dust.
const PAINT_PAPER: usize = 8;
/// The selection a stroke is drawn inside: one more than where its mask starts in the mask
/// words (0: none), split like `FILL_MASK`.
const PAINT_CLIP: usize = 12;
/// Where a fill's mask starts in the mask words, split in two whole numbers below 2^16 (high
/// part first) so each stays exact as a float.
const FILL_MASK: usize = 7;
/// A smoothed fill edge's canvas width and height; 0 for a hard edge (a transform's lift).
const FILL_EDGE: usize = 9;
/// Where a patch starts in the patch words, split like `FILL_MASK`.
const PATCH_AT: usize = 6;
/// A smudge (its residual's block in the arena is named by its draw): whether Alpha Lock keeps
/// the layer's alpha; whether it is the one being drawn (its residual is the frame's own). Its
/// clip sits where a paint op's does.
const SMUDGE_LOCKED: usize = 0;
const SMUDGE_ACTIVE: usize = 1;
const SLOTS: [(&str, usize); 12] = [
    ("PAINT_OPACITY", PAINT_OPACITY),
    ("PAINT_GRAIN", PAINT_GRAIN),
    ("PAINT_GRAIN_SCALE", PAINT_GRAIN_SCALE),
    ("PAINT_BLEND", PAINT_BLEND),
    ("PAINT_DENSITY", PAINT_DENSITY),
    ("PAINT_PAPER", PAINT_PAPER),
    ("PAINT_CLIP", PAINT_CLIP),
    ("FILL_MASK", FILL_MASK),
    ("FILL_EDGE", FILL_EDGE),
    ("PATCH_AT", PATCH_AT),
    ("SMUDGE_LOCKED", SMUDGE_LOCKED),
    ("SMUDGE_ACTIVE", SMUDGE_ACTIVE),
];

/// Words before a mask's rows in the mask words, and before a patch's pixels in the patch
/// words: origin x, origin y, width, height.
pub(crate) const MASK_HEADER: u32 = 4;

/// A paint op's blend, declared to WGSL as constants by `prelude`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Blend {
    /// Source over the layer.
    Over = 0,
    /// Source atop the layer: colour changes only where the layer has alpha, which stays.
    Atop = 1,
    /// Destination out: the layer loses the paint's alpha.
    Erase = 2,
}

const BLENDS: [(&str, Blend); 3] = [
    ("BLEND_OVER", Blend::Over),
    ("BLEND_ATOP", Blend::Atop),
    ("BLEND_ERASE", Blend::Erase),
];
/// What an op record is: the kind of item it carries (the raster core's draws read operands by
/// the slots its kind has, the CPU tells kinds apart by it).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Opcode {
    /// One settled span of a stroke's curve, flattened at the output's scale.
    Segment = 0,
    /// A finished stroke's paint: its coverage painted over its layer.
    EndStroke = 1,
    /// A layer's end onto the canvas or into its clipping chain.
    EndLayer = 2,
    /// The stroke being drawn's paint.
    EndActive = 3,
    /// A fill: its colour painted over its layer at the share of the output pixel its mask's
    /// smoothed edge covers (`fill_edge.wgsl`), or, for a hard edge, exactly where its mask holds
    /// the canvas pixel; like a stroke's paint op.
    Fill = 4,
    /// A transform's lifted pixels, sampled bilinearly where the pixel's centre maps back to,
    /// over its layer (premultiplied source over).
    Patch = 5,
    /// A smudge: its layer becomes `A x layer + R` from its residual at the pixel.
    Smudge = 6,
    /// A clipping chain's end onto the canvas by its base's opacity and mode.
    EndChain = 7,
    /// A group's end: its result replaces what lay under it by its opacity (pass-through).
    EndGroup = 8,
    /// A masked layer's paint is set aside and its mask starts (`Masking`).
    BeginMask = 9,
    /// An adjustment of its layer's colours, inside its clip (`protocol/adjust.rs`).
    Adjust = 10,
    /// From here the items lie in a transform record's content, drawn at the pixel's centre
    /// mapped through the frame's rows.
    Frame = 11,
    /// A moved selection's or a merged layer's content starts, from nothing.
    BeginMove = 12,
    /// The moved content is cut to its selection (or the canvas) where the centre maps back,
    /// then, for a selection, laid over what lay under it.
    EndMove = 13,
    /// Its layer becomes a filter's result, inside its clip (`protocol/adjust.rs`).
    Filter = 14,
    /// A span of the stroke being drawn's end that later samples may still change: laid every
    /// frame, never kept.
    Tail = 15,
    /// A merged layer's content (a nested record) ends: its mask, then its opacity, then by
    /// its mode over (or atop) what its layer held before it.
    EndNested = 16,
}

/// One op. Operands by opcode:
/// - `Segment` and `Tail`: `[ax, ay, bx, by, ra, rb, fa, fb, edge, ca, cb, tip, x1, y1, x2, y2,
///   flat, bend, arc, first, last, paint]`, one span of a stroke's curve in canvas pixels: the
///   cubic Bézier from `a` to `b` with controls `(x1, y1)` and `(x2, y2)`, drawn as
///   `curve::pieces(flat, bend, scale)` tapered capsules of equal parameter, the radius changing
///   evenly from `ra` at `a` to `rb` at `b`; its coverage (`Tip::Cover`) or density (the others)
///   is multiplied by the flow, `fa` to `fb` alike; `edge` is one over the width in canvas pixels
///   over which a covering segment fades in from the outline (1: a hard edge, antialiased over
///   one output pixel at any scale); `ca` to `cb` the paper contact along it
///   (`protocol/segment.rs`). A stamped span (`paint` not 0) is its stamps instead, `ca` to
///   `cb` the pen's azimuth (`protocol/stamp.rs`).
/// - `EndStroke` and `EndActive`: `[r, g, b, opacity, grain, grain_scale, blend, density,
///   paper scale, paper level, depth, dust, clip / 2^16, clip % 2^16, stamp...]`, straight sRGB
///   colour;
///   `grain` a whole number in `0..=GRAIN_ONE`, `grain_scale` whole pixels, `blend` a `Blend`;
///   `density` 0 for a covering stroke, else its maximum density; `paper scale` 0 without
///   paper; `clip` 0, or one more than where the mask of the selection the stroke is drawn
///   inside starts in the mask words: the paint lands only on the mask's pixels.
/// - `EndLayer`: `[opacity, into, mode, grouped, mask]`: `into` a `LayerInto`, `mode` a
///   `Mode`, `grouped` 1 inside a group, `mask` a `Masking` (`protocol/layers.rs`);
///   `EndChain`: `[opacity, _, mode, grouped]`, the base's; `EndGroup`: `[opacity]`;
///   `BeginMask`: none; `EndNested`: `[opacity, into, mode, _, mask]`, `into` `INTO_CLIPPED`
///   for atop, else over.
/// - `Fill`: a paint op's first seven operands, then where its mask starts in the mask words
///   (`offset / 2^16`, `offset % 2^16`), then the canvas size for a smoothed edge (`[0, 0]`:
///   hard); the mask is `MASK_HEADER` words, then rows of `ceil(width / 32)` words, bit
///   `x % 32` of word `x / 32`.
/// - `Patch`: `[a, b, c, d, e, f, at / 2^16, at % 2^16]`: the pixel centre `(x, y)` samples
///   the patch at `(a x + b y + c, d x + e y + f)` in canvas pixels; the patch starts at word
///   `at` of the patch words: `MASK_HEADER` words, then one premultiplied RGBA8 word per
///   pixel, rows top first.
/// - `Smudge`: `[locked, active, ..., clip / 2^16, clip % 2^16]`.
/// - `Adjust`: `[kind, hue, saturation, brightness, on, tables / 2^16, tables % 2^16, ...,
///   clip / 2^16, clip % 2^16]`; `Filter`: `[width, height, ..., clip]`.
/// - `Frame`: `[a, b, c, d, e, f, foot, level, back...]`: ops from here are drawn at `(a x + b y
///   + c, d x + e y + f)` for the centre `(x, y)` on the canvas; `foot` canvas pixels per pixel
///   there, `level` its base-2 log, `back` the six rows mapping content points to the canvas.
///   `BeginMove`: none. `EndMove`: `[mask / 2^16, mask % 2^16, aside, width, height]`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Op {
    code: u32,
    data: [f32; OP_DATA],
}

/// What a stroke's paint op carries: colour, opacity, grain, and how its density turns into
/// coverage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Paint {
    pub(crate) color: [f32; 3],
    pub(crate) opacity: f32,
    /// In `0..=GRAIN_ONE`.
    pub(crate) grain: u32,
    pub(crate) grain_scale: u32,
    pub(crate) blend: Blend,
    /// 0 for a covering stroke; an accumulating one's maximum density.
    pub(crate) density: f32,
    /// Texels per pixel (0: no paper), the mip level a pixel covers, depth, dust.
    pub(crate) paper: [f32; 4],
    /// 0, or one more than where the clip's mask starts in the mask words.
    pub(crate) clip: u32,
    /// A stamped stroke's stamps (`protocol/stamp.rs`); all 0 for the continuous capsule.
    pub(crate) stamp: [f32; STAMP_OPERANDS],
}

impl Paint {
    /// Plain colour: covering, full opacity, no grain; what a fill paints with.
    pub(crate) const fn solid(color: [f32; 3], blend: Blend) -> Self {
        Self {
            color,
            opacity: 1.0,
            grain: 0,
            grain_scale: 1,
            blend,
            density: 0.0,
            paper: [0.0; 4],
            clip: 0,
            stamp: [0.0; STAMP_OPERANDS],
        }
    }

    fn operands(&self) -> [f32; OP_DATA] {
        // Small whole numbers, exact as floats.
        let [r, g, b] = self.color;
        let [scale, level, depth, dust] = self.paper;
        let [clip_high, clip_low] = halves(self.clip);
        let mut operands = [0.0; OP_DATA];
        operands[..PAINT_STAMP].copy_from_slice(&[
            r,
            g,
            b,
            self.opacity,
            self.grain as f32,
            self.grain_scale as f32,
            self.blend as u32 as f32,
            self.density,
            scale,
            level,
            depth,
            dust,
            clip_high,
            clip_low,
        ]);
        operands[PAINT_STAMP..PAINT_STAMP + STAMP_OPERANDS].copy_from_slice(&self.stamp);
        operands
    }
}

/// A word index as two whole numbers below 2^16, high part first, each exact as a float.
pub(crate) fn halves(value: u32) -> [f32; 2] {
    [(value >> 16) as f32, (value & 0xffff) as f32]
}

impl Op {
    pub(crate) fn end_stroke(paint: &Paint) -> Self {
        Self::new(Opcode::EndStroke, &paint.operands())
    }

    pub(crate) fn end_layer(opacity: f32) -> Self {
        Self::new(Opcode::EndLayer, &[opacity])
    }

    pub(crate) fn end_active(paint: &Paint) -> Self {
        Self::new(Opcode::EndActive, &paint.operands())
    }

    /// A fill whose mask starts at word `mask` of the mask words: with the canvas's size its
    /// edge is smoothed, without (a transform's lift) hard.
    pub(crate) fn fill(paint: &Paint, mask: u32, smoothed: Option<[u32; 2]>) -> Self {
        let mut data = [0.0; OP_DATA];
        data[..FILL_MASK].copy_from_slice(&paint.operands()[..FILL_MASK]);
        data[FILL_MASK..FILL_MASK + 2].copy_from_slice(&halves(mask));
        // At most `MAX_CANVAS_SIDE`, exact as floats.
        let [width, height] = smoothed.unwrap_or([0, 0]);
        data[FILL_EDGE..FILL_EDGE + 2].copy_from_slice(&[width as f32, height as f32]);
        Self::new(Opcode::Fill, &data)
    }

    /// A patch starting at word `at` of the patch words, sampled through `rows` (the
    /// placement's inverse).
    pub(crate) fn patch(rows: [f32; 6], at: u32) -> Self {
        let mut data = [0.0; OP_DATA];
        data[..PATCH_AT].copy_from_slice(&rows);
        data[PATCH_AT..PATCH_AT + 2].copy_from_slice(&halves(at));
        Self::new(Opcode::Patch, &data)
    }

    /// A smudge, finished or (`active`) the one being drawn; `clip` as a paint op's.
    pub(crate) fn smudge(active: bool, clip: u32, locked: bool) -> Self {
        let mut data = [0.0; OP_DATA];
        data[SMUDGE_LOCKED] = f32::from(u8::from(locked));
        data[SMUDGE_ACTIVE] = f32::from(u8::from(active));
        data[PAINT_CLIP..PAINT_CLIP + 2].copy_from_slice(&halves(clip));
        Self::new(Opcode::Smudge, &data)
    }

    /// An op no draw refers to; it keeps a scene's item and op indices equal.
    pub(crate) fn unused() -> Self {
        Self::new(Opcode::EndLayer, &[])
    }

    fn new(code: Opcode, operands: &[f32]) -> Self {
        let mut data = [0.0; OP_DATA];
        data[..operands.len()].copy_from_slice(operands);
        Self {
            code: code as u32,
            data,
        }
    }
}

mod records;
pub(crate) use records::{Record, encode};

mod adjust;
pub(crate) use adjust::{AdjustOn, table_words};

mod layers;
pub(crate) use layers::{LayerInto, Masking, Mode};

mod moves;
pub(crate) use moves::MOVE_DEPTH;

mod segment;
use segment::{SEGMENT_SLOTS, TIPS, curve_constants};
pub(crate) use segment::{SegmentOperands, Tip};

mod stamp;
use stamp::{PAINT_STAMP, STAMP_CODES, STAMP_SLOTS};
pub(crate) use stamp::{
    ROTATE_AZIMUTH, ROTATE_DIRECTION, ROTATE_FIXED, ROTATIONS, STAMP_DISC, STAMP_ELLIPSE,
    STAMP_OPERANDS, STAMP_SQUARE, STAMP_TEXTURE, SpanStamps,
};

pub(crate) mod raster;

mod wgsl;
pub(crate) use wgsl::prelude;

#[cfg(test)]
mod tests;
