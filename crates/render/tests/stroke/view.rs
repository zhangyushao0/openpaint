//! Sharp zoom (S32): a view tier's tiles equal the banded export at the tier's scale for their
//! region, bit for bit; after a stroke, a fill, a transform record and undo exactly the tiles
//! the change reaches are evaluated again and every tile still equals the export (incremental
//! = cold per tile); a stroke drawn while zoomed shows at the tier as it will be committed (its
//! end taper from the lift on, S44); the canvas target stays the cold canvas. S59: the same for
//! a scene the raster core draws, its tiles and its export the raster core's.

use openpaint_core::{Artwork, Color, Placement, RegionMask, StrokeStyle};

use crate::fixture::{SIZE, cold, core, plain, sample, with_stroke};
use crate::raster::every_raster_source;
use crate::scale::{every_source, whole};
use crate::tiers::{Screen, changes_reach_their_tiles, frame, tiles_are_the_export};

#[test]
fn a_filtered_tier_tile_is_the_export_at_its_scale_bit_for_bit() {
    tiles_are_the_export(&core(), &crate::filters::filtered());
}

#[test]
fn a_tier_tile_is_the_export_at_its_scale_bit_for_bit() {
    let core = core();
    let artwork = every_source();
    tiles_are_the_export(&core, &artwork);
    // At or below about one screen pixel per canvas pixel the canvas itself shows.
    let mut screen = Screen::new(&core, &artwork);
    let fit = frame(1.3, [160.0, 100.0]);
    for _ in 0..3 {
        let work = screen.frame(&core, (&artwork, None), &[], &fit);
        assert_eq!((work.tier, work.bytes), (None, 0));
    }
}

/// The first tier's atlas is asked for ahead only while no tier was made (S60): a tier made while
/// the screen was touched, before any atlas was asked for, leaves none wanted, so frames rest.
#[test]
fn a_tier_made_before_the_screen_rested_wants_no_atlas_ahead() {
    let core = core();
    let artwork = every_raster_source();
    let mut screen = Screen::new(&core, &artwork);
    // A step while the screen is touched: no atlas is asked for.
    core.begin_frame();
    let zoomed = frame(2.0, [160.0, 100.0]);
    screen.update(&core, (&artwork, None), &[], &zoomed);
    assert!(screen.view.wants_spare(), "no tier yet, none asked for");
    screen.settle(&core, &artwork, &zoomed);
    assert!(!screen.view.wants_spare(), "the tier came first");
}

/// S59: the raster core draws the tiers of a scene without the routed kinds, as it exports it.
#[test]
fn a_raster_tier_tile_is_the_raster_export_at_its_scale_bit_for_bit() {
    tiles_are_the_export(&core(), &every_raster_source());
}

/// A stroke drawn at 8x over a scene the raster core draws: while drawn, the raster core draws the
/// tiles it reaches (S63), each the export of the stroke so far bit for bit; the lift's frame
/// leaves every tile the committed export, none drawn again.
#[test]
fn a_stroke_drawn_while_zoomed_over_the_raster_core_ends_as_its_export() {
    let core = core();
    let base = every_raster_source();
    let view = frame(7.0, [150.0, 110.0]);
    let mut screen = Screen::new(&core, &base);
    screen.settle(&core, &base, &view);
    let style = plain(5.0, 1.0, Color::rgb(0.8, 0.1, 0.4).expect("colour"));
    let first = sample(130.0, 95.0, 0.6, 0);
    let mut session = base.begin_stroke(style.clone(), first).expect("begin");
    let mut drawn = vec![first];
    for step in 1..8u16 {
        let at = f32::from(step);
        let next = sample(130.0 + 3.0 * at, 95.0 + 2.0 * at, 0.7, u32::from(step) * 8);
        session.extend(&[next]).expect("extend");
        drawn.push(next);
        screen.frame(&core, (&base, Some(session.active())), &[], &view);
        let so_far = whole(
            &core,
            &with_stroke(&base, (style.clone(), drawn.clone())),
            8,
        );
        screen.assert_tiles(&core, &so_far, 8);
    }
    let committed = session.finish().artwork;
    screen.frame(&core, (&committed, None), &[], &view);
    screen.assert_tiles(&core, &whole(&core, &committed, 8), 8);
    assert!(
        screen.settle(&core, &committed, &view).is_empty(),
        "nothing is drawn again"
    );
}

