//! A Pomodoro's block's end on its card (S124), in place of a lesson's end, over the rows drawn in
//! the block: its length ("25 minutes") with its seal beside, a red square a little turned reading
//! 番 over 茄 (FOCUS); "5 lessons · 19 exercises"; each result a mark, wrapping; under a rule each
//! skill that gained a dot with the level it reached and its three dots; the blind pair it asks
//! (S148, `end_pair.rs`: its lessons' ends showed none); under another rule the break's ring with
//! Break and the minutes left (the ring whole once it is over); Another pomodoro in its pill and
//! That's all for today, which goes home.

use std::sync::Arc;

use egui::{Align2, Pos2, Sense, TextStyle, Vec2};
use openpaint_practice::{Answer, Skill};
use openpaint_practice_app::{BlockEnd, CopyCard, PairShown};
use openpaint_ui::components::{pill_button, text_button};
use openpaint_ui::design::{INK, INK_SECONDARY, SPACE};

use super::end::{DOT, DOT_PITCH, level_of, result_marks, rule};
use super::end_pair;
use crate::components::{DOTS, Ring, paint_dots, paint_ring, paint_seal, seal_turn};
use crate::text::{self as t, Language, language};

/// The seal's side, the break's ring's radius, in points.
const SEAL: f32 = 58.0;
const BREAK_RING: f32 = 22.0;

/// What the block's end asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockTap {
    Another,
    Enough,
    /// Blind pair `n` answered (S148).
    Pair(u8, Answer),
}

/// The end of the block `end`, its break's minutes left and the share of it left while it counts
/// (`rest`), the `pairs` it asks with their copies' `cards` as read so far; what was tapped.
pub(crate) fn block_end_card(
    ui: &mut egui::Ui,
    end: &BlockEnd,
    rest: Option<(u32, f32)>,
    (pairs, cards): (&[PairShown], &[Arc<CopyCard>]),
) -> Option<BlockTap> {
    let language = language(ui.ctx());
    head(ui, end, language);
    if !end.tally.results.is_empty() {
        result_marks(ui, &end.tally.results);
    }
    let gained =
        (end.tally.skills.iter()).filter(|(_, before, after)| (*after).min(DOTS) > *before);
    let gained: Vec<&(Skill, u8, u8)> = gained.collect();
    if !gained.is_empty() {
        rule(ui);
        for skill in gained {
            skill_row(ui, *skill, language);
        }
    }
    let answered = end_pair::pairs(ui, pairs, cards);
    let mut tapped = answered.map(|(at, answer)| BlockTap::Pair(at, answer));
    rule(ui);
    break_row(ui, rest, language);
    if pill_button(ui, t::ANOTHER_POMODORO, true).clicked() {
        tapped = Some(BlockTap::Another);
    }
    if text_button(ui, t::ENOUGH, true).clicked() {
        tapped = Some(BlockTap::Enough);
    }
    tapped
}

/// Its length and its lessons and exercises at the left, its seal at the right.
fn head(ui: &mut egui::Ui, end: &BlockEnd, language: Language) {
    let size = Vec2::new(ui.available_width(), SEAL + SPACE);
    let (band, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    let heading = TextStyle::Heading.resolve(ui.style());
    let small = TextStyle::Small.resolve(ui.style());
    let length = t::block_length(language, end.seconds);
    let shown = painter.text(band.left_top(), Align2::LEFT_TOP, length, heading, INK);
    let counts = (end.tally.lessons, end.tally.exercises);
    let under = shown.left_bottom() + Vec2::new(0.0, SPACE / 2.0);
    let count = t::block_count(language, counts);
    painter.text(under, Align2::LEFT_TOP, count, small, INK_SECONDARY);
    let centre = Pos2::new(band.right() - SEAL / 2.0 - SPACE / 2.0, band.center().y);
    let lines = t::block_seal(language);
    paint_seal(painter, (centre, SEAL), (&lines, seal_turn(0)), 1.0);
}

/// A skill that gained a dot: its name and the level it reached, its three dots under them.
fn skill_row(ui: &mut egui::Ui, (skill, _, after): (Skill, u8, u8), language: Language) {
    let earned = after.min(DOTS);
    if let Some(level) = level_of(earned) {
        ui.label(t::skill_reached(language, skill, level));
    }
    let size = Vec2::new(ui.available_width(), 2.0 * DOT + SPACE);
    let (band, _) = ui.allocate_exact_size(size, Sense::hover());
    let left = Pos2::new(band.left() + DOT, band.center().y);
    paint_dots(ui.painter(), (left, DOT_PITCH, DOT), (earned, None), INK);
}

/// The break's ring with Break and the minutes left beside it; the ring whole once it is over.
fn break_row(ui: &mut egui::Ui, rest: Option<(u32, f32)>, language: Language) {
    let size = Vec2::new(ui.available_width(), 2.0 * BREAK_RING + 2.0 * SPACE);
    let (band, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    let centre = Pos2::new(band.left() + BREAK_RING + 2.0, band.center().y);
    let ring = match rest {
        Some((_, share)) => Ring::Counting { gone: 1.0 - share },
        None => Ring::Whole(INK),
    };
    paint_ring(painter, (centre, BREAK_RING), ring);
    let beside = Pos2::new(centre.x + BREAK_RING + 2.0 * SPACE, centre.y);
    let body = TextStyle::Body.resolve(ui.style());
    let word = t::BREAK.get(language);
    match rest {
        Some((minutes, _)) => {
            painter.text(beside, Align2::LEFT_BOTTOM, word, body, INK);
            let small = TextStyle::Small.resolve(ui.style());
            let left = t::minutes_left(language, minutes);
            let under = beside + Vec2::new(0.0, SPACE / 4.0);
            painter.text(under, Align2::LEFT_TOP, left, small, INK_SECONDARY);
        }
        None => {
            painter.text(beside, Align2::LEFT_CENTER, word, body, INK);
        }
    }
}
