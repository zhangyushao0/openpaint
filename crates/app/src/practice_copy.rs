//! The copy on a practice sheet: what the pen laid in an exercise's empty frame, read from the
//! artwork's records. StreamLine and QuickShape are off on a practice sheet, so a stroke's
//! samples are the pen's own: where it went, how hard and when (kept for a later study).
//!
//! A stroke belongs to the frame its first sample lies in (grown by `MARGIN` of its side, where
//! strokes may begin and are clipped to the frame). A line's copy, which the aligned overlay is
//! fitted to, is its paint strokes but the sketch pen's construction lines, oldest first, less
//! what a later eraser stroke passed over within its radius (the brush's size, the pressure not
//! counted). A colour's copy is the colours laid: each paint stroke's colour and each fill's whose
//! region lies in the frame, in the order they lie on their layer, layers bottom to top.

use openpaint_core::{Artwork, Color, Layer, Stroke, StrokeTool};
use openpaint_practice::{ColourCopy, Frame, Point, Rgb8, nearest};

use crate::practice_pens::PracticePens;

/// Strokes may begin this share of the frame's side outside it.
pub(crate) const MARGIN: f64 = 0.1;

fn first_point(stroke: &Stroke) -> Option<Point> {
    let first = stroke.samples().first()?.position();
    Some([f64::from(first.x()), f64::from(first.y())])
}

/// Whether `stroke` began in `frame`.
fn in_frame(stroke: &Stroke, frame: &Frame) -> bool {
    first_point(stroke).is_some_and(|p| frame.contains(p, MARGIN * frame.side))
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

/// The line copied in `frame` in pieces, in frame units: what the aligned overlay fits.
pub(crate) fn line_pieces(
    artwork: &Artwork,
    frame: &Frame,
    pens: &PracticePens,
) -> Vec<Vec<Point>> {
    let strokes = strokes_in(artwork, frame);
    let mut pieces = Vec::new();
    for (index, stroke) in strokes.iter().enumerate() {
        let style = stroke.style();
        if style.tool() != StrokeTool::Paint || pens.is_sketch(style) {
            continue;
        }
        let erasers: Vec<(Vec<Point>, f64)> = strokes[index + 1..]
            .iter()
            .filter(|later| later.style().tool() == StrokeTool::Erase)
            .map(|later| (positions(later), f64::from(later.style().size()) / 2.0))
            .collect();
        for piece in kept_pieces(&positions(stroke), &erasers) {
            pieces.push(piece.iter().map(|p| frame.to_frame(*p)).collect());
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
