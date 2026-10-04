//! The GPU protocol: opcodes and record layouts shared by the Rust encoder and the WGSL
//! evaluator. Defined here once; `prelude` generates the matching WGSL declarations, which are
//! prepended to every shader, and the layout test below proves both sides agree.

/// Side of a square evaluation tile in pixels; also the evaluator's workgroup size.
pub(crate) const TILE: u32 = 16;
/// Number of `f32` operands in one op record.
const OP_DATA: usize = 14;
/// A paint op's grain strength is a whole number in `0..=GRAIN_ONE`, so the shader's grain is
/// exact integer arithmetic on every backend.
pub(crate) const GRAIN_ONE: u32 = 256;

/// Operand slots the evaluator reads by name, declared to WGSL as constants by `prelude`.
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
/// The flow at `a`; the flow at `b` follows it.
const SEGMENT_FLOW: usize = 6;
const SEGMENT_EDGE: usize = 8;
/// The paper contact at `a`; the contact at `b` follows it.
const SEGMENT_CONTACT: usize = 9;
/// How the segment deposits: one of `TIPS`, a whole number.
const SEGMENT_TIP: usize = 11;
/// Where a fill's mask starts in the mask words, split in two whole numbers below 2^16 (high
/// part first) so each stays exact as a float.
const FILL_MASK: usize = 7;
/// A smoothed fill edge's canvas width and height (S30); 0 for a hard edge (a transform's lift).
const FILL_EDGE: usize = 9;
/// Where a patch starts in the patch words, split like `FILL_MASK`.
const PATCH_AT: usize = 6;
/// A smudge: the mask word holding where its residual starts in the arena, split like
/// `FILL_MASK`; whether Alpha Lock keeps the layer's alpha; whether it is the in-progress one
/// (its residual is the evaluation's own). Its clip sits where a paint op's does.
const SMUDGE_AT: usize = 0;
const SMUDGE_LOCKED: usize = 2;
const SMUDGE_ACTIVE: usize = 3;
const SLOTS: [(&str, usize); 17] = [
    ("PAINT_OPACITY", PAINT_OPACITY),
    ("PAINT_GRAIN", PAINT_GRAIN),
    ("PAINT_GRAIN_SCALE", PAINT_GRAIN_SCALE),
    ("PAINT_BLEND", PAINT_BLEND),
    ("PAINT_DENSITY", PAINT_DENSITY),
    ("PAINT_PAPER", PAINT_PAPER),
    ("PAINT_CLIP", PAINT_CLIP),
    ("SEGMENT_FLOW", SEGMENT_FLOW),
    ("SEGMENT_EDGE", SEGMENT_EDGE),
    ("SEGMENT_CONTACT", SEGMENT_CONTACT),
    ("SEGMENT_TIP", SEGMENT_TIP),
    ("FILL_MASK", FILL_MASK),
    ("FILL_EDGE", FILL_EDGE),
    ("PATCH_AT", PATCH_AT),
    ("SMUDGE_AT", SMUDGE_AT),
    ("SMUDGE_LOCKED", SMUDGE_LOCKED),
    ("SMUDGE_ACTIVE", SMUDGE_ACTIVE),
];

/// How a segment adds to its stroke at a pixel, declared to WGSL as constants by `prelude`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tip {
    /// Coverage of a soft-edged capsule; the stroke keeps the maximum (D16).
    Cover = 0,
    /// Optical density of a flat cross-section `1 - d^8` over the radii travelled, summed.
    Flat = 1,
    /// As `Flat` with the bell `(1 - d^2)^2`.
    Bell = 2,
}

