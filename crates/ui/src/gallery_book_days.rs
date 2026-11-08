//! The practice book's days (S89): a month as a calendar, Monday first, earlier months a swipe (or
//! the chevrons beside its name) away; a day practised an ink wash behind its number, darker the
//! longer, today ringed, days ahead pale, a day without practice plain paper. A day practised,
//! tapped, shows its card grown out of it over progress: its date, its time, its sessions' and
//! copies' sheets small (a tap opens one) and the colours matched, each a square split corner to
//! corner, the original above the diagonal and the copy's first pick below it.

use egui::{Align, Align2, Id, Layout, Order, Pos2, Rect, Sense, Shape, Stroke, TextStyle, Vec2};
use openpaint_practice::{Date, Rgb8, weekday};
use openpaint_practice_app::{BookProjection, BookRecord, Mark, SheetRead};

use super::{BookTap, layout};
use crate::components::chrome::Source;
use crate::components::{controls, icon_button, paint_sheet, paper};
use crate::design::{
    CORNER, DIVIDER, INK, INK_SECONDARY, Icon, MARK_LINE, PENCIL, SPACE, TOUCH_TARGET,
    surface_frame,
};
use crate::text::{self as t, Language};

/// The weekdays' row's height and a day's cell's at most, in points.
const WEEKDAYS: f32 = 28.0;
const CELL: f32 = 64.0;
/// The time from which a day's wash is darkest, in seconds, and its ink from lightest to darkest.
const DARKEST: f32 = 3600.0;
const WASH: (f32, f32) = (0.1, 0.42);
/// How far a finger swipes before the month turns, in points.
const SWIPE: f32 = 80.0;
/// A sheet's picture's height on a day's card, its widest, and a colour's square, in points.
const SHEET: f32 = 152.0;
const SHEET_WIDEST: f32 = 224.0;
const SQUARE: f32 = 36.0;
/// A day's card's widest, in points.
const CARD: f32 = 720.0;
/// A sheet's shape before it is read: a session's (2,920 by 3,160 pixels).
const SHEET_SHAPE: f32 = 2920.0 / 3160.0;

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

/// The card of `day` (practised), grown out of its `cell` over progress on `screen`; a sheet
/// tapped opens it.
pub(super) fn day_card(
    ctx: &egui::Context,
    book: &BookProjection,
    (day, cell): (i64, Rect),
    screen: Rect,
) -> Option<BookTap> {
    let shown = book.book.day(day)?;
    let [_, _, side] = layout(screen);
    let id = Id::new("practice book day");
    let source = Source::Rect {
        rect: cell,
        radius: (cell.height() / 2.0, CORNER),
        veil: false,
    };
    let area = egui::Area::new(id)
        .order(Order::Foreground)
        .movable(false)
        .constrain(false)
        .fade_in(false)
        .fixed_pos(side.min);
    let margin = 2.0 * SPACE;
    let (tapped, _) = paper(ctx, id, source, area, |ui| {
        surface_frame(egui::Margin::same(margin as i8))
            .show(ui, |ui| {
                ui.set_width(side.width().min(CARD) - 2.0 * margin);
                let language = t::language(ui.ctx());
                ui.heading(t::day_title(language, Date::of(day), weekday(day)));
                let time = t::practice_time(language, shown.seconds);
                ui.label(egui::RichText::new(time).color(INK_SECONDARY));
                ui.add_space(SPACE);
                let tapped = sheets(ui, book, &shown.records);
                colours(ui, book, &shown.records);
                tapped
            })
            .inner
    });
    tapped
}

/// The sheets of `records`, small, side by side, each named under its picture; the one tapped.
fn sheets(ui: &mut egui::Ui, book: &BookProjection, records: &[u64]) -> Option<BookTap> {
    let language = t::language(ui.ctx());
    let mut tapped = None;
    ui.horizontal_wrapped(|ui| {
        for record in records.iter().filter_map(|id| book.record(*id)) {
            let read = book.read.get(&record.id);
            let shape = read.map_or(SHEET_SHAPE, |read| {
                read.size[0] as f32 / read.size[1].max(1) as f32
            });
            let size = Vec2::new((SHEET * shape).min(SHEET_WIDEST), SHEET);
            let (english, shown) = name(record, language);
            ui.vertical(|ui| {
                let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                paint_sheet(ui.painter(), None, rect, egui::Color32::WHITE);
                if let Some(read) = read {
                    paint_marks(ui, rect, read);
                }
                controls::register(ui, &english, rect);
                ui.label(egui::RichText::new(shown).small().color(INK_SECONDARY));
                if response.clicked() {
                    tapped = Some(BookTap::Open(record.sheet, rect));
                }
            });
            ui.add_space(SPACE);
        }
    });
    tapped
}

/// A record's name, in English (its control's) and as `language` shows it.
fn name(record: &BookRecord, language: Language) -> (String, String) {
    if let Some(family) = record.drill {
        return (
            t::drill_number(Language::English, family, record.number),
            t::drill_number(language, family, record.number),
        );
    }
    match &record.copy {
        Some(title) => (
            title.as_str().to_owned(),
            t::shown_name(language, title.as_str()).into_owned(),
        ),
        None => (
            t::session_number(Language::English, record.number),
            t::session_number(language, record.number),
        ),
    }
}

/// What lies on a sheet read, drawn small in `rect`.
fn paint_marks(ui: &egui::Ui, rect: Rect, read: &SheetRead) {
    let painter = ui.painter().with_clip_rect(rect);
    let scale = rect.width() / read.size[0].max(1) as f32;
    let at = |[x, y]: [f32; 2]| rect.min + Vec2::new(x, y) * scale;
    for [min, max] in &read.frames {
        let frame = Rect::from_min_max(at(*min), at(*max));
        painter.rect_stroke(
            frame,
            0.0,
            Stroke::new(0.75, PENCIL),
            egui::StrokeKind::Inside,
        );
    }
    for mark in &read.marks {
        match mark {
            Mark::Line {
                points,
                colour,
                width,
            } => {
                let points: Vec<Pos2> = points.iter().copied().map(at).collect();
                let stroke = Stroke::new((width * scale).max(0.6), rgb(*colour));
                painter.add(Shape::line(points, stroke));
            }
            Mark::Fill {
                rect: [min, max],
                colour,
            } => {
                painter.rect_filled(Rect::from_min_max(at(*min), at(*max)), 0.0, rgb(*colour));
            }
        }
    }
}

/// The colours matched on `records`' sheets, seven to a row.
fn colours(ui: &mut egui::Ui, book: &BookProjection, records: &[u64]) {
    let pairs: Vec<(Rgb8, Rgb8)> = (records.iter())
        .filter_map(|id| book.read.get(id))
        .flat_map(|read| read.colours.iter().copied())
        .collect();
    if pairs.is_empty() {
        return;
    }
    ui.add_space(SPACE);
    for row in pairs.chunks(7) {
        ui.horizontal(|ui| {
            for (original, copy) in row {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(SQUARE), Sense::hover());
                let painter = ui.painter();
                let [tl, tr, br, bl] = [
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                ];
                let none = Stroke::NONE;
                painter.add(Shape::convex_polygon(
                    vec![tl, tr, bl],
                    rgb(*original),
                    none,
                ));
                painter.add(Shape::convex_polygon(vec![tr, br, bl], rgb(*copy), none));
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0, DIVIDER),
                    egui::StrokeKind::Inside,
                );
            }
        });
    }
}

fn rgb([r, g, b]: Rgb8) -> egui::Color32 {
    egui::Color32::from_rgb(r, g, b)
}
