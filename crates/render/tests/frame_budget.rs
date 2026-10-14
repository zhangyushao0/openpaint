//! The frame budget for background work (S53): a screen frame takes of the waiting tiles, a view
//! tier's missing tiles and a residual being evaluated only what its budget's estimate holds (or a
//! single piece), so the canvas fills in over frames to the cold canvas, a pinch's and a pan's
//! frames stay within it, a stroke drawn while tiles wait is evaluated at once and leaves them
//! waiting, and every smudge residual is evaluated once while nothing changes: by the canvas's
//! fill, none again by the tiers' fills or a pan.
//!
//! A test binary of its own: the tests set the device's frame budget, which every frame on the
//! device follows, so they run one at a time on a device no other test uses.

use std::f32::consts::PI;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use openpaint_core::{
    ActiveStroke, Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, StrokeSample, StrokeStyle,
    StrokeTool,
};
use openpaint_render::{
    BACKGROUND_NS, CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, RgbaImage,
    ViewCache, ViewFrame, stroke_region,
};

const SIZE: [u32; 2] = [320, 200];
/// A small budget, so these small canvases take many frames whatever the GPU.
const BUDGET: f64 = 10_000.0;
const SCREEN: [u32; 2] = [400, 300];

fn core() -> &'static Core {
    static CORE: OnceLock<Core> = OnceLock::new();
    CORE.get_or_init(|| Core::headless().expect("a GPU adapter is required for render tests"))
}

/// The tests one at a time, each on the small budget.
fn alone() -> MutexGuard<'static, ()> {
    static ONE: Mutex<()> = Mutex::new(());
    let guard = ONE.lock().unwrap_or_else(PoisonError::into_inner);
    core().set_frame_budget(BUDGET);
    guard
}

fn sample(x: f32, y: f32, pressure: f32, time_ms: u32) -> StrokeSample {
    let position = CanvasPoint::new(x, y).expect("point");
    StrokeSample::new(position, pressure, 0.0, 0.0, time_ms).expect("sample")
}

fn plain(size: f32, color: Color) -> StrokeStyle {
    StrokeStyle::new(Arc::new(BrushRecipe::plain()), size, 0.8, color).expect("style")
}

fn with_stroke(artwork: &Artwork, style: StrokeStyle, samples: &[StrokeSample]) -> Artwork {
    let mut stroke = artwork.begin_stroke(style, samples[0]).expect("begin");
    stroke.extend(&samples[1..]).expect("extend");
    stroke.finish().artwork
}

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(SIZE[0], SIZE[1]).expect("size"))
}

/// `count` strokes crossing the canvas's middle at every angle (as `pen-many` lays them).
fn dense(count: u32) -> Artwork {
    (0..count).fold(blank(), |artwork, k| {
        let angle = k as f32 * 2.399_963_2;
        let samples: Vec<StrokeSample> = (0..20u16)
            .map(|i| {
                let s = 180.0 * (f32::from(i) / 19.0 - 0.5);
                let p = 0.6 + 0.3 * (PI * f32::from(i) / 19.0).sin();
                sample(
                    160.0 + s * angle.cos(),
                    100.0 + s * angle.sin(),
                    p,
                    u32::from(i) * 8,
                )
            })
            .collect();
        let color = Color::rgb(0.2 + 0.08 * (k % 7) as f32, 0.3, 0.6).expect("colour");
        with_stroke(&artwork, plain(24.0, color), &samples)
    })
}

/// Two coloured bands and `count` smudges across them.
fn smudged(count: u16) -> Artwork {
    let red = Color::rgb(0.9, 0.1, 0.05).expect("red");
    let blue = Color::rgb(0.05, 0.2, 0.85).expect("blue");
    let band = |y| [sample(10.0, y, 1.0, 0), sample(310.0, y, 1.0, 8)];
    let mut artwork = with_stroke(&blank(), plain(50.0, red), &band(50.0));
    artwork = with_stroke(&artwork, plain(50.0, blue), &band(140.0));
    let smudge = plain(30.0, Color::BLACK).with_tool(StrokeTool::Smudge);
    for k in 0..count {
        let start = 20.0 + 32.0 * f32::from(k);
        let path: Vec<StrokeSample> = (0..24u16)
            .map(|i| {
                let t = f32::from(i) / 23.0;
                sample(start + 60.0 * t, 20.0 + 160.0 * t, 0.9, u32::from(i) * 8)
            })
            .collect();
        artwork = with_stroke(&artwork, smudge.clone(), &path);
    }
    artwork
}

fn request<'a>(artwork: &'a Artwork, active: Option<&'a ActiveStroke>) -> FrameRequest<'a> {
    FrameRequest {
        artwork,
        active,
        background: Color::WHITE,
    }
}

fn cold(artwork: &Artwork) -> RgbaImage {
    let mut target = CanvasTarget::new(core(), SIZE).expect("target");
    RegionEvaluator::new(core(), &mut target)
        .evaluate(&request(artwork, None), &[PixelRect::full(SIZE)])
        .expect("cold");
    target.read(core()).expect("read")
}

