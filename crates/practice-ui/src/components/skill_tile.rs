//! A skill's tile on the path (S122): a small sheet of paper with the skill's picture (its
//! folder's glyph, `Skill::glyph`, in `TILE`'s units: lines, tinted areas, paper, dots and rings,
//! drawn here without knowing the skill), its name and its three dots, each earned one filled. The
//! skill the next lesson practises is ringed in ink; one whose pictures are not kept is drawn in
//! pencil with what it lacks under its dots, and takes no tap. Pressed, the sheet darkens under the
//! hand. The home shows the unit in hand as smaller tiles (`small`). Named by its English
//! (`text.rs`).

use egui::{Align2, Color32, Pos2, Rect, Sense, Shape, Stroke, TextStyle, Vec2, pos2};
use openpaint_practice::{Skill, TileMark};

use openpaint_ui::components::{controls, paint_sheet};
use openpaint_ui::design::{INK, INK_SECONDARY, Outline, PANEL, SPACE, ink_line, paint_mark};

use super::marks::paint_dots;
use crate::palette::PENCIL;
use crate::text::Words;

/// A glyph's frame, in points: `TILE` drawn half as large again; a small tile's at `TILE`.
const GLYPH: Vec2 = Vec2::new(
    1.5 * openpaint_practice::TILE[0],
    1.5 * openpaint_practice::TILE[1],
);
const SMALL_GLYPH: Vec2 = Vec2::new(openpaint_practice::TILE[0], openpaint_practice::TILE[1]);
/// A tile's height and a small one's, in points.
const TILE_HEIGHT: f32 = GLYPH.y + 15.0 * SPACE;
pub(crate) const SMALL_TILE_HEIGHT: f32 = SMALL_GLYPH.y + 13.0 * SPACE;
/// The drawing's line, a dot's radius in the glyph's units, and the dots' radius and pitch, in
/// points.
const DRAWING_LINE: f32 = 1.75;
const GLYPH_DOT: f32 = 1.8;
const DOT: f32 = 5.0;
const DOT_PITCH: f32 = 20.0;

/// What a tile shows.
#[derive(Debug, Clone, Copy)]
pub struct SkillTile<'a> {
    pub skill: Skill,
    pub name: Words<'a>,
    /// Its dots, 0 to 3.
    pub dots: u8,
    /// What its pictures lack, said under its dots: greyed, it takes no tap.
    pub lacks: Option<Words<'a>>,
    /// The next lesson practises it: ringed.
    pub next: bool,
    /// The home's smaller tile, its drawing at `TILE` and its dots closer under its name.
    pub small: bool,
}

/// A tile `width` points wide; its response.
pub fn skill_tile(ui: &mut egui::Ui, tile: &SkillTile<'_>, width: f32) -> egui::Response {
    let sense = if tile.lacks.is_none() {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (glyph_size, height, gap) = if tile.small {
        (SMALL_GLYPH, SMALL_TILE_HEIGHT, 4.0 * SPACE)
    } else {
        (GLYPH, TILE_HEIGHT, 5.0 * SPACE)
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), sense);
    controls::register(ui, tile.name.id(), rect);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    let tint = if response.is_pointer_button_down_on() {
        Color32::from_gray(225)
    } else {
        Color32::WHITE
    };
    paint_sheet(painter, None, rect, tint);
    if tile.next {
        let ring = Outline {
            rect: rect.expand(SPACE / 2.0),
            corner: SPACE / 2.0,
        };
        paint_mark(painter, ring, 1.0);
    }
    let (ink, quiet) = if tile.lacks.is_some() {
        (PENCIL, PENCIL)
    } else {
        (INK, INK_SECONDARY)
    };
    let left = rect.left() + 2.0 * SPACE;
    let glyph = Rect::from_min_size(pos2(left, rect.top() + 2.0 * SPACE), glyph_size);
    paint_glyph(painter, glyph, tile.skill, ink);
    let body = TextStyle::Body.resolve(ui.style());
    let name = tile.name.shown(ui.ctx());
    let below = pos2(left, glyph.bottom() + 1.5 * SPACE);
    painter.text(below, Align2::LEFT_TOP, name, body, ink);
    let dots = pos2(left + DOT, below.y + gap);
    paint_dots(painter, (dots, DOT_PITCH, DOT), (tile.dots, None), ink);
    if let Some(lacks) = tile.lacks {
        let small = TextStyle::Small.resolve(ui.style());
        let under = pos2(left, dots.y + 2.0 * SPACE);
        painter.text(under, Align2::LEFT_TOP, lacks.shown(ui.ctx()), small, quiet);
    }
    response
}

/// The glyph of `skill` in `frame` in `colour`.
pub fn paint_glyph(painter: &egui::Painter, frame: Rect, skill: Skill, colour: Color32) {
    let scale = frame.width() / openpaint_practice::TILE[0];
    let at = |[x, y]: [f32; 2]| frame.min + Vec2::new(x, y) * scale;
    let line = Stroke::new(DRAWING_LINE, colour);
    for mark in skill.glyph() {
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
                painter.circle_filled(at(centre), GLYPH_DOT * scale, colour);
            }
            TileMark::Ring(centre) => {
                painter.circle_stroke(at(centre), 1.3 * GLYPH_DOT * scale, line);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_skill_s_glyph_has_a_line_and_lies_inside_its_frame() {
        let [width, height] = openpaint_practice::TILE;
        let inside = |[x, y]: [f32; 2]| (0.0..=width).contains(&x) && (0.0..=height).contains(&y);
        for skill in Skill::all() {
            let marks = skill.glyph();
            assert!(
                marks.iter().any(|m| matches!(m, TileMark::Line(_))),
                "{skill:?}"
            );
            for mark in &marks {
                let points = match mark {
                    TileMark::Line(points)
                    | TileMark::Tint(points, _)
                    | TileMark::Paper(points) => points.clone(),
                    TileMark::Dot(at) | TileMark::Ring(at) => vec![*at],
                };
                assert!(points.iter().all(|p| inside(*p)), "{skill:?}: {mark:?}");
            }
        }
    }
}
