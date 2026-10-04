//! The protocol's transform records (S31, D57): a frame moves where the ops after it are
//! evaluated; a moved selection's content starts from nothing, the layer so far set aside, and
//! its end cuts it to the selection (or, for a whole layer, the canvas) and lays it over what
//! was set aside. The slots here are declared to WGSL by `prelude` like the rest.

use super::{OP_DATA, Op, Opcode, halves};

/// A frame: after its six rows, canvas pixels per content pixel and its base-2 log.
const FRAME_FOOT: usize = 6;
const FRAME_LEVEL: usize = 7;
/// A move's end: its selection's mask (one more than where it starts, split like `FILL_MASK`;
/// 0: the whole layer), whether it sets the content aside, the canvas's width and height.
const MOVE_MASK: usize = 0;
const MOVE_ASIDE: usize = 2;
const MOVE_CANVAS: usize = 3;
/// How many moved selections a pixel can set aside at once (the evaluator's stack).
pub(crate) const MOVE_DEPTH: u32 = 16;

pub(super) const MOVE_SLOTS: [(&str, usize); 5] = [
    ("FRAME_FOOT", FRAME_FOOT),
    ("FRAME_LEVEL", FRAME_LEVEL),
    ("MOVE_MASK", MOVE_MASK),
    ("MOVE_ASIDE", MOVE_ASIDE),
    ("MOVE_CANVAS", MOVE_CANVAS),
];

impl Op {
    /// A frame mapping canvas points by `rows` into content of `foot` canvas pixels per pixel.
    pub(crate) fn frame(rows: [f32; 6], foot: f32, level: f32) -> Self {
        let mut data = [0.0; OP_DATA];
        data[..6].copy_from_slice(&rows);
        data[FRAME_FOOT] = foot;
        data[FRAME_LEVEL] = level;
        Self::new(Opcode::Frame, &data)
    }

    pub(crate) fn begin_move() -> Self {
        Self::new(Opcode::BeginMove, &[])
    }

    /// A move's end on a canvas of `canvas` pixels: cut to the mask at `mask` (one more than
    /// where it starts; 0, the canvas), laid over what was set aside when `aside`.
    pub(crate) fn end_move(mask: u32, aside: bool, canvas: [u32; 2]) -> Self {
        let mut data = [0.0; OP_DATA];
        data[MOVE_MASK..MOVE_MASK + 2].copy_from_slice(&halves(mask));
        data[MOVE_ASIDE] = f32::from(u8::from(aside));
        // At most `MAX_CANVAS_SIDE`, exact as floats.
        data[MOVE_CANVAS] = canvas[0] as f32;
        data[MOVE_CANVAS + 1] = canvas[1] as f32;
        Self::new(Opcode::EndMove, &data)
    }
}
