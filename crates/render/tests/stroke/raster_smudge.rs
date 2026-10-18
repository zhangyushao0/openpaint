//! Smudges on the raster core (S61, DECISIONS D88): an artwork holding smudges of every kind
//! (across hard bands, over a fill, over an inserted picture, nine crossing over one place, inside
//! a lasso, under Alpha Lock, on a grey mask) is drawn by the raster core with the smudge engine's
//! residuals: its export within one 8-bit level of the evaluator's image away from edges at 1x to
//! 8x, the same every time and in bands as in one piece; its view tiers its export bit for bit,
//! after changes as drawn cold, after a smudge drawn at a tier (S63: the raster core's while drawn), and
//! when the canvas's arena moved before the worker drew the tiles handed to it.

use openpaint_core::{
    Artwork, ArtworkCommand, Color, LayerCommand, LayerId, MaskCommand, Placement, StrokeSample,
    StrokeStyle, StrokeTool,
};
use openpaint_render::Banding;

use crate::fixture::{SIZE, airbrush, blank, core, plain, sample, with_layer, with_stroke};
use crate::raster::{assert_near, color, evaluated, layer, picture, region};
use crate::scale::{export, whole};
use crate::tiers::{Screen, changes_reach_their_tiles, frame, tiles_are_the_export};

/// Smudges the artwork holds.
const SMUDGES: usize = 15;

/// A smudge `size` wide from `from` to `to`, its pressure rising: a soft bell (`soft`) or the hard
/// round.
fn smudge(
    (from, to): ([f32; 2], [f32; 2]),
    size: f32,
    soft: bool,
) -> (StrokeStyle, Vec<StrokeSample>) {
    let style = if soft {
        StrokeStyle::new(airbrush(), size, 0.85, Color::BLACK).expect("style")
    } else {
        plain(size, 0.8, Color::BLACK)
    };
    let samples = (0..24u16)
        .map(|i| {
            let t = f32::from(i) / 23.0;
            let [x, y] = [0, 1].map(|axis| from[axis] + (to[axis] - from[axis]) * t);
            sample(x, y, 0.5 + 0.5 * t, u32::from(i) * 8)
        })
        .collect();
    (style.with_tool(StrokeTool::Smudge), samples)
}

/// Layer 1: a red band above a blue one and an orange disc filled; a smudge across the bands, one
/// over the disc, nine crossing about (120, 100), one inside a lasso of the lower part.
fn painted() -> Artwork {
    let band = |y: f32, c: Color| {
        let samples = vec![sample(10.0, y, 1.0, 0), sample(310.0, y, 1.0, 8)];
        (plain(50.0, 1.0, c), samples)
    };
    let red = band(50.0, color(0.9, 0.1, 0.05));
    let mut artwork = with_stroke(
        &with_stroke(&blank(), red),
        band(140.0, color(0.05, 0.2, 0.85)),
    );
    let disc = region(|x, y| {
        let (dx, dy) = (x as f32 - 250.0, y as f32 - 95.0);
        dx * dx + dy * dy <= 900.0
    });
    let orange = color(0.95, 0.6, 0.1);
    artwork = (artwork.fill_region(disc, orange)).expect("fill").artwork;
    artwork = with_stroke(&artwork, smudge(([70.0, 30.0], [180.0, 180.0]), 36.0, true));
    artwork = with_stroke(
        &artwork,
        smudge(([215.0, 95.0], [290.0, 100.0]), 24.0, false),
    );
    for k in 0..9u16 {
        let angle = std::f32::consts::PI * f32::from(k) / 9.0;
        let [dx, dy] = [50.0 * angle.cos(), 50.0 * angle.sin()];
        let ends = ([120.0 - dx, 100.0 - dy], [120.0 + dx, 100.0 + dy]);
        artwork = with_stroke(&artwork, smudge(ends, 20.0, k % 2 == 0));
    }
    let (style, samples) = smudge(([20.0, 60.0], [100.0, 170.0]), 30.0, true);
    let session = artwork.begin_stroke(style, samples[0]).expect("begin");
    let mut session = (session.clipped(region(|_, y| y >= 110))).expect("clipped");
    session.extend(&samples[1..]).expect("extend");
    session.finish().artwork
}

/// `painted`; layer 2 at 90 %: a turned picture, a smudge over it, then under Alpha Lock another;
/// layer 3: a green bar under a grey mask over its left part, and a smudge on the mask.
pub fn every_smudge() -> Artwork {
    let created = with_layer(&painted(), 0.9);
    let turn = Placement::around([72.0, 144.0], 1.2, 0.3, [false, false]).expect("turn");
    let mut artwork = (created.transform_region(None, picture(), turn))
        .expect("placed")
        .artwork;
    let pictured = artwork.active();
    artwork = with_stroke(
        &artwork,
        smudge(([40.0, 130.0], [110.0, 160.0]), 26.0, true),
    );
    let lock = LayerCommand::SetAlphaLocked {
        layer: pictured,
        locked: true,
    };
    artwork = layer(&artwork, lock);
    artwork = with_stroke(
        &artwork,
        smudge(([50.0, 165.0], [105.0, 125.0]), 22.0, false),
    );
    artwork = with_layer(&artwork, 1.0);
    let bar = vec![sample(150.0, 30.0, 1.0, 0), sample(300.0, 40.0, 1.0, 8)];
    artwork = with_stroke(&artwork, (plain(40.0, 1.0, color(0.2, 0.7, 0.3)), bar));
    let top = artwork.active();
    let mask = ArtworkCommand::Mask(MaskCommand::Add(top));
    artwork = artwork.apply(mask).expect("mask").artwork;
    let grey = color(0.3, 0.3, 0.3);
    artwork = (artwork.fill_region(region(|x, _| x < 220), grey))
        .expect("grey")
        .artwork;
    with_stroke(&artwork, smudge(([190.0, 10.0], [260.0, 70.0]), 30.0, true))
}

