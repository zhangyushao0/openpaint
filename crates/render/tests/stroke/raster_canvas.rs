//! The 1x canvas: every raster source and every smudge drawn into the
//! screen's canvas is its 1x export bit for bit, alike every time; filled over frames (at once where a change fits a frame, else by the raster worker's
//! tiles) through changes, an undo, a hidden layer, a stroke drawn and lifted and an
//! adjustment applied and undone, it ends as the canvas drawn cold; picks read it; layer samples
//! are the raster core's.

use std::time::Instant;

use openpaint_core::{
    Adjustment, Artwork, CanvasSize, Color, Hsb, LayerCommand, StrokeSample, StrokeTool,
};
use openpaint_render::{
    CanvasTarget, Core, PixelRect, RasterDrawer, RgbaImage, ViewCache, changed_regions,
};

use crate::fixture::{
    HANG, SIZE, cold, core, diagonal, layer, pixel, plain, request, sample, wave, with_stroke,
};
use crate::raster::every_raster_source;
use crate::raster_smudge::every_smudge;
use crate::scale::whole;

#[test]
fn the_canvas_is_its_1x_export_alike_every_time() {
    let core = core();
    for (name, artwork) in [
        ("every raster source", every_raster_source()),
        ("every smudge", every_smudge()),
    ] {
        let canvas = cold(&core, &artwork);
        assert!(canvas == cold(&core, &artwork), "{name}: alike every time");
        assert!(canvas == whole(&core, &artwork, 1), "{name}: the 1x export");
    }
}

/// A canvas target and its tiers, its frames served by a raster worker of its own.
struct Screen {
    target: CanvasTarget,
    view: ViewCache,
    drawer: RasterDrawer,
}

impl Screen {
    fn new(core: &Core, size: [u32; 2]) -> Self {
        Self {
            target: CanvasTarget::new(core, size).expect("target"),
            view: ViewCache::new(core),
            drawer: RasterDrawer::default(),
        }
    }

    /// Frames of `artwork` (with `regions` changed in the first) until nothing waits; how many
    /// tiles the worker drew.
    fn fill(&mut self, core: &Core, artwork: &Artwork, regions: &[PixelRect]) -> u64 {
        let before = core.counts().rastered;
        let mut regions = regions.to_vec();
        let start = Instant::now();
        loop {
            core.begin_frame();
            self.target
                .draw_frame(core, &request(artwork), &regions)
                .expect("frame");
            regions.clear();
            self.drawer.serve(core, (&mut self.target, &mut self.view));
            if !self.target.pending() {
                return core.counts().rastered - before;
            }
            assert!(start.elapsed() < HANG, "the canvas never filled in");
        }
    }

    fn read(&self, core: &Core) -> RgbaImage {
        self.target.read(core).expect("read")
    }
}

/// A canvas larger than a frame's change in place (`EDIT_PIXELS`): 900 x 600, waves across it.
fn large() -> Artwork {
    let mut artwork = Artwork::new(CanvasSize::new(900, 600).expect("size"));
    for k in 0..6u16 {
        let samples: Vec<StrokeSample> = (0..40u16)
            .map(|i| {
                let t = f32::from(i) / 39.0;
                let y = 80.0 + 90.0 * f32::from(k) + 40.0 * (std::f32::consts::TAU * t).sin();
                sample(30.0 + 840.0 * t, y, 0.5 + 0.5 * t, u32::from(i) * 8)
            })
            .collect();
        let color = Color::rgb(0.1 * f32::from(k), 0.4, 0.7).expect("colour");
        artwork = with_stroke(&artwork, (plain(30.0, 0.8, color), samples));
    }
    artwork
}

/// The large canvas drawn cold.
fn large_cold(core: &Core, artwork: &Artwork) -> RgbaImage {
    let mut target = CanvasTarget::new(core, [900, 600]).expect("target");
    let all = [PixelRect::full([900, 600])];
    target.draw(core, &request(artwork), &all).expect("cold");
    target.read(core).expect("read")
}

#[test]
fn a_canvas_filled_by_the_worker_and_changed_over_frames_ends_as_drawn_cold() {
    let core = core();
    let first = large();
    let mut screen = Screen::new(&core, [900, 600]);
    let handed = screen.fill(&core, &first, &[PixelRect::full([900, 600])]);
    assert!(handed > 0, "the worker drew tiles");
    assert!(screen.read(&core) == large_cold(&core, &first), "opened");
    // A stroke across it, its undo, a hidden layer.
    let (style, _) = diagonal();
    let path: Vec<StrokeSample> = (0..30u16)
        .map(|i| {
            sample(
                50.0 + 27.0 * f32::from(i),
                40.0 + 17.0 * f32::from(i),
                0.9,
                8,
            )
        })
        .collect();
    let stroked = with_stroke(&first, (style, path));
    let hide = LayerCommand::SetVisible {
        layer: first.active(),
        visible: false,
    };
    let hidden = layer(&first, hide);
    let mut shown = first.clone();
    for next in [stroked, first.clone(), hidden, first.clone()] {
        screen.fill(&core, &next, &changed_regions(&shown, &next));
        assert!(screen.read(&core) == large_cold(&core, &next), "changed");
        shown = next;
    }
}

