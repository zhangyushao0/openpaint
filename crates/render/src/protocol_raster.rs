//! What the raster core (S57's probe made product code in S59, `raster.rs`) reads of an op on
//! the CPU: how many pieces a continuous span is drawn as, which stamps a stamped span holds, how
//! its stroke deposits, what a region edit draws and how a layer ends, the same operands its
//! shaders read (`segment.wgsl`, `stamp.wgsl`, `raster_paint.wgsl`).

use super::layers::Mode;
use super::layers::{LAYER_INTO, LAYER_MASK, LAYER_MODE};
use super::segment::{SEGMENT_BEND, SEGMENT_FLAT, SEGMENT_TIP};
use super::stamp::{STAMP_FIRST, STAMP_LAST, STAMP_PAINT};
use super::{Blend, LayerInto, Masking, Op, Opcode, PAINT_BLEND, PAINT_PAPER, Tip};
use crate::curve::pieces;

/// What a region edit's op draws over its layer (`Item::Fill`, `Item::Clear`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edit {
    /// A fill (or a lift's erasing fill): its colour inside its mask's edge, by its blend.
    Fill,
    /// A transform's patch, over the layer.
    Patch,
    Adjust,
    Filter,
    /// Any other op (none is listed as an edit).
    Other,
}

/// How a layer (or a chain) ends, as the raster core draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerEnd {
    pub(crate) into: LayerInto,
    pub(crate) mode: Mode,
    pub(crate) masking: Masking,
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

    /// A paint op's (or a fill's) blend with its layer.
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

    /// What a region edit's op draws.
    pub(crate) fn edit(&self) -> Edit {
        match self.code {
            c if c == Opcode::Fill as u32 => Edit::Fill,
            c if c == Opcode::Patch as u32 => Edit::Patch,
            c if c == Opcode::Adjust as u32 => Edit::Adjust,
            c if c == Opcode::Filter as u32 => Edit::Filter,
            _ => Edit::Other,
        }
    }

    /// A layer end's (or a chain end's) place, mode and masking. A chain's end goes onto the
    /// canvas without a mask (its operands there are 0).
    pub(crate) fn layer_end(&self) -> LayerEnd {
        let into = match self.data[LAYER_INTO] as u32 {
            1 => LayerInto::Base,
            2 => LayerInto::Clipped,
            _ => LayerInto::Canvas,
        };
        let mode = match self.data[LAYER_MODE] as u32 {
            1 => Mode::Multiply,
            2 => Mode::Screen,
            3 => Mode::Overlay,
            _ => Mode::Normal,
        };
        let masking = match self.data[LAYER_MASK] as u32 {
            1 => Masking::Shown,
            2 => Masking::Inverted,
            _ => Masking::None,
        };
        LayerEnd {
            into,
            mode,
            masking,
        }
    }
}
