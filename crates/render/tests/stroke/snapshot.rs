//! The in-progress stroke over its layer's snapshot (S48): a stroke drawn a sample a frame over
//! 300 finished strokes equals the stroke so far evaluated cold on every frame, and its lift's
//! frame the commit, while each frame lists only a handful of ops per tile; the snapshot
//! carries the committed stroke into the next one, starts over after an undo, on another layer
//! and on the mask; it holds every kind of op beneath the pen (smudges, fills, a pencil on its
//! paper, a selection's transform record, a merged layer's nested record); and over 300
//! Textured Round strokes it lists as few ops.
//!
//! Every comparison is bit for bit with a cold evaluation, but for textured stamps on the
//! development machine's GPU (an RTX 5070 on Vulkan, 2026-10-09), which evaluates them in one
//! of two results a level apart in a few pixels, switching every second or so, at the code
//! before S48 too (STATUS): the Textured Round test accepts that level.

use std::f32::consts::{PI, TAU};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use openpaint_core::{
    Artwork, ArtworkCommand, BrushRecipe, Color, LayerCommand, MaskCommand, StrokeSample,
    StrokeStyle, StrokeTool,
};
use openpaint_render::{
    CanvasTarget, Core, EvaluationWork, FrameRequest, PixelRect, RegionEvaluator, RgbaImage,
    changed_regions, stroke_region,
};

use crate::fixture::{SIZE, airbrush, blank, cold, core, request, sample, with_layer, with_stroke};
use crate::merge::merged;
use crate::records::{busy, by_record, lasso, turned};
use crate::stamps::brushes;

/// A colour of its own for stroke `seed`.
fn hue(seed: u32) -> Color {
    let share = (seed % 7) as f32 / 7.0;
    Color::rgb(0.2 + 0.6 * share, 0.3, 0.8 - 0.6 * share).expect("colour")
}

/// A plain round brush of `size`, translucent.
fn round(size: f32, seed: u32) -> StrokeStyle {
    StrokeStyle::new(Arc::new(BrushRecipe::plain()), size, 0.7, hue(seed)).expect("style")
}

/// A Textured Round of `size` with `seed`.
fn textured(size: f32, seed: u32) -> StrokeStyle {
    let (_, recipe, _) = brushes().swap_remove(0);
    StrokeStyle::new(Arc::new(recipe), size, 0.9, hue(seed))
        .expect("style")
        .with_seed(seed)
}

/// Stroke `k` of many crossing the canvas's middle at every angle: a 180 px line turned by `k`
/// golden angles and moved a little, 20 samples (as `cargo device-input pen-many` lays them).
fn crossing(k: usize) -> Vec<StrokeSample> {
    let angle = k as f32 * 2.399_963_2;
    let shift = [((k * 37) % 61) as f32 - 30.0, ((k * 59) % 41) as f32 - 20.0];
    (0..20u16)
        .map(|i| {
            let t = f32::from(i) / 19.0;
            let s = 180.0 * (t - 0.5);
            let x = 160.0 + shift[0] + s * angle.cos();
            let y = 100.0 + shift[1] + s * angle.sin();
            sample(x, y, 0.6 + 0.3 * (PI * t).sin(), u32::from(i) * 8)
        })
        .collect()
}

/// A wave across the canvas, starting `phase` turns in.
fn wave(phase: f32) -> Vec<StrokeSample> {
    (0..40u16)
        .map(|i| {
            let t = f32::from(i) / 39.0;
            let y = 100.0 + 60.0 * (TAU * (t + phase)).sin();
            sample(30.0 + 260.0 * t, y, 0.8, u32::from(i) * 8)
        })
        .collect()
}

/// `base` with `count` crossing strokes of `style(k)` on its active layer.
fn painted(base: &Artwork, count: usize, style: impl Fn(u32) -> StrokeStyle) -> Artwork {
    (0..count).fold(base.clone(), |artwork, k| {
        with_stroke(&artwork, (style(k as u32), crossing(k)))
    })
}

