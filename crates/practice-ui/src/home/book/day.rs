//! A day's card in the practice book (S89): tapped on the calendar, a day practised shows its card
//! grown out of its cell over progress, covering it whole (S143: no seal of progress shows past
//! it): its date, its time with a small red seal for each Pomodoro block completed on it (S124),
//! its sheets small in rows as many as fit, as the path lays its tiles (a day's sheet the rows its
//! lessons used, named by them under it within its width, S143; a tap opens one) and the colours
//! matched, each a square split corner to corner, the original above the diagonal and the copy's
//! first pick below it. Past the card's foot it scrolls.

use std::ops::Range;

use egui::{Id, Order, Pos2, Rect, Sense, Shape, Stroke, TextStyle, Vec2};
use openpaint_practice::{Day, Rgb8};
use openpaint_practice_app::{BookProjection, BookRecord, Mark, SheetRead};
use openpaint_ui::components::chrome::Source;
use openpaint_ui::components::{controls, paint_sheet, paper};
use openpaint_ui::dates::{Date, weekday};
use openpaint_ui::design::{CORNER, DIVIDER, INK_SECONDARY, SPACE, surface_frame};

use super::super::options::scrolled;
use super::super::pomodoro::{small_seals, small_seals_width};
use super::{BookTap, layout};
use crate::palette::PENCIL;
use crate::text::{self as t, Language};

/// A sheet's picture's height on a day's card, its widest, the gap between two, and a colour's
/// square, in points.
const SHEET: f32 = 152.0;
const SHEET_WIDEST: f32 = 224.0;
const GAP: f32 = 2.0 * SPACE;
const SQUARE: f32 = 36.0;
/// A sheet's shape before it is read: six rows' (2,920 by 3,160 pixels).
const SHEET_SHAPE: f32 = 2920.0 / 3160.0;

/// The card of `day` (practised), grown out of its `cell` over progress on `screen`; a sheet
/// tapped opens it.
pub(super) fn day_card(
    ctx: &egui::Context,
    book: &BookProjection,
    (day, cell): (i64, Rect),
    screen: Rect,
) -> Option<BookTap> {
    let shown = book.book.day(day)?;
    let [_, side] = layout(screen);
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
    let (tapped, _) = paper(ctx, id, source, area, |ui| {
        surface_frame(egui::Margin::ZERO)
            .show(ui, |ui| {
                ui.set_width(side.width());
                ui.set_min_height(side.height());
                // Each day's from its top; the sheets' shadows clipped at the card's edge only.
                ui.push_id(day, |ui| {
                    scrolled(ui, side.height(), |ui| {
                        egui::Frame::NONE
                            .inner_margin(2.0 * SPACE)
                            .show(ui, |ui| card(ui, book, (day, shown), side.size()))
                            .inner
                    })
                })
                .inner
            })
            .inner
    });
    tapped
}

/// What a day's card of `size` holds: its date, its time with its Pomodoro seals, its sheets and
/// its colours; the sheet tapped.
fn card(
    ui: &mut egui::Ui,
    book: &BookProjection,
    (day, shown): (i64, &Day),
    size: Vec2,
) -> Option<BookTap> {
    let margin = 2.0 * SPACE;
    ui.set_width(size.x - 2.0 * margin);
    ui.set_min_height(size.y - 2.0 * margin);
    let language = t::language(ui.ctx());
    ui.heading(t::day_title(language, Date::of(day), weekday(day)));
    let time = t::practice_time(language, shown.seconds);
    let blocks = u8::try_from(shown.blocks).unwrap_or(u8::MAX);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(time).color(INK_SECONDARY));
        seals(ui, blocks);
    });
    ui.add_space(SPACE);
    let tapped = sheets(ui, book, &shown.records);
    colours(ui, book, &shown.records);
    tapped
}

/// A day's Pomodoro blocks completed, each a small red seal beside its time (S124).
fn seals(ui: &mut egui::Ui, blocks: u8) {
    if blocks == 0 {
        return;
    }
    let size = Vec2::new(
        small_seals_width(blocks),
        ui.text_style_height(&TextStyle::Body),
    );
    let (band, _) = ui.allocate_exact_size(size, Sense::hover());
    small_seals(ui.painter(), (band.left_center(), blocks));
}

/// The sheets of `records`, small, in rows across the card, as many to a row as fit (as the
/// path's tiles), each named under its picture within its width; the one tapped.
fn sheets(ui: &mut egui::Ui, book: &BookProjection, records: &[u64]) -> Option<BookTap> {
    let shown: Vec<(&BookRecord, Option<&SheetRead>, Vec2)> = (records.iter())
        .filter_map(|id| book.record(*id))
        .map(|record| {
            let read = book.read.get(&record.id).map(AsRef::as_ref);
            let shape = read.map_or(SHEET_SHAPE, |read| {
                read.size[0] as f32 / read.size[1].max(1) as f32
            });
            (
                record,
                read,
                Vec2::new((SHEET * shape).min(SHEET_WIDEST), SHEET),
            )
        })
        .collect();
    let widths: Vec<f32> = shown.iter().map(|(_, _, size)| size.x).collect();
    let mut tapped = None;
    for row in rows(&widths, ui.available_width()) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for (record, read, size) in &shown[row] {
                if let Some(at) = sheet(ui, record, *read, *size) {
                    tapped = Some(BookTap::Open(record.sheet, at));
                }
            }
        });
        ui.add_space(SPACE);
    }
    tapped
}

/// The rows pictures `widths` wide fall into across `room` points, `GAP` apart: each as many as
/// fit, at least one.
pub(super) fn rows(widths: &[f32], room: f32) -> Vec<Range<usize>> {
    let mut rows = Vec::new();
    let (mut start, mut used) = (0, 0.0);
    for (index, width) in widths.iter().enumerate() {
        if index > start && used + GAP + width > room {
            rows.push(start..index);
            (start, used) = (index, 0.0);
        }
        used += if index == start { *width } else { GAP + width };
    }
    if start < widths.len() {
        rows.push(start..widths.len());
    }
    rows
}

/// A sheet's picture `size` and its name under it, wrapped within its width; where it lies when
/// tapped.
fn sheet(
    ui: &mut egui::Ui,
    record: &BookRecord,
    read: Option<&SheetRead>,
    size: Vec2,
) -> Option<Rect> {
    let (english, shown) = name(record, ui.ctx());
    ui.vertical(|ui| {
        ui.set_width(size.x);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        paint_sheet(ui.painter(), None, rect, egui::Color32::WHITE);
        if let Some(read) = read {
            paint_marks(ui, rect, read);
        }
        controls::register(ui, &english, rect);
        ui.label(egui::RichText::new(shown).small().color(INK_SECONDARY));
        response.clicked().then_some(rect)
    })
    .inner
}

/// A sheet's name, in English (its control's) and as the pass's language shows it: a colour
/// copy's title, a day's sheet by its lessons (S121c, S143).
fn name(record: &BookRecord, ctx: &egui::Context) -> (String, String) {
    if let Some(title) = &record.copy {
        return (
            title.as_str().to_owned(),
            t::shown_name(ctx, title.as_str()).into_owned(),
        );
    }
    let named = |language: Language| t::sheet_lessons(language, &record.rows);
    (named(Language::English), named(t::language(ctx)))
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
