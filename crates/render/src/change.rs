//! What switching the canvas from one artwork version to another means for its pixels: the
//! regions to evaluate again when the version changes other than by committing the stroke the
//! target is drawing (undo and redo; later layer edits). DECISIONS D17: callers pass these.

use openpaint_core::{Artwork, CanvasSize, RegionEdit, StrokeList};

use crate::edits::{edit_rect, placed_rect};
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
    if before.canvas() != canvas || before.layers().len() != after.layers().len() {
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
        regions.extend(edit_regions(canvas, old.edits(), new.edits()));
    }
    regions
}

/// The rectangles of the edits either list has beyond their common start: an edit reaches only
/// its own rectangle (`edit_rect`), wherever it lies among the strokes, so an edit added,
/// undone or swapped for another (a preview changing) costs only those rectangles. A
/// transform that only moved its patch (a transform being dragged) costs where the patch
/// showed and where it shows.
fn edit_regions(canvas: CanvasSize, a: &[RegionEdit], b: &[RegionEdit]) -> Vec<PixelRect> {
    let common = a.iter().zip(b).take_while(|(x, y)| x.same(y)).count();
    let (a, b) = (&a[common..], &b[common..]);
    if let ([RegionEdit::Transform(x)], [RegionEdit::Transform(y)]) = (a, b) {
        let lift = match (x.lift(), y.lift()) {
            (Some(p), Some(q)) => p.shares(q),
            (None, None) => true,
            _ => false,
        };
        if lift && x.patch().shares(y.patch()) && x.after() == y.after() {
            return vec![placed_rect(x), placed_rect(y)];
        }
    }
    a.iter()
        .chain(b)
        .map(|edit| edit_rect(canvas, edit))
        .collect()
}

/// The segments of the strokes the longer list has beyond the shorter, when the shorter is its
/// prefix, as for a version and one it was built from (lists only grow at the end, so sharing
/// the shorter list's last stroke means sharing all of it). `None` when the lists diverge, as
/// two branches of a history do even where their stroke ids agree.
fn tail_regions(a: &StrokeList, b: &StrokeList) -> Option<Vec<PixelRect>> {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if let Some(last) = short.last() {
        let same = long.get(short.len() - 1)?;
        if !same.shares_samples(last) {
            return None;
        }
    }
    let tail = long
        .since(short.len())
        .flat_map(|stroke| segment_regions(stroke.style(), stroke.samples(), 0));
    Some(tail.collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_core::{
        ArtworkCommand, CanvasPoint, Color, LayerCommand, RegionMask, StrokeSample, StrokeStyle,
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
        assert_eq!(undo.len(), 2, "two segments");
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
        let rect = |x: [i32; 2]| PixelRect {
            min: [x[0], 5],
            max: [x[1], 9],
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
    fn layer_changes_and_diverging_strokes_are_the_whole_canvas() {
        let base = blank();
        let full = vec![PixelRect::full([200, 40])];
        let command = ArtworkCommand::Layer(LayerCommand::Create);
        let layered = base.apply(command).expect("create").artwork;
        assert_eq!(changed_regions(&base, &layered), full);
        let left = draw(&base, 0.0);
        let right = draw(&base, 100.0);
        assert_eq!(changed_regions(&left, &right), full);
    }
}