const TIPS: [(&str, Tip); 3] = [
    ("TIP_COVER", Tip::Cover),
    ("TIP_FLAT", Tip::Flat),
    ("TIP_BELL", Tip::Bell),
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
/// A plan index with this bit set refers to the evaluation's fresh ops (the in-progress
/// stroke's new segments and its `EndActive`), not to the scene's ops.
pub(crate) const FRESH: u32 = 1 << 31;

/// What one op record tells the evaluator to do at a pixel.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Opcode {
    /// Coverage of one stroke segment joins the current stroke's coverage (their maximum).
    Segment = 0,
    /// The current stroke's coverage is painted over the current layer and reset.
    EndStroke = 1,
    /// The current layer is composited over the canvas and reset.
    EndLayer = 2,
    /// The in-progress stroke: the coverage its earlier segments left in the pixel's coverage
    /// texel joins the current coverage (the new segments' maximum), the result is stored back
    /// for the next evaluation, then it is painted like `EndStroke`.
    EndActive = 3,
    /// A fill: its colour painted over the current layer at the share of the output pixel its
    /// mask's smoothed edge covers (`fill_edge.wgsl`), or, for a hard edge, exactly where its
    /// mask holds the canvas pixel; like a stroke's paint op.
    Fill = 4,
    /// A transform's lifted pixels, sampled bilinearly where the pixel's centre maps back to,
    /// over the current layer (premultiplied source over).
    Patch = 5,
    /// A smudge: the current layer becomes `A x layer + R` from its residual at the pixel.
    Smudge = 6,
    /// A clipping chain is composited onto the canvas by its base's opacity and mode, and reset.
    EndChain = 7,
    /// A group's result replaces what lay under it by the group's opacity (pass-through).
    EndGroup = 8,
    /// The current layer's paint is set aside and its mask starts (`Masking`).
    BeginMask = 9,
    /// An adjustment of the current layer's colours, inside its clip (`protocol_adjust.rs`).
    Adjust = 10,
    /// From here the ops are evaluated at the pixel's centre mapped through the frame's rows:
    /// content a transform record shows where it moved it (S31).
    Frame = 11,
    /// The layer so far is set aside and a moved selection's content starts from nothing.
    BeginMove = 12,
    /// The moved content is cut to its selection (or the canvas) where the centre maps back,
    /// then, when set aside, laid over what was set aside.
    EndMove = 13,
}

impl Opcode {
    const ALL: [Self; 14] = [
        Self::Segment,
        Self::EndStroke,
        Self::EndLayer,
        Self::EndActive,
        Self::Fill,
        Self::Patch,
        Self::Smudge,
        Self::EndChain,
        Self::EndGroup,
        Self::BeginMask,
        Self::Adjust,
        Self::Frame,
        Self::BeginMove,
        Self::EndMove,
    ];

    const fn wgsl_name(self) -> &'static str {
        match self {
            Self::Segment => "OP_SEGMENT",
            Self::EndStroke => "OP_END_STROKE",
            Self::EndLayer => "OP_END_LAYER",
            Self::EndActive => "OP_END_ACTIVE",
            Self::Fill => "OP_FILL",
            Self::Patch => "OP_PATCH",
            Self::Smudge => "OP_SMUDGE",
            Self::EndChain => "OP_END_CHAIN",
            Self::EndGroup => "OP_END_GROUP",
            Self::BeginMask => "OP_BEGIN_MASK",
            Self::Adjust => "OP_ADJUST",
            Self::Frame => "OP_FRAME",
            Self::BeginMove => "OP_BEGIN_MOVE",
            Self::EndMove => "OP_END_MOVE",
        }
    }
}

