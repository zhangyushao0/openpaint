//! A lesson's end on its card (S122), the rows it used in view over the sheet, each exercise's
//! result a small mark under its pair (`rows.rs`): the lesson's name, its exercises and the time
//! they took ("5 exercises · 3 min 12 s"); their results in a row of marks (none on the first
//! lesson, which scores nothing); under a rule the skill's three dots, the one just earned
//! inking in with the level it reached named ("Up to “Fluent”"), each seal the lesson earned
//! stamping down beside them; under another rule One more in its pill with what it will be (the
//! skill, its exercises and minutes) and That's all for today, which goes home. No counts of days,
//! no quotas. While a Pomodoro's block runs a lesson's end shows only when the lesson chained after
//! it was rolled back (S124, `bar.rs`); a block's end
//! (`block_end.rs`) shares its marks and its rule.

use egui::{Pos2, Sense, Vec2};
use openpaint_practice::{Outcome, SkillLevel};
use openpaint_practice_app::End;

use openpaint_ui::components::{animate_from, card_text, keep_moving, pill_button, text_button};
use openpaint_ui::design::motion::OPEN;
use openpaint_ui::design::{DIVIDER, INK, SPACE};

use crate::components::{DOTS, paint_dots, paint_result, paint_seal, seal_turn};
use crate::palette::PENCIL;
use crate::text::{self as t, Text, Words, language};

/// A result's mark's radius and pitch, a dot's radius and pitch, and a new seal's side, points.
const MARK: f32 = 7.0;
const MARK_PITCH: f32 = 4.0 * SPACE;
pub(super) const DOT: f32 = 7.0;
pub(super) const DOT_PITCH: f32 = 3.0 * SPACE;
const SEAL: f32 = 64.0;
/// A result not judged: a pencil ring this heavy, in points.
const RING_LINE: f32 = 1.25;
/// Seconds after the end first shows that the new dot inks in (the card has grown by then) and
/// that a new seal stamps down.
const INK_AFTER: f64 = 0.4;
const STAMP_AFTER: f64 = 0.75;

/// What the end's card asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndTap {
    OneMore,
    Enough,
}

/// The end of the lesson called `name`, `end` as the flow gives it; what was tapped.
pub(crate) fn end_card(ui: &mut egui::Ui, end: &End, name: Text) -> Option<EndTap> {
    let language = language(ui.ctx());
    ui.heading(Words::from(name).shown(ui.ctx()));
    let took = t::lesson_took(language, (end.exercises, end.seconds));
    card_text(ui, Words::made(&took), true);
    // The first lesson scores nothing: no row of marks.
    if !end.results.is_empty() {
        let outcomes: Vec<Option<Outcome>> = end.results.iter().map(|(_, o)| *o).collect();
        result_marks(ui, &outcomes);
    }
    // The first lesson earns no dot; a lesson without a dot or a seal shows none of them.
    if end.skill.is_some() || !end.sealed.is_empty() {
        rule(ui);
        progress(ui, end);
    }
    rule(ui);
    let mut tapped = None;
    if let Some(next) = &end.next {
        if pill_button(ui, t::ONE_MORE, true).clicked() {
            tapped = Some(EndTap::OneMore);
        }
        card_text(ui, Words::made(&t::one_more_line(language, next)), true);
    }
    if text_button(ui, t::ENOUGH, true).clicked() {
        tapped = Some(EndTap::Enough);
    }
    tapped
}