/// A stroke and a smudge drawn by the raster core over the canvas its worker drew: nothing
/// waits while they are drawn or after their lift, whose frame leaves the canvas as drawn cold.
#[test]
fn a_stroke_drawn_and_lifted_leaves_nothing_to_draw_again_and_the_canvas_cold() {
    let core = core();
    let (smudge, _) = diagonal();
    let smudge = smudge.with_tool(StrokeTool::Smudge);
    let strokes = [
        (every_raster_source(), wave()),
        (every_smudge(), (smudge, wave().1)),
    ];
    for (under, (style, samples)) in strokes {
        let mut screen = Screen::new(&core, SIZE);
        screen.fill(&core, &under, &[PixelRect::full(SIZE)]);
        let mut session = under.begin_stroke(style, samples[0]).expect("begin");
        for chunk in samples[1..].chunks(5) {
            session.extend(chunk).expect("extend");
            let frame = openpaint_render::FrameRequest {
                active: Some(session.active()),
                ..request(&under)
            };
            screen.target.draw_frame(&core, &frame, &[]).expect("frame");
        }
        assert!(!screen.target.pending(), "nothing waits for the stroke");
        let lifted = session.finish().artwork;
        screen
            .target
            .draw_frame(&core, &request(&lifted), &[])
            .expect("the lift");
        assert!(!screen.target.pending(), "nothing is drawn again");
        assert!(screen.read(&core) == cold(&core, &lifted), "drawn cold");
    }
}

#[test]
fn picks_read_the_canvas() {
    let core = core();
    let artwork = every_raster_source();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    target
        .draw(&core, &request(&artwork), &[PixelRect::full(SIZE)])
        .expect("drawn");
    let canvas = cold(&core, &artwork);
    let mut picks = 0;
    for y in (5..SIZE[1] - 5).step_by(9) {
        for x in (5..SIZE[0] - 5).step_by(13) {
            let picked = target.pick(&core, [x, y]).expect("pick").expect("inside");
            assert_eq!(picked, pixel(&canvas, [x, y]), "{x}, {y}");
            picks += 1;
        }
    }
    assert!(picks > 400, "{picks} picks");
    assert_eq!(target.pick(&core, SIZE).expect("pick"), None, "outside");
}

/// An adjustment over the smudges, applied and undone over frames, leaves the canvas as
/// drawn cold each time.
#[test]
fn an_adjustment_applied_and_undone_over_frames_ends_as_drawn_cold() {
    let core = core();
    let raster = every_smudge();
    let adjust = Adjustment::Hsb(Hsb::new(0.6, 0.7, 0.5));
    let adjusted = (raster.adjust_region(None, adjust))
        .expect("adjusted")
        .artwork;
    let drawn = cold(&core, &adjusted);
    let mut screen = Screen::new(&core, SIZE);
    screen.fill(&core, &raster, &[PixelRect::full(SIZE)]);
    screen.fill(&core, &adjusted, &changed_regions(&raster, &adjusted));
    assert!(screen.read(&core) == drawn, "applied");
    screen.fill(&core, &raster, &changed_regions(&adjusted, &raster));
    assert!(screen.read(&core) == cold(&core, &raster), "undone");
}

#[test]
fn layer_samples_are_the_raster_cores() {
    let core = core();
    let artwork = every_raster_source();
    let bottom = artwork.layers()[0].id();
    // The bottom layer alone over the paper: its sample laid over white on the CPU.
    let mut alone = artwork.clone();
    for other in artwork.layers().iter().skip(1) {
        let hide = LayerCommand::SetVisible {
            layer: other.id(),
            visible: false,
        };
        alone = layer(&alone, hide);
    }
    let sample = core.layer_pixels(&artwork, bottom).expect("sampled");
    let canvas = cold(&core, &alone);
    let over: Vec<u8> = (sample.pixels().as_chunks::<4>().0.iter())
        .flat_map(|[r, g, b, a]| {
            let under = 255 - u16::from(*a);
            [*r, *g, *b].map(|c| (u16::from(c) + under).min(255) as u8)
        })
        .collect();
    let shown: Vec<u8> = (canvas.pixels().as_chunks::<4>().0.iter())
        .flat_map(|[r, g, b, _]| [*r, *g, *b])
        .collect();
    let apart = over.iter().zip(&shown).map(|(a, b)| a.abs_diff(*b)).max();
    assert!(apart <= Some(1), "{apart:?} levels apart");
    let thumbnail = core.layer_thumbnail(&artwork, (bottom, false), [80, 50]);
    assert_eq!(thumbnail.expect("picture").size(), [80, 50]);
}