/// One screen frame: the budget whole again, the canvas, then (with a view) its tiers; the
/// frame's background work must have kept within the budget.
fn frame(
    target: &mut CanvasTarget,
    (artwork, active): (&Artwork, Option<&ActiveStroke>),
    regions: &[PixelRect],
    view: Option<(&mut ViewCache, &ViewFrame, f64)>,
) {
    let core = core();
    core.begin_frame();
    let request = request(artwork, active);
    RegionEvaluator::new(core, target)
        .evaluate_frame(&request, regions)
        .expect("frame");
    if let Some((cache, frame, now)) = view {
        cache
            .update(core, target, &request, (frame, now))
            .expect("view");
    }
    // What this frame took shows as the frame before the next one's.
    core.begin_frame();
    let taken = core.background();
    assert!(taken.within(BUDGET), "the frame took {taken:?}");
}

/// `zoom` screen pixels per canvas pixel, canvas point `centre` in the screen's middle.
fn view(zoom: f32, [cx, cy]: [f32; 2]) -> ViewFrame {
    let s = 1.0 / zoom;
    let [w, h] = SCREEN.map(|side| side as f32 / 2.0);
    ViewFrame {
        to_canvas: [s, 0.0, cx - w * s, 0.0, s, cy - h * s],
        origin: [0, 0],
        size: SCREEN,
    }
}

#[test]
fn a_frame_takes_its_budget_and_the_rest_fills_in_over_the_next_frames() {
    let _one = alone();
    let artwork = dense(300);
    let mut target = CanvasTarget::new(core(), SIZE).expect("target");
    let before = core().counts();
    frame(
        &mut target,
        (&artwork, None),
        &[PixelRect::full(SIZE)],
        None,
    );
    assert!(target.pending(), "a dense canvas takes more than a frame");
    let mut frames = 1;
    while target.pending() {
        frame(&mut target, (&artwork, None), &[], None);
        frames += 1;
        assert!(frames < 100_000, "the canvas never filled in");
    }
    let counts = core().counts().since(before);
    eprintln!(
        "the dense canvas filled in over {frames} frames, {} plans",
        counts.plans
    );
    assert!(frames > 2, "{frames} frames");
    assert!(
        counts.plans * 8 < frames,
        "batches binned once, not every frame: {} plans",
        counts.plans
    );
    assert!(
        target.read(core()).expect("read") == cold(&artwork),
        "filled in = cold"
    );
}

#[test]
fn a_stroke_drawn_while_tiles_wait_shows_at_once_and_leaves_them_waiting() {
    let _one = alone();
    let artwork = dense(300);
    let mut target = CanvasTarget::new(core(), SIZE).expect("target");
    frame(
        &mut target,
        (&artwork, None),
        &[PixelRect::full(SIZE)],
        None,
    );
    let samples: Vec<StrokeSample> = (0..8u16)
        .map(|i| sample(40.0 + 12.0 * f32::from(i), 30.0, 0.9, u32::from(i) * 8))
        .collect();
    let style = plain(10.0, Color::BLACK);
    let mut stroke = artwork
        .begin_stroke(style.clone(), samples[0])
        .expect("begin");
    stroke.extend(&samples[1..]).expect("extend");
    frame(&mut target, (&artwork, Some(stroke.active())), &[], None);
    assert!(target.pending(), "the waiting tiles wait for the stroke");
    let so_far = stroke.finish().artwork;
    let reached = stroke_region(&style, &samples, 0).expect("region");
    let [shown, wanted] = [target.read(core()).expect("read"), cold(&so_far)];
    for y in reached.min[1].max(0)..reached.max[1].min(SIZE[1] as i32) {
        for x in reached.min[0].max(0)..reached.max[0].min(SIZE[0] as i32) {
            let at = (y as usize * SIZE[0] as usize + x as usize) * 4;
            assert_eq!(
                shown.pixels()[at..at + 4],
                wanted.pixels()[at..at + 4],
                "{x}, {y}"
            );
        }
    }
    let mut frames = 0;
    while target.pending() {
        frame(&mut target, (&so_far, None), &[], None);
        frames += 1;
        assert!(frames < 100_000, "the canvas never filled in");
    }
    assert!(target.read(core()).expect("read") == wanted, "then = cold");
}