/// A stroke, a fill, then its square moved by a record (`moved`) or cut, then back to `first`.
fn edits(first: &Artwork, moved: bool) -> [Artwork; 4] {
    let stroke = with_stroke(
        first,
        (
            plain(6.0, 1.0, Color::BLACK),
            vec![sample(120.0, 90.0, 1.0, 0), sample(150.0, 100.0, 1.0, 8)],
        ),
    );
    let square = |x: u32, y: u32| (130..150).contains(&x) && (100..118).contains(&y);
    let pixels: Vec<bool> = (0..SIZE[0] * SIZE[1])
        .map(|at| square(at % SIZE[0], at / SIZE[0]))
        .collect();
    let canvas = stroke.canvas();
    let mask = RegionMask::from_pixels(canvas, &pixels).expect("mask");
    let filled = stroke
        .fill_region(mask.clone(), Color::rgb(0.2, 0.8, 0.3).expect("colour"))
        .expect("fill")
        .artwork;
    let turn = Placement::around([140.0, 109.0], 1.2, 0.3, [false, false]).expect("turn");
    let last = if moved {
        filled.place_region(Some(mask), turn).expect("moved")
    } else {
        filled.cut(Some(&mask)).expect("cut")
    };
    [stroke, filled, last.artwork, first.clone()]
}

#[test]
fn changes_evaluate_again_exactly_the_tiles_they_reach() {
    let first = every_source();
    changes_reach_their_tiles(&first, &edits(&first, true));
}

/// S59: a scene the raster core draws, its tiles drawn again where a change reaches them; each
/// tier then equals the tier drawn cold (the export) bit for bit.
#[test]
fn changes_draw_again_exactly_the_tiles_they_reach_on_the_raster_core() {
    let first = every_raster_source();
    changes_reach_their_tiles(&first, &edits(&first, false));
}

#[test]
fn a_stroke_drawn_while_zoomed_shows_at_the_tier_as_committed() {
    let style = plain(5.0, 1.0, Color::rgb(0.8, 0.1, 0.4).expect("colour"));
    drawn_while_zoomed(&style, &style);
}

/// The same with a taper at both ends (S44): while drawn, the tiles are the export of the
/// stroke so far without its end taper; from the lift on, of the committed stroke.
#[test]
fn a_tapered_stroke_drawn_while_zoomed_narrows_its_end_at_the_tier_at_the_lift() {
    let color = Color::rgb(0.8, 0.1, 0.4).expect("colour");
    let tapered = |end: f32| {
        let taper = openpaint_core::Taper::new(1.0, end, 0.1).expect("taper");
        let recipe = openpaint_core::BrushRecipe::plain().with_taper(taper);
        StrokeStyle::new(std::sync::Arc::new(recipe), 5.0, 1.0, color).expect("style")
    };
    drawn_while_zoomed(&tapered(2.0), &tapered(0.0));
}

/// `style` drawn sample by sample at 8x: each frame's tiles are the export of the stroke so far
/// drawn with `blunt` (`style` without its end taper), and once committed the export of it.
pub fn drawn_while_zoomed(style: &StrokeStyle, blunt: &StrokeStyle) {
    let core = core();
    let base = every_source();
    let scale = 8;
    let view = frame(7.0, [150.0, 110.0]);
    let mut screen = Screen::new(&core, &base);
    screen.settle(&core, &base, &view);
    let first = sample(130.0, 95.0, 0.6, 0);
    let mut session = base.begin_stroke(style.clone(), first).expect("begin");
    let mut drawn = vec![first];
    for step in 1..12u16 {
        let at = f32::from(step);
        let next = sample(
            130.0 + 3.0 * at,
            95.0 + 2.0 * at,
            0.6 + 0.03 * at,
            u32::from(step) * 8,
        );
        session.extend(&[next]).expect("extend");
        drawn.push(next);
        let work = screen.frame(&core, (&base, Some(session.active())), &[], &view);
        assert!(
            work.dropped.is_empty(),
            "a stroke's few pixels are evaluated in place"
        );
        let so_far = with_stroke(&base, (blunt.clone(), drawn.clone()));
        screen.assert_tiles(&core, &whole(&core, &so_far, scale), scale);
    }
    let committed = session.finish().artwork;
    screen.frame(&core, (&committed, None), &[], &view);
    screen.assert_tiles(&core, &whole(&core, &committed, scale), scale);
    assert_eq!(
        screen.target.read(&core).expect("canvas"),
        cold(&core, &committed)
    );
}
