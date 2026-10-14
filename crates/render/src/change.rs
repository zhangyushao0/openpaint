//! What switching the canvas from one artwork version to another means for its pixels: the
//! regions to evaluate again when the version changes other than by committing the stroke the
//! target is drawing (undo and redo; later layer edits). DECISIONS D82: callers pass these.

use openpaint_core::{Artwork, RegionEdit, StrokeList};

use crate::edits::{edit_rect, moved_rect, placed_rect};
use crate::filter::{MAX_EXTENT, kernel_of};
use crate::geom::PixelRect;
use crate::stroke::segment_regions;

/// The pixels that can differ between `before` and `after`. Strokes one version has beyond the
/// other count segment by segment, so an undo costs only the undone stroke's tiles; fills count
/// by their mask's rectangle. Any other
/// difference (layers added, removed, moved or with other properties) is the whole canvas.
#[must_use]
pub fn changed_regions(before: &Artwork, after: &Artwork) -> Vec<PixelRect> {
    if before.same_version(after) {
        return Vec::new();
    }
    let canvas = after.canvas();
    let full = vec![PixelRect::full([canvas.width(), canvas.height()])];
    if before.canvas() != canvas
        || before.layers().len() != after.layers().len()
        || before.groups() != after.groups()
    {
        return full;
    }
    let mut regions = Vec::new();
    for (old, new) in before.layers().iter().zip(after.layers()) {
        if old.id() != new.id() || old.props() != new.props() {
            return full;
        }
        match tail_regions(old.strokes(), new.strokes()) {
            Some(tail) => regions.extend(tail),
            None => return full,
        }
        let size = [canvas.width(), canvas.height()];
        let paint = ((old.strokes(), old.edits()), (new.strokes(), new.edits()));
        regions.extend(edit_regions((size, false), paint.0, paint.1));
        match (old.mask(), new.mask()) {
            (None, None) => {}
            // Strokes and fills on a mask count as a layer's; a mask added, deleted, enabled,
            // disabled or inverted changes the whole layer.
            (Some(a), Some(b)) if (a.enabled(), a.inverted()) == (b.enabled(), b.inverted()) => {
                match tail_regions(a.strokes(), b.strokes()) {
                    Some(tail) => regions.extend(tail),
                    None => return full,
                }
                let size = [canvas.width(), canvas.height()];
                let mask = ((a.strokes(), a.edits()), (b.strokes(), b.edits()));
                regions.extend(edit_regions((size, true), mask.0, mask.1));
            }
            _ => return full,
        }
    }
    regions
}

/// The rectangles of the edits either list has beyond their common start: an edit reaches only
/// its own rectangle (`edit_rect`), wherever it lies among the strokes, so an edit added,
/// undone or swapped for another (a preview changing) costs only those rectangles. A
/// transform that only moved (a transform being dragged) costs where its content showed and
/// where it shows: for a record, the content `b` (strokes, edits) had before it. On a mask
/// (`on_mask`) an adjustment reaches its clip or the canvas; on paint see `edit_reach`.
fn edit_regions(
    (canvas, on_mask): ([u32; 2], bool),
    (old_strokes, all_a): (&StrokeList, &[RegionEdit]),
    (strokes, b): (&StrokeList, &[RegionEdit]),
) -> Vec<PixelRect> {
    let common = all_a.iter().zip(b).take_while(|(x, y)| x.same(y)).count();
    let (a, rest) = (&all_a[common..], &b[common..]);
    if let ([RegionEdit::Transform(x)], [RegionEdit::Transform(y)]) = (a, rest) {
        let lift = match (x.lift(), y.lift()) {
            (Some(p), Some(q)) => p.shares(q),
            (None, None) => true,
            _ => false,
        };
        let patch = match (x.patch(), y.patch()) {
            (Some(p), Some(q)) => p.shares(q),
            (None, None) => true,
            _ => false,
        };
        if lift && patch && x.after() == y.after() {
            let moved = moved_rect(canvas, y);
            let moved = match y.patch() {
                Some(_) => moved,
                None => within(
                    moved,
                    content_rect(canvas, strokes, &b[..common], y.after()),
                ),
            };
            return vec![
                placed_rect(moved, x.placement()),
                placed_rect(moved, y.placement()),
            ];
        }
    }
    let reach = |(strokes, edits): (&StrokeList, &[RegionEdit]), index| {
        edit_reach((canvas, on_mask), (strokes, edits), index)
    };
    let old = (common..all_a.len()).map(|index| reach((old_strokes, all_a), index));
    let new = (common..b.len()).map(|index| reach((strokes, b), index));
    old.chain(new).collect()
}