#[test]
fn a_pinch_and_a_pan_take_no_more_than_their_budget_and_the_tiers_fill_in() {
    let _one = alone();
    let artwork = dense(300);
    let mut target = CanvasTarget::new(core(), SIZE).expect("target");
    let mut cache = ViewCache::new(core());
    let mut now = 0.0;
    let mut step = |target: &mut CanvasTarget, cache: &mut ViewCache, shown: &ViewFrame| {
        now += 1.0 / 120.0;
        frame(target, (&artwork, None), &[], Some((cache, shown, now)));
    };
    frame(
        &mut target,
        (&artwork, None),
        &[PixelRect::full(SIZE)],
        None,
    );
    // A pinch from 2.2 to 4 screen pixels a canvas pixel about the middle, then still, then a
    // pan; every frame within the budget (`frame`).
    for k in 0..240 {
        let zoom = 2.2 + 1.8 * (k as f32 / 60.0).min(1.0);
        step(&mut target, &mut cache, &view(zoom, [160.0, 100.0]));
    }
    for k in 0..240 {
        step(
            &mut target,
            &mut cache,
            &view(4.0, [160.0 - k as f32 * 0.3, 100.0]),
        );
    }
    let mut frames = 0;
    let last = view(4.0, [88.0, 100.0]);
    while target.pending() || cache.tier() != Some(4) || frames < 2 {
        step(&mut target, &mut cache, &last);
        frames += 1;
        assert!(frames < 100_000, "the tier never filled in");
    }
    assert!(target.read(core()).expect("read") == cold(&artwork));
}

#[test]
fn each_residual_is_evaluated_once_by_the_canvas_and_never_again_by_the_tiers() {
    let _one = alone();
    let artwork = smudged(9);
    let mut target = CanvasTarget::new(core(), SIZE).expect("target");
    let mut cache = ViewCache::new(core());
    let before = core().counts();
    frame(
        &mut target,
        (&artwork, None),
        &[PixelRect::full(SIZE)],
        None,
    );
    while target.pending() {
        frame(&mut target, (&artwork, None), &[], None);
    }
    let filled = core().counts().since(before);
    assert_eq!(filled.derived, 9, "each smudge's residual once");
    assert!(
        target.read(core()).expect("read") == cold(&artwork),
        "= cold"
    );
    let after = core().counts();
    let mut now = 0.0;
    for (zoom, centre) in [
        (2.2, [100.0, 80.0]),
        (4.0, [100.0, 80.0]),
        (4.0, [180.0, 120.0]),
    ] {
        let shown = view(zoom, centre);
        for _ in 0..2000 {
            now += 1.0 / 120.0;
            frame(
                &mut target,
                (&artwork, None),
                &[],
                Some((&mut cache, &shown, now)),
            );
        }
    }
    let tiers = core().counts().since(after);
    assert!(tiers.filled > 0, "the tiers filled in");
    assert_eq!(
        tiers.derived, 0,
        "no residual evaluated again while nothing changed"
    );
    core().set_frame_budget(BACKGROUND_NS);
}

/// A canvas shown complete from its display cache (S54) holds no residual yet: they are evaluated
/// over frames as background work, each once (none at once, outside the budget), while a 4x
/// tier waits for those its tiles read; the canvas = cold and nothing waits on it meanwhile.
#[test]
fn a_cached_canvas_evaluates_its_residuals_over_frames_and_its_tier_waits_for_them() {
    let _one = alone();
    let artwork = smudged(9);
    let mut source = CanvasTarget::new(core(), SIZE).expect("target");
    let all = [PixelRect::full(SIZE)];
    (RegionEvaluator::new(core(), &mut source))
        .evaluate(&request(&artwork, None), &all)
        .expect("cold");
    let pixels = source.copy_out(core()).bytes(core()).expect("read back");
    let mut target = CanvasTarget::new(core(), SIZE).expect("target");
    target.show_cached(core(), &pixels, true).expect("shown");
    let mut cache = ViewCache::new(core());
    let shown = view(4.0, [100.0, 80.0]);
    let before = core().counts();
    let mut frames = 0;
    while frames < 2000 || target.warming() {
        let now = f64::from(frames) / 120.0;
        let view = Some((&mut cache, &shown, now));
        frame(&mut target, (&artwork, None), &[], view);
        assert!(!target.pending(), "nothing waits on the canvas");
        frames += 1;
        assert!(frames < 40_000, "the residuals were never all evaluated");
    }
    let counts = core().counts().since(before);
    assert_eq!(counts.derived, 9, "each residual once, over frames");
    assert!(counts.filled > 0, "the tier filled in");
    assert!(target.read(core()).expect("read") == cold(&artwork));
    core().set_frame_budget(BACKGROUND_NS);
}

/// The residuals needed together pass the arena's room (the tablet's 96 MiB at its scale): each
/// placed beside the others the waiting tiles need, none let go while tiles that read it wait,
/// so the canvas fills in evaluating each once, to the cold canvas.
#[test]
fn a_canvas_fills_in_when_its_residuals_pass_the_arenas_room() {
    let _one = alone();
    let artwork = smudged(9);
    let mut target = CanvasTarget::new(core(), SIZE).expect("target");
    let before = core().counts();
    let mut frames = 0;
    let mut regions = vec![PixelRect::full(SIZE)];
    while frames == 0 || target.pending() {
        target.set_arena_room(core(), 1 << 20).expect("room");
        frame(&mut target, (&artwork, None), &regions, None);
        regions.clear();
        frames += 1;
        assert!(frames < 20_000, "the canvas never filled in");
    }
    let filled = core().counts().since(before);
    assert_eq!(
        filled.derived, 9,
        "each residual once, though they pass the room"
    );
    assert!(
        target.read(core()).expect("read") == cold(&artwork),
        "= cold"
    );
}
