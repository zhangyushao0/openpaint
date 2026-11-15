//! A stroke while a view tier shows (D91): the tier draws its frames alone, each ready tile the
//! export of the stroke so far, a frame crossing tiles in one pass chain; the canvas draws a frame
//! only where the tier cannot (a tile the
//! stroke reaches that is not shown or not ready), from where it last drew, and takes the stroke
//! whole at its lift, in the lift's frame. So every reader of the canvas after the lift reads the
//! stroke: the blit's fallback under a tile not ready, the canvas shown again at fit, a pick, the
//! thumbnail, an undo and a redo, the next stroke's snapshots; a read mid-stroke draws it into the
//! canvas first; a dropped stroke leaves the canvas as it was.

use std::sync::Arc;

use openpaint_core::{Artwork, BrushRecipe, Color, StrokeSample, StrokeStyle, StrokeTool, Taper};
use openpaint_render::{changed_regions, take_spent, thumbnail_reference, thumbnail_size};

use crate::drawing::{begun, committed, draws, path, screen, shows};
use crate::fixture::{SIZE, cold, core, graphite, pixel, plain, sample};
use crate::raster::{color, every_raster_source};
use crate::scale::whole;
use crate::tiers::{Screen, frame, request};

/// The view: the 2x tier about the canvas's middle, showing canvas x 69 to 251 and y 32 to 168
/// (its tiles 128 canvas pixels a side: columns 0 and 1 shown, column 2 the ring).
const ZOOM: f32 = 2.2;
const CENTRE: [f32; 2] = [160.0, 100.0];

/// A frame's check: the screen after it, the stroke so far committed (without its end taper),
/// the tiles the tier could not draw.
type Check<'a> = &'a mut dyn FnMut(&Screen, &Artwork, &[[u32; 2]]);

/// `style` (`blunt` without its end taper) through `samples` drawn over `base` three samples a
/// frame through `view`, each frame checked by `check`, then lifted: the lift's frame shows the
/// commit cold on the canvas and as its export at the tier. The artwork lifted.
fn drawn(
    screen: &mut Screen,
    base: &Artwork,
    (style, blunt, samples): (&StrokeStyle, &StrokeStyle, &[StrokeSample]),
    check: Check<'_>,
) -> Artwork {
    let core = core();
    let view = frame(ZOOM, CENTRE);
    let mut session = begun(base, (style, &samples[..1]), None);
    let mut end = 1;
    loop {
        let work = screen.frame(&core, (base, Some(session.active())), &[], &view);
        check(
            screen,
            &committed(base, (blunt, &samples[..end]), None),
            &work.dropped,
        );
        if end == samples.len() {
            break;
        }
        let next = (end + 3).min(samples.len());
        session.extend(&samples[end..next]).expect("extend");
        end = next;
    }
    let lifted = session.finish().artwork;
    screen.frame(&core, (&lifted, None), &[], &view);
    shows(screen, &lifted, Some(2), "lifted");
    lifted
}

/// A round brush ending in a taper of `end` sizes.
fn tapered(end: f32) -> StrokeStyle {
    let recipe = BrushRecipe::plain().with_taper(Taper::new(1.0, end, 0.1).expect("taper"));
    StrokeStyle::new(Arc::new(recipe), 9.0, 0.8, color(0.2, 0.4, 0.9)).expect("style")
}

/// Inside the shown tiles the canvas draws nothing of the stroke until its lift: a tapered
/// stroke, whose lift narrows its end, and a smudge, whose contacts trail it (its frames are
/// `drawing_kinds.rs`'s).
#[test]
fn a_stroke_inside_the_shown_tiles_is_the_tiers_alone_until_its_lift() {
    let core = core();
    let base = every_raster_source();
    let before = cold(&core, &base);
    let smudge = plain(14.0, 0.8, Color::BLACK).with_tool(StrokeTool::Smudge);
    let strokes = [(tapered(2.0), tapered(0.0)), (smudge.clone(), smudge)];
    for (style, blunt) in &strokes {
        let (mut screen, _) = screen(&base, (ZOOM, CENTRE));
        let paint = style.tool() == StrokeTool::Paint;
        let mut check = |screen: &Screen, so_far: &Artwork, dropped: &[[u32; 2]]| {
            assert!(
                dropped.is_empty(),
                "every tile it reaches is shown and ready"
            );
            let canvas = screen.target.read(&core).expect("read");
            assert!(canvas == before, "the canvas is left to the lift");
            if paint {
                screen.assert_tiles(&core, &whole(&core, so_far, 2), 2);
            }
        };
        drawn(
            &mut screen,
            &base,
            (style, blunt, &path(CENTRE, 40.0, 16)),
            &mut check,
        );
    }
}

