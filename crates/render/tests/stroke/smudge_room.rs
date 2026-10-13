//! Smudge residuals within a room (S51): a target whose arena should hold almost nothing and
//! binds little (as the tablet's 128 MiB does at its scale) keeps only the residuals near what
//! it evaluates, some only in part, dropping the rest and evaluating them again from the artwork
//! when a later evaluation needs them. Every pixel stays what a cold evaluation (an arena holding
//! everything) gives, bit for bit: the canvas evaluated a quarter at a time, a smudge drawn frame
//! by frame over earlier ones, its commit, undo and redo; an evaluation needing more than the
//! room holds it, and is cut back right after.

use openpaint_core::{Artwork, Color, StrokeSample, StrokeStyle, StrokeTool};
use openpaint_render::{
    CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, changed_regions,
};

use crate::fixture::{SIZE, airbrush, blank, cold, core, plain, request, sample, with_stroke};

/// The most the tight arena binds: about a third of what every residual whole takes.
const LIMIT: u64 = 3 << 20;

/// Two coloured bands and nine smudges across them, each crossing the ones before.
fn smudged() -> Artwork {
    let band = |y: f32, color: Color| {
        let samples = (0..2u16).map(|i| sample(10.0 + 300.0 * f32::from(i), y, 1.0, 0));
        (plain(50.0, 1.0, color), samples.collect::<Vec<_>>())
    };
    let red = Color::rgb(0.9, 0.1, 0.05).expect("red");
    let blue = Color::rgb(0.05, 0.2, 0.85).expect("blue");
    let mut artwork = with_stroke(&with_stroke(&blank(), band(50.0, red)), band(140.0, blue));
    for k in 0..9u16 {
        artwork = with_stroke(&artwork, (style(), path(k)));
    }
    artwork
}

fn style() -> StrokeStyle {
    StrokeStyle::new(airbrush(), 30.0, 0.85, Color::BLACK)
        .expect("style")
        .with_tool(StrokeTool::Smudge)
}

/// Smudge `k`: a slanted line from a place of its own, down or up across the bands.
fn path(k: u16) -> Vec<StrokeSample> {
    let start = 20.0 + 32.0 * f32::from(k);
    let down = k.is_multiple_of(2);
    (0..24u16)
        .map(|i| {
            let t = f32::from(i) / 23.0;
            let y = if down {
                20.0 + 160.0 * t
            } else {
                180.0 - 160.0 * t
            };
            sample(start + 60.0 * t, y, 0.5 + 0.5 * t, u32::from(i) * 8)
        })
        .collect()
}

/// A target whose arena binds `LIMIT`.
fn tight(core: &Core) -> CanvasTarget {
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    target.set_arena_limit(LIMIT);
    target
}

/// One frame into the tight target: the budget's step keeping its arena within a few bytes,
/// then `request` over `regions`.
fn frame(
    core: &Core,
    target: &mut CanvasTarget,
    request: &FrameRequest<'_>,
    regions: &[PixelRect],
) {
    target.set_arena_room(core, 64).expect("room");
    RegionEvaluator::new(core, target)
        .evaluate(request, regions)
        .expect("evaluate");
}

/// The canvas evaluated a quarter a frame.
fn by_quarters(core: &Core, target: &mut CanvasTarget, artwork: &Artwork) {
    let [w, h] = SIZE.map(|side| i32::try_from(side / 2).expect("side"));
    for [x, y] in [[0, 0], [w, 0], [0, h], [w, h]] {
        let quarter = PixelRect {
            min: [x, y],
            max: [x + w, y + h],
        };
        frame(core, target, &request(artwork), &[quarter]);
    }
}

#[test]
fn a_canvas_evaluated_a_quarter_at_a_time_in_a_tight_arena_equals_cold() {
    let core = core();
    let artwork = smudged();
    let mut open = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut open)
        .evaluate(&request(&artwork), &[PixelRect::full(SIZE)])
        .expect("open");
    let mut target = tight(&core);
    by_quarters(&core, &mut target, &artwork);
    assert_eq!(target.read(&core).expect("read"), cold(&core, &artwork));
    let (tight, all) = (target.bytes().arena, open.bytes().arena);
    assert!(
        tight < all,
        "the tight arena holds less: {tight} against {all}"
    );
    // Again: the quarters' residuals let go are evaluated anew, the same.
    by_quarters(&core, &mut target, &artwork);
    assert_eq!(target.read(&core).expect("read"), cold(&core, &artwork));
}

#[test]
fn an_evaluation_needing_more_than_the_room_is_cut_back_after_it_and_stays_cold() {
    let core = core();
    let artwork = smudged();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    // About a ninth of what the residuals take whole.
    target.set_arena_room(&core, 1 << 20).expect("room");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&artwork), &[PixelRect::full(SIZE)])
        .expect("whole");
    let grown = target.bytes().arena;
    let freed = target.settle_arena(&core).expect("settle");
    assert!(freed > 0, "cut back at once");
    assert_eq!(target.bytes().arena, grown - freed);
    // Nothing needed since, and nothing more to give back: within the room.
    assert_eq!(target.settle_arena(&core).expect("again"), 0);
    by_quarters(&core, &mut target, &artwork);
    assert_eq!(target.read(&core).expect("read"), cold(&core, &artwork));
}

#[test]
fn a_smudge_over_dropped_residuals_drawn_committed_undone_and_redone_equals_cold() {
    let core = core();
    let base = smudged();
    let mut target = tight(&core);
    by_quarters(&core, &mut target, &base);
    // Across every earlier smudge, a few samples a frame.
    let across: Vec<StrokeSample> = (0..36u16)
        .map(|i| sample(15.0 + 8.0 * f32::from(i), 95.0, 0.9, u32::from(i) * 8))
        .collect();
    let mut session = base.begin_stroke(style(), across[0]).expect("begin");
    for chunk in across[1..].chunks(5) {
        session.extend(chunk).expect("extend");
        let drawing = FrameRequest {
            active: Some(session.active()),
            ..request(&base)
        };
        frame(&core, &mut target, &drawing, &[]);
    }
    let finished = session.finish().artwork;
    frame(&core, &mut target, &request(&finished), &[]);
    let committed = cold(&core, &finished);
    assert_eq!(
        target.read(&core).expect("read"),
        committed,
        "commit = cold"
    );
    assert_ne!(committed, cold(&core, &base), "the smudge moved colour");
    let undo = changed_regions(&finished, &base);
    frame(&core, &mut target, &request(&base), &undo);
    assert_eq!(
        target.read(&core).expect("read"),
        cold(&core, &base),
        "undo"
    );
    let redo = changed_regions(&base, &finished);
    frame(&core, &mut target, &request(&finished), &redo);
    assert_eq!(target.read(&core).expect("read"), committed, "redo");
}
