//! A kind's tile (S91): a small framed picture of an exercise kind with its name under it, for
//! the practice options' drills. The picture is its kind's (`Family::tile`, made in its kind's
//! folder, S109b): a line drawing of a fixed example in `TILE`'s units, lines, tinted areas, paper,
//! dots and rings, drawn here without knowing the kind. A kind compared at least once is drawn and
//! named in ink, one never compared in pencil, as the road draws what is done and what lies ahead;
//! a drill left unfinished is ringed in ink, as Next is on the road. No ticks, counts or words
//! beyond the name. Pressed, a faint wash fills round the frame. Named by its English
//! (`text.rs`).

use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, TextStyle, Vec2, pos2};
use openpaint_practice::{Family, TileMark};

use crate::components::controls;
use crate::design::{INK, Outline, PANEL, PENCIL, SPACE, ink_line, paint_pressed};
use crate::text::Words;

/// A tile's frame, in points; its picture is drawn in as many units (`openpaint_practice::TILE`).
pub const TILE: Vec2 = Vec2::new(openpaint_practice::TILE[0], openpaint_practice::TILE[1]);
/// The weights of the frame, the drawing and the ring round an unfinished drill, in points.
const FRAME_LINE: f32 = 1.0;
const DRAWING_LINE: f32 = 1.25;
const RING_LINE: f32 = 1.5;
/// How far the ring keeps off the frame, in points: a ring round it, never a second frame.
const RING_GAP: f32 = 6.0;
/// A dot's radius, in the picture's units.
const DOT: f32 = 1.8;

/// What a tile shows.
#[derive(Debug, Clone, Copy)]
pub struct KindTile<'a> {
    pub family: Family,
    /// Its name, under the frame.
    pub name: Words<'a>,
    /// Compared at least once: ink, else pencil.
    pub inked: bool,
    /// A drill of it left unfinished: ringed.
    pub ringed: bool,
}

/// A tile `width` points wide (its frame centred, its name wrapped under it); its response.
pub fn kind_tile(ui: &mut egui::Ui, tile: &KindTile<'_>, width: f32) -> egui::Response {
    let colour = if tile.inked { INK } else { PENCIL };
    let font = TextStyle::Small.resolve(ui.style());
    let shown = tile.name.shown(ui.ctx()).to_owned();
    // Broken between words only: a word longer than the cell may reach past it a little.
    let widest = (shown.split(' '))
        .map(|word| {
            let galley = ui
                .painter()
                .layout_no_wrap(word.to_owned(), font.clone(), colour);
            galley.size().x
        })
        .fold(0.0, f32::max);
    let wrap = (width - SPACE / 2.0).max(widest + 1.0);
    let galley = ui.painter().layout(shown, font, colour, wrap);
    let height = TILE.y + 2.0 * SPACE + galley.size().y;
    let (cell, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    controls::register(ui, tile.name.id(), cell);
    if !ui.is_rect_visible(cell) {
        return response;
    }
    let top = pos2(cell.center().x - TILE.x / 2.0, cell.top() + SPACE / 2.0);
    let frame = Rect::from_min_size(top, TILE);
    let painter = ui.painter();
    if response.is_pointer_button_down_on() {
        let wash = Outline {
            rect: frame.expand(SPACE / 2.0),
            corner: SPACE / 2.0,
        };
        paint_pressed(painter, wash);
    }
    let frame_line = Stroke::new(FRAME_LINE, colour);
    painter.rect_stroke(frame, 2.0, frame_line, egui::StrokeKind::Inside);
    paint_kind(painter, frame, tile.family, colour);
    if tile.ringed {
        let ring = Stroke::new(RING_LINE, INK);
        painter.rect_stroke(
            frame.expand(RING_GAP),
            2.0 * RING_GAP,
            ring,
            egui::StrokeKind::Middle,
        );
    }
    let name_top = pos2(
        cell.center().x - galley.size().x / 2.0,
        frame.bottom() + SPACE,
    );
    painter.galley(name_top, galley, colour);
    response
}

/// The picture of `family` in `frame` in `colour`.
pub fn paint_kind(painter: &egui::Painter, frame: Rect, family: Family, colour: Color32) {
    let scale = frame.width() / TILE.x;
    let at = |[x, y]: [f32; 2]| frame.min + Vec2::new(x, y) * scale;
    let line = Stroke::new(DRAWING_LINE, colour);
    for mark in family.tile() {
        match mark {
            TileMark::Line(points) => {
                let points: Vec<Pos2> = points.into_iter().map(at).collect();
                ink_line(painter, &points, line);
            }
            TileMark::Tint(corners, share) => {
                let points = corners.into_iter().map(at).collect();
                let fill = colour.gamma_multiply(share);
                painter.add(Shape::convex_polygon(points, fill, Stroke::NONE));
            }
            TileMark::Paper(corners) => {
                let points = corners.into_iter().map(at).collect();
                painter.add(Shape::convex_polygon(points, PANEL, Stroke::NONE));
            }
            TileMark::Dot(centre) => {
                painter.circle_filled(at(centre), DOT * scale, colour);
            }
            TileMark::Ring(centre) => {
                painter.circle_stroke(at(centre), 1.3 * DOT * scale, line);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_s_picture_has_a_line_and_lies_inside_its_frame() {
        let inside = |[x, y]: [f32; 2]| {
            (2.0..=TILE.x - 2.0).contains(&x) && (2.0..=TILE.y - 2.0).contains(&y)
        };
        for family in Family::ALL {
            let marks = family.tile();
            assert!(
                marks.iter().any(|m| matches!(m, TileMark::Line(_))),
                "{family:?}"
            );
            for mark in &marks {
                let points = match mark {
                    TileMark::Line(points)
                    | TileMark::Tint(points, _)
                    | TileMark::Paper(points) => points.clone(),
                    TileMark::Dot(at) | TileMark::Ring(at) => vec![*at],
                };
                assert!(points.iter().all(|p| inside(*p)), "{family:?}: {mark:?}");
            }
        }
        // The tricks differ only by their marks on the same shaded box.
        let tricks = [
            Family::Centre,
            Family::Halves,
            Family::Thirds,
            Family::Step,
            Family::Mirror,
        ];
        let pictures: Vec<Vec<TileMark>> = tricks.iter().map(|f| f.tile()).collect();
        for (a, first) in pictures.iter().enumerate() {
            for second in &pictures[a + 1..] {
                assert_ne!(first, second);
            }
        }
    }
}
