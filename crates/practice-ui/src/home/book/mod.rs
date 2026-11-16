//! The practice book (S89): Practice book on the home's column or the path page opens it, a page
//! of paper grown out of the word over the home, Gallery (its island) or the back button putting it
//! back. At its top its name, its two pages (Calendar, Progress; S147) and, quietly, the lessons
//! done and the time practised; on the calendar's page the month's calendar (`days.rs`; a day
//! tapped shows its card over what lies beside it, `day.rs`: its time, its sheets, which open, and
//! its colours) and beside it the first and latest lines and the seals (`progress/`); on the
//! Progress page each skill's curve (`progress/curves.rs`). No legends, no days in a row, nothing
//! to fill: what was done, drawn.

mod day;
mod days;
mod progress;
#[cfg(test)]
mod tests;

use std::collections::HashSet;

use egui::{Align, Align2, Id, Layout, Order, Pos2, Rect, Sense, TextStyle, UiBuilder};
use openpaint_app::{GalleryIntent, Intent};
use openpaint_practice_app::{BookProjection, OptionsIntent, PathProjection};
use openpaint_ui::components::carry::Opening;
use openpaint_ui::components::chrome::{self, Source};
use openpaint_ui::components::{
    ISLAND_TOP, Place, UNDER_ISLANDS, choices, island, paper, text_button, text_choice,
};
use openpaint_ui::dates::Date;
use openpaint_ui::design::{INK, INK_SECONDARY, SPACE, TOUCH_TARGET};

use super::HomePart;
use crate::text::{self as t, Language, Words};

/// The width the two pages' words take at the top band's right, points.
const PAGES: f32 = 240.0;

/// The book's state while the home shows.
#[derive(Debug, Default)]
pub(crate) struct BookState {
    /// Where it grew from (Practice book, points) while it shows.
    from: Option<Rect>,
    /// It has grown over the whole home (the last pass).
    covers: bool,
    /// The month shown, by its first day; none: today's.
    month: Option<Date>,
    /// The day whose card shows, and its cell (points).
    day: Option<(i64, Rect)>,
    /// The records whose sheets were asked of the store.
    asked: HashSet<u64>,
    /// How far a finger has swiped across the calendar so far (points, rightwards).
    swipe: f32,
    /// The Progress page shows (S147), else the calendar; the curves were asked of the store and
    /// not come yet; the copies' cards asked.
    progress: bool,
    refitting: bool,
    cards: HashSet<openpaint_practice::CopyRef>,
}

impl BookState {
    /// Opens it, grown out of `from`, on today's month.
    pub(in crate::home) fn open(&mut self, from: Rect) {
        *self = Self {
            from: Some(from),
            ..Self::default()
        };
    }

    /// Closes it. Returns whether it was open.
    pub(in crate::home) fn close(&mut self) -> bool {
        let open = self.from.is_some();
        (self.from, self.day, self.covers) = (None, None, false);
        open
    }

    pub(in crate::home) const fn shows(&self) -> bool {
        self.from.is_some()
    }

    /// It lies over the whole home: nothing of the home needs drawing.
    pub(in crate::home) const fn covers(&self) -> bool {
        self.from.is_some() && self.covers
    }
}

/// What a tap on the page asks.
pub(super) enum BookTap {
    /// A day's card, from its cell (points), or none.
    Day(Option<(i64, Rect)>),
    /// The month shown, by its first day.
    Month(Date),
    /// A sheet opened, growing out of its picture (points).
    Open(openpaint_app::ProjectId, Rect),
    /// The Progress page (true) or the calendar (S147).
    Progress(bool),
}

impl HomePart {
    /// The book's island: Gallery, which puts the book back.
    pub(in crate::home) fn book_island(&mut self, ctx: &egui::Context) {
        let (back, _) = island(ctx, "book island", Place::top_left(), |ui| {
            text_button(ui, t::HOME, true).clicked()
        });
        if back {
            self.book.close();
        }
    }

    /// The book's page while it shows, grown out of Practice book, with the day's card over it.
    pub(in crate::home) fn book_page(
        &mut self,
        ctx: &egui::Context,
        path: &PathProjection,
        (taps, options): (&mut Vec<Intent>, &mut Vec<OptionsIntent>),
    ) {
        let Some(from) = self.book.from else {
            return;
        };
        self.ask_sheets(&path.book, options);
        let screen = ctx.content_rect();
        let id = Id::new("practice book");
        let source = Source::Rect {
            rect: from,
            radius: (0.0, 0.0),
            veil: false,
        };
        let grown = chrome::declare(ctx, id, source, Order::Middle, true).map_or(1.0, |s| s.grown);
        self.book.covers = grown >= 1.0;
        let area = egui::Area::new(id)
            .order(Order::Middle)
            .movable(false)
            .constrain(false)
            .fade_in(false)
            .fixed_pos(screen.min);
        let (month, shown) = (self.book.month, self.book.progress);
        let (tapped, _) = paper(ctx, id, source, area, |ui| {
            ui.set_min_size(screen.size());
            page(ui, screen, (&path.book, month, shown), &mut self.book.swipe)
        });
        let chosen = self.book.day.filter(|_| !shown);
        let carded = chosen.and_then(|day| day::day_card(ctx, &path.book, day, screen));
        match carded.or(tapped) {
            Some(BookTap::Day(day)) => self.book.day = day,
            Some(BookTap::Month(first)) => (self.book.month, self.book.day) = (Some(first), None),
            Some(BookTap::Progress(on)) => (self.book.progress, self.book.day) = (on, None),
            Some(BookTap::Open(sheet, from)) => {
                taps.push(Intent::Gallery(GalleryIntent::Open(sheet)));
                self.opening = Some(Opening {
                    from,
                    id: Some(sheet),
                    picture: None,
                });
            }
            None => {}
        }
    }

