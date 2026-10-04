//! Plan binning: which ops each tile lists, in scene order.

use super::*;
use crate::geom::Bounds;
use crate::protocol::{Blend, Op, Paint, SegmentOperands, Tip};

fn segment(bounds: [f32; 4]) -> Item {
    let operands = SegmentOperands {
        a: [0.0; 2],
        b: [0.0; 2],
        radii: [1.0; 2],
        flows: [1.0; 2],
        edge: 1.0,
        contacts: [0.0; 2],
        tip: Tip::Cover,
    };
    Item::Segment {
        op: Op::segment(&operands),
        bounds: Bounds(bounds),
    }
}

fn paint() -> Paint {
    Paint::solid([0.0; 3], Blend::Over)
}

fn black() -> Op {
    Op::end_stroke(&paint())
}

fn end_layer() -> Item {
    Item::EndLayer {
        op: Op::end_layer(1.0),
        chain: false,
        grouped: false,
    }
}

fn scene() -> Scene {
    Scene {
        items: vec![
            segment([2.0, 2.0, 20.0, 4.0]),
            Item::EndStroke(black()),
            segment([40.0, 2.0, 41.0, 3.0]),
            Item::EndStroke(black()),
            end_layer(),
        ],
        ..Scene::default()
    }
}

fn lists(plan: &Plan) -> Vec<([u32; 2], Vec<u32>)> {
    let list = |tile: &TileRecord| {
        let range = tile.start as usize..(tile.start + tile.count) as usize;
        (tile.origin, plan.indices[range].to_vec())
    };
    plan.tiles.iter().map(list).collect()
}

#[test]
fn tiles_list_only_the_ops_that_reach_them_in_scene_order() {
    let none = Fresh::default();
    let plan = Plan::build(&scene(), &none, [48, 20], &[PixelRect::full([48, 20])]);
    let expected = [
        ([0, 0], vec![0, 1, 4]),
        ([16, 0], vec![0, 1, 4]),
        ([32, 0], vec![2, 3, 4]),
        ([0, 16], vec![]),
        ([16, 16], vec![]),
        ([32, 16], vec![]),
    ];
    assert_eq!(lists(&plan), expected);
}

#[test]
fn a_mask_begins_where_the_paint_reached_and_the_end_lists_where_either_did() {
    let mut items = scene().items;
    let end = items.pop().expect("the end");
    items.push(Item::BeginMask(Op::begin_mask()));
    items.push(segment([18.0, 10.0, 19.0, 19.0]));
    items.push(Item::EndStroke(black()));
    items.push(end);
    let scene = Scene {
        items,
        ..Scene::default()
    };
    let plan = Plan::build(
        &scene,
        &Fresh::default(),
        [48, 20],
        &[PixelRect::full([48, 20])],
    );
    let listed = lists(&plan);
    assert_eq!(listed[0], ([0, 0], vec![0, 1, 4, 7]));
    assert_eq!(
        listed[1],
        ([16, 0], vec![0, 1, 4, 5, 6, 7]),
        "paint and mask"
    );
    assert_eq!(listed[2], ([32, 0], vec![2, 3, 4, 7]));
    assert_eq!(listed[4], ([16, 16], vec![5, 6, 7]), "the mask alone");
    assert_eq!(listed[3], ([0, 16], vec![]));
}

#[test]
fn a_fill_reaches_the_tiles_of_its_rectangle_and_ends_their_layer() {
    let mut items = scene().items;
    let fill = Item::Fill {
        op: Op::fill(&paint(), 0),
        bounds: Bounds([16.0, 16.0, 19.5, 19.5]),
    };
    items.insert(2, fill);
    let scene = Scene {
        items,
        ..Scene::default()
    };
    let plan = Plan::build(
        &scene,
        &Fresh::default(),
        [48, 20],
        &[PixelRect::full([48, 20])],
    );
    let listed = lists(&plan);
    assert_eq!(
        listed[4],
        ([16, 16], vec![2, 5]),
        "the fill, then its layer"
    );
    assert_eq!(listed[3], ([0, 16], vec![]));
    assert_eq!(listed[0], ([0, 0], vec![0, 1, 5]));
}

#[test]
fn regions_are_clipped_widened_to_tiles_and_deduplicated() {
    let regions = [
        PixelRect {
            min: [-50, -50],
            max: [3, 3],
        },
        PixelRect {
            min: [1, 1],
            max: [2, 2],
        },
        PixelRect {
            min: [47, 19],
            max: [900, 900],
        },
        PixelRect {
            min: [5, 5],
            max: [5, 9],
        },
    ];
    let plan = Plan::build(&scene(), &Fresh::default(), [48, 20], &regions);
    let expected = [([0, 0], vec![0, 1, 4]), ([32, 16], vec![])];
    assert_eq!(lists(&plan), expected);
}

#[test]
fn every_tile_paints_the_active_stroke_after_the_fresh_segments_that_reach_it() {
    let scene = Scene {
        items: vec![
            segment([2.0, 2.0, 4.0, 4.0]),
            Item::EndStroke(black()),
            Item::Active,
            end_layer(),
        ],
        ..Scene::default()
    };
    let fresh = Fresh {
        ops: vec![black(); 2],
        bounds: vec![Bounds([20.0, 2.0, 22.0, 3.0])],
    };
    let plan = Plan::build(&scene, &fresh, [48, 16], &[PixelRect::full([48, 16])]);
    let expected = [
        ([0, 0], vec![0, 1, FRESH, 3]),
        ([16, 0], vec![FRESH | 1, FRESH, 3]),
        ([32, 0], vec![FRESH, 3]),
    ];
    assert_eq!(lists(&plan), expected);
}
