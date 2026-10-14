//! The protocol's layout test: the generated WGSL declares exactly the Rust records, opcodes,
//! named slots and blends.

use super::wgsl::LAYOUTS;
use super::*;
use crate::paper::{MOMENT_ONE, PAPER_SIZE, level_count};
use wgpu::naga;

fn parsed() -> naga::Module {
    let source = crate::program::shader_source(crate::gpu::TEN_BITS);
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

fn operands(tip: Tip) -> SegmentOperands {
    SegmentOperands {
        a: [1.0, 2.0],
        b: [3.0, 4.0],
        radii: [5.0, 6.0],
        flows: [0.25, 0.75],
        edge: 0.5,
        contacts: [0.375, 0.625],
        tip,
        controls: [[1.5, 2.5], [2.5, 3.5]],
        flat: 0.125,
        bend: 0.25,
        stamps: None,
        azimuths: [-1.5, 2.5],
    }
}

#[test]
fn named_slots_hold_what_the_constructors_write() {
    let op = Op::segment(&operands(Tip::Bell), 3);
    assert_eq!(
        op.data[segment::SEGMENT_FLOW..],
        [
            0.25, 0.75, 0.5, 0.375, 0.625, 2.0, 1.5, 2.5, 2.5, 3.5, 0.125, 0.25, 0.0, 0.0, 0.0,
            0.0, 0.0
        ],
        "flows, the edge, the contacts, the tip, the controls, flatness and bend; not stamped"
    );
    let paint = Paint {
        color: [0.1, 0.2, 0.3],
        opacity: 0.4,
        grain: 77,
        grain_scale: 5,
        blend: Blend::Erase,
        density: 0.875,
        paper: [4.5, 2.25, 0.3, 0.25],
        clip: (5 << 16) | 11,
        stamp: [3.0, 12.0, 0.5, 2.0, 0.25, 0.5, 0.125, 5.0, 7.0],
    };
    let op = Op::end_stroke(&paint);
    assert_eq!(op.data[PAINT_OPACITY], 0.4);
    assert_eq!(op.data[PAINT_GRAIN], 77.0);
    assert_eq!(op.data[PAINT_GRAIN_SCALE], 5.0);
    assert_eq!(op.data[PAINT_BLEND], 2.0);
    assert_eq!(op.data[PAINT_DENSITY], 0.875);
    assert_eq!(op.data[PAINT_PAPER..PAINT_CLIP], [4.5, 2.25, 0.3, 0.25]);
    assert_eq!(
        op.data[PAINT_CLIP..PAINT_CLIP + 2],
        [5.0, 11.0],
        "the clip, high part first"
    );
    assert_eq!(op.data[PAINT_STAMP..], paint.stamp, "the stamps last");
    assert_eq!(Op::end_active(&paint).data, op.data);
    let fill = Op::fill(&paint, (3 << 16) | 9, Some([2732, 2048]));
    assert_eq!(
        fill.data[..FILL_MASK],
        op.data[..FILL_MASK],
        "a paint op's operands"
    );
    assert_eq!(
        fill.data[FILL_MASK..FILL_MASK + 2],
        [3.0, 9.0],
        "the mask offset, high part first"
    );
    assert_eq!(fill.data[FILL_EDGE..FILL_EDGE + 2], [2732.0, 2048.0]);
    let hard = Op::fill(&paint, 0, None);
    assert_eq!(
        hard.data[FILL_EDGE..FILL_EDGE + 2],
        [0.0, 0.0],
        "a hard edge"
    );
}

#[test]
fn a_stamped_span_carries_its_stamps_and_where_its_paint_op_lies() {
    let stamps = SpanStamps {
        arc: 40.5,
        first: 7.0,
        last: 12.0,
    };
    let stamped = Op::tail(
        &SegmentOperands {
            stamps: Some(stamps),
            ..operands(Tip::Cover)
        },
        -4,
    );
    assert_eq!(
        stamped.data[stamp::STAMP_ARC..],
        [40.5, 7.0, 12.0, -4.0, 0.0],
        "the arc, the stamps, the paint op"
    );
    assert_eq!(
        stamped.data[segment::SEGMENT_CONTACT..segment::SEGMENT_CONTACT + 2],
        [-1.5, 2.5],
        "the azimuths in the contacts' place"
    );
    let module = parsed();
    for (name, value) in STAMP_SLOTS
        .map(|(name, slot)| (name, slot as u32))
        .into_iter()
        .chain(STAMP_CODES)
        .chain([
            ("SHAPES_AT", crate::paper::paper_words()),
            ("SHAPE_WORDS", crate::shapes::shape_words()),
        ])
    {
        assert_eq!(constant(&module, name), value, "{name}");
    }
}

/// The value of the WGSL constant `name`, a `u32`.
fn constant(module: &naga::Module, name: &str) -> u32 {
    let (_, constant) = module
        .constants
        .iter()
        .find(|(_, constant)| constant.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("{name} declared in WGSL"));
    match module.global_expressions[constant.init] {
        naga::Expression::Literal(naga::Literal::U32(value)) => value,
        ref other => panic!("{name} is {other:?}"),
    }
}

#[test]
fn encoding_fills_exactly_the_record_size() {
    let op = Op::segment(&operands(Tip::Cover), 1);
    assert_eq!(encode(&[op, op]).len(), 2 * size_of::<Op>());
    let tile = TileRecord {
        origin: [16, 32],
        output: [48, 64],
        start: 7,
        count: 9,
    };
    assert_eq!(encode(&[tile]).len(), size_of::<TileRecord>());
    assert_eq!(&encode(&[tile])[16..20], &7u32.to_le_bytes());
    let params = Params {
        background: [1.0; 4],
        tiles: 1,
        row: 1,
        size: [1, 1],
        kept: 1,
        level: 0.0,
        scale: 1.0,
        inverse: 1.0,
        base_x: [1.0, 0.0, 0.0, 1.0],
        base_y: [0.0, 1.0, 0.0, 0.0],
        snapshot: [16, 32, 48, 1],
        display: [1, 0, 0, 0],
    };
    assert_eq!(encode(&[params]).len(), size_of::<Params>());
}

#[test]
fn opcode_constants_match_the_enum() {
    let module = parsed();
    let value = |name: &str| constant(&module, name);
    for opcode in Opcode::ALL {
        assert_eq!(value(opcode.wgsl_name()), opcode as u32);
    }
    assert_eq!(value("TILE"), TILE);
    assert_eq!(value("FRESH"), FRESH);
    assert_eq!(value("GRAIN_ONE"), GRAIN_ONE);
    assert_eq!(value("MASK_HEADER"), MASK_HEADER);
    for (name, slot) in SLOTS.into_iter().chain(SEGMENT_SLOTS) {
        assert_eq!(value(name) as usize, slot, "{name}");
    }
    for (name, blend) in BLENDS {
        assert_eq!(value(name), blend as u32, "{name}");
    }
    for (name, tip) in TIPS {
        assert_eq!(value(name), tip as u32, "{name}");
    }
    for (name, slot) in layers::LAYER_SLOTS {
        assert_eq!(value(name) as usize, slot, "{name}");
    }
    for (name, mode) in layers::MODES {
        assert_eq!(value(name), mode as u32, "{name}");
    }
    for (name, into) in layers::INTOS {
        assert_eq!(value(name), into as u32, "{name}");
    }
    for (name, masking) in layers::MASKINGS {
        assert_eq!(value(name), masking as u32, "{name}");
    }
    let masked = Op::end_layer(1.0).masked(Masking::Inverted);
    assert_eq!(masked.data[..5], [1.0, 0.0, 0.0, 0.0, 2.0], "the mask last");
    let end = Op::end_layer_into(0.5, LayerInto::Clipped, layers::Mode::Overlay, true);
    assert_eq!(
        end.data[..4],
        [0.5, 2.0, 3.0, 1.0],
        "opacity, into, mode, grouped"
    );
    assert_eq!(
        Op::end_layer_into(0.25, LayerInto::Canvas, layers::Mode::Normal, false),
        Op::end_layer(0.25),
        "a plain layer's end as before blend modes"
    );
    assert_eq!(value("PAPER_WIDTH"), PAPER_SIZE[0]);
    assert_eq!(value("PAPER_HEIGHT"), PAPER_SIZE[1]);
    assert_eq!(value("PAPER_LEVELS"), level_count());
    assert_eq!(value("MOMENT_ONE"), MOMENT_ONE);
}

#[test]
fn a_tail_span_is_laid_out_as_a_segment_and_the_curve_constants_reach_wgsl() {
    assert_eq!(
        Op::tail(&operands(Tip::Flat), 1).data,
        Op::segment(&operands(Tip::Flat), 1).data
    );
    let wgsl = prelude();
    assert!(wgsl.contains(&format!(
        "const MAX_PIECES: u32 = {}u;",
        crate::curve::MAX_PIECES
    )));
    assert!(wgsl.contains("const TOLERANCE: f32 = 0.0625;"));
}

/// `mip_offset` in deposit.wgsl, as the shader computes it.
fn mip_offset(bits: u32, level: u32) -> u32 {
    ((1 << bits) - (1 << (bits - 2 * level))) / 3 * 4
}

#[test]
fn the_closed_form_mip_offsets_are_the_levels_summed() {
    let module = parsed();
    let [paper, shape] = ["PAPER_BITS", "SHAPE_BITS"].map(|name| constant(&module, name));
    let mut words = 0;
    for level in 0..level_count() {
        assert_eq!(mip_offset(paper, level), words, "paper level {level}");
        words += (PAPER_SIZE[0] >> level).max(1) * (PAPER_SIZE[1] >> level).max(1);
    }
    assert_eq!(words, crate::paper::paper_words());
    let size = crate::shapes::SHAPE_SIZE;
    let mut words = 0;
    for level in 0..crate::shapes::SHAPE_LEVELS {
        assert_eq!(mip_offset(shape, level), words, "shape level {level}");
        words += (size >> level) * (size >> level);
    }
    assert_eq!(words, crate::shapes::shape_words());
}
