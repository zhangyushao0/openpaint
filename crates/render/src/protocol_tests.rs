//! The protocol's layout test: the generated WGSL declares exactly the Rust records, opcodes,
//! named slots and blends.

use super::wgsl::LAYOUTS;
use super::*;
use crate::paper::{MOMENT_ONE, PAPER_SIZE, level_count};
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

fn operands(tip: Tip) -> SegmentOperands {
    SegmentOperands {
        a: [1.0, 2.0],
        b: [3.0, 4.0],
        radii: [5.0, 6.0],
        flows: [0.25, 0.75],
        edge: 0.5,
        contacts: [0.375, 0.625],
        tip,
    }
}

#[test]
fn named_slots_hold_what_the_constructors_write() {
    let op = Op::segment(&operands(Tip::Bell));
    assert_eq!(
        op.data[SEGMENT_FLOW..=SEGMENT_TIP],
        [0.25, 0.75, 0.5, 0.375, 0.625, 2.0],
        "flows, the edge, the contacts, the tip"
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
    };
    let op = Op::end_stroke(&paint);
    assert_eq!(op.data[PAINT_OPACITY], 0.4);
    assert_eq!(op.data[PAINT_GRAIN], 77.0);
    assert_eq!(op.data[PAINT_GRAIN_SCALE], 5.0);
    assert_eq!(op.data[PAINT_BLEND], 2.0);
    assert_eq!(op.data[PAINT_DENSITY], 0.875);
    assert_eq!(op.data[PAINT_PAPER..PAINT_CLIP], [4.5, 2.25, 0.3, 0.25]);
    assert_eq!(
        op.data[PAINT_CLIP..],
        [5.0, 11.0],
        "the clip, high part first"
    );
    assert_eq!(Op::end_active(&paint).data, op.data);
    let fill = Op::fill(&paint, (3 << 16) | 9);
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
}

#[test]
fn encoding_fills_exactly_the_record_size() {
    let op = Op::segment(&operands(Tip::Cover));
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
    assert_eq!(value("MASK_HEADER"), MASK_HEADER);
    for (name, slot) in SLOTS {
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
