//! What kind of work an op is at a pixel (S53), for the frame budget's estimate (`cost.rs`): read
//! from its code and, for a stamped span, from its stroke's paint op, as the evaluator reads them
//! (`stamp.wgsl`: how many stamps a pixel tests; `segment.wgsl`: how many pieces a span is).

use openpaint_core::MAX_STAMPS;

use super::segment::{SEGMENT_BEND, SEGMENT_FLAT};
use super::stamp::{PAINT_STAMP, STAMP_FIRST, STAMP_LAST, STAMP_PAINT};
use super::{Op, Opcode, ROTATIONS, STAMP_SQUARE, STAMP_TEXTURE};
use crate::curve::pieces;

/// One op's work at an output pixel of a tile it is listed in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum OpKind {
    /// A continuous span flattened into this many capsules.
    Span {
        pieces: f32,
    },
    /// A stamped span whose pixels test about this many stamps, textured or of an analytic shape.
    Stamped {
        stamps: f32,
        texture: bool,
    },
    Fill,
    Patch,
    Smudge,
    Filter,
    /// Ends, frames, moves, masks, adjustments: a few operations a pixel.
    Other,
}

impl Op {
    /// Where a stamped span's stroke's paint op lies from it, in ops; `None` for any other op.
    pub(crate) fn stamp_paint(&self) -> Option<i32> {
        let span = self.code == Opcode::Segment as u32 || self.code == Opcode::Tail as u32;
        let offset = self.data[STAMP_PAINT];
        (span && offset != 0.0).then_some(offset as i32)
    }

    /// Its work at a pixel; `paint` is a stamped span's stroke's paint op (`stamp_paint`).
    pub(crate) fn kind(&self, paint: Option<&Self>) -> OpKind {
        let code = |opcode: Opcode| self.code == opcode as u32;
        if code(Opcode::Segment) || code(Opcode::Tail) {
            return match paint.filter(|_| self.stamp_paint().is_some()) {
                Some(paint) => self.stamping(paint),
                None => OpKind::Span {
                    pieces: pieces(self.data[SEGMENT_FLAT], self.data[SEGMENT_BEND], 1.0) as f32,
                },
            };
        }
        match self.code {
            c if c == Opcode::Fill as u32 => OpKind::Fill,
            c if c == Opcode::Patch as u32 => OpKind::Patch,
            c if c == Opcode::Smudge as u32 => OpKind::Smudge,
            c if c == Opcode::Filter as u32 => OpKind::Filter,
            _ => OpKind::Other,
        }
    }

    /// A stamped span's work: the stamps within reach of a pixel's place along it (twice the
    /// reach over the step), no more than the span holds or the evaluator takes.
    fn stamping(&self, paint: &Self) -> OpKind {
        let stamp = &paint.data[PAINT_STAMP..];
        let (step, shape) = (stamp[0], stamp[1] as u32 % ROTATIONS);
        let (aspect, scatter, size_jitter) = (stamp[2], stamp[3], stamp[4]);
        let extent = if shape == STAMP_SQUARE || shape >= STAMP_TEXTURE {
            aspect.mul_add(aspect, 1.0).sqrt()
        } else {
            1.0
        };
        let reach = self.data[4].max(self.data[5])
            * (1.0 + size_jitter).mul_add(extent, std::f32::consts::SQRT_2 * scatter);
        let held = self.data[STAMP_LAST] - self.data[STAMP_FIRST] + 1.0;
        let within = 2.0 * reach / step.max(1e-3) + 1.0;
        OpKind::Stamped {
            stamps: within.min(held).clamp(0.0, MAX_STAMPS as f32),
            texture: shape >= STAMP_TEXTURE,
        }
    }
}