/// A frame whose samples cross a tier tile's corner (four tiles at once) is one pass chain
/// finishing into each tile's slot, every ready tile the tier drawn cold (its export), every frame
/// and the lift.
#[test]
fn a_frame_crossing_tier_tiles_is_one_chain_and_each_tile_its_export() {
    let core = core();
    let base = every_raster_source();
    let view = frame(ZOOM, CENTRE);
    let (mut screen, _) = screen(&base, (ZOOM, CENTRE));
    let pencil = StrokeStyle::new(graphite(), 6.0, 0.8, Color::BLACK).expect("pencil");
    // Tiles of 128 canvas pixels: the third batch goes from (122, 122) over the corner to (140, 140).
    let samples: Vec<StrokeSample> = (0..7u16)
        .map(|i| {
            let at = 104.0 + 6.0 * f32::from(i);
            sample(at, at, 0.8, u32::from(i) * 8)
        })
        .collect();
    let mut session = begun(&base, (&pencil, &samples[..1]), None);
    let mut end = 1;
    let mut most = 0;
    let _ = take_spent();
    loop {
        let work = screen.frame(&core, (&base, Some(session.active())), &[], &view);
        let spent = take_spent();
        assert!(work.dropped.is_empty(), "every tile it reaches is shown");
        assert_eq!(spent.tier.regions, 1, "one chain after {end} samples");
        most = most.max(work.updated.len());
        let so_far = committed(&base, (&pencil, &samples[..end]), None);
        screen.assert_tiles(&core, &whole(&core, &so_far, 2), 2);
        if end == samples.len() {
            break;
        }
        end = (end + 3).min(samples.len());
        session.extend(&samples[end - 3..end]).expect("extend");
    }
    assert_eq!(most, 4, "a frame drew the four tiles at the corner");
    let lifted = session.finish().artwork;
    screen.frame(&core, (&lifted, None), &[], &view);
    shows(&screen, &lifted, Some(2), "lifted");
}

/// A stroke's frames draw with a tile's targets and finish through one: no render pass of theirs
/// loads or stores more than a tile's float target (a GPU drawing in tiles pays a pass's whole
/// attachment, S134), at fit and at a tier; every frame the canvas drawn cold, or every ready
/// tile of the tier its export.
#[test]
fn a_strokes_frames_pass_over_a_tiles_targets_alone() {
    let core = core();
    let base = every_raster_source();
    let style = plain(9.0, 0.8, color(0.3, 0.6, 0.2));
    let samples = path(CENTRE, 40.0, 16);
    // A tile's float target, at most 16 bytes a texel.
    let tile = 256 * 256 * 16;
    for (zoom, scale) in [(1.0, None), (ZOOM, Some(2))] {
        let (mut screen, view) = screen(&base, (zoom, CENTRE));
        let mut session = begun(&base, (&style, &samples[..1]), None);
        let _ = take_spent();
        for end in 2..=samples.len() {
            session.extend(&samples[end - 1..end]).expect("extend");
            screen.frame(&core, (&base, Some(session.active())), &[], &view);
            let spent = take_spent();
            let attached = spent.canvas.attached.plus(spent.tier.attached);
            let most = 2 * tile * u64::from(attached.passes);
            assert!(attached.passes > 0, "at {zoom}x the frame drew");
            assert!(
                attached.loaded + attached.stored <= most,
                "at {zoom}x after {end} samples: {attached:?}"
            );
            let so_far = committed(&base, (&style, &samples[..end]), None);
            draws(
                &screen,
                &so_far,
                scale,
                &format!("at {zoom}x after {end} samples"),
            );
        }
    }
}

/// A stroke from the shown tiles into the ring and back: a frame reaching a tile the tier cannot
/// draw is drawn on the canvas too (the stroke so far, the frames it left included); the frames
/// after leave the canvas behind again, and the lift brings it up.
#[test]
fn a_frame_reaching_a_tile_the_tier_cannot_draw_is_drawn_on_the_canvas_too() {
    let core = core();
    let base = every_raster_source();
    let (mut screen, _) = screen(&base, (ZOOM, CENTRE));
    let style = plain(10.0, 0.8, color(0.9, 0.3, 0.2));
    let samples: Vec<StrokeSample> = (0..31u16)
        .map(|i| {
            let t = f32::from(i) / 30.0;
            let x = 150.0 + 125.0 * (std::f32::consts::PI * t).sin();
            sample(x, 50.0 + 100.0 * t, 0.8, u32::from(i) * 8)
        })
        .collect();
    let (mut left, mut drawn_too) = (0, 0);
    let mut check = |screen: &Screen, so_far: &Artwork, dropped: &[[u32; 2]]| {
        screen.assert_tiles(&core, &whole(&core, so_far, 2), 2);
        if dropped.is_empty() {
            left += 1;
        } else {
            drawn_too += 1;
            let canvas = screen.target.read(&core).expect("read");
            assert!(
                canvas == cold(&core, so_far),
                "the canvas holds the stroke so far"
            );
        }
    };
    drawn(&mut screen, &base, (&style, &style, &samples), &mut check);
    assert!(
        left > 1 && drawn_too > 0,
        "{left} frames left, {drawn_too} drawn too"
    );
}

