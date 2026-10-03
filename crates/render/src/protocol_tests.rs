//! The protocol's layout test: the generated WGSL declares exactly the Rust records, opcodes,
//! named slots and blends.

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
        blend: Blend::Erase,
    };
    let op = Op::end_stroke(&paint);
    assert_eq!(op.data[PAINT_OPACITY], 0.4);
    assert_eq!(op.data[PAINT_GRAIN], 77.0);
    assert_eq!(op.data[PAINT_GRAIN_SCALE], 5.0);
    assert_eq!(op.data[PAINT_BLEND], 2.0);
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
    for (name, blend) in BLENDS {
        assert_eq!(value(name), blend as u32, "{name}");
    }
}
