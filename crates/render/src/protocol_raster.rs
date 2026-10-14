//! What the raster core probe (S57, `raster.rs`) reads of an op on the CPU: how many pieces a
//! continuous span is drawn as, which stamps a stamped span holds, how its stroke deposits, and a
//! layer's end, the same operands the evaluator reads (`segment.wgsl`, `stamp.wgsl`).

use super::layers::{LAYER_INTO, LAYER_MASK, LAYER_MODE};
use super::segment::{SEGMENT_BEND, SEGMENT_FLAT, SEGMENT_TIP};
use super::stamp::{STAMP_FIRST, STAMP_LAST, STAMP_PAINT};
use super::{Blend, Op, PAINT_BLEND, PAINT_PAPER, Tip};
use crate::curve::pieces;

/// A layer's end as the probe draws it: its opacity, and whether it goes onto the canvas by
/// Normal without a mask (the only layer end the probe draws).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayerEnd {
    pub(crate) opacity: f32,
    pub(crate) plain: bool,
}

impl Op {
    /// A span's pieces at `scale` output pixels per canvas pixel (`span_pieces` in WGSL).
    pub(crate) fn span_pieces(&self, scale: f32) -> u32 {
        pieces(self.data[SEGMENT_FLAT], self.data[SEGMENT_BEND], scale)
    }

    /// A span holds stamps of its stroke rather than being a capsule (`STAMP_PAINT` not 0).
    pub(crate) fn is_stamped(&self) -> bool {
        self.data[STAMP_PAINT] != 0.0
    }

    /// The indices of the stamps a stamped span holds, first and last (none when last < first).
    pub(crate) fn stamp_indices(&self) -> Option<(u32, u32)> {
        let (first, last) = (self.data[STAMP_FIRST], self.data[STAMP_LAST]);
        // Whole numbers below 2^24 (`stroke_stamp.rs`), exact as floats.
        (self.is_stamped() && last >= first).then_some((first as u32, last as u32))
    }

    /// A span of a covering stroke: its coverage is a maximum, else a density summed.
    pub(crate) fn covers(&self) -> bool {
        self.data[SEGMENT_TIP] as u32 == Tip::Cover as u32
    }

    /// A paint op's blend with its layer.
    pub(crate) fn paint_blend(&self) -> Blend {
        match self.data[PAINT_BLEND] as u32 {
            1 => Blend::Atop,
            2 => Blend::Erase,
            _ => Blend::Over,
        }
    }

    /// A paint op lays its density through the paper, which needs the stroke's contact.
    pub(crate) fn on_paper(&self) -> bool {
        self.data[PAINT_PAPER] > 0.0
    }

    /// A layer end's opacity and whether it is plain: onto the canvas, Normal, unmasked.
    pub(crate) fn layer_end(&self) -> LayerEnd {
        LayerEnd {
            opacity: self.data[0],
            plain: self.data[LAYER_INTO] == 0.0
                && self.data[LAYER_MODE] == 0.0
                && self.data[LAYER_MASK] == 0.0,
        }
    }
}
