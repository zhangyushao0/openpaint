//! Stamped spans on the GPU (S45, DECISIONS D72): the slots a stamped stroke's paint op and its
//! `Segment` ops add, and the codes of its shapes and rotations (`stamp.wgsl`).
//!
//! A stroke's stamps are the same for every span: they live in its paint op (`EndStroke` or
//! `EndActive`), from `PAINT_STAMP` on: `[step, code, aspect, scatter, size jitter, angle, turn
//! jitter, seed / 2^16, seed % 2^16]`: `step` the canvas pixels between stamps (0: the
//! continuous capsule), `code` a shape plus `ROTATIONS` times a rotation, angles in radians.
//! A span adds which stamps it holds: `STAMP_ARC` the stroke's length at its start (along the
//! knots' chords, `stroke_path.rs`), `STAMP_FIRST` and `STAMP_LAST` the indices of the first
//! and last stamps whose places lie in it (whole numbers below 2^24; none when last < first),
//! and `STAMP_PAINT` where its stroke's paint op lies from it, in ops (0: not stamped).

use super::{OP_DATA, Op};

pub(super) const PAINT_STAMP: usize = 14;
pub(super) const STAMP_ARC: usize = 18;
pub(super) const STAMP_FIRST: usize = 19;
pub(super) const STAMP_LAST: usize = 20;
pub(super) const STAMP_PAINT: usize = 21;
/// Operands of a paint op's stamp.
pub(crate) const STAMP_OPERANDS: usize = 9;
const _: () = assert!(PAINT_STAMP + STAMP_OPERANDS <= OP_DATA && STAMP_PAINT < OP_DATA);

pub(super) const STAMP_SLOTS: [(&str, usize); 5] = [
    ("PAINT_STAMP", PAINT_STAMP),
    ("STAMP_ARC", STAMP_ARC),
    ("STAMP_FIRST", STAMP_FIRST),
    ("STAMP_LAST", STAMP_LAST),
    ("STAMP_PAINT", STAMP_PAINT),
];

/// The shape codes: `STAMP_TEXTURE` plus a texture's place in `ShapeTexture::ALL` for one.
pub(crate) const STAMP_DISC: u32 = 1;
pub(crate) const STAMP_SQUARE: u32 = 2;
pub(crate) const STAMP_ELLIPSE: u32 = 3;
pub(crate) const STAMP_TEXTURE: u32 = 4;
/// A code's rotation is its whole number of `ROTATIONS`: fixed, the direction, the azimuth.
pub(crate) const ROTATIONS: u32 = 8;
pub(crate) const ROTATE_FIXED: u32 = 0;
pub(crate) const ROTATE_DIRECTION: u32 = 1;
pub(crate) const ROTATE_AZIMUTH: u32 = 2;
/// Most straight chords a span's stamps are laid along (`stamp.wgsl`).
pub(super) const STAMP_CHORDS: u32 = 8;

pub(super) const STAMP_CODES: [(&str, u32); 10] = [
    ("STAMP_DISC", STAMP_DISC),
    ("STAMP_SQUARE", STAMP_SQUARE),
    ("STAMP_ELLIPSE", STAMP_ELLIPSE),
    ("STAMP_TEXTURE", STAMP_TEXTURE),
    ("ROTATIONS", ROTATIONS),
    ("ROTATE_FIXED", ROTATE_FIXED),
    ("ROTATE_DIRECTION", ROTATE_DIRECTION),
    ("ROTATE_AZIMUTH", ROTATE_AZIMUTH),
    ("STAMP_CHORDS", STAMP_CHORDS),
    ("MAX_STAMPS", openpaint_core::MAX_STAMPS),
];

/// Which stamps of its stroke a span holds, as `STAMP_ARC`, `STAMP_FIRST` and `STAMP_LAST`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SpanStamps {
    pub(crate) arc: f32,
    pub(crate) first: f32,
    pub(crate) last: f32,
}

impl Op {
    /// A span's stamps and where its stroke's paint op lies from it (`paint` ops on: after it
    /// in the scene, before it in the in-progress stroke's fresh ops).
    pub(super) fn stamped(mut self, stamps: SpanStamps, paint: i32) -> Self {
        self.data[STAMP_ARC] = stamps.arc;
        self.data[STAMP_FIRST] = stamps.first;
        self.data[STAMP_LAST] = stamps.last;
        // Never 0 (a span is not its paint op), and small: exact as a float.
        self.data[STAMP_PAINT] = paint as f32;
        self
    }
}
