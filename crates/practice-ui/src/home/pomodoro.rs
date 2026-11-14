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

/// The row under the next lesson; what was tapped and where.
pub(super) fn pomodoro_row(ui: &mut egui::Ui, timer: &TimerShown) -> Option<(TimerIntent, Rect)> {
    ui.add_space(SPACE);
    rule(ui);
    label(ui, t::POMODORO);
    let language = t::language(ui.ctx());
    ui.horizontal(|ui| {
        let (glyph, _) =
            ui.allocate_exact_size(Vec2::new(2.5 * RING, TOUCH_TARGET), Sense::hover());
        let centre = Pos2::new(glyph.left() + RING + 2.0, glyph.center().y);
        let under_way = timer
            .block
            .as_ref()
            .filter(|block| matches!(block, BlockShown::Running { .. } | BlockShown::Up));
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
        paint_ring(ui.painter(), (centre, RING), ring);
        let go = if under_way.is_some() {
            t::CONTINUE
        } else {
            t::START_TIMER
        };
        let tapped = ui
            .with_layout(Layout::right_to_left(Align::Center), |ui| {
                pill_button(ui, go, true)
            })
            .inner;
        let started = tapped
            .clicked()
            .then_some((TimerIntent::Start, tapped.rect));
        chosen
            .map(|(length, at)| (TimerIntent::Length(length), at))
            .or(started)
    })
    .inner
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