/// What the edit at `index` of `edits` can change: its `edit_rect`; but an adjustment without a
/// clip, on paint, over content no transform record moves, only where the content before it
/// lies (S35: transparent paint stays so), widened by a filter's longest reach. The scene
/// bounds its op alike (`scene_adjust.rs`), within this.
fn edit_reach(
    (canvas, on_mask): ([u32; 2], bool),
    (strokes, edits): (&StrokeList, &[RegionEdit]),
    index: usize,
) -> PixelRect {
    let edit = &edits[index];
    let rect = edit_rect(canvas, edit);
    let Some(adjust) = edit.adjust().filter(|adjust| adjust.clip().is_none()) else {
        return rect;
    };
    if on_mask || edits.iter().any(|edit| edit.transform().is_some()) {
        return rect;
    }
    let content = strokes
        .iter()
        .take(adjust.after())
        .flat_map(|stroke| segment_regions(stroke.style(), stroke.samples()))
        .chain(edits[..index].iter().map(|edit| edit_rect(canvas, edit)))
        .filter(|rect| !rect.is_empty())
        .reduce(|all, next| all.union(&next));
    // Over nothing it changes nothing.
    let Some(content) = content else {
        return PixelRect {
            min: [0, 0],
            max: [0, 0],
        };
    };
    let reach = if kernel_of(adjust.adjustment()).is_some() {
        MAX_EXTENT as i32
    } else {
        0
    };
    let widened = PixelRect {
        min: content.min.map(|v| v - reach),
        max: content.max.map(|v| v + reach),
    };
    widened.intersect(&PixelRect::full(canvas))
}

/// Where the first `after` strokes and `edits` can have painted, as `[x0, y0, x1, y1]` (the
/// canvas when nothing narrows it).
fn content_rect(
    canvas: [u32; 2],
    strokes: &StrokeList,
    edits: &[RegionEdit],
    after: usize,
) -> [f32; 4] {
    let regions = strokes
        .iter()
        .take(after)
        .flat_map(|stroke| segment_regions(stroke.style(), stroke.samples()))
        .chain(edits.iter().map(|edit| edit_rect(canvas, edit)));
    let rect = regions.reduce(|all, next| all.union(&next));
    rect.map_or([0.0, 0.0, canvas[0] as f32, canvas[1] as f32], |rect| {
        let [x0, y0] = rect.min;
        let [x1, y1] = rect.max;
        [x0 as f32, y0 as f32, x1 as f32, y1 as f32]
    })
}

/// The overlap of two rectangles `[x0, y0, x1, y1]` (empty ones stay empty).
fn within(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ]
}

/// The segments of the strokes either list has beyond their common start, when they share all
/// but at most their last `DIVERGED` strokes: a version and one it was built from, two branches
/// of a history, a stroke's mirrored copies redrawn as it grows, a snapped shape edited (S42).
/// Lists only grow at the end, so sharing a stroke means sharing every one before it. `None`
/// when they diverge earlier.
fn tail_regions(a: &StrokeList, b: &StrokeList) -> Option<Vec<PixelRect>> {
    let shortest = a.len().min(b.len());
    let shares = |count: usize| {
        count == 0
            || a.get(count - 1)
                .zip(b.get(count - 1))
                .is_some_and(|(x, y)| x.shares_samples(y))
    };
    let common = (shortest.saturating_sub(DIVERGED)..=shortest)
        .rev()
        .find(|count| shares(*count))?;
    let tail = a
        .since(common)
        .chain(b.since(common))
        .flat_map(|stroke| segment_regions(stroke.style(), stroke.samples()));
    Some(tail.collect())
}

