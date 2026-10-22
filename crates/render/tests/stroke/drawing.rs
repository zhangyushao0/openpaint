//! The stroke being drawn on the raster core: drawn a few samples a frame
//! over an artwork the raster core draws, after every batch the canvas (and every ready tile of the
//! view tier shown) equals the artwork with the stroke so far committed drawn cold (its end not
//! tapered yet), bit for bit; the lift's frame equals the committed artwork drawn cold; for
//! paint, erasing, Alpha Lock, a selection's clip, a mask, a clipping chain's layer (a smudge, a
//! stroke snapped to a shape and one drawn with mirrored copies: `drawing_kinds.rs`; strokes one
//! after another and the snapshots under them: `drawing_ahead.rs`), over a moved selection's
//! record (inside every other kind, `raster_kinds.rs`).

use openpaint_core::{
    Artwork, BrushRecipe, Color, LayerCommand, RegionMask, StrokeSample, StrokeSession,
    StrokeStyle, StrokeTool, Taper,
};

use crate::fixture::{cold, core, graphite, layer, plain, region, sample};
use crate::raster::{color, every_raster_source};
use crate::records::{busy, by_record, lasso, turned};
use crate::scale::whole;
use crate::stamps::brushes;
use crate::tiers::{Screen, frame};

/// Samples each frame takes in turn.
pub const CHUNKS: [usize; 5] = [1, 2, 3, 5, 8];

/// A stroke to draw: its style (and the same without its end taper), its samples, its clip.
struct Case {
    name: &'static str,
    base: Artwork,
    style: StrokeStyle,
    blunt: StrokeStyle,
    samples: Vec<StrokeSample>,
    clip: Option<RegionMask>,
}

/// A wavy path about `centre`, `reach` pixels each way, pressed from 0.5 to 1.
pub fn path(centre: [f32; 2], reach: f32, count: u16) -> Vec<StrokeSample> {
    (0..count)
        .map(|i| {
            let t = f32::from(i) / f32::from(count - 1);
            let x = centre[0] - reach + 2.0 * reach * t;
            let y = centre[1] + 0.4 * reach * (std::f32::consts::TAU * t).sin();
            sample(x, y, 0.5 + 0.5 * t, u32::from(i) * 8)
        })
        .collect()
}

fn case(name: &'static str, base: Artwork, style: StrokeStyle, samples: Vec<StrokeSample>) -> Case {
    Case {
        name,
        base,
        blunt: style.clone(),
        style,
        samples,
        clip: None,
    }
}

/// The stroke of `style` through `samples` begun over `base` (inside `clip`).
pub fn begun(
    base: &Artwork,
    (style, samples): (&StrokeStyle, &[StrokeSample]),
    clip: Option<&RegionMask>,
) -> StrokeSession {
    let session = base.begin_stroke(style.clone(), samples[0]).expect("begin");
    let mut session = match clip {
        Some(clip) => session.clipped(clip.clone()).expect("clipped"),
        None => session,
    };
    session.extend(&samples[1..]).expect("extend");
    session
}

/// `base` with the stroke committed.
pub fn committed(
    base: &Artwork,
    stroke: (&StrokeStyle, &[StrokeSample]),
    clip: Option<&RegionMask>,
) -> Artwork {
    begun(base, stroke, clip).finish().artwork
}

/// The screen at `zoom` (below √2: the canvas itself) about `centre`, and the tier's scale.
pub fn screen(
    base: &Artwork,
    (zoom, centre): (f32, [f32; 2]),
) -> (Screen, openpaint_render::ViewFrame) {
    let core = core();
    let view = frame(zoom, centre);
    let mut screen = Screen::new(&core, base);
    if zoom > 1.5 {
        screen.settle(&core, base, &view);
    }
    (screen, view)
}

/// Every check of a frame: the canvas the cold draw of `so_far`, the tier's ready tiles its
/// export at `scale`.
pub fn shows(screen: &Screen, so_far: &Artwork, scale: Option<u32>, what: &str) {
    let core = core();
    assert!(
        screen.target.read(&core).expect("read") == cold(&core, so_far),
        "{what}: the canvas"
    );
    if let Some(scale) = scale {
        screen.assert_tiles(&core, &whole(&core, so_far, scale), scale);
    }
}

