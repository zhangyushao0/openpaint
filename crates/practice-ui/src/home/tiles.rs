//! The unit in hand's skills on the home's column (S122, S156): every one of them as a small tile
//! with its dots, four a row where the column leaves each the room for its glyph and every word of
//! its name (its type smaller, to the UI's Small, before a word would break, S158h), else three;
//! more than a row holds wraps into rows as even as they come (six as two of three). The sheet
//! grows to hold them (`column.rs`); only where the column itself is too short are they pressed
//! (`TileRow::fitted`: a smaller glyph, then none), every skill shown in every state of the home
//! on the tablet and the tests' screen (S158h). A tap starts that skill's lesson, as on the path;
//! a skill lacking its pictures is greyed, no tap.

use egui::{Rect, Vec2};
use openpaint_practice::Skill;
use openpaint_practice_app::{PathProjection, SkillShown};
use openpaint_ui::design::SPACE;

use super::column::rule;
use crate::components::{NARROWEST_SMALL, SkillTile, TileRow, skill_tile};
use crate::text::{self as t, Words};

/// The most tiles a row.
const MOST: usize = 4;
/// Between tiles, in points.
const GAP: f32 = 2.0 * SPACE;

/// The unit in hand's skills as small tiles under a rule, every row ending above `foot` (a row
/// that would pass it even pressed all the way left out, on a screen far shorter than the
/// tablet's); the skill tapped and where.
pub(super) fn unit_tiles(
    ui: &mut egui::Ui,
    path: &PathProjection,
    foot: f32,
) -> Option<(Skill, Rect)> {
    let unit = path.units.iter().find(|unit| unit.number == path.unit)?;
    ui.add_space(SPACE);
    rule(ui);
    ui.add_space(SPACE);
    let names = || (unit.skills.iter()).map(|shown| t::skill_name(shown.skill).into());
    let fits = |width| TileRow::name_size(ui, names(), width).is_some();
    let (across, width) = grid(ui.available_width(), unit.skills.len(), fits);
    // Pressed only as far as every row fits above the foot.
    let rows = (
        unit.skills.len().div_ceil(across),
        ui.spacing().item_spacing.y,
    );
    let words = |shown: &SkillShown| (t::skill_name(shown.skill).into(), lacks(shown));
    let laid = TileRow::of(ui, unit.skills.iter().map(words), width);
    let laid = laid.fitted(ui, rows, foot - ui.cursor().top());
    let height = laid.height(ui, true);
    let next = path
        .next
        .map(|offer| offer.skill)
        .filter(|_| path.open.is_none());
    let mut tapped = None;
    for row in unit.skills.chunks(across) {
        if ui.cursor().top() + height > foot {
            break;
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(GAP);
            for shown in row {
                let tile = SkillTile {
                    skill: shown.skill,
                    name: t::skill_name(shown.skill).into(),
                    dots: shown.dots,
                    lacks: lacks(shown),
                    next: next == Some(shown.skill),
                    small: true,
                    row: laid,
                };
                let response = skill_tile(ui, &tile, width);
                if response.clicked() {
                    tapped = Some((shown.skill, response.rect));
                }
            }
        });
    }
    tapped
}

/// What `shown`'s pictures lack, said under its dots.
fn lacks(shown: &SkillShown) -> Option<Words<'static>> {
    shown.lacks.map(|needs| Words::from(t::needs(needs)))
}

/// How many of `count` tiles a row across `width` points and how wide each: four a row where each
/// keeps `NARROWEST_SMALL` and its name's every word at some size (`fits`), else three; the rows
/// as even as they come; a tile never wider than three a row's.
fn grid(width: f32, count: usize, fits: impl Fn(f32) -> bool) -> (usize, f32) {
    let wide = |across: usize| (width - GAP * (across - 1) as f32) / across as f32;
    let four = wide(MOST);
    let most = if four >= NARROWEST_SMALL && fits(four) {
        MOST
    } else {
        MOST - 1
    };
    let rows = count.div_ceil(most).max(1);
    let across = count.div_ceil(rows).max(1);
    (across, wide(across.max(MOST - 1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_unit_s_tiles_go_four_a_row_where_they_fit_and_wrap_into_even_rows() {
        let fit = |_: f32| true;
        // The column's sheet on the tablet in landscape: 536 points inside.
        assert_eq!(grid(536.0, 3, fit), (3, 168.0));
        assert_eq!(grid(536.0, 4, fit), (4, 122.0));
        assert_eq!(grid(536.0, 6, fit), (3, 168.0));
        assert_eq!(grid(536.0, 5, fit), (3, 168.0));
        assert_eq!(grid(536.0, 1, fit), (1, 168.0));
        // A word too long for a tile four a row would break: two rows of two.
        assert_eq!(grid(536.0, 4, |width| width > 130.0), (2, 168.0));
        // In portrait (368 inside) four is too narrow: two rows of two, three a row's width.
        assert_eq!(grid(368.0, 4, fit), (2, 112.0));
        assert_eq!(grid(368.0, 6, fit), (3, 112.0));
    }
}
