//! Snapshots standing through the strokes that do not reach their tile (D91, D96): a stroke
//! appended leaves alone the snapshots it does not reach, so the tiles one frame meets may stand
//! before different units; a region resumes at the latest unit they all stand before, else the
//! tiles lacking strokes are brought up. Every frame and lift is still the canvas drawn cold (at a
//! tier its ready tiles the export); after a stroke the worker lays again only what it reached;
//! with a view, only the tiles the screen shows and a ring round them hold snapshots.

use std::collections::BTreeSet;

use openpaint_core::{Artwork, CanvasSize, Color};
use openpaint_render::{Focus, PixelRect, changed_regions, take_spent};

use crate::drawing::path;
use crate::drawing_ahead::{Loop, dense, drawn, own_core, stroke};
use crate::fixture::{plain, with_stroke};
use crate::tiers::frame;

/// The 4x tier about the canvas's middle: canvas x 106 to 214 and y 59 to 141 shown, its tiles
/// 64 canvas pixels a side.
fn tier() -> Option<(openpaint_render::ViewFrame, u32)> {
    Some((frame(3.7, [160.0, 100.0]), 4))
}

/// The tiles of `side` canvas pixels that `rects` meet on a canvas of `size` pixels.
fn met(rects: &[PixelRect], side: u32, size: [u32; 2]) -> BTreeSet<[u32; 2]> {
    let mut tiles = BTreeSet::new();
    for rect in rects {
        let [x0, y0] = rect.min.map(|at| at.max(0) as u32 / side);
        let [x1, y1] = [0, 1].map(|axis| {
            let end = rect.max[axis].clamp(0, size[axis] as i32) as u32;
            end.div_ceil(side)
        });
        for row in y0..y1 {
            for column in x0..x1 {
                tiles.insert([column, row]);
            }
        }
    }
    tiles
}

/// Strokes one after another without the screen resting between them (the pen hovers): the first
/// leaves the tiles it reached lagging and the rest standing past it, the second crosses both
/// kinds (they meet at the unit before the first, which it lays again), one idle frame brings a
/// few up, the third crosses tiles standing before units they share none of (brought up in its
/// frames). Every frame and lift cold, at the canvas and at a tier; after a settle a stroke takes
/// no snapshot.
#[test]
fn strokes_over_tiles_standing_before_different_units_are_cold() {
    let core = own_core();
    let base = dense();
    for shown in [None, tier()] {
        let mut screen = Loop::new(&core, &base, shown);
        screen.settle(&core, &base);
        screen.touched = true;
        let _ = take_spent();
        let first = drawn(&core, &mut screen, &base, ([120.0, 90.0], 30.0));
        let second = drawn(&core, &mut screen, &first, ([200.0, 100.0], 90.0));
        screen.touched = false;
        screen.frame(&core, (&second, None), &[]);
        screen.touched = true;
        let third = drawn(&core, &mut screen, &second, ([160.0, 110.0], 100.0));
        let spent = take_spent();
        match shown {
            None => assert!(spent.canvas.laid_again > 0, "{spent:?}"),
            Some(_) => assert!(
                spent.tier.laid_again > 0 && spent.tier.snapshots > 0,
                "{spent:?}"
            ),
        }
        screen.settle(&core, &third);
        stroke(&core, &mut screen, &third, [150.0, 95.0]);
    }
}

/// After a stroke the raster worker lays again the snapshots of the tiles it reached and fills
/// the ring's tiles it reached, nothing else: no more regions than those tiles.
#[test]
fn after_a_stroke_and_a_settle_the_worker_drew_no_more_regions_than_the_tiles_it_reached() {
    let core = own_core();
    let base = dense();
    let size = [320, 200];
    for shown in [None, tier()] {
        let mut screen = Loop::new(&core, &base, shown);
        screen.settle(&core, &base);
        screen.touched = true;
        let lifted = drawn(&core, &mut screen, &base, ([150.0, 90.0], 20.0));
        let before = core.counts().rastered;
        screen.settle(&core, &lifted);
        let drew = core.counts().rastered - before;
        let reached = changed_regions(&base, &lifted);
        let mut tiles = met(&reached, 256, size).len();
        if let Some((_, scale)) = shown {
            tiles += met(&reached, 256 / scale, size).len();
        }
        assert!(
            drew > 0 && drew <= tiles as u64,
            "{drew} regions, {tiles} tiles"
        );
    }
}

/// With a view, the snapshots are taken for the tiles it shows and a ring of one tile, and only
/// those are held: looking elsewhere, the new place's are taken in the slots the old ones held.
#[test]
fn with_a_view_only_the_shown_tiles_and_their_ring_hold_snapshots() {
    let core = own_core();
    let size = [2048, 1536];
    let mut artwork = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    for k in 0..6u16 {
        let at = [300.0 + 280.0 * f32::from(k), 250.0 + 200.0 * f32::from(k)];
        artwork = with_stroke(
            &artwork,
            (plain(40.0, 0.7, Color::BLACK), path(at, 120.0, 12)),
        );
    }
    let mut screen = Loop::new(&core, &artwork, None);
    let look = |screen: &mut Loop, min: [i32; 2], max: [i32; 2]| {
        let shown = PixelRect { min, max };
        let middle = [0, 1].map(|axis| (min[axis] + max[axis]) as f32 / 2.0);
        screen.target.look_at(Some(Focus { middle, shown }));
        let ring = PixelRect {
            min: min.map(|at| at - 256),
            max: max.map(|at| at + 256),
        };
        met(&[ring], 256, size)
    };
    let here = look(&mut screen, [600, 500], [900, 700]);
    let before = core.counts().rastered;
    screen.settle(&core, &artwork);
    let taken = core.counts().rastered - before;
    assert_eq!(
        taken,
        here.len() as u64,
        "a snapshot for each tile of {here:?}"
    );
    let held = screen.target.bytes().snapshots;
    let there = look(&mut screen, [1500, 1100], [1800, 1300]);
    let before = core.counts().rastered;
    screen.settle(&core, &artwork);
    let taken = core.counts().rastered - before;
    assert_eq!(taken, there.difference(&here).count() as u64);
    assert!(
        screen.target.bytes().snapshots <= held,
        "the slots let go are taken again"
    );
}