/// `case` drawn in batches at `zoom` about its middle; checked after every batch and the lift.
fn drawn(case: &Case, zoom: f32, scale: Option<u32>) {
    let core = core();
    let middle = case.samples[case.samples.len() / 2].position();
    let (mut screen, view) = screen(&case.base, (zoom, [middle.x(), middle.y()]));
    let mut end = 1;
    let mut session = begun(
        &case.base,
        (&case.style, &case.samples[..1]),
        case.clip.as_ref(),
    );
    for size in CHUNKS.iter().cycle() {
        screen.frame(&core, (&case.base, Some(session.active())), &[], &view);
        let so_far = (&case.blunt, &case.samples[..end]);
        let what = format!("{} at {zoom}x after {end} samples", case.name);
        shows(
            &screen,
            &committed(&case.base, so_far, case.clip.as_ref()),
            scale,
            &what,
        );
        if end == case.samples.len() {
            break;
        }
        let next = (end + size).min(case.samples.len());
        session.extend(&case.samples[end..next]).expect("extend");
        end = next;
    }
    let lifted = session.finish().artwork;
    screen.frame(&core, (&lifted, None), &[], &view);
    shows(&screen, &lifted, scale, &format!("{} lifted", case.name));
}

/// The cases: paint (a translucent round, a stamped Textured Round, the pencil on its paper with
/// an end taper), erasing; then the places (`places`).
fn cases() -> Vec<Case> {
    let base = every_raster_source();
    let through = path([160.0, 100.0], 120.0, 40);
    let mut all = vec![case(
        "paint",
        base.clone(),
        plain(22.0, 0.6, color(0.1, 0.5, 0.8)),
        through.clone(),
    )];
    let (_, textured, _) = brushes().swap_remove(0);
    let stamped = StrokeStyle::new(
        std::sync::Arc::new(textured),
        36.0,
        0.9,
        color(0.7, 0.3, 0.2),
    );
    all.push(case(
        "stamps",
        base.clone(),
        stamped.expect("style").with_seed(9),
        through.clone(),
    ));
    let pencil = StrokeStyle::new(graphite(), 14.0, 0.8, Color::BLACK).expect("pencil");
    all.push(case("pencil", base.clone(), pencil, through.clone()));
    let taper = |end: f32| {
        let recipe = BrushRecipe::plain().with_taper(Taper::new(1.0, end, 0.1).expect("taper"));
        StrokeStyle::new(std::sync::Arc::new(recipe), 16.0, 1.0, color(0.2, 0.2, 0.2))
            .expect("style")
    };
    let mut tapered = case("tapered", base.clone(), taper(2.0), through.clone());
    tapered.blunt = taper(0.0);
    all.push(tapered);
    let eraser = plain(26.0, 0.9, Color::BLACK).with_tool(StrokeTool::Erase);
    all.push(case("erase", base.clone(), eraser, through.clone()));
    all.extend(places(&base, &through));
    all
}

/// The places a stroke lays: Alpha Lock, a lasso's clip, a mask, a clipping chain's layer, over a
/// moved selection's record.
fn places(base: &Artwork, through: &[StrokeSample]) -> Vec<Case> {
    let first = layer(base, LayerCommand::Select(base.layers()[0].id()));
    let style = |size: f32, opacity: f32, rgb: [f32; 3]| {
        plain(size, opacity, color(rgb[0], rgb[1], rgb[2]))
    };
    let through = through.to_vec();
    let mut all = vec![case(
        "alpha lock",
        first,
        style(30.0, 0.9, [0.9, 0.9, 0.1]),
        through.clone(),
    )];
    let mut clipped = case(
        "lasso",
        base.clone(),
        style(30.0, 0.9, [0.3, 0.9, 0.3]),
        through.clone(),
    );
    clipped.clip = Some(region(|x, y| {
        (x as i32 - 150).abs() + (y as i32 - 100).abs() < 60
    }));
    all.push(clipped);
    let picture = base.layers()[1].id();
    let on_mask = (base.apply(openpaint_core::ArtworkCommand::Mask(
        openpaint_core::MaskCommand::Select(picture),
    )))
    .expect("mask")
    .artwork;
    let black = plain(30.0, 1.0, Color::BLACK);
    all.push(case("mask", on_mask, black, through.clone()));
    let chained = layer(base, LayerCommand::Select(base.layers()[2].id()));
    let chain = style(26.0, 0.8, [0.6, 0.1, 0.6]);
    all.push(case("chain", chained, chain, through.clone()));
    let moved = by_record(&busy(), Some(&lasso()), turned());
    let blue = style(24.0, 0.7, [0.2, 0.6, 0.9]);
    all.push(case("record", moved, blue, through));
    all
}

#[test]
fn a_stroke_drawn_in_batches_is_the_stroke_so_far_drawn_cold_on_the_canvas() {
    for case in cases() {
        drawn(&case, 1.0, None);
    }
}

#[test]
fn a_stroke_drawn_in_batches_is_its_export_at_the_tiers() {
    for case in cases() {
        drawn(&case, 3.7, Some(4));
    }
    let mut zoomed = cases().swap_remove(1);
    zoomed.samples = path([160.0, 100.0], 18.0, 24);
    drawn(&zoomed, 7.0, Some(8));
    zoomed.samples = path([160.0, 100.0], 50.0, 30);
    drawn(&zoomed, 2.2, Some(2));
}
