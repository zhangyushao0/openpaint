//! Snapshots under the next stroke (S63, DECISIONS D90) as the frame loop keeps them: taken ahead
//! by the raster worker while nothing touches the screen, they outlive a stroke (its successor
//! over the same place lays it from the snapshots taken before it), an undo lets go only those it
//! reaches, a layer switch every one; after each, the strokes drawn take no snapshot on the frame
//! loop and every frame and lift is the canvas drawn cold. At a view tier the same for its shown
//! tiles, every ready tile its export. Without the worker (the tests' screen) strokes one after
//! another over a dense place are cold too, their snapshots taken and brought up by the frames.

use openpaint_core::{ActiveStroke, Artwork, Color, LayerCommand};
use openpaint_render::{
    CanvasTarget, Core, FrameRequest, PixelRect, RasterDrawer, ViewCache, ViewFrame,
    changed_regions,
};

use crate::drawing::{begun, committed, path, screen, shows};
use crate::fixture::{SIZE, cold, plain, with_stroke};
use crate::raster::{color, every_raster_source, layer};
use crate::scale::whole;
use crate::tiers::{assert_tiles, frame};

/// The canvas (and its view tier, shown through `view`) as the frame loop drives it, the raster
/// worker's orders served after each frame.
struct Loop {
    target: CanvasTarget,
    view: ViewCache,
    drawer: RasterDrawer,
    shown: Option<(ViewFrame, u32)>,
    /// The frame clock, 60 frames a second, and whether the last frame's tier waited.
    now: f64,
    pending: bool,
}

impl Loop {
    /// `artwork` opened, shown through `shown` (its tier's scale) or at fit.
    fn new(core: &Core, artwork: &Artwork, shown: Option<(ViewFrame, u32)>) -> Self {
        let mut target = CanvasTarget::new(core, SIZE).expect("target");
        target
            .draw(core, &request(artwork, None), &[PixelRect::full(SIZE)])
            .expect("opened");
        Self {
            target,
            view: ViewCache::new(core),
            drawer: RasterDrawer::default(),
            shown,
            now: 0.0,
            pending: false,
        }
    }

    /// One frame of `artwork` (with the stroke `active`, else while nothing touches the screen)
    /// with `regions` changed.
    fn frame(
        &mut self,
        core: &Core,
        (artwork, active): (&Artwork, Option<&ActiveStroke>),
        regions: &[PixelRect],
    ) {
        core.begin_frame_idle(active.is_none());
        let request = request(artwork, active);
        self.target
            .draw_frame(core, &request, regions)
            .expect("frame");
        if let Some((view, _)) = self.shown {
            self.now += 1.0 / 60.0;
            let work = (self.view)
                .update(core, &mut self.target, &request, (&view, self.now))
                .expect("view");
            self.pending = work.pending();
        }
        self.drawer.serve(core, (&mut self.target, &mut self.view));
    }

    /// Idle frames of `artwork` until nothing waits and every snapshot is taken.
    fn settle(&mut self, core: &Core, artwork: &Artwork) {
        for _ in 0..400 {
            self.frame(core, (artwork, None), &[]);
            let taking = self.target.taking_ahead() || self.view.taking_ahead();
            if !self.target.pending() && !self.pending && !taking {
                return;
            }
        }
        panic!("the snapshots were never taken");
    }

    /// The canvas is `artwork` drawn cold, and every ready tile of the tier its export.
    fn shows(&self, core: &Core, artwork: &Artwork, what: &str) {
        let read = self.target.read(core).expect("read");
        assert!(read == cold(core, artwork), "{what}");
        if let Some((_, scale)) = self.shown {
            let export = whole(core, artwork, scale);
            let compared = assert_tiles(core, &self.view, (&export, scale));
            assert!(compared > 0, "{what}: tiles of the tier");
        }
    }
}

fn request<'a>(artwork: &'a Artwork, active: Option<&'a ActiveStroke>) -> FrameRequest<'a> {
    FrameRequest {
        artwork,
        active,
        background: Color::WHITE,
    }
}

/// A stroke about `centre` drawn over `artwork` a few samples a frame and lifted, each frame cold
/// and none taking a snapshot; the artwork after.
fn stroke(core: &Core, screen: &mut Loop, artwork: &Artwork, centre: [f32; 2]) -> Artwork {
    let style = plain(28.0, 0.8, color(0.9, 0.3, 0.2));
    let samples = path(centre, 80.0, 25);
    let before = core.counts().snapshots;
    let mut session = begun(artwork, (&style, &samples[..1]), None);
    let mut end = 1;
    for chunk in samples[1..].chunks(4) {
        session.extend(chunk).expect("extend");
        end += chunk.len();
        screen.frame(core, (artwork, Some(session.active())), &[]);
        let so_far = committed(artwork, (&style, &samples[..end]), None);
        screen.shows(core, &so_far, &format!("{centre:?} after {end} samples"));
    }
    let lifted = session.finish().artwork;
    screen.frame(core, (&lifted, None), &[]);
    screen.shows(core, &lifted, &format!("{centre:?} lifted"));
    let taken = core.counts().snapshots - before;
    assert_eq!(taken, 0, "{centre:?}: snapshots taken while drawing");
    lifted
}

/// Every raster source with 40 strokes more about the canvas's middle: a dense place.
fn dense() -> Artwork {
    let mut base = every_raster_source();
    for k in 0..40u16 {
        let angle = f32::from(k) * 2.4;
        let centre = [160.0 + 30.0 * angle.cos(), 100.0 + 30.0 * angle.sin()];
        let style = plain(20.0, 0.5, color(0.1 * f32::from(k % 9), 0.4, 0.6));
        base = with_stroke(&base, (style, path(centre, 50.0, 12)));
    }
    base
}

