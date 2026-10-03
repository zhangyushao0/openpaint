//! The GPU protocol: opcodes and record layouts shared by the Rust encoder and the WGSL
//! evaluator. Defined here once; `prelude` generates the matching WGSL declarations, which are
//! prepended to every shader, and the layout test below proves both sides agree.

use std::mem::{offset_of, size_of};

/// Side of a square evaluation tile in pixels; also the evaluator's workgroup size.
pub(crate) const TILE: u32 = 16;
/// Number of `f32` operands in one op record.
const OP_DATA: usize = 9;
/// A paint op's grain strength is a whole number in `0..=GRAIN_ONE`, so the shader's grain is
/// exact integer arithmetic on every backend.
pub(crate) const GRAIN_ONE: u32 = 256;

/// Operand slots the evaluator reads by name, declared to WGSL as constants by `prelude`.
const PAINT_OPACITY: usize = 3;
const PAINT_GRAIN: usize = 4;
const PAINT_GRAIN_SCALE: usize = 5;
/// How the paint combines with the layer: one of `BLENDS`, a whole number.
const PAINT_BLEND: usize = 6;
/// The flow at `a`; the flow at `b` follows it.
const SEGMENT_FLOW: usize = 6;
const SEGMENT_EDGE: usize = 8;
/// Where a fill's mask starts in the mask words, split in two whole numbers below 2^16 (high
/// part first) so each stays exact as a float.
const FILL_MASK: usize = 7;
const SLOTS: [(&str, usize); 7] = [
    ("PAINT_OPACITY", PAINT_OPACITY),
    ("PAINT_GRAIN", PAINT_GRAIN),
    ("PAINT_GRAIN_SCALE", PAINT_GRAIN_SCALE),
    ("PAINT_BLEND", PAINT_BLEND),
    ("SEGMENT_FLOW", SEGMENT_FLOW),
    ("SEGMENT_EDGE", SEGMENT_EDGE),
    ("FILL_MASK", FILL_MASK),
];
/// Words before a mask's rows in the mask words: origin x, origin y, width, height.
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
    /// A fill: its colour painted over the current layer wherever its mask holds the pixel
    /// (coverage exactly 1 or 0), like a stroke's paint op.
    Fill = 4,
}

impl Opcode {
    const ALL: [Self; 5] = [
        Self::Segment,
        Self::EndStroke,
        Self::EndLayer,
        Self::EndActive,
        Self::Fill,
    ];

    const fn wgsl_name(self) -> &'static str {
        match self {
            Self::Segment => "OP_SEGMENT",
            Self::EndStroke => "OP_END_STROKE",
            Self::EndLayer => "OP_END_LAYER",
            Self::EndActive => "OP_END_ACTIVE",
            Self::Fill => "OP_FILL",
        }
    }
}

/// One op. Operands by opcode:
/// - `Segment`: `[ax, ay, bx, by, ra, rb, fa, fb, edge]`, a tapered capsule from `a` (radius
///   `ra`) to `b` (radius `rb`) in target pixels; its coverage is multiplied by the flow, `fa`
///   at `a` changing evenly to `fb` at `b`; `edge` is one over the width in pixels over which
///   the coverage fades in from the outline (1: a hard, antialiased edge).
/// - `EndStroke` and `EndActive`: `[r, g, b, opacity, grain, grain_scale, blend, _, _]`,
///   straight sRGB colour; `grain` a whole number in `0..=GRAIN_ONE`, `grain_scale` whole
///   pixels, `blend` a `Blend`.
/// - `EndLayer`: `[opacity, _, ...]`.
/// - `Fill`: a paint op's first seven operands, then where its mask starts in the mask words
///   (`offset / 2^16`, `offset % 2^16`); the mask is `MASK_HEADER` words, then rows of
///   `ceil(width / 32)` words, bit `x % 32` of word `x / 32`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Op {
    code: u32,
    data: [f32; OP_DATA],
}

/// What a stroke's paint op carries: colour, opacity and grain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Paint {
    pub(crate) color: [f32; 3],
    pub(crate) opacity: f32,
    /// In `0..=GRAIN_ONE`.
    pub(crate) grain: u32,
    pub(crate) grain_scale: u32,
    pub(crate) blend: Blend,
}

impl Paint {
    fn operands(&self) -> [f32; 7] {
        // Small whole numbers, exact as floats.
        let [r, g, b] = self.color;
        [
            r,
            g,
            b,
            self.opacity,
            self.grain as f32,
            self.grain_scale as f32,
            self.blend as u32 as f32,
        ]
    }
}

