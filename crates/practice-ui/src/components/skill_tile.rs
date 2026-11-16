//! A skill's tile on the path (S122): a small sheet of paper with the skill's picture (its
//! folder's glyph, `Skill::glyph`, in `TILE`'s units: lines, tinted areas, paper, dots and rings,
//! drawn here without knowing the skill), its name and its three dots, each earned one filled. The
//! skill the next lesson practises is ringed in ink; one whose pictures are not kept is drawn in
//! pencil with what it lacks under its dots, and takes no tap. Pressed, the sheet darkens under the
//! hand. The home shows the unit in hand as smaller tiles (`small`). A row lays its tiles alike
//! (`TileRow`, S156): a name too long for its tile wraps under its glyph, the row giving every
//! name as many lines as its longest takes; room under the dots only while one of the row lacks
//! its pictures; a small row pressed to fit its column, its glyph smaller and the room over and
//! under it less, as far as they stay legible. Named by its English (`text.rs`).

use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, TextStyle, Vec2, pos2};
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
/// A tile's height and a small one's with a name of one line, in points.
const TILE_HEIGHT: f32 = GLYPH.y + 15.0 * SPACE;
const SMALL_TILE_HEIGHT: f32 = SMALL_GLYPH.y + 13.0 * SPACE;
/// The room a small tile keeps under its dots for what its pictures lack, in points.
const LACKS_ROOM: f32 = 3.0 * SPACE;
/// Pressed all the way, a small glyph's share left, and the room over and under it.
const PRESSED_GLYPH: f32 = 0.5;
const PRESSED_ROOM: f32 = 0.5;
/// The room over a glyph and between it and the name, in points.
const OVER: f32 = 2.0 * SPACE;
const UNDER: f32 = 1.5 * SPACE;
/// What a small tile gives up pressed all the way, in points.
const PRESS: f32 = (1.0 - PRESSED_GLYPH) * SMALL_GLYPH.y + (1.0 - PRESSED_ROOM) * (OVER + UNDER);
/// A small tile's narrowest: its glyph twice over, so a name of two lines stays readable.
pub(crate) const NARROWEST_SMALL: f32 = 2.0 * SMALL_GLYPH.x;
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
    /// How its row lays it.
    pub row: TileRow,
}

/// How a row lays its tiles alike: the lines it gives a name, the lines it gives what a tile's
/// pictures lack (0 while none of it lacks them), and how far a small row is pressed (0 as
/// drawn, 1 all the way).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileRow {
    pub lines: usize,
    pub lacks: usize,
    pub pressed: f32,
}

impl TileRow {
    /// A row of tiles `width` points wide, each its name and what its pictures lack; not
    /// pressed.
    pub(crate) fn of<'a>(
        ui: &egui::Ui,
        tiles: impl IntoIterator<Item = (Words<'a>, Option<Words<'a>>)>,
        width: f32,
    ) -> Self {
        let rows = |words: Words<'a>, style: TextStyle| {
            let shown = words.shown(ui.ctx()).to_owned();
            let font = style.resolve(ui.style());
            ui.painter()
                .layout(shown, font, INK, wrap(width))
                .rows
                .len()
        };
        let (mut lines, mut lacks) = (1, 0);
        for (name, lacking) in tiles {
            lines = lines.max(rows(name, TextStyle::Body));
            let lacking = lacking.map(|words| rows(words, TextStyle::Small));
            lacks = lacks.max(lacking.unwrap_or(0));
        }
        Self {
            lines,
            lacks,
            pressed: 0.0,
        }
    }

    /// Whether every word of `names` fits a tile `width` points wide, so none breaks inside.
    pub(crate) fn unbroken<'a>(
        ui: &egui::Ui,
        names: impl IntoIterator<Item = Words<'a>>,
        width: f32,
    ) -> bool {
        let body = TextStyle::Body.resolve(ui.style());
        let fits = |word: &str| {
            let galley = ui
                .painter()
                .layout_no_wrap(word.to_owned(), body.clone(), INK);
            galley.size().x <= wrap(width)
        };
        (names.into_iter()).all(|name| name.shown(ui.ctx()).split_whitespace().all(fits))
    }

    /// Its small tiles pressed as little as lets `rows` of them, `apart` points apart, fit in
    /// `room` points, at most all the way.
    pub(crate) fn fitted(self, ui: &egui::Ui, (rows, apart): (usize, f32), room: f32) -> Self {
        let rows = rows.max(1) as f32;
        let need = rows * self.height(ui, true) + (rows - 1.0) * apart;
        // A point to spare, so the last row stays clear of the edge it was fitted to.
        let pressed = ((need - room + 1.0) / (rows * PRESS)).clamp(0.0, 1.0);
        Self { pressed, ..self }
    }

    /// Its tiles' height, small or not.
    pub(crate) fn height(&self, ui: &egui::Ui, small: bool) -> f32 {
        let one = if small {
            let room = if self.lacks > 0 { 0.0 } else { LACKS_ROOM };
            SMALL_TILE_HEIGHT - room - self.pressed * PRESS
        } else {
            TILE_HEIGHT
        };
        let lacks = more(ui, self.lacks, TextStyle::Small);
        one + more(ui, self.lines, TextStyle::Body) + lacks
    }
}

/// A tile `width` points wide; its response.
pub fn skill_tile(ui: &mut egui::Ui, tile: &SkillTile<'_>, width: f32) -> egui::Response {
    let sense = if tile.lacks.is_none() {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (glyph_size, gap, pressed) = if tile.small {
        (SMALL_GLYPH, 4.0 * SPACE, tile.row.pressed)
    } else {
        (GLYPH, 5.0 * SPACE, 0.0)
    };
    let height = tile.row.height(ui, tile.small);
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
    let room = 1.0 - (1.0 - PRESSED_ROOM) * pressed;
    let glyph_size = glyph_size * (1.0 - (1.0 - PRESSED_GLYPH) * pressed);
    let left = rect.left() + 2.0 * SPACE;
    let glyph = Rect::from_min_size(pos2(left, rect.top() + OVER * room), glyph_size);
    paint_glyph(painter, glyph, tile.skill, ink);
    let body = TextStyle::Body.resolve(ui.style());
    let name = tile.name.shown(ui.ctx()).to_owned();
    let below = pos2(left, glyph.bottom() + UNDER * room);
    painter.galley(below, painter.layout(name, body, ink, wrap(width)), ink);
    let dots = pos2(
        left + DOT,
        below.y + gap + more(ui, tile.row.lines, TextStyle::Body),
    );
    paint_dots(painter, (dots, DOT_PITCH, DOT), (tile.dots, None), ink);
    if let Some(lacks) = tile.lacks {
        let small = TextStyle::Small.resolve(ui.style());
        let under = pos2(left, dots.y + 2.0 * SPACE);
        let shown = lacks.shown(ui.ctx()).to_owned();
        painter.galley(
            under,
            painter.layout(shown, small, quiet, wrap(width)),
            quiet,
        );
    }
    response
}

/// The height the lines past the first of words in `style` take.
fn more(ui: &egui::Ui, lines: usize, style: TextStyle) -> f32 {
    lines.saturating_sub(1) as f32 * ui.text_style_height(&style)
}

/// The width a name wraps at in a tile `width` points wide: in from its left as the glyph, a
/// space from its right.
fn wrap(width: f32) -> f32 {
    width - 3.0 * SPACE
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