/// A stroke over many strokes and a stroke after its lift reuse nothing they should not: the
/// second stroke over the first, committed between, is as cold as the first (the frames of the
/// tests' screen, which takes no snapshot ahead).
#[test]
fn strokes_one_after_another_over_a_dense_place_are_cold() {
    let core = crate::fixture::core();
    let base = dense();
    let (mut screen, view) = screen(&base, (1.0, [160.0, 100.0]));
    let mut artwork = base;
    let centres = [[150.0, 90.0], [170.0, 110.0], [160.0, 100.0]];
    for (k, centre) in centres.into_iter().enumerate() {
        let style = plain(28.0, 0.8, color(0.9, 0.2 * k as f32, 0.3));
        let samples = path(centre, 80.0, 25);
        let mut session = begun(&artwork, (&style, &samples[..1]), None);
        for chunk in samples[1..].chunks(4) {
            session.extend(chunk).expect("extend");
            screen.frame(&core, (&artwork, Some(session.active())), &[], &view);
        }
        let so_far = committed(&artwork, (&style, &samples), None);
        shows(&screen, &so_far, None, "drawn");
        artwork = session.finish().artwork;
        screen.frame(&core, (&artwork, None), &[], &view);
        shows(&screen, &artwork, None, "lifted");
    }
}

/// A device of its own: its counts and its frames' pacing are the test's alone.
fn own_core() -> Core {
    let core = Core::headless().expect("a GPU adapter is required for render tests");
    core.set_frame_budget(f64::INFINITY);
    core
}

#[test]
fn snapshots_taken_ahead_carry_strokes_an_undo_and_a_layer_switch_and_every_frame_is_cold() {
    let core = own_core();
    let base = dense();
    let mut screen = Loop::new(&core, &base, None);
    screen.settle(&core, &base);
    let first = stroke(&core, &mut screen, &base, [150.0, 90.0]);
    // Over the first: its tiles lay it onto their snapshots, ahead.
    screen.settle(&core, &first);
    let second = stroke(&core, &mut screen, &first, [170.0, 110.0]);
    screen.settle(&core, &second);
    // Undone: what it reached is taken again; a stroke elsewhere over the first.
    screen.frame(&core, (&first, None), &changed_regions(&second, &first));
    screen.settle(&core, &first);
    screen.shows(&core, &first, "undone");
    let third = stroke(&core, &mut screen, &first, [120.0, 120.0]);
    // Another layer: every snapshot goes, and is taken again for it.
    let switched = layer(&third, LayerCommand::Select(third.layers()[0].id()));
    screen.frame(&core, (&switched, None), &[]);
    screen.settle(&core, &switched);
    stroke(&core, &mut screen, &switched, [160.0, 100.0]);
}

/// S64: the lift takes its stroke into the scene in place (no scene built), on a layer under
/// others (a mask, a clipping chain, a smudge on the top layer): the lift's frame and every frame
/// after it are the canvas drawn cold, at the canvas and at a tier; an undo builds the scene
/// before, a redo takes the stroke in place again, a stroke after both is cold.
#[test]
fn a_lift_takes_its_stroke_into_the_scene_and_undo_and_redo_across_it_are_cold() {
    let core = own_core();
    let mut base = dense();
    let top = base.layers().len() - 1;
    base = layer(&base, LayerCommand::Select(base.layers()[top].id()));
    let smudge = plain(30.0, 0.8, Color::BLACK).with_tool(openpaint_core::StrokeTool::Smudge);
    base = with_stroke(&base, (smudge, path([170.0, 100.0], 60.0, 14)));
    base = layer(&base, LayerCommand::Select(base.layers()[0].id()));
    for shown in [None, Some((frame(3.7, [160.0, 100.0]), 4))] {
        let mut screen = Loop::new(&core, &base, shown);
        screen.settle(&core, &base);
        let appended = || core.counts().appended;
        let before = appended();
        let first = stroke(&core, &mut screen, &base, [150.0, 90.0]);
        assert_eq!(appended() - before, 1, "the lift appended its stroke");
        screen.settle(&core, &first);
        screen.frame(&core, (&base, None), &changed_regions(&first, &base));
        screen.settle(&core, &base);
        screen.shows(&core, &base, "undone");
        let before = appended();
        screen.frame(&core, (&first, None), &changed_regions(&base, &first));
        assert_eq!(appended() - before, 1, "the redo appended its stroke");
        screen.settle(&core, &first);
        screen.shows(&core, &first, "redone");
        stroke(&core, &mut screen, &first, [165.0, 105.0]);
    }
}

#[test]
fn a_tiers_snapshots_taken_ahead_carry_strokes_and_its_tiles_are_its_export() {
    let core = own_core();
    let base = dense();
    let shown = (frame(3.7, [160.0, 100.0]), 4);
    let mut screen = Loop::new(&core, &base, Some(shown));
    screen.settle(&core, &base);
    let first = stroke(&core, &mut screen, &base, [160.0, 100.0]);
    screen.settle(&core, &first);
    let second = stroke(&core, &mut screen, &first, [150.0, 95.0]);
    screen.frame(&core, (&first, None), &changed_regions(&second, &first));
    screen.settle(&core, &first);
    stroke(&core, &mut screen, &first, [165.0, 105.0]);
}
