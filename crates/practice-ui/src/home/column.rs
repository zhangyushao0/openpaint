//! The home's practice column (S122), the entrance beside the studio's: "Practice" with the unit
//! in hand beside it and the practice options' button at its right; under them a sheet of paper
//! with the next lesson (its skill, its level, exercises and minutes, Start in its pill), for a
//! fresh learner the first lesson, while today's lesson is unfinished its name and place
//! ("Lines 3/6") and Continue, a unit's start re-test in its place while due (S147); under a rule,
//! while a redo waits, the review on the side (its skill, two exercises; Review); while a unit's
//! end re-test is due its row (Re-test); under another the unit in hand's skills as small tiles with
//! their dots, every one (`tiles.rs`; a tap starts that skill's lesson, as on the path); at the
//! sheet's foot All units (the path page grows out of it) and Practice book. No counts of days, no
//! quotas. Under the next lesson the Pomodoro's row, and at the header's right today's seals
//! (`pomodoro.rs`, S124). The sheet is `SHEET` tall where the column allows, longer only while what
//! it holds needs it (S156), never past the column.

use egui::{
    Align, Align2, Color32, Layout, Rect, RichText, Shape, TextStyle, UiBuilder, Vec2, pos2,
};
use openpaint_practice_app::{BlockShown, PathProjection, Start, TimerIntent, TimerShown};
use openpaint_ui::components::carry::Opening;
use openpaint_ui::components::{icon_button, pill_button, text_button};
use openpaint_ui::design::{DIVIDER, INK, INK_SECONDARY, Icon, SPACE, TOUCH_TARGET, sheet_shadow};

use super::pomodoro::{pomodoro_row, today_seals};
use super::tiles::unit_tiles;
use super::{FIRST, HomePart, Taps};
use crate::text::{self as t, Text, Words};

/// What the column's sheet holds in from its edges, in points.
const MARGIN: f32 = 4.0 * SPACE;
/// The sheet's height while what it holds fits, in points: the studio beside it keeps the eye.
const SHEET: f32 = 600.0;

/// What a tap in the column asks.
enum Tapped {
    Start(Start, Rect),
    Timer(TimerIntent, Rect),
    Path(Rect),
    Book(Rect),
    Options,
}

