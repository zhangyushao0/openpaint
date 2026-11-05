//! The copy on a practice sheet: what the pen laid in an exercise's empty frame, read from the
//! artwork's records. StreamLine and QuickShape are off on a practice sheet, so a stroke's
//! samples are the pen's own: where it went, how hard and when (kept for a later study).
//!
//! A stroke belongs to the frame its first sample lies in (grown by `FRAME_REACH` of its longer
//! side, where strokes may begin and are clipped to the frame: `Frame::holds`). A line's copy,
//! which the aligned overlay is fitted to, is its paint strokes but the sketch pen's construction
//! lines, oldest first, less what a later eraser stroke passed over within its radius (the
//! brush's size, the pressure not counted). A colour's copy is the colours laid: each paint
//! stroke's colour and each fill's whose region lies in the frame, in the order they lie on their
//! layer, layers bottom to top.

use std::ops::Range;

use openpaint_core::{Artwork, Color, Layer, Stroke, StrokeTool};
use openpaint_practice::{ColourCopy, Frame, Level, Point, Rgb8, nearest};

use crate::practice_pens::{levelled, sketched};

/// The paper under a sheet, as the canvas shows it where nothing was laid (the runtime's).
pub(crate) const PAPER: Rgb8 = [255; 3];

fn first_point(stroke: &Stroke) -> Option<Point> {
    let first = stroke.samples().first()?.position();
    Some([f64::from(first.x()), f64::from(first.y())])
}

/// Whether `stroke` began in `frame`.
fn in_frame(stroke: &Stroke, frame: &Frame) -> bool {
    first_point(stroke).is_some_and(|p| frame.holds(p))
}

/// Every stroke of `artwork` begun in `frame`, oldest first.
fn strokes_in<'a>(artwork: &'a Artwork, frame: &Frame) -> Vec<&'a Stroke> {
    let mut strokes: Vec<&Stroke> = artwork
        .layers()
        .iter()
        .flat_map(|layer| layer.strokes().iter())
        .filter(|stroke| in_frame(stroke, frame))
        .collect();
    strokes.sort_by_key(|stroke| stroke.id());
    strokes
}

/// The pen's positions along `stroke`, in canvas pixels.
fn positions(stroke: &Stroke) -> Vec<Point> {
    stroke
        .samples()
        .iter()
        .map(|s| [f64::from(s.position().x()), f64::from(s.position().y())])
        .collect()
}

/// Every stroke id.
pub(crate) const ALL_STROKES: Range<u64> = 0..u64::MAX;

/// The line copied in `frame` in pieces, in frame units, of the strokes whose ids lie in `ids`:
/// what the aligned overlay fits, a geometry check reads and the practice book shows.
pub(crate) fn line_pieces(artwork: &Artwork, frame: &Frame, ids: Range<u64>) -> Vec<Vec<Point>> {
    let taken = |stroke: &Stroke| {
        let style = stroke.style();
        let kept = style.tool() == StrokeTool::Paint && !sketched(style);
        (kept && ids.contains(&stroke.id().get())).then_some(())
    };
    (pieces_of(artwork, frame, taken).into_iter())
        .map(|((), piece)| piece)
        .collect()
}

/// A division's lines drawn in `frame` in pieces, in frame units, each with the level its pen
/// draws (S95): the strokes of the levels' pens, less what a later eraser passed over.
pub(crate) fn division_pieces(artwork: &Artwork, frame: &Frame) -> Vec<(Level, Vec<Point>)> {
    let level = |stroke: &Stroke| {
        let style = stroke.style();
        (style.tool() == StrokeTool::Paint)
            .then(|| levelled(style))
            .flatten()
    };
    pieces_of(artwork, frame, level)
}

/// The strokes begun in `frame` that `taken` keeps (with what it tells of each), cut where a later
/// eraser stroke passed over them, in frame units, oldest first.
fn pieces_of<T: Copy>(
    artwork: &Artwork,
    frame: &Frame,
    taken: impl Fn(&Stroke) -> Option<T>,
) -> Vec<(T, Vec<Point>)> {
    let strokes = strokes_in(artwork, frame);
    let mut pieces = Vec::new();
    for (index, stroke) in strokes.iter().enumerate() {
        let Some(told) = taken(stroke) else {
            continue;
        };
        let erasers: Vec<(Vec<Point>, f64)> = strokes[index + 1..]
            .iter()
            .filter(|later| later.style().tool() == StrokeTool::Erase)
            .map(|later| (positions(later), f64::from(later.style().size()) / 2.0))
            .collect();
        for piece in kept_pieces(&positions(stroke), &erasers) {
            pieces.push((told, piece.iter().map(|p| frame.to_frame(*p)).collect()));
        }
    }
    pieces
}

/// `path` cut where any of `erasers` (a path and its radius) passed over it.
fn kept_pieces(path: &[Point], erasers: &[(Vec<Point>, f64)]) -> Vec<Vec<Point>> {
    let erased = |p: Point| {
        erasers.iter().any(|(eraser, radius)| {
            nearest(&[eraser.as_slice()], p).is_some_and(|(_, d)| d <= *radius)
        })
    };
    let mut pieces = Vec::new();
    let mut piece = Vec::new();
    for &p in path {
        if erased(p) {
            if !piece.is_empty() {
                pieces.push(std::mem::take(&mut piece));
            }
        } else {
            piece.push(p);
        }
    }
    if !piece.is_empty() {
        pieces.push(piece);
    }
    pieces
}

/// Whether anything was laid in `frame`: a paint stroke begun there or a fill there.
pub(crate) fn holds_copy(artwork: &Artwork, frame: &Frame) -> bool {
    artwork.layers().iter().any(|layer| {
        let painted = layer
            .strokes()
            .iter()
            .any(|s| s.style().tool() == StrokeTool::Paint && in_frame(s, frame));
        painted || !laid(layer, frame).is_empty()
    })
}

/// The colours laid in `frame`, in order.
pub(crate) fn colour_copy(artwork: &Artwork, frame: &Frame) -> ColourCopy {
    let picks = artwork
        .layers()
        .iter()
        .flat_map(|layer| laid(layer, frame))
        .collect();
    ColourCopy { picks }
}

/// The colours laid in `frame` on `layer`, in the order they lie there: a fill after the strokes
/// under it.
fn laid(layer: &Layer, frame: &Frame) -> Vec<Rgb8> {
    // Stroke `i` at `2 i + 1`, a fill over `a` strokes at `2 a`: a fill lies after the strokes
    // under it and before the next.
    let mut placed: Vec<(usize, Rgb8)> = layer
        .strokes()
        .iter()
        .enumerate()
        .filter(|(_, s)| s.style().tool() == StrokeTool::Paint && in_frame(s, frame))
        .map(|(i, s)| (2 * i + 1, rgb8(s.style().color())))
        .collect();
    for edit in layer.edits() {
        if let Some(fill) = edit.fill() {
            let [x, y] = fill.mask().origin();
            let [w, h] = fill.mask().size();
            let middle = [
                f64::from(x) + f64::from(w) / 2.0,
                f64::from(y) + f64::from(h) / 2.0,
            ];
            if frame.contains(middle, 0.0) {
                placed.push((2 * fill.after(), rgb8(fill.color())));
            }
        }
    }
    placed.sort_by_key(|(at, _)| *at);
    placed.into_iter().map(|(_, colour)| colour).collect()
}

/// `color` in 8 bits a channel.
pub(crate) fn rgb8(color: Color) -> Rgb8 {
    [color.r(), color.g(), color.b()].map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
}