/// `every_smudge` with the first layer's paint chosen.
fn on_the_first_layer() -> Artwork {
    layer(&every_smudge(), LayerCommand::Select(LayerId::new(1)))
}

#[test]
fn every_smudge_exports_on_the_raster_core_within_a_level_of_the_evaluator_away_from_edges() {
    let core = core();
    let artwork = every_smudge();
    let drawn = core.time_raster(&artwork, (1, [0, 0]), SIZE, (1, 1024));
    let drawn = drawn.expect("the raster core draws every smudge");
    assert!(drawn.repeatable, "it draws alike every time");
    assert_eq!(drawn.counts.smudges, SMUDGES, "{:?}", drawn.counts);
    for scale in [1, 2, 4, 8] {
        let rastered = whole(&core, &artwork, scale);
        assert_near(
            &evaluated(&core, &artwork, scale),
            &rastered,
            &format!("{scale}x"),
        );
    }
}

#[test]
fn every_smudge_draws_alike_every_time_and_in_bands_as_in_one_piece() {
    let core = core();
    let artwork = every_smudge();
    for scale in [1, 2, 4] {
        let one = whole(&core, &artwork, scale);
        assert!(one == whole(&core, &artwork, scale), "{scale}x again");
        let banding = Banding {
            rows: 37 * scale,
            columns: 61 * scale + 5,
        };
        assert!(
            export(&core, &artwork, scale, banding) == one,
            "{scale}x bands"
        );
    }
}

#[test]
fn every_smudges_tier_tiles_are_its_export_bit_for_bit() {
    tiles_are_the_export(&core(), &every_smudge());
}

/// A smudge across the nine, a stroke over it, its undo, the smudge's undo: each draws again
/// exactly the tiles it reaches, each tier then its export.
#[test]
fn changes_over_smudges_draw_again_exactly_the_tiles_they_reach() {
    let first = on_the_first_layer();
    let smudged = with_stroke(&first, smudge(([130.0, 60.0], [170.0, 140.0]), 28.0, true));
    let bar = vec![sample(140.0, 90.0, 1.0, 0), sample(160.0, 110.0, 1.0, 8)];
    let over = with_stroke(&smudged, (plain(8.0, 1.0, Color::BLACK), bar));
    changes_reach_their_tiles(&first, &[smudged.clone(), over, smudged, first.clone()]);
}

/// A smudge drawn at 8x (S63: by the raster core at the tier too): the tiles it reaches follow it,
/// and the lift's frame leaves every tile the committed export bit for bit, none drawn again.
#[test]
fn a_smudge_drawn_while_zoomed_ends_as_its_export() {
    let core = core();
    let base = on_the_first_layer();
    let view = frame(7.0, [150.0, 110.0]);
    let mut screen = Screen::new(&core, &base);
    screen.settle(&core, &base, &view);
    let (style, samples) = smudge(([125.0, 92.0], [175.0, 125.0]), 16.0, true);
    let mut session = base.begin_stroke(style, samples[0]).expect("begin");
    let mut evaluated = 0;
    for chunk in samples[1..].chunks(4) {
        session.extend(chunk).expect("extend");
        let work = screen.frame(&core, (&base, Some(session.active())), &[], &view);
        evaluated += work.updated.len() + work.dropped.len();
    }
    assert!(evaluated > 0, "the tiles it reaches follow it");
    let committed = session.finish().artwork;
    let lift = screen.frame(&core, (&committed, None), &[], &view);
    screen.assert_tiles(&core, &whole(&core, &committed, 8), 8);
    assert!(lift.dropped.is_empty(), "{lift:?}");
    assert!(
        screen.settle(&core, &committed, &view).is_empty(),
        "nothing is drawn again"
    );
}

/// D88: the tiles handed to the raster worker draw the residuals they were handed with, though the
/// canvas's arena moves into a new buffer before the worker draws them (a room of a few bytes:
/// each frame lets go what it did not need).
#[test]
fn tiles_drawn_after_the_arena_moved_are_the_export() {
    let core = core();
    let artwork = every_smudge();
    let view = frame(3.7, [150.0, 110.0]);
    let mut screen = Screen::new(&core, &artwork);
    let mut moved = 0;
    for _ in 0..400 {
        let work = screen.update(&core, (&artwork, None), &[], &view);
        let freed = screen.target.set_arena_room(&core, 64).expect("room");
        moved += usize::from(freed > 0 && work.waiting > 0);
        screen.serve(&core);
        if work.tier.is_some() && work.waiting == 0 {
            break;
        }
    }
    assert!(
        moved > 0,
        "the arena moved while tiles waited for the worker"
    );
    let compared = screen.assert_tiles(&core, &whole(&core, &artwork, 4), 4);
    assert!(compared >= 4, "{compared} tiles");
}