/// From the lift on the canvas holds the stroke for whatever reads it: a pick, the thumbnail,
/// the next stroke over it, an undo and a redo, the canvas shown again at fit.
#[test]
fn after_the_lift_every_reader_of_the_canvas_reads_the_stroke() {
    let core = core();
    let base = every_raster_source();
    let (mut screen, view) = screen(&base, (ZOOM, CENTRE));
    let style = plain(8.0, 0.8, color(0.2, 0.4, 0.9));
    let none = &mut |_: &Screen, _: &Artwork, _: &[[u32; 2]]| {};
    let first = drawn(
        &mut screen,
        &base,
        (&style, &style, &path(CENTRE, 40.0, 16)),
        &mut *none,
    );
    let canvas = cold(&core, &first);
    let at = [160, 100];
    let picked = screen.target.pick(&core, at).expect("pick");
    assert_eq!(picked, Some(pixel(&canvas, at)), "a pick on the stroke");
    let max = [80, 80];
    let thumbnail = core.thumbnail(&screen.target, max).expect("thumbnail");
    let reference = thumbnail_reference(&canvas, thumbnail_size(SIZE, max));
    assert!(thumbnail.pixels() == reference.as_slice(), "the thumbnail");
    let other = plain(12.0, 0.6, color(0.1, 0.8, 0.3));
    let crossing = path([150.0, 95.0], 40.0, 16);
    let second = drawn(&mut screen, &first, (&other, &other, &crossing), &mut *none);
    screen.frame(
        &core,
        (&first, None),
        &changed_regions(&second, &first),
        &view,
    );
    screen.settle(&core, &first, &view);
    shows(&screen, &first, Some(2), "undone");
    screen.frame(
        &core,
        (&second, None),
        &changed_regions(&first, &second),
        &view,
    );
    screen.settle(&core, &second, &view);
    shows(&screen, &second, Some(2), "redone");
    let fit = frame(1.0, CENTRE);
    for _ in 0..60 {
        if screen
            .frame(&core, (&second, None), &[], &fit)
            .tier
            .is_none()
        {
            break;
        }
    }
    assert_eq!(screen.view.tier(), None, "the canvas shows at fit");
    shows(&screen, &second, None, "at fit");
}

/// A read mid-stroke (a pick, the gallery picture: the canvas evaluated whole) draws the stroke
/// so far into the canvas, and the lift what came after; a dropped stroke leaves it untouched.
#[test]
fn a_read_mid_stroke_draws_it_into_the_canvas_and_a_dropped_stroke_leaves_it() {
    let core = core();
    let base = every_raster_source();
    let view = frame(ZOOM, CENTRE);
    let style = plain(9.0, 0.8, color(0.6, 0.2, 0.7));
    let samples = path(CENTRE, 40.0, 16);
    for read in [true, false] {
        let (mut screen, _) = screen(&base, (ZOOM, CENTRE));
        let mut session = begun(&base, (&style, &samples[..6]), None);
        screen.frame(&core, (&base, Some(session.active())), &[], &view);
        let canvas = |screen: &Screen| screen.target.read(&core).expect("read");
        assert!(canvas(&screen) == cold(&core, &base), "left to the tier");
        assert!(screen.target.behind(), "a read must draw it first");
        if read {
            let request = request(&base, Some(session.active()));
            screen.target.draw(&core, &request, &[]).expect("read");
            let so_far = committed(&base, (&style, &samples[..6]), None);
            assert!(canvas(&screen) == cold(&core, &so_far), "read mid-stroke");
            assert!(!screen.target.behind());
        }
        session.extend(&samples[6..]).expect("extend");
        screen.frame(&core, (&base, Some(session.active())), &[], &view);
        let after = if read {
            session.finish().artwork
        } else {
            session.cancel()
        };
        screen.frame(&core, (&after, None), &[], &view);
        assert!(!screen.target.behind(), "the lift's frame drew it");
        shows(
            &screen,
            &after,
            Some(2),
            if read { "lifted" } else { "dropped" },
        );
    }
}
