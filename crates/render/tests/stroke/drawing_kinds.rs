//! The stroke being drawn on the raster core (`drawing.rs`), of other kinds: a
//! smudge, whose contacts trail the pen until it lifts, equals the same frame drawn on a canvas
//! showing the artwork until then, and its lift the commit cold; a stroke snapped to a shape
//! (QuickShape), one drawn with mirrored copies (symmetry) and one dropped are cold every frame.

use openpaint_core::{Artwork, Color, StrokeSample, StrokeTool};
use openpaint_render::changed_regions;

use crate::drawing::{CHUNKS, begun, committed, path, screen, shows};
use crate::fixture::{core, plain, sample, with_layer, with_stroke};
use crate::raster::{color, every_raster_source};
use crate::raster_smudge::every_smudge;
use crate::tiers::Screen;

/// A smudge, whose contacts trail the pen until its lift: each frame as the same request
/// drawn on a canvas that showed the artwork until then; the lift the commit drawn cold.
#[test]
fn a_smudge_drawn_in_batches_is_the_frame_drawn_at_once_and_its_lift_the_commit() {
    let core = core();
    let base = every_smudge();
    let samples = path([120.0, 100.0], 70.0, 30);
    let style = plain(26.0, 0.8, Color::BLACK).with_tool(StrokeTool::Smudge);
    for (zoom, scale) in [(1.0, None), (3.7, Some(4))] {
        let (mut screen, view) = screen(&base, (zoom, [120.0, 100.0]));
        let mut session = begun(&base, (&style, &samples[..1]), None);
        let mut end = 1;
        for size in CHUNKS.iter().cycle() {
            screen.frame(&core, (&base, Some(session.active())), &[], &view);
            let (mut once, _) = self::screen(&base, (zoom, [120.0, 100.0]));
            once.frame(&core, (&base, Some(session.active())), &[], &view);
            let what = format!("a smudge at {zoom}x after {end} samples");
            let read = |screen: &Screen| screen.target.read(&core).expect("read");
            assert!(read(&screen) == read(&once), "{what}");
            if scale.is_some() {
                same_tiles(&screen, &once, &what);
            }
            if end == samples.len() {
                break;
            }
            let next = (end + size).min(samples.len());
            session.extend(&samples[end..next]).expect("extend");
            end = next;
        }
        let lifted = session.finish().artwork;
        screen.frame(&core, (&lifted, None), &[], &view);
        shows(&screen, &lifted, scale, "a smudge lifted");
    }
}

/// The tier's tiles ready on both screens are the same (a tile of the ring round the shown ones
/// a change reached is dropped while a stroke is drawn: the two histories differ there).
fn same_tiles(screen: &Screen, once: &Screen, what: &str) {
    let core = core();
    let tiles = |screen: &Screen| -> Vec<_> {
        (0..5u32)
            .flat_map(|y| (0..5u32).map(move |x| [x, y]))
            .map(|tile| screen.view.tile_pixels(&core, tile).expect("tile"))
            .collect()
    };
    let (a, b) = (tiles(screen), tiles(once));
    let both: Vec<_> = (a.iter().zip(&b))
        .filter_map(|pair| match pair {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        })
        .collect();
    assert!(both.len() > 4, "{what}: tiles ready in both");
    assert!(both.iter().all(|(a, b)| a == b), "{what} at the tier");
}

/// An artwork with a layer more and a stroke on it.
fn under() -> Artwork {
    with_stroke(
        &with_layer(&every_raster_source(), 0.9),
        (
            plain(30.0, 1.0, color(0.5, 0.5, 0.1)),
            path([100.0, 60.0], 40.0, 10),
        ),
    )
}

/// QuickShape: the stroke's samples replaced whole mid-stroke by a shape's.
#[test]
fn a_reshaped_stroke_is_cold_every_frame() {
    let core = core();
    let base = under();
    let style = plain(18.0, 0.7, color(0.1, 0.3, 0.9));
    let drawn = path([160.0, 110.0], 90.0, 20);
    let (mut screen, view) = screen(&base, (1.0, [160.0, 100.0]));
    let mut session = begun(&base, (&style, &drawn), None);
    screen.frame(&core, (&base, Some(session.active())), &[], &view);
    let line: Vec<StrokeSample> = (0..12u16)
        .map(|i| {
            let at = f32::from(i);
            sample(40.0 + 20.0 * at, 150.0 - 8.0 * at, 0.8, u32::from(i) * 8)
        })
        .collect();
    session.reshape(&line).expect("snapped");
    screen.frame(&core, (&base, Some(session.active())), &[], &view);
    let reshaped = committed(&base, (&style, &line), None);
    shows(&screen, &reshaped, None, "reshaped");
    let lifted = session.finish().artwork;
    screen.frame(&core, (&lifted, None), &[], &view);
    shows(&screen, &lifted, None, "reshaped and lifted");
}

/// Mirrored copies (symmetry): the version under the stroke holds its copy about x = 160, a
/// committed stroke that changes every frame; then the stroke dropped (the platform cancelled it).
#[test]
fn a_mirrored_and_a_dropped_stroke_are_cold_every_frame() {
    let core = core();
    let base = under();
    let style = plain(18.0, 0.7, color(0.1, 0.3, 0.9));
    let drawn = path([160.0, 110.0], 90.0, 20);
    let (mut screen, view) = screen(&base, (1.0, [160.0, 100.0]));
    let mirrored = |samples: &[StrokeSample]| -> Vec<StrokeSample> {
        (samples.iter())
            .map(|s| {
                let at = s.position();
                sample(320.0 - at.x(), at.y(), s.pressure(), s.time_ms())
            })
            .collect()
    };
    let mut shown = base.clone();
    let mut session = begun(&base, (&style, &drawn[..1]), None);
    for end in [3, 7, 12, 20] {
        let from = session.active().samples().len();
        session.extend(&drawn[from..end]).expect("extend");
        let preview = committed(&base, (&style, &mirrored(&drawn[..end])), None);
        let moved = changed_regions(&shown, &preview);
        screen.frame(&core, (&preview, Some(session.active())), &moved, &view);
        let so_far = committed(&preview, (&style, &drawn[..end]), None);
        shows(&screen, &so_far, None, "mirrored");
        shown = preview;
    }
    let both = committed(&shown, (&style, &drawn), None);
    screen.frame(&core, (&both, None), &changed_regions(&shown, &both), &view);
    shows(&screen, &both, None, "mirrored and lifted");
    // Dropped: the canvas shows the version under it again.
    let mut session = begun(&both, (&style, &drawn[..8]), None);
    screen.frame(&core, (&both, Some(session.active())), &[], &view);
    session.extend(&drawn[8..]).expect("extend");
    screen.frame(&core, (&both, Some(session.active())), &[], &view);
    let base = session.cancel();
    screen.frame(&core, (&base, None), &[], &view);
    shows(&screen, &base, None, "dropped");
}