/// A target showing `artwork`, evaluated whole.
fn shown(core: &Core, artwork: &Artwork) -> CanvasTarget {
    let mut target = CanvasTarget::new(core, SIZE).expect("target");
    RegionEvaluator::new(core, &mut target)
        .evaluate(&request(artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    target
}

/// How a target must match a cold evaluation: bit for bit, or (textured stamps) within the
/// GPU's two results.
#[derive(Clone, Copy, PartialEq)]
enum Match {
    Exact,
    Stamped,
}

/// A stroke drawn by `drawn_cold`: the commit, the most op entries per tile a frame after the
/// first listed, what the first frame did, and the tiles its frames carried the strokes
/// committed before it into.
struct Drawn {
    artwork: Artwork,
    densest: f32,
    first: EvaluationWork,
    carried: usize,
}

/// Draws `samples` with `style` over `base` (what `target` shows) a sample a frame and commits
/// it. Each frame, the pixels its samples can change (the new spans, the tail and the tail
/// drawn before: `stroke_region` from four samples back) match the stroke so far evaluated cold
/// (but a smudge's, whose contacts trail the pen until it lifts, D46), and where its last
/// samples reach the lift's frame matches the commit cold: every pixel the stroke reaches is
/// compared in the frame that draws it.
fn drawn_cold(
    core: &Core,
    (target, how): (&mut CanvasTarget, Match),
    base: &Artwork,
    (style, samples): (StrokeStyle, Vec<StrokeSample>),
) -> Drawn {
    let mut session = base.begin_stroke(style.clone(), samples[0]).expect("begin");
    let (mut densest, mut first, mut carried) = (0.0f32, EvaluationWork::default(), 0);
    let recent = |end: usize| stroke_region(&style, &samples[..end], end.saturating_sub(4));
    for end in 1..=samples.len() {
        if end > 1 {
            session.extend(&samples[end - 1..end]).expect("extend");
        }
        let frame = FrameRequest {
            active: Some(session.active()),
            ..request(base)
        };
        RegionEvaluator::new(core, target)
            .evaluate(&frame, &[])
            .expect("evaluate");
        let work = target.work();
        carried += work.carried;
        if end == 1 {
            first = work;
        } else if work.tiles > 0 {
            densest = densest.max(work.entries as f32 / work.tiles as f32);
        }
        if style.tool() != StrokeTool::Smudge {
            let so_far = with_stroke(base, (style.clone(), samples[..end].to_vec()));
            let at = recent(end).expect("a region");
            assert!(
                matches(core, (target, how), &so_far, at),
                "after {end} samples"
            );
        }
    }
    let artwork = session.finish().artwork;
    RegionEvaluator::new(core, target)
        .evaluate(&request(&artwork), &[])
        .expect("evaluate");
    let at = recent(samples.len()).expect("a region");
    assert!(
        matches(core, (target, how), &artwork, at),
        "the lift = the commit"
    );
    Drawn {
        artwork,
        densest,
        first,
        carried,
    }
}

/// Whether `target` shows `artwork` inside `region` as a cold evaluation does: one of up to 16
/// cold results, taken while the frame that drew those pixels has just run, bit for bit; or,
/// for `Match::Stamped`, the last of them but one level apart in at most 2 % of the channels.
fn matches(
    core: &Core,
    (target, how): (&CanvasTarget, Match),
    artwork: &Artwork,
    region: PixelRect,
) -> bool {
    let shown = target.read(core).expect("read");
    let [x0, x1] = [region.min[0], region.max[0]].map(|x| x.clamp(0, SIZE[0] as i32) as usize);
    let [y0, y1] = [region.min[1], region.max[1]].map(|y| y.clamp(0, SIZE[1] as i32) as usize);
    let inside = |image: &RgbaImage| -> Vec<u8> {
        let width = SIZE[0] as usize;
        (y0..y1)
            .flat_map(|y| image.pixels()[(y * width + x0) * 4..(y * width + x1) * 4].to_vec())
            .collect()
    };
    let wanted = inside(&shown);
    let mut last = Vec::new();
    for _ in 0..16 {
        last = inside(&cold(core, artwork));
        if wanted == last {
            return true;
        }
    }
    let apart: Vec<u8> = wanted
        .iter()
        .zip(&last)
        .map(|(a, b)| a.abs_diff(*b))
        .collect();
    let differ = apart.iter().filter(|d| **d > 0).count();
    how == Match::Stamped && apart.iter().all(|d| *d <= 1) && differ * 50 <= apart.len()
}

/// These tests one at a time: the GPU's load sets how often stamps change result (`matches`).
fn alone() -> MutexGuard<'static, ()> {
    static ONE: Mutex<()> = Mutex::new(());
    ONE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Shows `next` after `before` as the runtime does for an undo or any other change.
fn switched(core: &Core, target: &mut CanvasTarget, before: &Artwork, next: &Artwork) {
    let changed = changed_regions(before, next);
    RegionEvaluator::new(core, target)
        .evaluate(&request(next), &changed)
        .expect("evaluate");
    let all = changed.into_iter().reduce(|a, b| a.union(&b));
    let all = all.unwrap_or(PixelRect::full(SIZE));
    assert!(matches(core, (target, Match::Exact), next, all));
}

/// Fewer entries a tile than this: the 300 strokes beneath are not walked.
const FEW: f32 = 30.0;

#[test]
fn a_stroke_over_300_strokes_equals_cold_every_frame_and_lists_few_ops() {
    let _alone = alone();
    let core = core();
    let base = painted(&blank(), 300, |k| round(24.0, k));
    let mut target = shown(&core, &base);
    let exact = (&mut target, Match::Exact);
    let one = drawn_cold(&core, exact, &base, (round(24.0, 1000), wave(0.0)));
    eprintln!(
        "over 300 strokes: at most {:.1} entries a tile",
        one.densest
    );
    assert!(one.densest < FEW, "{} entries a tile", one.densest);
    assert!(one.first.taken > 0 && one.carried == 0, "{:?}", one.first);
    // The next stroke resumes from the snapshot, which took the first one in.
    let exact = (&mut target, Match::Exact);
    let two = drawn_cold(&core, exact, &one.artwork, (round(30.0, 1001), wave(0.3)));
    assert!(
        two.densest < FEW,
        "{} entries a tile after a commit",
        two.densest
    );
    assert!(two.carried > 0, "{:?}", two.first);
    // Undo both: it starts over.
    switched(&core, &mut target, &two.artwork, &base);
    let exact = (&mut target, Match::Exact);
    let three = drawn_cold(&core, exact, &base, (round(20.0, 1002), wave(0.6)));
    assert!(
        three.densest < FEW,
        "{} entries a tile after an undo",
        three.densest
    );
    assert!(three.carried == 0, "{:?}", three.first);
}

#[test]
fn a_textured_round_over_300_textured_rounds_lists_few_ops_and_matches_cold() {
    let _alone = alone();
    let core = core();
    let base = painted(&blank(), 300, |k| textured(24.0, k));
    let mut target = shown(&core, &base);
    let stamped = (&mut target, Match::Stamped);
    let one = drawn_cold(&core, stamped, &base, (textured(24.0, 1000), wave(0.0)));
    eprintln!(
        "over 300 textured strokes: at most {:.1} entries a tile",
        one.densest
    );
    assert!(one.densest < FEW, "{} entries a tile", one.densest);
    let stamped = (&mut target, Match::Stamped);
    let two = drawn_cold(
        &core,
        stamped,
        &one.artwork,
        (textured(30.0, 1001), wave(0.3)),
    );
    assert!(two.densest < FEW && two.carried > 0, "{:?}", two.first);
}

#[test]
fn the_snapshot_starts_over_on_another_layer_and_on_the_mask() {
    let _alone = alone();
    let core = core();
    let base = painted(&blank(), 100, |k| round(24.0, k));
    let lower = base.active();
    let mut target = shown(&core, &base);
    let exact = (&mut target, Match::Exact);
    let one = drawn_cold(&core, exact, &base, (round(24.0, 7), wave(0.1)));
    let above = painted(&with_layer(&one.artwork, 0.8), 100, |k| round(24.0, k));
    switched(&core, &mut target, &one.artwork, &above);
    let exact = (&mut target, Match::Exact);
    let two = drawn_cold(&core, exact, &above, (round(24.0, 8), wave(0.4)));
    assert!(two.carried == 0, "another layer: {:?}", two.first);
    let select = ArtworkCommand::Layer(LayerCommand::Select(lower));
    let back = two.artwork.apply(select).expect("select").artwork;
    switched(&core, &mut target, &two.artwork, &back);
    let exact = (&mut target, Match::Exact);
    let three = drawn_cold(&core, exact, &back, (round(24.0, 9), wave(0.7)));
    // A mask with strokes of its own grey, drawn on again.
    let add = ArtworkCommand::Mask(MaskCommand::Add(lower));
    let masked = three.artwork.apply(add).expect("mask").artwork;
    let grey = Color::rgb(0.3, 0.3, 0.3).expect("grey");
    let style = |seed| {
        let style = StrokeStyle::new(airbrush(), 40.0, 0.7, grey).expect("style");
        style.with_seed(seed)
    };
    let greyed = painted(&masked, 20, style);
    switched(&core, &mut target, &three.artwork, &greyed);
    let exact = (&mut target, Match::Exact);
    let four = drawn_cold(&core, exact, &greyed, (style(99), wave(0.2)));
    // On the mask the layer's paint (held while the mask is painted) is walked still.
    assert!(four.first.taken > 0, "{:?}", four.first);
}

#[test]
fn the_snapshot_holds_smudges_fills_transform_records_and_merged_layers() {
    let _alone = alone();
    let core = core();
    // The wave, a bar, a pencil diagonal, a fill and a smudge; a lasso moved and turned; then a
    // layer of strokes merged down into it as a nested record.
    let moved = by_record(&busy(), Some(&lasso()), turned());
    let upper = painted(&with_layer(&moved, 0.9), 40, |k| round(18.0, k));
    let base = merged(&upper);
    let mut target = shown(&core, &base);
    let exact = (&mut target, Match::Exact);
    let one = drawn_cold(&core, exact, &base, (round(24.0, 5), wave(0.0)));
    assert!(one.densest < FEW, "{} entries a tile", one.densest);
    // A smudge drawn over all of it, and a stroke after the smudge.
    let smudge = StrokeStyle::new(airbrush(), 30.0, 0.8, Color::BLACK)
        .expect("style")
        .with_tool(StrokeTool::Smudge);
    let exact = (&mut target, Match::Exact);
    let two = drawn_cold(&core, exact, &one.artwork, (smudge, wave(0.5)));
    assert!(two.carried > 0, "{:?}", two.first);
    let exact = (&mut target, Match::Exact);
    let three = drawn_cold(&core, exact, &two.artwork, (round(16.0, 6), wave(0.25)));
    assert!(three.carried > 0, "after a smudge: {:?}", three.first);
}