/// One op. Operands by opcode:
/// - `Segment`: `[ax, ay, bx, by, ra, rb, fa, fb, edge, ca, cb, tip]`, a tapered capsule from
///   `a` (radius `ra`) to `b` (radius `rb`) in canvas pixels; its coverage (`Tip::Cover`) or
///   its density (the others) is multiplied by the flow, `fa` at `a` changing evenly to `fb` at
///   `b`; `edge` is one over the width in canvas pixels over which a covering segment fades in
///   from the outline (1: a hard edge, antialiased over one output pixel at any scale); `ca`
///   to `cb` the paper contact along it.
/// - `EndStroke` and `EndActive`: `[r, g, b, opacity, grain, grain_scale, blend, density,
///   paper scale, paper level, depth, dust, clip / 2^16, clip % 2^16]`, straight sRGB colour;
///   `grain` a whole number in `0..=GRAIN_ONE`, `grain_scale` whole pixels, `blend` a `Blend`;
///   `density` 0 for a covering stroke, else its maximum density; `paper scale` 0 without
///   paper; `clip` 0, or one more than where the mask of the selection the stroke is drawn
///   inside starts in the mask words: the paint lands only on the mask's pixels.
/// - `EndLayer`: `[opacity, into, mode, grouped, mask]`: `into` a `LayerInto`, `mode` a
///   `Mode`, `grouped` 1 inside a group, `mask` a `Masking` (`protocol_layers.rs`);
///   `EndChain`: `[opacity, _, mode, grouped]`, the base's; `EndGroup`: `[opacity]`;
///   `BeginMask`: none.
/// - `Fill`: a paint op's first seven operands, then where its mask starts in the mask words
///   (`offset / 2^16`, `offset % 2^16`), then the canvas size for a smoothed edge (`[0, 0]`:
///   hard); the mask is `MASK_HEADER` words, then rows of `ceil(width / 32)` words, bit
///   `x % 32` of word `x / 32`.
/// - `Patch`: `[a, b, c, d, e, f, at / 2^16, at % 2^16]`: the pixel centre `(x, y)` samples
///   the patch at `(a x + b y + c, d x + e y + f)` in canvas pixels; the patch starts at word
///   `at` of the patch words: `MASK_HEADER` words, then one premultiplied RGBA8 word per
///   pixel, rows top first.
/// - `Smudge`: `[at / 2^16, at % 2^16, locked, active, ..., clip / 2^16, clip % 2^16]`.
/// - `Adjust`: `[kind, hue, saturation, brightness, on, tables / 2^16, tables % 2^16, ...,
///   clip / 2^16, clip % 2^16]` (`protocol_adjust.rs`).
/// - `Frame`: `[a, b, c, d, e, f, foot, level]`: ops from here are evaluated at `(a x + b y +
///   c, d x + e y + f)` for the centre `(x, y)` on the canvas; `foot` canvas pixels per pixel
///   there, `level` its base-2 log. `BeginMove`: none. `EndMove`: `[mask / 2^16, mask % 2^16,
///   aside, width, height]`.
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
        }
    }

    fn operands(&self) -> [f32; OP_DATA] {
        // Small whole numbers, exact as floats.
        let [r, g, b] = self.color;
        let [scale, level, depth, dust] = self.paper;
        let [clip_high, clip_low] = halves(self.clip);
        [
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
        ]
    }
}

/// A word index as two whole numbers below 2^16, high part first, each exact as a float.
fn halves(value: u32) -> [f32; 2] {
    [(value >> 16) as f32, (value & 0xffff) as f32]
}

/// One segment's geometry and strength, as `Op::segment` lays it out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SegmentOperands {
    pub(crate) a: [f32; 2],
    pub(crate) b: [f32; 2],
    pub(crate) radii: [f32; 2],
    pub(crate) flows: [f32; 2],
    pub(crate) edge: f32,
    pub(crate) contacts: [f32; 2],
    pub(crate) tip: Tip,
}

impl Op {
    pub(crate) fn segment(s: &SegmentOperands) -> Self {
        let mut data = [0.0; OP_DATA];
        data[..6].copy_from_slice(&[s.a[0], s.a[1], s.b[0], s.b[1], s.radii[0], s.radii[1]]);
        data[SEGMENT_FLOW] = s.flows[0];
        data[SEGMENT_FLOW + 1] = s.flows[1];
        data[SEGMENT_EDGE] = s.edge;
        data[SEGMENT_CONTACT] = s.contacts[0];
        data[SEGMENT_CONTACT + 1] = s.contacts[1];
        data[SEGMENT_TIP] = s.tip as u32 as f32;
        Self::new(Opcode::Segment, &data)
    }

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

    /// A finished smudge whose residual's base is in mask word `at`, or (`None`) the one in
    /// progress; `clip` as a paint op's.
    pub(crate) fn smudge(at: Option<u32>, clip: u32, locked: bool) -> Self {
        let mut data = [0.0; OP_DATA];
        data[SMUDGE_AT..SMUDGE_AT + 2].copy_from_slice(&halves(at.unwrap_or(0)));
        data[SMUDGE_LOCKED] = f32::from(u8::from(locked));
        data[SMUDGE_ACTIVE] = f32::from(u8::from(at.is_none()));
        data[PAINT_CLIP..].copy_from_slice(&halves(clip));
        Self::new(Opcode::Smudge, &data)
    }

    /// An op no plan refers to; it keeps a scene's item and op indices equal.
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

#[path = "protocol_records.rs"]
mod records;
pub(crate) use records::{Params, Record, TileRecord, encode};

#[path = "protocol_adjust.rs"]
mod adjust;
pub(crate) use adjust::{AdjustOn, table_words};

#[path = "protocol_layers.rs"]
mod layers;
pub(crate) use layers::{LayerInto, Masking};

#[path = "protocol_moves.rs"]
mod moves;
pub(crate) use moves::MOVE_DEPTH;

#[path = "protocol_wgsl.rs"]
mod wgsl;
pub(crate) use wgsl::prelude;

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
