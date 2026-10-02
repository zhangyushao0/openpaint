//! The GPU protocol: opcodes and record layouts shared by the Rust encoder and the WGSL
//! evaluator. Defined here once; `prelude` generates the matching WGSL declarations, which are
//! prepended to every shader, and the layout test below proves both sides agree.

use std::mem::{offset_of, size_of};

/// Side of a square evaluation tile in pixels; also the evaluator's workgroup size.
pub(crate) const TILE: u32 = 16;
/// Number of `f32` operands in one op record.
const OP_DATA: usize = 7;

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
}

impl Opcode {
    const ALL: [Self; 3] = [Self::Segment, Self::EndStroke, Self::EndLayer];

    const fn wgsl_name(self) -> &'static str {
        match self {
            Self::Segment => "OP_SEGMENT",
            Self::EndStroke => "OP_END_STROKE",
            Self::EndLayer => "OP_END_LAYER",
        }
    }
}

/// One op. Operands by opcode:
/// - `Segment`: `[ax, ay, bx, by, ra, rb, _]`, a tapered capsule from `a` (radius `ra`) to `b`
///   (radius `rb`) in target pixels.
/// - `EndStroke`: `[r, g, b, opacity, _, _, _]`, straight sRGB colour.
/// - `EndLayer`: `[opacity, _, _, _, _, _, _]`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Op {
    code: u32,
    data: [f32; OP_DATA],
}

impl Op {
    pub(crate) fn segment(a: [f32; 2], b: [f32; 2], radii: [f32; 2]) -> Self {
        Self::new(
            Opcode::Segment,
            &[a[0], a[1], b[0], b[1], radii[0], radii[1]],
        )
    }

    pub(crate) fn end_stroke(color: [f32; 3], opacity: f32) -> Self {
        Self::new(Opcode::EndStroke, &[color[0], color[1], color[2], opacity])
    }

    pub(crate) fn end_layer(opacity: f32) -> Self {
        Self::new(Opcode::EndLayer, &[opacity])
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
            ("data", "array<f32, 7>", offset_of!(Op, data)),
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

/// WGSL declarations of the protocol: the tile size, one constant per opcode, one struct per
/// record.
pub(crate) fn prelude() -> String {
    let mut wgsl = String::from("// Generated by crates/render/src/protocol.rs.\n");
    wgsl.push_str(&format!("const TILE: u32 = {TILE}u;\n"));
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
    fn encoding_fills_exactly_the_record_size() {
        let op = Op::segment([1.0, 2.0], [3.0, 4.0], [5.0, 6.0]);
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
    }
}
