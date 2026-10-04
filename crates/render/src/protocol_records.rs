//! Records as GPU bytes: every record that crosses to the GPU writes its fields in declaration
//! order, little endian, which reproduces its `repr(C)` layout (`protocol_tests.rs`).

use super::Op;

/// One tile of a plan: its top-left pixel in the target, the output pixel that pixel holds (S32:
/// a view tier's target holds its tiles at places of their own) and its range in the plan's op
/// index list.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TileRecord {
    pub(crate) origin: [u32; 2],
    pub(crate) output: [u32; 2],
    pub(crate) start: u32,
    pub(crate) count: u32,
}

/// Per-dispatch values.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Params {
    /// Opaque premultiplied background, alpha 1.
    pub(crate) background: [f32; 4],
    /// Tiles in this dispatch.
    pub(crate) tiles: u32,
    /// Workgroups per dispatch row: tile index = `row * group.y + group.x`.
    pub(crate) row: u32,
    /// Target size in pixels.
    pub(crate) size: [u32; 2],
    /// 1 when the target keeps the in-progress stroke's coverage between evaluations (the
    /// canvas); 0 when every evaluation lists all of its segments (a view tier, S32).
    pub(crate) kept: u32,
    /// The base-2 log of `scale`.
    pub(crate) level: f32,
    /// Output pixels per canvas pixel (a power of two) and its inverse: an output pixel's
    /// centre `(p + 0.5) x inverse` in canvas pixels.
    pub(crate) scale: f32,
    pub(crate) inverse: f32,
    /// Where those centres lie on the canvas (S31; the identity but for a smudge's material
    /// seen from a transform record): `x = a cx + b cy + c` with `[a, b, c, foot]`, `y` from
    /// `[d, e, f, level]`; `foot` one over canvas pixels per centre pixel, `level` its log.
    pub(crate) base_x: [f32; 4],
    pub(crate) base_y: [f32; 4],
}

/// A record that crosses to the GPU. Every field is a 4-byte scalar or an array of them, so
/// writing the fields in declaration order reproduces the `repr(C)` layout without padding.
pub(crate) trait Record {
    fn write(&self, out: &mut Vec<u8>);
}

impl Record for Op {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend(self.code.to_le_bytes());
        out.extend(self.data.iter().flat_map(|value| value.to_le_bytes()));
    }
}

impl Record for TileRecord {
    fn write(&self, out: &mut Vec<u8>) {
        let [x, y] = self.origin;
        let [output_x, output_y] = self.output;
        let words = [x, y, output_x, output_y, self.start, self.count];
        out.extend(words.iter().flat_map(|word| word.to_le_bytes()));
    }
}

impl Record for Params {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend(self.background.iter().flat_map(|value| value.to_le_bytes()));
        let words = [self.tiles, self.row, self.size[0], self.size[1]];
        out.extend(words.iter().flat_map(|word| word.to_le_bytes()));
        out.extend(self.kept.to_le_bytes());
        out.extend(self.level.to_le_bytes());
        out.extend(self.scale.to_le_bytes());
        out.extend(self.inverse.to_le_bytes());
        let base = self.base_x.iter().chain(&self.base_y);
        out.extend(base.flat_map(|value| value.to_le_bytes()));
    }
}

impl Record for u32 {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend(self.to_le_bytes());
    }
}

/// The records as GPU bytes, little endian.
pub(crate) fn encode<R: Record>(records: &[R]) -> Vec<u8> {
    let mut out = Vec::with_capacity(size_of_val(records));
    for record in records {
        record.write(&mut out);
    }
    out
}