    /// Asks the store for the sheets the page shows (the lines' lessons, the day's) not read
    /// or asked yet; on the Progress page for the curves while none are fitted for the records
    /// as they stand, and for the re-test pairs' copies (S147).
    fn ask_sheets(&mut self, book: &BookProjection, taps: &mut Vec<OptionsIntent>) {
        let mut wanted: Vec<u64> = book.book.lines.iter().map(|line| line.record).collect();
        if let Some(day) = self.book.day.and_then(|(day, _)| book.book.day(day)) {
            wanted.extend(&day.records);
        }
        let asked = &mut self.book.asked;
        let new: Vec<u64> = (wanted.into_iter())
            .filter(|id| !book.read.contains_key(id) && asked.insert(*id))
            .collect();
        if !new.is_empty() {
            taps.push(OptionsIntent::ReadSheets(new));
        }
        if !self.book.progress {
            return;
        }
        if book.progress.is_some() {
            self.book.refitting = false;
        } else if !self.book.refitting {
            self.book.refitting = true;
            taps.push(OptionsIntent::Refit);
        }
        let cards = &mut self.book.cards;
        let copies: Vec<_> = (progress::cards_wanted(book).into_iter())
            .filter(|copy| cards.insert(*copy))
            .collect();
        if !copies.is_empty() {
            taps.push(OptionsIntent::ReadCards(copies));
        }
    }
}

/// The page on `screen`: its head, then the calendar with the lines and seals beside it or, on
/// the Progress page, the curves; what was tapped.
fn page(
    ui: &mut egui::Ui,
    screen: Rect,
    (book, month, curves): (&BookProjection, Option<Date>, bool),
    swipe: &mut f32,
) -> Option<BookTap> {
    let away = ui.interact(screen, ui.id().with("page"), Sense::click());
    let language = t::language(ui.ctx());
    let switched = head(ui, screen, (book, curves), language);
    if curves {
        progress::curves(ui, under(screen), book);
        return switched;
    }
    let [calendar, side] = layout(screen);
    let shown = month.unwrap_or_else(|| Date::of(book.today).first_of_month());
    let tapped = days::calendar(ui, calendar, (book, shown), swipe);
    progress::beside(ui, side, book);
    switched
        .or(tapped)
        .or_else(|| away.clicked().then_some(BookTap::Day(None)))
}

/// Where the calendar and progress lie on `screen` (points): side by side when the page is wide,
/// else one over the other.
pub(super) fn layout(screen: Rect) -> [Rect; 2] {
    let below = under(screen);
    if below.width() >= below.height() {
        let middle = below.center().x;
        let calendar =
            Rect::from_min_max(below.min, Pos2::new(middle - 2.0 * SPACE, below.bottom()));
        let side = Rect::from_min_max(Pos2::new(middle + 2.0 * SPACE, below.top()), below.max);
        [calendar, side]
    } else {
        let middle = below.top() + below.height() * 0.5;
        let calendar = Rect::from_min_max(below.min, Pos2::new(below.right(), middle - SPACE));
        let side = Rect::from_min_max(Pos2::new(below.left(), middle + SPACE), below.max);
        [calendar, side]
    }
}

/// What lies under the top band on `screen`, inside the page's margins.
fn under(screen: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(screen.left() + 3.0 * SPACE, UNDER_ISLANDS + 3.0 * SPACE),
        Pos2::new(screen.right() - 3.0 * SPACE, screen.bottom() - 3.0 * SPACE),
    )
}

/// The page's name in the top band's middle and at its right its two pages, Calendar and
/// Progress, the one shown in its pill (S147), quietly left of them the lessons done and the time
/// practised; the page tapped.
fn head(
    ui: &mut egui::Ui,
    screen: Rect,
    (book, curves): (&BookProjection, bool),
    language: Language,
) -> Option<BookTap> {
    let y = ISLAND_TOP + TOUCH_TARGET / 2.0;
    let heading = TextStyle::Heading.resolve(ui.style());
    let name = Words::from(t::PRACTICE_BOOK).shown(ui.ctx());
    let centre = Pos2::new(screen.center().x, y);
    ui.painter()
        .text(centre, Align2::CENTER_CENTER, name, heading, INK);
    let right = screen.right() - 3.0 * SPACE;
    let band = Rect::from_min_max(
        Pos2::new(right - PAGES, ISLAND_TOP),
        Pos2::new(right, ISLAND_TOP + TOUCH_TARGET),
    );
    let mut pages = ui.new_child(
        UiBuilder::new()
            .id_salt("book pages")
            .max_rect(band)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let mut tapped = None;
    choices(&mut pages, "book pages", |ui, group| {
        if text_choice(ui, group, t::CALENDAR, !curves).clicked() {
            tapped = Some(BookTap::Progress(false));
        }
        if text_choice(ui, group, t::PROGRESS, curves).clicked() {
            tapped = Some(BookTap::Progress(true));
        }
    });
    let totals = t::book_totals(language, book.book.lessons, book.book.seconds);
    let body = TextStyle::Body.resolve(ui.style());
    let at = Pos2::new(band.left() - 2.0 * SPACE, y);
    ui.painter()
        .text(at, Align2::RIGHT_CENTER, totals, body, INK_SECONDARY);
    tapped
}
