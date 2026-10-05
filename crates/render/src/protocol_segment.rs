//! A stroke's curve spans on the GPU (S34): the `Segment` op's layout, its tips, and the
//! flattening constants the evaluator shares with `curve.rs`.

use super::{OP_DATA, Op, Opcode};
use crate::curve::{MAX_PIECES, TOLERANCE};

/// The flow at `a`; the flow at `b` follows it.
pub(super) const SEGMENT_FLOW: usize = 6;
pub(super) const SEGMENT_EDGE: usize = 8;
/// The paper contact at `a`; the contact at `b` follows it.
pub(super) const SEGMENT_CONTACT: usize = 9;
/// How the segment deposits: one of `TIPS`, a whole number.
pub(super) const SEGMENT_TIP: usize = 11;
/// The span's two Bézier controls, `[x1, y1, x2, y2]`.
const SEGMENT_CONTROLS: usize = 12;
/// At most how far the curve lies from its chord, and three quarters of its larger second
/// difference, in canvas pixels (`curve::pieces`).
const SEGMENT_FLAT: usize = 16;
const SEGMENT_BEND: usize = 17;

pub(super) const SEGMENT_SLOTS: [(&str, usize); 7] = [
    ("SEGMENT_FLOW", SEGMENT_FLOW),
    ("SEGMENT_EDGE", SEGMENT_EDGE),
    ("SEGMENT_CONTACT", SEGMENT_CONTACT),
    ("SEGMENT_TIP", SEGMENT_TIP),
    ("SEGMENT_CONTROLS", SEGMENT_CONTROLS),
    ("SEGMENT_FLAT", SEGMENT_FLAT),
    ("SEGMENT_BEND", SEGMENT_BEND),
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

pub(super) const TIPS: [(&str, Tip); 3] = [
    ("TIP_COVER", Tip::Cover),
    ("TIP_FLAT", Tip::Flat),
    ("TIP_BELL", Tip::Bell),
];

/// The flattening's tolerance in output pixels and its most pieces, as WGSL constants.
pub(super) fn curve_constants() -> String {
    format!("const TOLERANCE: f32 = {TOLERANCE:?};\nconst MAX_PIECES: u32 = {MAX_PIECES}u;\n")
}

/// One span's geometry and strength, as `Op::segment` lays it out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SegmentOperands {
    pub(crate) a: [f32; 2],
    pub(crate) b: [f32; 2],
    pub(crate) radii: [f32; 2],
    pub(crate) flows: [f32; 2],
    pub(crate) edge: f32,
    pub(crate) contacts: [f32; 2],
    pub(crate) tip: Tip,
    /// The Bézier controls between `a` and `b`.
    pub(crate) controls: [[f32; 2]; 2],
    pub(crate) flat: f32,
    pub(crate) bend: f32,
}

impl Op {
    pub(crate) fn segment(s: &SegmentOperands) -> Self {
        Self::span(Opcode::Segment, s)
    }

    /// A span of the in-progress stroke's end that later samples may still change: evaluated
    /// as a segment but never kept (`OP_TAIL`).
    pub(crate) fn tail(s: &SegmentOperands) -> Self {
        Self::span(Opcode::Tail, s)
    }

    fn span(code: Opcode, s: &SegmentOperands) -> Self {
        let mut data = [0.0; OP_DATA];
        data[..6].copy_from_slice(&[s.a[0], s.a[1], s.b[0], s.b[1], s.radii[0], s.radii[1]]);
        data[SEGMENT_FLOW] = s.flows[0];
        data[SEGMENT_FLOW + 1] = s.flows[1];
        data[SEGMENT_EDGE] = s.edge;
        data[SEGMENT_CONTACT] = s.contacts[0];
        data[SEGMENT_CONTACT + 1] = s.contacts[1];
        data[SEGMENT_TIP] = s.tip as u32 as f32;
        let [[x1, y1], [x2, y2]] = s.controls;
        data[SEGMENT_CONTROLS..SEGMENT_FLAT].copy_from_slice(&[x1, y1, x2, y2]);
        data[SEGMENT_FLAT] = s.flat;
        data[SEGMENT_BEND] = s.bend;
        Self::new(code, &data)
    }
}
