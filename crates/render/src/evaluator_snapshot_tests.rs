//! S56 at a stroke's ends: the pen touching down keeps the scene and the lift's scene copies the
//! spans the scene before had, item for item; a lift is evaluated over the in-progress stroke's
//! snapshot (a few entries a tile, the commit cold, bit for bit); and strokes committed are laid
//! onto that snapshot in idle frames within their budget, ending bit for bit where laying them
//! all at once ends.

use std::sync::Arc;

use openpaint_core::{
    ActiveStroke, BrushRecipe, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeStyle,
};

use super::{Artwork, CanvasTarget, RegionEvaluator};
use crate::core::{Core, test_core};
use crate::pace::BACKGROUND_NS;
use crate::profile::take_spent;
use crate::scene::{ActivePlace, Scene};
use crate::{FrameRequest, PixelRect, RgbaImage};

const SIZE: [u32; 2] = [320, 192];

fn sample(x: f32, y: f32, time_ms: u32) -> StrokeSample {
    let point = CanvasPoint::new(x, y).expect("point");
    StrokeSample::new(point, 0.8, 0.0, 0.0, time_ms).expect("sample")
}

/// A plain round brush, its colour by `seed`.
fn style(size: f32, seed: u32) -> StrokeStyle {
    let share = (seed % 7) as f32 / 7.0;
    let color = Color::rgb(0.2 + 0.6 * share, 0.3, 0.8 - 0.6 * share).expect("colour");
    StrokeStyle::new(Arc::new(BrushRecipe::plain()), size, 0.7, color).expect("style")
}

/// Line `k` of many crossing the canvas's middle: 160 px turned by `k` golden angles, 12 samples.
fn line(k: usize) -> Vec<StrokeSample> {
    let angle = k as f32 * 2.399_963_2;
    (0..12u16)
        .map(|i| {
            let s = 160.0 * (f32::from(i) / 11.0 - 0.5);
            let shift = ((k * 37) % 41) as f32 - 20.0;
            let [x, y] = [160.0 + shift + s * angle.cos(), 96.0 + s * angle.sin()];
            sample(x, y, u32::from(i) * 8)
        })
        .collect()
}

fn painted(count: usize) -> Artwork {
    let blank = Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"));
    (0..count).fold(blank, |artwork, k| {
        let samples = line(k);
        let mut session = artwork
            .begin_stroke(style(18.0, k as u32), samples[0])
            .expect("begin");
        session.extend(&samples[1..]).expect("extend");
        session.finish().artwork
    })
}

fn request<'a>(artwork: &'a Artwork, active: Option<&'a ActiveStroke>) -> FrameRequest<'a> {
    FrameRequest {
        artwork,
        active,
        background: Color::WHITE,
    }
}

fn shown(core: &Core, artwork: &Artwork) -> CanvasTarget {
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(artwork, None), &[PixelRect::full(SIZE)])
        .expect("shown");
    target
}

fn cold(core: &Core, artwork: &Artwork) -> RgbaImage {
    shown(core, artwork).read(core).expect("read")
}

/// Draws `samples` over `base` on `target` a sample a frame, all but the last, which arrives
/// with the lift (so the lift's frame evaluates the stroke's last spans), and commits it.
fn drawn(core: &Core, target: &mut CanvasTarget, base: &Artwork, k: usize) -> Artwork {
    let samples = line(k);
    let mut session = base
        .begin_stroke(style(30.0, k as u32), samples[0])
        .expect("begin");
    for end in 1..samples.len() {
        if end > 1 {
            session.extend(&samples[end - 1..end]).expect("extend");
        }
        RegionEvaluator::new(core, target)
            .evaluate(&request(base, Some(session.active())), &[])
            .expect("frame");
    }
    session.extend(&samples[samples.len() - 1..]).expect("last");
    let committed = session.finish().artwork;
    RegionEvaluator::new(core, target)
        .evaluate(&request(&committed, None), &[])
        .expect("lift");
    committed
}

#[test]
fn a_pen_down_keeps_the_scene_and_a_lift_copies_the_spans_it_had() {
    let core = test_core();
    let base = painted(120);
    let mut target = shown(core, &base);
    let mut session = base
        .begin_stroke(style(30.0, 9), sample(50.0, 50.0, 0))
        .expect("begin");
    session.extend(&[sample(90.0, 70.0, 8)]).expect("extend");
    let _ = take_spent();
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(&base, Some(session.active())), &[])
        .expect("pen-down");
    assert_eq!(take_spent().scenes, 0, "the pen-down keeps the scene");
    let done = session.finish().artwork;
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(&done, None), &[])
        .expect("lift");
    assert_eq!(take_spent().scenes, 1, "the lift builds the new version's");
    let kept = &target.scene.as_ref().expect("a scene").scene;
    let cold = Scene::build(&done, Some(&ActivePlace::of_request(&done, None))).expect("cold");
    assert!(
        kept.items == cold.items,
        "the spans copied are the spans computed"
    );
    assert!(target.read(core).expect("read") == shown(core, &done).read(core).expect("cold"));
}

#[test]
fn the_lift_over_the_snapshot_equals_the_commit_cold_and_lists_few_ops() {
    let core = test_core();
    let base = painted(200);
    let mut target = shown(core, &base);
    let committed = drawn(core, &mut target, &base, 1000);
    let work = target.work();
    assert!(
        work.tiles > 0,
        "the lift evaluated its last spans: {work:?}"
    );
    let each = work.entries as f32 / work.tiles as f32;
    assert!(
        each < 12.0,
        "{each} entries a tile: the 200 strokes are walked"
    );
    let shown = target.read(core).expect("read");
    assert!(
        shown == cold(core, &committed),
        "the lift = the commit, cold"
    );
}

#[test]
fn strokes_laid_in_idle_frames_equal_strokes_laid_at_once() {
    let core = test_core();
    let base = painted(60);
    let (mut idle, mut once) = (shown(core, &base), shown(core, &base));
    let [one, two] = [&mut idle, &mut once].map(|target| drawn(core, target, &base, 7));
    // A tile a frame (a frame takes one piece whatever its budget; the budget set anew each frame
    // forgives what the last spent past it), then all in one frame.
    let mut frames = 0;
    loop {
        core.set_frame_budget(1.0);
        core.begin_frame();
        RegionEvaluator::new(core, &mut idle)
            .evaluate_frame(&request(&one, None), &[])
            .expect("idle frame");
        frames += 1;
        if !idle.catching_up() {
            break;
        }
        assert!(frames < 5000, "the snapshot never caught up");
    }
    core.set_frame_budget(1e15);
    core.begin_frame();
    RegionEvaluator::new(core, &mut once)
        .evaluate_frame(&request(&two, None), &[])
        .expect("one frame");
    core.set_frame_budget(BACKGROUND_NS);
    assert!(frames > 2 && !once.catching_up(), "{frames} frames");
    let held = |target: &CanvasTarget| target.held.as_ref().expect("held").contents(core);
    let (laid, at_once) = (held(&idle), held(&once));
    assert!(laid.1 == at_once.1, "every tile holds as many items");
    assert!(laid.0 == at_once.0, "the snapshots are equal bit for bit");
    assert!(idle.read(core).expect("read") == cold(core, &one));
}
