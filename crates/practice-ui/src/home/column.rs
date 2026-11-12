//! The home's practice column (S122), the entrance beside the studio's: "Practice" with the unit
//! in hand beside it and the practice options' button at its right; under them a sheet of paper
//! with the next lesson (its skill, its level, exercises and minutes, Start in its pill), for a
//! fresh learner the first lesson, while today's lesson is unfinished Start continuing it; under a
//! rule, while a redo waits, the review on the side (its skill, two exercises; Review); at the
//! sheet's foot All units (the path page grows out of it) and Practice book. No counts of days,
//! no quotas.

use egui::{Align, Align2, Layout, Rect, RichText, TextStyle, UiBuilder, Vec2};
use openpaint_practice_app::{PathProjection, Start};
use openpaint_ui::components::{icon_button, paint_sheet, pill_button, text_button};
use openpaint_ui::design::{DIVIDER, INK, INK_SECONDARY, Icon, SPACE, TOUCH_TARGET};

use super::{FIRST, HomePart, Taps};
use crate::text::{self as t, Text, Words};

/// What the column's sheet holds in from its edges, in points.
const MARGIN: f32 = 4.0 * SPACE;
/// The sheet's height at most, in points.
const SHEET: f32 = 600.0;

/// What a tap in the column asks.
enum Tapped {
    Start(Start, Rect),
    Path(Rect),
    Book(Rect),
    Options,
}

impl HomePart {
    /// The column in `rect`; returns where its sheet lies (a sheet going back lands in it).
    pub(super) fn column(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        (path, enabled): (&PathProjection, bool),
        taps: Taps<'_>,
    ) -> Rect {
        let head = Rect::from_min_size(rect.min, Vec2::new(rect.width(), TOUCH_TARGET));
        let sheet = Rect::from_min_max(rect.min + Vec2::new(0.0, TOUCH_TARGET + SPACE), rect.max);
        // No taller than its lessons need: the studio beside it keeps the eye.
        let sheet = Rect::from_min_size(
            sheet.min,
            Vec2::new(sheet.width(), sheet.height().min(SHEET)),
        );
        let options = heading(ui, head, path, enabled);
        paint_sheet(ui.painter(), None, sheet, egui::Color32::WHITE);
        let inner = sheet.shrink(MARGIN);
        let mut page = ui.new_child(
            UiBuilder::new()
                .id_salt("practice column")
                .max_rect(inner)
                .layout(Layout::top_down(Align::Min)),
        );
        page.spacing_mut().item_spacing = Vec2::new(SPACE, SPACE);
        let tapped = page
            .add_enabled_ui(enabled, |ui| {
                let next = next_lesson(ui, path);
                let reviewed = review(ui, path).map(|from| Tapped::Start(Start::Review, from));
                let tapped = reviewed.or(next);
                foot(ui, inner).or(tapped)
            })
            .inner;
        if let Some(tapped) = tapped.or(options.then_some(Tapped::Options)) {
            self.column_tap(tapped, rect, path, taps);
        }
        sheet
    }

    fn column_tap(&mut self, tapped: Tapped, rect: Rect, path: &PathProjection, taps: Taps<'_>) {
        match tapped {
            Tapped::Start(start, from) => self.start(start, from, path, taps),
            Tapped::Path(from) => self.path.open(from, path.unit),
            Tapped::Book(from) => self.book.open(from),
            Tapped::Options => self.practice.open(rect),
        }
    }
}

/// "Practice" with the unit in hand beside it, quieter, and the options' button at the right;
/// whether the button was tapped.
fn heading(ui: &mut egui::Ui, head: Rect, path: &PathProjection, enabled: bool) -> bool {
    let language = t::language(ui.ctx());
    let painter = ui.painter();
    let font = TextStyle::Heading.resolve(ui.style());
    let at = head.left_center();
    let name = Words::from(t::PRACTICE).shown(ui.ctx());
    let shown = painter.text(at, Align2::LEFT_CENTER, name, font, INK);
    let unit = t::unit_title(language, path.unit);
    let body = TextStyle::Body.resolve(ui.style());
    let beside = shown.right_center() + Vec2::new(2.0 * SPACE, 0.0);
    painter.text(beside, Align2::LEFT_CENTER, unit, body, INK_SECONDARY);
    let mut right = ui.new_child(
        UiBuilder::new()
            .id_salt("practice options button")
            .max_rect(head)
            .layout(Layout::right_to_left(Align::Center)),
    );
    icon_button(&mut right, Icon::More, t::PRACTICE_OPTIONS, enabled).clicked()
}

/// A quiet label over a block.
pub(super) fn label(ui: &mut egui::Ui, text: Text) {
    let shown = Words::from(text).shown(ui.ctx());
    ui.label(RichText::new(shown).small().color(INK_SECONDARY));
}

/// The next lesson: what it is and Start; the first lesson for a fresh learner, today's lesson
/// while it is unfinished.
fn next_lesson(ui: &mut egui::Ui, path: &PathProjection) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let (over, name, line) = match (path.open, path.first, &path.next) {
        (Some(_), _, _) => (t::UNDER_WAY, None, None),
        (None, true, _) => (
            t::NEXT_LESSON,
            Some(t::FIRST_LESSON),
            Some(t::exercises_line(language, FIRST)),
        ),
        (None, false, Some(offer)) => (
            t::NEXT_LESSON,
            Some(t::skill_name(offer.skill)),
            Some(t::offer_line(language, offer)),
        ),
        (None, false, None) => return None,
    };
    label(ui, over);
    if let Some(name) = name {
        ui.heading(Words::from(name).shown(ui.ctx()));
    }
    if let Some(line) = line {
        ui.label(RichText::new(line).color(INK_SECONDARY));
    }
    ui.add_space(SPACE);
    let start = pill_button(ui, t::START, true);
    start
        .clicked()
        .then_some(Tapped::Start(Start::Next, start.rect))
}

/// The review on the side while a redo waits: a rule, its skill and two exercises, Review;
/// where Review lies when it was tapped.
pub(super) fn review(ui: &mut egui::Ui, path: &PathProjection) -> Option<Rect> {
    let offer = path.review.as_ref()?;
    let language = t::language(ui.ctx());
    ui.add_space(SPACE);
    rule(ui);
    label(ui, t::ON_THE_SIDE);
    let tapped = ui
        .horizontal(|ui| {
            ui.label(t::review_line(language, offer));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                text_button(ui, t::REVIEW, true)
            })
            .inner
        })
        .inner;
    label(ui, t::REDO_WAITS);
    tapped.clicked().then_some(tapped.rect)
}

/// A hairline across the column.
pub(super) fn rule(ui: &mut egui::Ui) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y, (1.0, DIVIDER));
}

/// All units and Practice book at the foot of `inner`.
fn foot(ui: &mut egui::Ui, inner: Rect) -> Option<Tapped> {
    let row = Rect::from_min_max(
        inner.left_bottom() - Vec2::new(0.0, TOUCH_TARGET),
        inner.max,
    );
    let mut foot = ui.new_child(
        UiBuilder::new()
            .id_salt("practice column foot")
            .max_rect(row)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let units = text_button(&mut foot, t::ALL_UNITS, true);
    let book = text_button(&mut foot, t::PRACTICE_BOOK, true);
    if units.clicked() {
        Some(Tapped::Path(units.rect))
    } else {
        book.clicked().then_some(Tapped::Book(book.rect))
    }
}
