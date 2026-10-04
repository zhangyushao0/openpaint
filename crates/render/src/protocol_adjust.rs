//! The protocol's adjustments (S28, D54): hue, saturation and brightness, or tone curves,
//! applied to the current layer at the op's place, inside its clip. The slots and values here
//! are declared to WGSL by `prelude` like the rest of the protocol.

use super::{OP_DATA, Op, Opcode, PAINT_CLIP, halves};

/// Which adjustment: one of `ADJUST_KINDS`, a whole number.
const ADJUST_KIND: usize = 0;
/// Hue, saturation and brightness, each in `0..=1` (0.5: no change).
const ADJUST_HSB: usize = 1;
/// What the layer holds: one of `ADJUST_ONS`, a whole number.
const ADJUST_ON: usize = 4;
/// Where the curves' three tables start in the mask words, split like `FILL_MASK`: per colour
/// channel 256 bytes, four to a word, low byte first.
const ADJUST_TABLES: usize = 5;
pub(super) const ADJUST_SLOTS: [(&str, usize); 4] = [
    ("ADJUST_KIND", ADJUST_KIND),
    ("ADJUST_HSB", ADJUST_HSB),
    ("ADJUST_ON", ADJUST_ON),
    ("ADJUST_TABLES", ADJUST_TABLES),
];

/// Which adjustment an `Adjust` op makes.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdjustKind {
    Hsb = 0,
    Curves = 1,
}

pub(super) const ADJUST_KINDS: [(&str, AdjustKind); 2] = [
    ("ADJUST_KIND_HSB", AdjustKind::Hsb),
    ("ADJUST_KIND_CURVES", AdjustKind::Curves),
];

/// What the current layer holds where an `Adjust` op acts: paint (premultiplied colour), or a
/// mask's grey over transparent (`1 - (a - r)` shows), as it is stored or turned over.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdjustOn {
    Paint = 0,
    Mask = 1,
    InvertedMask = 2,
}

pub(super) const ADJUST_ONS: [(&str, AdjustOn); 3] = [
    ("ADJUST_ON_PAINT", AdjustOn::Paint),
    ("ADJUST_ON_MASK", AdjustOn::Mask),
    ("ADJUST_ON_INVERTED_MASK", AdjustOn::InvertedMask),
];

impl Op {
    /// Hue, saturation and brightness `hsb` on what `on` says the layer holds; `clip` as a
    /// paint op's.
    pub(crate) fn adjust_hsb(hsb: [f32; 3], on: AdjustOn, clip: u32) -> Self {
        let mut data = [0.0; OP_DATA];
        data[ADJUST_KIND] = AdjustKind::Hsb as u32 as f32;
        data[ADJUST_HSB..ADJUST_HSB + 3].copy_from_slice(&hsb);
        Self::adjusting(data, on, clip)
    }

    /// Tone curves through the tables at word `tables` of the mask words.
    pub(crate) fn adjust_curves(tables: u32, on: AdjustOn, clip: u32) -> Self {
        let mut data = [0.0; OP_DATA];
        data[ADJUST_KIND] = AdjustKind::Curves as u32 as f32;
        data[ADJUST_TABLES..ADJUST_TABLES + 2].copy_from_slice(&halves(tables));
        Self::adjusting(data, on, clip)
    }

    fn adjusting(mut data: [f32; OP_DATA], on: AdjustOn, clip: u32) -> Self {
        data[ADJUST_ON] = on as u32 as f32;
        data[PAINT_CLIP..PAINT_CLIP + 2].copy_from_slice(&halves(clip));
        Self::new(Opcode::Adjust, &data)
    }
}

/// The curves' tables as mask words: per channel 64 words, four bytes each, low byte first.
pub(crate) fn table_words(luts: &[[u8; 256]; 3]) -> Vec<u32> {
    luts.iter()
        .flat_map(|lut| {
            lut.as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_le_bytes(*b))
        })
        .collect()
}