/// How many last strokes two lists may differ in for `tail_regions` (the radial guide's seven
/// copies and the stroke, twice over).
const DIVERGED: usize = 16;

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_core::{
        ArtworkCommand, CanvasPoint, CanvasSize, Color, LayerCommand, RegionMask, StrokeSample,
        StrokeStyle,
    };

    fn sample(x: f32, time_ms: u32) -> StrokeSample {
        StrokeSample::new(
            CanvasPoint::new(x, 10.0).expect("point"),
            1.0,
            0.0,
            0.0,
            time_ms,
        )
        .expect("sample")
    }

    fn draw(artwork: &Artwork, from: f32) -> Artwork {
        let recipe = std::sync::Arc::new(openpaint_core::BrushRecipe::plain());
        let style = StrokeStyle::new(recipe, 4.0, 1.0, Color::BLACK).expect("style");
        let mut session = artwork.begin_stroke(style, sample(from, 0)).expect("begin");
        session
            .extend(&[sample(from + 10.0, 4), sample(from + 20.0, 8)])
            .expect("extend");
        session.finish().artwork
    }

    fn blank() -> Artwork {
        Artwork::new(CanvasSize::new(200, 40).expect("size"))
    }

    #[test]
    fn undoing_a_stroke_changes_only_its_segments_either_way() {
        let first = draw(&blank(), 0.0);
        let second = draw(&first, 100.0);
        let undo = changed_regions(&second, &first);
        assert_eq!(undo, changed_regions(&first, &second));
        assert_eq!(
            undo.len(),
            1,
            "one span: the middle sample is a resample on the chord"
        );
        assert!(undo.iter().all(|region| region.min[0] >= 96));
        assert!(changed_regions(&first, &first).is_empty());
    }

    fn mask(x: std::ops::Range<usize>) -> RegionMask {
        let mut inside = vec![false; 200 * 40];
        for y in 5..9 {
            for column in x.clone() {
                inside[y * 200 + column] = true;
            }
        }
        RegionMask::from_pixels(CanvasSize::new(200, 40).expect("size"), &inside).expect("mask")
    }

    #[test]
    fn a_fill_changes_its_rectangle_and_a_new_preview_both_rectangles() {
        let base = draw(&blank(), 0.0);
        let left = base
            .fill_region(mask(3..9), Color::WHITE)
            .expect("fill")
            .artwork;
        // The mask's rectangle and a pixel round it, which its smoothed edge reaches.
        let rect = |x: [i32; 2]| PixelRect {
            min: [x[0] - 1, 4],
            max: [x[1] + 1, 10],
        };
        assert_eq!(changed_regions(&base, &left), [rect([3, 9])]);
        assert_eq!(changed_regions(&left, &base), [rect([3, 9])]);
        let right = base
            .fill_region(mask(150..160), Color::WHITE)
            .expect("fill")
            .artwork;
        assert_eq!(
            changed_regions(&left, &right),
            [rect([3, 9]), rect([150, 160])]
        );
        let later = draw(&left, 100.0);
        assert!(
            changed_regions(&left, &later)
                .iter()
                .all(|r| r.min[0] >= 96)
        );
    }

    #[test]
    fn layer_changes_are_the_whole_canvas_and_branches_only_their_strokes() {
        let base = draw(&blank(), 150.0);
        let full = vec![PixelRect::full([200, 40])];
        let command = ArtworkCommand::Layer(LayerCommand::Create);
        let layered = base.apply(command).expect("create").artwork;
        assert_eq!(changed_regions(&base, &layered), full);
        // Two branches from `base`, or a last stroke replaced (a copy redrawn, a shape edited):
        // both strokes, never the one they share.
        let left = draw(&base, 0.0);
        let right = draw(&base, 60.0);
        let both = changed_regions(&left, &right);
        assert_eq!(both.len(), 2, "{both:?}");
        assert!(both.iter().all(|r| r.max[0] <= 100), "{both:?}");
        assert_eq!(
            changed_regions(&draw(&left, 0.0), &draw(&right, 0.0)).len(),
            4
        );
    }
}
