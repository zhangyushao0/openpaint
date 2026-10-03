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
/// The flow at `a`; the flow at `b` follows it.
const SEGMENT_FLOW: usize = 6;
const SEGMENT_EDGE: usize = 8;
const SLOTS: [(&str, usize); 5] = [
    ("PAINT_OPACITY", PAINT_OPACITY),
    ("PAINT_GRAIN", PAINT_GRAIN),
    ("PAINT_GRAIN_SCALE", PAINT_GRAIN_SCALE),
    ("SEGMENT_FLOW", SEGMENT_FLOW),
    ("SEGMENT_EDGE", SEGMENT_EDGE),
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
}

impl Opcode {
    const ALL: [Self; 4] = [
        Self::Segment,
        Self::EndStroke,
        Self::EndLayer,
        Self::EndActive,
    ];

    const fn wgsl_name(self) -> &'static str {
        match self {
            Self::Segment => "OP_SEGMENT",
            Self::EndStroke => "OP_END_STROKE",
            Self::EndLayer => "OP_END_LAYER",
            Self::EndActive => "OP_END_ACTIVE",
        }
    }
}

/// One op. Operands by opcode:
/// - `Segment`: `[ax, ay, bx, by, ra, rb, fa, fb, edge]`, a tapered capsule from `a` (radius
///   `ra`) to `b` (radius `rb`) in target pixels; its coverage is multiplied by the flow, `fa`
///   at `a` changing evenly to `fb` at `b`; `edge` is one over the width in pixels over which
///   the coverage fades in from the outline (1: a hard, antialiased edge).
/// - `EndStroke` and `EndActive`: `[r, g, b, opacity, grain, grain_scale, _, _, _]`, straight
///   sRGB colour; `grain` a whole number in `0..=GRAIN_ONE`, `grain_scale` whole pixels.
/// - `EndLayer`: `[opacity, _, ...]`.
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
}

impl Paint {
    fn operands(&self) -> [f32; 6] {
        // Both are small whole numbers, exact as floats.
        let [r, g, b] = self.color;
        [
            r,
            g,
            b,
            self.opacity,
            self.grain as f32,
            self.grain_scale as f32,
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
    for (name, slot) in SLOTS {
        wgsl.push_str(&format!("const {name}: u32 = {slot}u;\n"));
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
mod tests {
    use super::*;
    use wgpu::naga;

    fn parsed() -> naga::Module {
        let source = crate::program::shader_source();
        naga::front::wgsl::parse_str(&source).expect("the evaluator shader parses")
    }

    #[test]
    fn wgsl_layouts_match_the_rust_records() {
        let module = parsed();
        for layout in &LAYOUTS {
            let (_, ty) = module
                .types
                .iter()
                .find(|(_, ty)| ty.name.as_deref() == Some(layout.name))
                .unwrap_or_else(|| panic!("struct {} in WGSL", layout.name));
            let naga::TypeInner::Struct { members, span } = &ty.inner else {
                panic!("{} is a struct", layout.name);
            };
            assert_eq!(*span as usize, layout.size, "size of {}", layout.name);
            let wgsl: Vec<_> = members
                .iter()
                .map(|member| (member.name.as_deref().unwrap_or(""), member.offset as usize))
                .collect();
            let rust: Vec<_> = layout.fields.iter().map(|(n, _, o)| (*n, *o)).collect();
            assert_eq!(wgsl, rust, "fields of {}", layout.name);
        }
    }

    #[test]
    fn named_slots_hold_what_the_constructors_write() {
        let op = Op::segment([1.0, 2.0], [3.0, 4.0], [5.0, 6.0], [0.25, 0.75], 0.5);
        assert_eq!(
            op.data[SEGMENT_FLOW..=SEGMENT_EDGE],
            [0.25, 0.75, 0.5],
            "flows, then the edge"
        );
        let paint = Paint {
            color: [0.1, 0.2, 0.3],
            opacity: 0.4,
            grain: 77,
            grain_scale: 5,
        };
        let op = Op::end_stroke(&paint);
        assert_eq!(op.data[PAINT_OPACITY], 0.4);
        assert_eq!(op.data[PAINT_GRAIN], 77.0);
        assert_eq!(op.data[PAINT_GRAIN_SCALE], 5.0);
        assert_eq!(Op::end_active(&paint).data, op.data);
    }

    #[test]
    fn encoding_fills_exactly_the_record_size() {
        let op = Op::segment([1.0, 2.0], [3.0, 4.0], [5.0, 6.0], [1.0; 2], 1.0);
        assert_eq!(encode(&[op, op]).len(), 2 * size_of::<Op>());
        let tile = TileRecord {
            origin: [16, 32],
            start: 7,
            count: 9,
        };
        assert_eq!(encode(&[tile]).len(), size_of::<TileRecord>());
        assert_eq!(&encode(&[tile])[8..12], &7u32.to_le_bytes());
        let params = Params {
            background: [1.0; 4],
            tiles: 1,
            row: 1,
            size: [1, 1],
        };
        assert_eq!(encode(&[params]).len(), size_of::<Params>());
    }

    #[test]
    fn opcode_constants_match_the_enum() {
        let module = parsed();
        let value = |name: &str| {
            let (_, constant) = module
                .constants
                .iter()
                .find(|(_, constant)| constant.name.as_deref() == Some(name))
                .unwrap_or_else(|| panic!("{name} declared in WGSL"));
            match module.global_expressions[constant.init] {
                naga::Expression::Literal(naga::Literal::U32(value)) => value,
                ref other => panic!("{name} is {other:?}"),
            }
        };
        for opcode in Opcode::ALL {
            assert_eq!(value(opcode.wgsl_name()), opcode as u32);
        }
        assert_eq!(value("TILE"), TILE);
        assert_eq!(value("FRESH"), FRESH);
        assert_eq!(value("GRAIN_ONE"), GRAIN_ONE);
        for (name, slot) in SLOTS {
            assert_eq!(value(name) as usize, slot, "{name}");
        }
    }
}