impl Op {
    pub(crate) fn segment(
        a: [f32; 2],
        b: [f32; 2],
        radii: [f32; 2],
        flows: [f32; 2],
        edge: f32,
    ) -> Self {
        let mut data = [0.0; OP_DATA];
        data[..6].copy_from_slice(&[a[0], a[1], b[0], b[1], radii[0], radii[1]]);
        data[SEGMENT_FLOW] = flows[0];
        data[SEGMENT_FLOW + 1] = flows[1];
        data[SEGMENT_EDGE] = edge;
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

    /// A fill whose mask starts at word `mask` of the mask words.
    pub(crate) fn fill(paint: &Paint, mask: u32) -> Self {
        let mut data = [0.0; OP_DATA];
        data[..7].copy_from_slice(&paint.operands());
        // Each part below 2^16: exact as a float.
        data[FILL_MASK] = (mask >> 16) as f32;
        data[FILL_MASK + 1] = (mask & 0xffff) as f32;
        Self::new(Opcode::Fill, &data)
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

/// One tile of a plan: its top-left pixel and its range in the plan's op index list.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TileRecord {
    pub(crate) origin: [u32; 2],
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
        let words = [self.origin[0], self.origin[1], self.start, self.count];
        out.extend(words.iter().flat_map(|word| word.to_le_bytes()));
    }
}

impl Record for Params {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend(self.background.iter().flat_map(|value| value.to_le_bytes()));
        let words = [self.tiles, self.row, self.size[0], self.size[1]];
        out.extend(words.iter().flat_map(|word| word.to_le_bytes()));
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

/// A WGSL struct generated from a Rust record: name, size and `(field, WGSL type, offset)`.
struct Layout {
    name: &'static str,
    size: usize,
    fields: &'static [(&'static str, &'static str, usize)],
}

const LAYOUTS: [Layout; 3] = [
    Layout {
        name: "Op",
        size: size_of::<Op>(),
        fields: &[
            ("code", "u32", offset_of!(Op, code)),
            ("data", "array<f32, 9>", offset_of!(Op, data)),
        ],
    },
    Layout {
        name: "Tile",
        size: size_of::<TileRecord>(),
        fields: &[
            ("origin", "vec2<u32>", offset_of!(TileRecord, origin)),
            ("start", "u32", offset_of!(TileRecord, start)),
            ("count", "u32", offset_of!(TileRecord, count)),
        ],
    },
    Layout {
        name: "Params",
        size: size_of::<Params>(),
        fields: &[
            ("background", "vec4<f32>", offset_of!(Params, background)),
            ("tiles", "u32", offset_of!(Params, tiles)),
            ("row", "u32", offset_of!(Params, row)),
            ("size", "vec2<u32>", offset_of!(Params, size)),
        ],
    },
];

/// WGSL declarations of the protocol: the tile size, one constant per opcode and per named
/// operand slot, one struct per record.
pub(crate) fn prelude() -> String {
    let mut wgsl = String::from("// Generated by crates/render/src/protocol.rs.\n");
    wgsl.push_str(&format!("const TILE: u32 = {TILE}u;\n"));
    wgsl.push_str(&format!("const FRESH: u32 = {FRESH}u;\n"));
    wgsl.push_str(&format!("const GRAIN_ONE: u32 = {GRAIN_ONE}u;\n"));
    wgsl.push_str(&format!("const MASK_HEADER: u32 = {MASK_HEADER}u;\n"));
    for (name, slot) in SLOTS {
        wgsl.push_str(&format!("const {name}: u32 = {slot}u;\n"));
    }
    for (name, blend) in BLENDS {
        wgsl.push_str(&format!("const {name}: u32 = {}u;\n", blend as u32));
    }
    for opcode in Opcode::ALL {
        wgsl.push_str(&format!(
            "const {}: u32 = {}u;\n",
            opcode.wgsl_name(),
            opcode as u32
        ));
    }
    for layout in &LAYOUTS {
        wgsl.push_str(&format!(
            "// {} bytes, as in Rust.\nstruct {} {{\n",
            layout.size, layout.name
        ));
        for (field, ty, _) in layout.fields {
            wgsl.push_str(&format!("    {field}: {ty},\n"));
        }
        wgsl.push_str("}\n");
    }
    wgsl
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
