//! An end asks the learner's own eye (S147, 比一比; a lesson's, or a Pomodoro's block's, S148):
//! under a rule Which is better (哪张更好) over two earlier copies side by side, the same size, no
//! date, no mark, and under them Left better, Same, Right better (左边好 · 差不多 · 右边好).
//! Answered, the buttons go: each card's date under it, the newer's in ink and the older's
//! quieter, the one chosen outlined; no sentence. A unit's end re-test asks one pair a skill in
//! turn, its skill named over it, each revealed with its result's mark before the date once
//! answered and drawn half as wide, two a row, the next asked under them. And the extra drill's
//! row (加练) when a lesson's end offers one.

use std::sync::Arc;

use egui::{Align, Align2, Layout, Pos2, Rect, Sense, TextStyle, UiBuilder, Vec2};
use openpaint_practice::{Answer, CopyRef};
use openpaint_practice_app::{CopyCard, End, PairShown};
use openpaint_ui::components::text_button;
use openpaint_ui::design::{INK, INK_SECONDARY, SPACE, TOUCH_TARGET};

use super::end::{MARK, rule};
use crate::components::{paint_copy, paint_result};
use crate::text::{self as t, Words, language};

/// A card's height over its width (the mock's 120 by 140), the gap between the two, and the
/// line its date takes, in points.
const TALL: f32 = 120.0 / 140.0;
const GAP: f32 = 14.0;
const DATE: f32 = 30.0;

/// How a pair is drawn: a lesson's or a block's only pair (its dates alone); a re-test's across
/// the card (each date after its result's mark); a re-test's answered, half as wide (the date in
/// small type under the mark, where a long date still fits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drawn {
    Single,
    Marked,
    Small,
}

/// An end's `pairs`, their copies' cards as read so far; the pair answered and its answer.
pub(super) fn pairs(
    ui: &mut egui::Ui,
    pairs: &[PairShown],
    cards: &[Arc<CopyCard>],
) -> Option<(u8, Answer)> {
    if pairs.is_empty() {
        return None;
    }
    rule(ui);
    if let [pair] = pairs {
        ui.label(egui::RichText::new(Words::from(t::WHICH_BETTER).shown(ui.ctx())).size(18.0));
        return one(ui, pair, cards, Drawn::Single).map(|answer| (0, answer));
    }
    // In turn: those answered half as wide, two a row, each under its skill, so that a unit's six
    // leave One more on the screen; the next asked under them across the card.
    let answered = pairs
        .iter()
        .take_while(|pair| pair.answer.is_some())
        .count();
    for row in pairs[..answered].chunks(2) {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0 * GAP;
            ui.columns(2, |columns| {
                for (column, pair) in columns.iter_mut().zip(row) {
                    named(column, pair);
                    one(column, pair, cards, Drawn::Small);
                }
            });
        });
    }
    let asked = pairs.get(answered)?;
    named(ui, asked);
    let answer = one(ui, asked, cards, Drawn::Marked)?;
    Some((u8::try_from(answered).unwrap_or(u8::MAX), answer))
}

/// `pair`'s skill over it (a re-test's).
fn named(ui: &mut egui::Ui, pair: &PairShown) {
    let name = pair.skill.map_or(t::RETEST, t::skill_name);
    ui.label(Words::from(name).shown(ui.ctx()));
}

/// `pair`'s two cards and its buttons or, answered, its dates (with its marks on a re-test's);
/// the answer tapped.
fn one(
    ui: &mut egui::Ui,
    pair: &PairShown,
    cards: &[Arc<CopyCard>],
    drawn: Drawn,
) -> Option<Answer> {
    let width = (ui.available_width() - GAP) / 2.0;
    let below = match (pair.answer, drawn) {
        (None, _) => 0.0,
        (Some(_), Drawn::Small) => DATE + 2.0 * MARK,
        (Some(_), Drawn::Single | Drawn::Marked) => DATE,
    };
    let size = Vec2::new(ui.available_width(), width * TALL + below);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let card = |copy: CopyRef| cards.iter().find(|card| card.copy == copy).map(|c| &**c);
    let chosen = match pair.answer {
        Some(Answer::Left) => Some(0),
        Some(Answer::Right) => Some(1),
        Some(Answer::Alike) | None => None,
    };
    for (side, copy) in pair.sides().into_iter().enumerate() {
        let left = rect.left() + side as f32 * (width + GAP);
        let frame =
            Rect::from_min_size(Pos2::new(left, rect.top()), Vec2::new(width, width * TALL));
        let shown = card(copy);
        paint_copy(ui, frame, shown, chosen == Some(side));
        if let Some(shown) = shown.filter(|_| pair.answer.is_some()) {
            dated(ui, frame, shown, (copy == pair.asked.newer, drawn));
        }
    }
    if pair.answer.is_some() {
        return None;
    }
    buttons(ui)
}

/// Under `frame`: its copy's result's mark on a re-test's and its date, the newer's in ink and
/// the older's quieter; in a small pair the date in small type under the mark.
fn dated(ui: &egui::Ui, frame: Rect, shown: &CopyCard, (newer, drawn): (bool, Drawn)) {
    let mut at = Pos2::new(frame.left(), frame.bottom() + DATE / 2.0);
    if let Some(outcome) = shown.result.filter(|_| drawn != Drawn::Single) {
        let centre = at + Vec2::new(MARK, 0.0);
        paint_result(ui.painter(), (centre, MARK), outcome, INK);
        match drawn {
            Drawn::Small => at.y += 2.0 * MARK + SPACE / 2.0,
            Drawn::Single | Drawn::Marked => at.x += 2.0 * MARK + SPACE,
        }
    }
    let date = t::copy_date(language(ui.ctx()), shown.day);
    let ink = if newer { INK } else { INK_SECONDARY };
    let style = if drawn == Drawn::Small {
        TextStyle::Small
    } else {
        TextStyle::Body
    };
    let font = style.resolve(ui.style());
    ui.painter().text(at, Align2::LEFT_CENTER, date, font, ink);
}

/// Left better, Same, Right better across the card, under the left card, between the two and
/// under the right one; the one tapped.
fn buttons(ui: &mut egui::Ui) -> Option<Answer> {
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (row, _) = ui.allocate_exact_size(size, Sense::hover());
    let third = row.width() / 3.0;
    let words = [
        (t::LEFT_BETTER, Answer::Left, Align::Min),
        (t::ALIKE, Answer::Alike, Align::Center),
        (t::RIGHT_BETTER, Answer::Right, Align::Max),
    ];
    let mut tapped = None;
    for (place, (text, answer, align)) in words.into_iter().enumerate() {
        let min = row.min + Vec2::new(place as f32 * third, 0.0);
        let part = Rect::from_min_size(min, Vec2::new(third, row.height()));
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt(("pair answer", place))
                .max_rect(part)
                .layout(Layout::left_to_right(Align::Center).with_main_align(align)),
        );
        if text_button(&mut child, text, true).clicked() {
            tapped = Some(answer);
        }
    }
    tapped
}

/// The extra drill's row when the end offers one: Extra drill quietly, then its skill and
/// exercises with Start beside them; whether Start was tapped.
pub(super) fn drill(ui: &mut egui::Ui, end: &End) -> bool {
    let Some(offer) = &end.drill else {
        return false;
    };
    rule(ui);
    let label = Words::from(t::EXTRA_DRILL).shown(ui.ctx());
    ui.label(egui::RichText::new(label).small().color(INK_SECONDARY));
    let language = language(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t::drill_line(language, offer)).size(18.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            text_button(ui, t::START, true).clicked()
        })
        .inner
    })
    .inner
}
