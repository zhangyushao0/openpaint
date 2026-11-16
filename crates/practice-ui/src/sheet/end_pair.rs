//! A lesson's end asks the learner's own eye (S147, 比一比): under a rule Which is better (哪张更好)
//! over two earlier copies side by side, the same size, no date, no mark, and under them Left
//! better, Same, Right better (左边好 · 差不多 · 右边好). Answered, the buttons go:
//! each card's date under it, the newer's in ink and the older's quieter, the one chosen outlined;
//! no sentence. A unit's end re-test asks one pair a skill in turn, its skill named over it, each
//! revealed with its result's mark before the date once answered, the next shown after it. And
//! the extra drill's row (加练) when the end offers one.

use std::sync::Arc;

use egui::{Align, Align2, Layout, Pos2, Rect, Sense, TextStyle, UiBuilder, Vec2};
use openpaint_practice::{Answer, CopyRef};
use openpaint_practice_app::{CopyCard, End, PairShown};
use openpaint_ui::components::text_button;
use openpaint_ui::design::{INK, INK_SECONDARY, SPACE, TOUCH_TARGET};

use super::end::{EndTap, MARK, rule};
use crate::components::{paint_copy, paint_result};
use crate::text::{self as t, Words, language};

/// A card's height over its width (the mock's 120 by 140), the gap between the two, and the
/// line its date takes, in points.
const TALL: f32 = 120.0 / 140.0;
const GAP: f32 = 14.0;
const DATE: f32 = 30.0;

/// The end's pairs in `end`, their copies' cards as read so far; what was tapped.
pub(super) fn pairs(ui: &mut egui::Ui, end: &End, cards: &[Arc<CopyCard>]) -> Option<EndTap> {
    if end.pairs.is_empty() {
        return None;
    }
    rule(ui);
    let several = end.pairs.len() > 1;
    if !several {
        ui.label(egui::RichText::new(Words::from(t::WHICH_BETTER).shown(ui.ctx())).size(18.0));
    }
    let mut tapped = None;
    for (at, pair) in end.pairs.iter().enumerate() {
        if several {
            let name = pair.skill.map_or(t::RETEST, t::skill_name);
            ui.label(Words::from(name).shown(ui.ctx()));
        }
        let asked = one(ui, pair, cards, several);
        if let Some(answer) = asked {
            tapped = Some(EndTap::Pair(u8::try_from(at).unwrap_or(u8::MAX), answer));
        }
        // In turn: the next pair once this one is answered.
        if pair.answer.is_none() {
            break;
        }
    }
    tapped
}

/// `pair`'s two cards and its buttons or, answered, its dates (with its marks on a re-test's,
/// `marked`); the answer tapped.
fn one(
    ui: &mut egui::Ui,
    pair: &PairShown,
    cards: &[Arc<CopyCard>],
    marked: bool,
) -> Option<Answer> {
    let width = (ui.available_width() - GAP) / 2.0;
    let below = if pair.answer.is_some() { DATE } else { 0.0 };
    let size = Vec2::new(ui.available_width(), width * TALL + below);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let card = |copy: CopyRef| cards.iter().find(|card| card.copy == copy).map(|c| &**c);
    let chosen = match pair.answer {
        Some(Answer::Left) => Some(0),
        Some(Answer::Right) => Some(1),
        Some(Answer::Alike) | None => None,
    };
    let language = language(ui.ctx());
    for (side, copy) in pair.sides().into_iter().enumerate() {
        let left = rect.left() + side as f32 * (width + GAP);
        let frame =
            Rect::from_min_size(Pos2::new(left, rect.top()), Vec2::new(width, width * TALL));
        let shown = card(copy);
        paint_copy(ui, frame, shown, chosen == Some(side));
        if pair.answer.is_none() {
            continue;
        }
        let Some(shown) = shown else {
            continue;
        };
        let newer = copy == pair.asked.newer;
        let mut at = Pos2::new(frame.left(), frame.bottom() + DATE / 2.0);
        if let Some(outcome) = shown.result.filter(|_| marked) {
            paint_result(
                ui.painter(),
                (at + Vec2::new(MARK, 0.0), MARK),
                outcome,
                INK,
            );
            at.x += 2.0 * MARK + SPACE;
        }
        let date = t::copy_date(language, shown.day);
        let ink = if newer { INK } else { INK_SECONDARY };
        let font = TextStyle::Body.resolve(ui.style());
        ui.painter().text(at, Align2::LEFT_CENTER, date, font, ink);
    }
    if pair.answer.is_some() {
        return None;
    }
    buttons(ui)
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