/// Each exercise's result as a mark, in rows as wide as the card (a block's twenty take two); one
/// not judged a pencil ring.
pub(super) fn result_marks(ui: &mut egui::Ui, outcomes: &[Option<Outcome>]) {
    let line = 2.0 * MARK + SPACE;
    let width = ui.available_width();
    let across = (((width - 2.0 * MARK) / MARK_PITCH).floor() as usize + 1).max(1);
    let rows = outcomes.len().div_ceil(across);
    let (block, _) = ui.allocate_exact_size(Vec2::new(width, line * rows as f32), Sense::hover());
    for (place, outcome) in outcomes.iter().enumerate() {
        let (row, column) = (place / across, place % across);
        let centre = Pos2::new(
            block.left() + MARK + MARK_PITCH * column as f32,
            block.top() + line * (row as f32 + 0.5),
        );
        match outcome {
            Some(outcome) => paint_result(ui.painter(), (centre, MARK), *outcome, INK),
            None => {
                let ring = egui::Stroke::new(RING_LINE, PENCIL);
                ui.painter().circle_stroke(centre, MARK - 1.0, ring);
            }
        }
    }
}

/// The skill's dots (the one just earned inking in, the level reached named) and the seals the
/// lesson earned, stamping down beside them.
fn progress(ui: &mut egui::Ui, end: &End) {
    let ctx = ui.ctx().clone();
    let id = egui::Id::new(("lesson end", end.results.first().map(|(index, _)| *index)));
    // Each motion waits until the card has grown and the one before it is under way.
    let now = ctx.input(|input| input.time);
    let first = ctx.data_mut(|data| *data.get_temp_mut_or_insert_with(id.with("seen"), || now));
    let after = |wait: f64| {
        if now - first < wait {
            keep_moving(&ctx);
            0.0
        } else {
            1.0
        }
    };
    let language = language(&ctx);
    let reached = (end.skill)
        .filter(|(_, before, dots)| (*dots).min(DOTS) > *before)
        .and_then(|(_, _, dots)| level_of(dots.min(DOTS)));
    // The band holds the dots, the level reached above them and the seals beside: no taller than
    // what it holds, so dots alone leave no empty line above them.
    let height = if reached.is_none() && end.sealed.is_empty() {
        2.0 * DOT + 2.0 * SPACE
    } else {
        SEAL.max(2.0 * DOT + 4.0 * SPACE)
    };
    let (band, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    if let Some((_, before, after_lesson)) = end.skill {
        let earned = after_lesson.min(DOTS);
        let fresh = (earned > before)
            .then(|| animate_from(&ctx, id.with("dot"), 0.0, after(INK_AFTER), OPEN));
        let dots = Pos2::new(band.left() + DOT, band.bottom() - DOT - SPACE);
        paint_dots(ui.painter(), (dots, DOT_PITCH, DOT), (earned, fresh), INK);
        if let Some(reached) = reached {
            let words = t::level_reached(language, reached);
            let font = egui::TextStyle::Body.resolve(ui.style());
            let at = Pos2::new(band.left(), band.top() + SPACE);
            ui.painter()
                .text(at, egui::Align2::LEFT_TOP, words, font, INK);
        }
    }
    for (place, seal) in end.sealed.iter().enumerate() {
        let wait = STAMP_AFTER + 0.3 * place as f64;
        let stamped = animate_from(&ctx, id.with(place), 0.0, after(wait), OPEN);
        let side = SEAL * (1.4 - 0.4 * stamped);
        let centre = Pos2::new(
            band.right() - SEAL / 2.0 - (SEAL + SPACE) * place as f32,
            band.center().y,
        );
        let lines = t::seal_lines(language, *seal);
        paint_seal(
            ui.painter(),
            (centre, side),
            (&lines, seal_turn(place + 1)),
            stamped,
        );
    }
}

/// The level a skill reaches with its `dots`th dot: the first with one, the third with three.
pub(super) fn level_of(dots: u8) -> Option<SkillLevel> {
    SkillLevel::ALL
        .get(usize::from(dots).checked_sub(1)?)
        .copied()
}

/// A hairline across the card.
pub(super) fn rule(ui: &mut egui::Ui) {
    let (rule, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), SPACE), Sense::hover());
    ui.painter()
        .hline(rule.x_range(), rule.center().y, (1.0, DIVIDER));
}
