//! The Pomodoro on the home (S124): under the next lesson a quiet label Pomodoro and its row, the
//! ring with one hand, the lengths 15, 25 and 40 (the one chosen ringed) and "min", Start timer in
//! its pill; with a block under way the ring showing what is left, the minutes (Paused while held,
//! Time in the seal's red at the time) and Continue. At the right of the Practice header, only
//! when there is one, Today and a small red seal for each block completed today.

use egui::{Align, Layout, Pos2, Rect, Sense, TextStyle, Vec2};
use openpaint_practice::Length;
use openpaint_practice_app::{BlockShown, TimerIntent, TimerShown};
use openpaint_ui::components::{choices, pill_button, text_choice};
use openpaint_ui::design::{INK, INK_SECONDARY, SPACE, TOUCH_TARGET};

use super::column::{label, rule};
use crate::components::{Ring, paint_ring, paint_square, seal_turn};
use crate::palette::{VERMILION, VERMILION_INK};
use crate::text::{self as t, Words};

/// The row's ring's radius, a small seal's side and the room each takes, in points.
const RING: f32 = 16.0;
const SMALL_SEAL: f32 = 12.0;
const SEAL_PITCH: f32 = 22.0;

/// The row under the next lesson; what was tapped and where. Start timer (Continue) stands at
/// the row's right where the row leaves it room, else on a line of its own under it, at the left
/// as Start's pill; where even the lengths leave no room for the ring's glyph, they go without it.
pub(super) fn pomodoro_row(ui: &mut egui::Ui, timer: &TimerShown) -> Option<(TimerIntent, Rect)> {
    ui.add_space(SPACE);
    rule(ui);
    label(ui, t::POMODORO);
    let under_way = timer
        .block
        .as_ref()
        .filter(|block| matches!(block, BlockShown::Running { .. } | BlockShown::Up));
    let go = if under_way.is_some() {
        t::CONTINUE
    } else {
        t::START_TIMER
    };
    let pill = word_width(ui, Words::from(go)) + PILL_PAD;
    let (chosen, started) = ui
        .horizontal(|ui| {
            let chosen = state_or_lengths(ui, timer, under_way);
            // Only beside the row when it fits there whole: never over a length.
            if ui.available_width() < pill {
                return (chosen, None);
            }
            let tapped = ui
                .with_layout(Layout::right_to_left(Align::Center), |ui| {
                    pill_button(ui, go, true)
                })
                .inner;
            (chosen, Some(tapped))
        })
        .inner;
    let tapped = started.unwrap_or_else(|| pill_button(ui, go, true));
    let started = (tapped.clicked()).then_some((TimerIntent::Start, tapped.rect));
    chosen
        .map(|(length, at)| (TimerIntent::Length(length), at))
        .or(started)
}

/// The room a pilled word takes beyond its word, as `pill_button` lays it, in points.
const PILL_PAD: f32 = 7.0 * SPACE;

/// The width `words` take in a button's font, in points.
fn word_width(ui: &egui::Ui, words: Words<'_>) -> f32 {
    let font = TextStyle::Button.resolve(ui.style());
    let shown = words.shown(ui.ctx()).to_owned();
    ui.painter().layout_no_wrap(shown, font, INK).size().x
}

/// The ring and the block's state under way (the minutes left, Paused, Time), or the ring's glyph
/// and the lengths; the length tapped and where.
fn state_or_lengths(
    ui: &mut egui::Ui,
    timer: &TimerShown,
    under_way: Option<&BlockShown>,
) -> Option<(Length, Rect)> {
    let language = t::language(ui.ctx());
    let glyph_width = 2.5 * RING;
    // The lengths keep their room before the glyph does.
    let glyph = under_way.is_some() || ui.available_width() >= glyph_width + lengths_width(ui);
    let centre = glyph.then(|| {
        let size = Vec2::new(glyph_width, TOUCH_TARGET);
        let (glyph, _) = ui.allocate_exact_size(size, Sense::hover());
        Pos2::new(glyph.left() + RING + 2.0, glyph.center().y)
    });
    let (ring, chosen) = match under_way {
        Some(BlockShown::Running {
            left,
            share,
            paused,
        }) => {
            let gone = 1.0 - share;
            let (ring, word) = if *paused {
                (Ring::Held { gone }, t::PAUSED.get(language).to_owned())
            } else {
                (Ring::Counting { gone }, t::minutes_left(language, *left))
            };
            ui.label(egui::RichText::new(word).color(INK));
            (ring, None)
        }
        Some(_) => {
            let word = t::TIME_UP.get(language);
            ui.label(egui::RichText::new(word).color(VERMILION_INK));
            (Ring::Whole(VERMILION), None)
        }
        None => (Ring::Glyph, lengths(ui, timer.length)),
    };
    if let Some(centre) = centre {
        paint_ring(ui.painter(), (centre, RING), ring);
    }
    chosen
}

/// The room the three lengths and "min" take in a row, in points.
fn lengths_width(ui: &egui::Ui) -> f32 {
    let gap = ui.spacing().item_spacing.x;
    let numbers: f32 = (Length::ALL.into_iter())
        .map(|length| {
            let number = t::length_number(length);
            (word_width(ui, Words::made(&number)) + 4.0 * SPACE).max(TOUCH_TARGET) + gap
        })
        .sum();
    numbers + word_width(ui, Words::from(t::MINUTES)) + gap
}

/// The three lengths, the one chosen ringed, and "min" after them; the one tapped and where.
fn lengths(ui: &mut egui::Ui, chosen: Length) -> Option<(Length, Rect)> {
    let mut tapped = None;
    choices(ui, "pomodoro lengths", |ui, group| {
        for length in Length::ALL {
            let number = t::length_number(length);
            let word = text_choice(ui, group, Words::made(&number), length == chosen);
            if word.clicked() && length != chosen {
                tapped = Some((length, word.rect));
            }
        }
    });
    let min = Words::from(t::MINUTES).shown(ui.ctx());
    ui.label(egui::RichText::new(min).color(INK_SECONDARY));
    tapped
}

/// Today and a small red seal for each of today's `sealed` blocks, laid right to left in `ui`
/// (the Practice header's right, left of its options' button); nothing while there is none.
pub(super) fn today_seals(ui: &mut egui::Ui, sealed: u8) {
    if sealed == 0 {
        return;
    }
    let width = small_seals_width(sealed);
    let (band, _) = ui.allocate_exact_size(Vec2::new(width, TOUCH_TARGET), Sense::hover());
    small_seals(ui.painter(), (band.left_center(), sealed));
    let small = TextStyle::Small.resolve(ui.style());
    let today = Words::from(t::TODAY).shown(ui.ctx());
    let galley = ui
        .painter()
        .layout_no_wrap(today.to_owned(), small, INK_SECONDARY);
    let (word, _) = ui.allocate_exact_size(galley.size() + Vec2::new(SPACE, 0.0), Sense::hover());
    let at = Pos2::new(word.left(), band.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, INK_SECONDARY);
}

/// `count` small red seals in a row from `left` (its middle height), each a little turned: the
/// home's today and a day's in the practice book.
pub(crate) fn small_seals(painter: &egui::Painter, (left, count): (Pos2, u8)) {
    for place in 0..count {
        let x = left.x + SEAL_PITCH * (f32::from(place) + 0.5);
        let centre = Pos2::new(x, left.y);
        paint_square(painter, (centre, SMALL_SEAL), seal_turn(0), 1.0);
    }
}

/// The room `count` small seals take, in points.
pub(crate) fn small_seals_width(count: u8) -> f32 {
    SEAL_PITCH * f32::from(count)
}
