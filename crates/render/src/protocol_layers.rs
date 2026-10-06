//! The protocol's layer ends: how a layer, a clipping chain and a group composite (D50). The
//! slots and values here are declared to WGSL by `prelude` like the rest of the protocol.

use openpaint_core::BlendMode;

use super::{OP_DATA, Op, Opcode};

/// Where an `EndLayer` puts the layer: one of `INTOS`, a whole number.
const LAYER_INTO: usize = 1;
/// A layer's or chain's blend mode: one of `MODES`, a whole number.
const LAYER_MODE: usize = 2;
/// 1 when the layer or chain lies in a group, whose `EndGroup` follows.
const LAYER_GROUPED: usize = 3;
/// How a layer's mask applies: one of `MASKINGS`, a whole number.
const LAYER_MASK: usize = 4;
/// A nested record's end (S39): 1 where its start set the layer so far aside.
const NESTED_ASIDE: usize = 5;
pub(super) const LAYER_SLOTS: [(&str, usize); 5] = [
    ("LAYER_INTO", LAYER_INTO),
    ("LAYER_MODE", LAYER_MODE),
    ("LAYER_GROUPED", LAYER_GROUPED),
    ("LAYER_MASK", LAYER_MASK),
    ("NESTED_ASIDE", NESTED_ASIDE),
];

/// How an `EndLayer` applies the layer's mask (S24), declared to WGSL as constants by
/// `prelude`. With a mask the layer's paint waits in `held` from its `BeginMask` on, the mask's
/// strokes and fills paint over transparent in `layer`, read as grey over white
/// (`m = r + 1 - a`), and the end shows `held x m` (or `x (1 - m)`).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Masking {
    None = 0,
    Shown = 1,
    Inverted = 2,
}

pub(super) const MASKINGS: [(&str, Masking); 3] = [
    ("MASK_NONE", Masking::None),
    ("MASK_SHOWN", Masking::Shown),
    ("MASK_INVERTED", Masking::Inverted),
];

/// A layer's or chain's blend mode, declared to WGSL as constants by `prelude`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
}

impl From<BlendMode> for Mode {
    fn from(blend: BlendMode) -> Self {
        match blend {
            BlendMode::Normal => Self::Normal,
            BlendMode::Multiply => Self::Multiply,
            BlendMode::Screen => Self::Screen,
            BlendMode::Overlay => Self::Overlay,
        }
    }
}

pub(super) const MODES: [(&str, Mode); 4] = [
    ("MODE_NORMAL", Mode::Normal),
    ("MODE_MULTIPLY", Mode::Multiply),
    ("MODE_SCREEN", Mode::Screen),
    ("MODE_OVERLAY", Mode::Overlay),
];

/// Where a finished layer goes, declared to WGSL as constants by `prelude`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayerInto {
    /// Onto the canvas by its opacity and mode.
    Canvas = 0,
    /// It starts a clipping chain: the chain becomes the layer (its opacity and mode wait for
    /// the chain's end).
    Base = 1,
    /// Onto the chain, atop it: by its opacity and mode against the chain so far, the chain's
    /// alpha kept.
    Clipped = 2,
}

pub(super) const INTOS: [(&str, LayerInto); 3] = [
    ("INTO_CANVAS", LayerInto::Canvas),
    ("INTO_BASE", LayerInto::Base),
    ("INTO_CLIPPED", LayerInto::Clipped),
];

impl Op {
    /// The end of a layer at `opacity` going `into` the canvas or a chain by `mode`.
    pub(crate) fn end_layer_into(opacity: f32, into: LayerInto, mode: Mode, grouped: bool) -> Self {
        let mut data = [0.0; OP_DATA];
        data[0] = opacity;
        data[LAYER_INTO] = into as u32 as f32;
        data[LAYER_MODE] = mode as u32 as f32;
        data[LAYER_GROUPED] = f32::from(u8::from(grouped));
        Self::new(Opcode::EndLayer, &data)
    }

    /// This layer end applying the layer's mask by `masking`.
    pub(crate) fn masked(mut self, masking: Masking) -> Self {
        self.data[LAYER_MASK] = masking as u32 as f32;
        self
    }

    /// Where a masked layer's paint is set aside and its mask starts (`Masking`).
    pub(crate) fn begin_mask() -> Self {
        Self::new(Opcode::BeginMask, &[])
    }

    /// The end of a clipping chain whose base has `opacity` and `mode`.
    pub(crate) fn end_chain(opacity: f32, mode: Mode, grouped: bool) -> Self {
        let mut data = [0.0; OP_DATA];
        data[0] = opacity;
        data[LAYER_MODE] = mode as u32 as f32;
        data[LAYER_GROUPED] = f32::from(u8::from(grouped));
        Self::new(Opcode::EndChain, &data)
    }

    /// The end of a merged layer's content (S39): through `masking`, at `opacity`, by `mode`
    /// atop (`clipped`) or over what its layer held before it, which its start set `aside`
    /// (elsewhere the layer held nothing there).
    pub(crate) fn end_nested(
        (opacity, mode): (f32, Mode),
        (clipped, masking): (bool, Masking),
        aside: bool,
    ) -> Self {
        let into = if clipped {
            LayerInto::Clipped
        } else {
            LayerInto::Canvas
        };
        let mut data = [0.0; OP_DATA];
        data[0] = opacity;
        data[LAYER_INTO] = into as u32 as f32;
        data[LAYER_MODE] = mode as u32 as f32;
        data[LAYER_MASK] = masking as u32 as f32;
        data[NESTED_ASIDE] = f32::from(u8::from(aside));
        Self::new(Opcode::EndNested, &data)
    }

    /// The end of a group of `opacity`.
    pub(crate) fn end_group(opacity: f32) -> Self {
        Self::new(Opcode::EndGroup, &[opacity])
    }
}