impl HomePart {
    /// The column in `rect` for `path` and the Pomodoro's `timer`, its taps to `taps` and the
    /// timer's to `timed`; returns where Practice book lies: a sheet left goes back into the book,
    /// where the home keeps every sheet's picture (S144).
    pub(super) fn column(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        (path, timer, enabled): (&PathProjection, &TimerShown, bool),
        (taps, timed): (Taps<'_>, &mut Vec<TimerIntent>),
    ) -> Rect {
        let head = Rect::from_min_size(rect.min, Vec2::new(rect.width(), TOUCH_TARGET));
        let room = Rect::from_min_max(rect.min + Vec2::new(0.0, TOUCH_TARGET + SPACE), rect.max);
        let options = heading(ui, head, path, (enabled, timer.sealed));
        // The sheet lies behind what it holds, laid once its length is known.
        let behind = ui.painter().add(Shape::Noop);
        let inner = room.shrink(MARGIN);
        let least = inner.bottom().min(room.top() + SHEET - MARGIN);
        let mut page = ui.new_child(
            UiBuilder::new()
                .id_salt("practice column")
                .max_rect(inner)
                .layout(Layout::top_down(Align::Min)),
        );
        page.spacing_mut().item_spacing = Vec2::new(SPACE, SPACE);
        let (tapped, book, bottom) = page
            .add_enabled_ui(enabled, |ui| {
                // While a block is under way the row's Continue opens its lesson.
                let timed = (timer.block.as_ref()).is_some_and(|block| {
                    matches!(block, BlockShown::Running { .. } | BlockShown::Up)
                });
                let next = next_lesson(ui, path, !timed);
                let next = next.map(|from| Tapped::Start(Start::Next, from));
                let offered = path.open.is_some() || path.first || path.next.is_some();
                let row = offered.then(|| pomodoro_row(ui, timer)).flatten();
                let row = row.map(|(intent, from)| Tapped::Timer(intent, from));
                let reviewed = review(ui, path).map(|from| Tapped::Start(Start::Review, from));
                let retested = retest_row(ui, path);
                let tile = unit_tiles(ui, path, inner.bottom() - TOUCH_TARGET - SPACE);
                let tile = tile.map(|(skill, from)| Tapped::Start(Start::Skill(skill), from));
                let tapped = reviewed.or(retested).or(next).or(row).or(tile);
                // The foot under the tiles, at the sheet's least length's foot while they fit.
                let bottom = (ui.cursor().top() + TOUCH_TARGET).max(least);
                let under = Rect::from_x_y_ranges(inner.x_range(), bottom - TOUCH_TARGET..=bottom);
                let (foot, book) = foot(ui, under);
                (foot.or(tapped), book, bottom)
            })
            .inner;
        let sheet = Rect::from_min_max(room.min, pos2(room.right(), bottom + MARGIN));
        ui.painter().set(behind, sheet_shape(sheet));
        if let Some(tapped) = tapped.or(options.then_some(Tapped::Options)) {
            self.column_tap(tapped, rect, path, (taps, timed));
        }
        book
    }

    fn column_tap(
        &mut self,
        tapped: Tapped,
        rect: Rect,
        path: &PathProjection,
        (taps, timed): (Taps<'_>, &mut Vec<TimerIntent>),
    ) {
        match tapped {
            Tapped::Start(start, from) => self.start(start, from, path, taps),
            Tapped::Timer(intent, from) => {
                if intent == TimerIntent::Start {
                    // The sheet grows out of Start timer or Continue.
                    let id = path.open.map(|open| open.sheet);
                    let picture = None;
                    self.opening = Some(Opening { from, id, picture });
                }
                timed.push(intent);
            }
            Tapped::Path(from) => self.path.open(from, path.unit),
            Tapped::Book(from) => self.book.open(from),
            Tapped::Options => self.practice.open(rect),
        }
    }
}

/// "Practice" with the unit in hand beside it, quieter, and the options' button at the right,
/// today's seals left of it; whether the button was tapped.
fn heading(
    ui: &mut egui::Ui,
    head: Rect,
    path: &PathProjection,
    (enabled, sealed): (bool, u8),
) -> bool {
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
    let tapped = icon_button(&mut right, Icon::More, t::PRACTICE_OPTIONS, enabled).clicked();
    today_seals(&mut right, sealed);
    tapped
}

/// A quiet label over a block.
pub(super) fn label(ui: &mut egui::Ui, text: Text) {
    let shown = Words::from(text).shown(ui.ctx());
    ui.label(RichText::new(shown).small().color(INK_SECONDARY));
}

/// The next lesson, on the home and the path page: today's lesson while it is unfinished (its
/// name and place, "Lines 3/6", and Continue), the first lesson for a fresh learner, else the
/// lesson offered (its skill, level, exercises and minutes) with Start (no pill but while `go`);
/// where the pill lies when it was tapped (`Start::Next`, which opens the unfinished lesson's
/// sheet).
pub(super) fn next_lesson(ui: &mut egui::Ui, path: &PathProjection, go: bool) -> Option<Rect> {
    let language = t::language(ui.ctx());
    let (over, name, line, word) = match (&path.open, path.first, &path.next) {
        (Some(open), _, _) => (
            t::UNDER_WAY,
            t::open_lesson(language, open),
            None,
            t::CONTINUE,
        ),
        (None, true, _) => (
            t::NEXT_LESSON,
            Words::from(t::FIRST_LESSON).shown(ui.ctx()).to_owned(),
            Some(t::exercises_line(language, FIRST)),
            t::START,
        ),
        // A unit's start re-test in the next lesson's place while due (S147).
        (None, false, _) if path.retest.is_some() => {
            let retest = path.retest.as_ref()?;
            (
                t::NEXT_LESSON,
                t::retest_name(language, retest.unit),
                Some(t::retest_line(language, retest)),
                t::START,
            )
        }
        (None, false, Some(offer)) => (
            t::NEXT_LESSON,
            Words::from(t::skill_name(offer.skill))
                .shown(ui.ctx())
                .to_owned(),
            Some(t::offer_line(language, offer)),
            t::START,
        ),
        (None, false, None) => return None,
    };
    label(ui, over);
    ui.heading(name);
    if let Some(line) = line {
        ui.label(RichText::new(line).color(INK_SECONDARY));
    }
    if !go {
        return None;
    }
    ui.add_space(SPACE);
    let pill = pill_button(ui, word, true);
    pill.clicked().then_some(pill.rect)
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

/// A unit's end re-test while due (S147): a rule, its name and line, Re-test; what was tapped.
fn retest_row(ui: &mut egui::Ui, path: &PathProjection) -> Option<Tapped> {
    let retest = path.retest_end.as_ref()?;
    let language = t::language(ui.ctx());
    ui.add_space(SPACE);
    rule(ui);
    let tapped = ui
        .horizontal(|ui| {
            ui.label(t::retest_name(language, retest.unit));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                text_button(ui, t::RETEST, true)
            })
            .inner
        })
        .inner;
    let line = t::retest_line(language, retest);
    ui.label(RichText::new(line).small().color(INK_SECONDARY));
    let start = Start::Retest(retest.unit, true);
    tapped
        .clicked()
        .then_some(Tapped::Start(start, tapped.rect))
}

/// The sheet's shapes at `sheet`: its shadow on the desk, its paper.
fn sheet_shape(sheet: Rect) -> Shape {
    let corners = [
        sheet.left_top(),
        sheet.right_top(),
        sheet.right_bottom(),
        sheet.left_bottom(),
    ];
    let [contact, soft] = sheet_shadow(corners);
    Shape::Vec(vec![
        contact,
        soft,
        Shape::rect_filled(sheet, 0.0, Color32::WHITE),
    ])
}

/// A hairline across the column.
pub(super) fn rule(ui: &mut egui::Ui) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y, (1.0, DIVIDER));
}

/// All units and Practice book in `row`, the sheet's foot; what was tapped, and where Practice
/// book lies.
fn foot(ui: &mut egui::Ui, row: Rect) -> (Option<Tapped>, Rect) {
    let mut foot = ui.new_child(
        UiBuilder::new()
            .id_salt("practice column foot")
            .max_rect(row)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let units = text_button(&mut foot, t::ALL_UNITS, true);
    let book = text_button(&mut foot, t::PRACTICE_BOOK, true);
    let tapped = if units.clicked() {
        Some(Tapped::Path(units.rect))
    } else {
        book.clicked().then_some(Tapped::Book(book.rect))
    };
    (tapped, book.rect)
}
