//! What an exercise gives beside its frame (S153, `openpaint_practice::beside`: frame units from
//! the frame's top right corner): a gamut mask on its small hue and chroma disc
//! (`card_disc::masked_disc`), and a colour's swatch with its name under it where it has one (a
//! gamut mask's corners, picked whole by the eyedropper).

use egui::{Align2, FontId, Pos2, Shape, Stroke};
use openpaint_practice::{BESIDE_SIDE, FRAME, Frame, Point, Rgb8, Say, beside};

use openpaint_ui::design::{INK, INK_SECONDARY};

use super::card_disc::masked_disc;
use super::mark_lines::At;
use super::marks::colour;
use crate::text::{self as t, language};

/// A name's size and its gap under its swatch, canvas pixels of a lesson's frame (a project's
/// grown with its frame); the least size it shows at, in points.
const NAME_SIZE: f32 = 11.0;
const NAME_GAP: f32 = 4.0;
const LEAST_NAME: f32 = 6.0;

/// A gamut mask's disc of `radius` (frame units) centred `at` beside `frame`.
pub(crate) fn gamut(
    painter: &egui::Painter,
    (frame, at): (&Frame, At<'_>),
    (centre, radius): (Point, f64),
    corners: &[Point],
) {
    let middle = at.of(beside(frame, centre));
    let rim = at.of(beside(frame, [centre[0] + radius, centre[1]]));
    masked_disc(painter, (middle, middle.distance(rim)), corners);
}

/// A swatch of `rgb` centred `centre` beside `frame`, outlined in ink, its name's key `name` read
/// under it; `scale` points to a canvas pixel.
pub(crate) fn swatch(
    painter: &egui::Painter,
    (frame, at, scale): (&Frame, At<'_>, f32),
    (centre, rgb, name): (Point, Rgb8, Option<&'static str>),
) {
    let half = BESIDE_SIDE / 2.0;
    let corners: Vec<Pos2> = [[-half, -half], [half, -half], [half, half], [-half, half]]
        .into_iter()
        .map(|[dx, dy]| at.of(beside(frame, [centre[0] + dx, centre[1] + dy])))
        .collect();
    let bottom = corners[2].y;
    painter.add(Shape::convex_polygon(
        corners,
        colour(rgb),
        Stroke::new(1.0, INK),
    ));
    let Some(key) = name else {
        return;
    };
    let words = t::said(language(painter.ctx()), &Say::plain(key));
    // In step with the frame: a project's frame is four times a lesson's (S154).
    let grown = (frame.unit() / FRAME) as f32;
    let below = Pos2::new(
        at.of(beside(frame, centre)).x,
        bottom + NAME_GAP * scale * grown,
    );
    let font = FontId::proportional((NAME_SIZE * scale * grown).max(LEAST_NAME));
    painter.text(below, Align2::CENTER_TOP, words, font, INK_SECONDARY);
}
