//! The practice book's days (S89): a month as a calendar, Monday first, earlier months a swipe (or
//! the chevrons beside its name) away; a day practised an ink wash behind its number, darker the
//! longer, today ringed, days ahead pale, a day without practice plain paper. A day practised,
//! tapped, shows its card (`day.rs`).

use egui::{Align, Align2, Layout, Pos2, Rect, Sense, Stroke, TextStyle, Vec2};
use openpaint_practice_app::BookProjection;
use openpaint_ui::components::{controls, icon_button};
use openpaint_ui::dates::{Date, weekday};
use openpaint_ui::design::{INK, Icon, MARK_LINE, SPACE, TOUCH_TARGET};

use super::BookTap;
use crate::palette::PENCIL;
use crate::text::{self as t, Language};

/// The weekdays' row's height and a day's cell's at most, in points.
const WEEKDAYS: f32 = 28.0;
const CELL: f32 = 64.0;
/// The time from which a day's wash is darkest, in seconds, and its ink from lightest to darkest.
const DARKEST: f32 = 3600.0;
const WASH: (f32, f32) = (0.1, 0.42);
/// How far a finger swipes before the month turns, in points.
const SWIPE: f32 = 80.0;

/// The month `shown` (its first day) in `rect`: its name and chevrons, the weekdays and the days;
/// what was tapped.
pub(super) fn calendar(
    ui: &mut egui::Ui,
    rect: Rect,
    (book, shown): (&BookProjection, Date),
    swipe: &mut f32,
) -> Option<BookTap> {
    let language = t::language(ui.ctx());
    let this_month = Date::of(book.today).first_of_month();
    let head = Rect::from_min_size(rect.min, Vec2::new(rect.width(), TOUCH_TARGET));
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(head)
            .layout(Layout::left_to_right(Align::Center)),
    );
    row.heading(t::month_title(language, shown));
    row.add_space(SPACE);
    let earlier = icon_button(&mut row, Icon::Earlier, t::EARLIER_MONTH, true).clicked();
    let later_offered = shown.days() < this_month.days();
    let later = icon_button(&mut row, Icon::Later, t::LATER_MONTH, later_offered).clicked();
    let width = rect.width() / 7.0;
    let painter = ui.painter();
    let small = TextStyle::Small.resolve(ui.style());
    for (column, initial) in t::WEEKDAYS.iter().enumerate() {
        let x = rect.left() + width * (column as f32 + 0.5);
        let at = Pos2::new(x, head.bottom() + WEEKDAYS / 2.0);
        let word = initial.get(language);
        painter.text(at, Align2::CENTER_CENTER, word, small.clone(), PENCIL);
    }
    let top = head.bottom() + WEEKDAYS;
    let height = ((rect.bottom() - top) / 6.0).min(CELL);
    let tapped = days(ui, (rect, top, [width, height]), (book, shown), swipe);
    if earlier {
        return Some(BookTap::Month(shown.months_on(-1)));
    }
    if later && later_offered {
        return Some(BookTap::Month(shown.months_on(1)));
    }
    match tapped {
        Some(BookTap::Month(month)) if month.days() > this_month.days() => None,
        tapped => tapped,
    }
}

/// The days of month `shown` in rows of `cell` points from `top`, each practised one named as a
/// control by its date; a tap on a practised day or a swipe across the month.
fn days(
    ui: &mut egui::Ui,
    (rect, top, [width, height]): (Rect, f32, [f32; 2]),
    (book, shown): (&BookProjection, Date),
    swipe: &mut f32,
) -> Option<BookTap> {
    let first = shown.days();
    let lead = weekday(first) as usize;
    let count = shown.days_in_month() as usize;
    let rows = (lead + count).div_ceil(7);
    let grid = Rect::from_min_size(
        Pos2::new(rect.left(), top),
        Vec2::new(rect.width(), height * rows as f32),
    );
    let response = ui.interact(grid, ui.id().with("days"), Sense::click_and_drag());
    let cell_of = |n: usize| {
        let place = lead + n;
        let min = Pos2::new(
            rect.left() + width * (place % 7) as f32,
            top + height * (place / 7) as f32,
        );
        Rect::from_min_size(min, Vec2::new(width, height))
    };
    for n in 0..count {
        let day = first + n as i64;
        let cell = cell_of(n);
        let practised = book.book.day(day).map(|day| day.seconds);
        paint_day(ui, cell, (day, book.today), practised);
        if practised.is_some() {
            let name = t::day_title(Language::English, Date::of(day), weekday(day));
            controls::register(ui, &name, cell);
        }
    }
    if response.drag_started() {
        *swipe = 0.0;
    }
    if response.dragged() {
        *swipe += response.drag_delta().x;
    }
    if response.drag_stopped() {
        let across = std::mem::take(swipe);
        if across.abs() >= SWIPE {
            // Swiped to the right, the month before comes in from the left.
            let by = if across > 0.0 { -1 } else { 1 };
            return Some(BookTap::Month(shown.months_on(by)));
        }
    }
    let at = response
        .clicked()
        .then(|| response.interact_pointer_pos())??;
    let n = (0..count).find(|n| cell_of(*n).contains(at))?;
    let day = first + n as i64;
    book.book
        .day(day)
        .map(|_| BookTap::Day(Some((day, cell_of(n)))))
}

/// Day `day` in `cell`: its number, an ink wash behind it as long as it was `practised`
/// (seconds), a ring round today's, pale ahead of `today`.
fn paint_day(ui: &egui::Ui, cell: Rect, (day, today): (i64, i64), practised: Option<u64>) {
    let painter = ui.painter();
    let radius = cell.width().min(cell.height()) * 0.36;
    if let Some(seconds) = practised {
        let share = (seconds as f32 / DARKEST).min(1.0);
        let wash = WASH.0 + (WASH.1 - WASH.0) * share;
        painter.circle_filled(cell.center(), radius, INK.gamma_multiply(wash));
    }
    if day == today {
        painter.circle_stroke(cell.center(), radius + 2.0, Stroke::new(MARK_LINE, INK));
    }
    let ink = if day > today { PENCIL } else { INK };
    let body = TextStyle::Body.resolve(ui.style());
    let number = Date::of(day).day.to_string();
    painter.text(cell.center(), Align2::CENTER_CENTER, number, body, ink);
}
