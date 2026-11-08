//! The practice book (S89): the Today card's sheet opens it, a page of paper grown out of the card
//! over the gallery, Gallery (its island) or the back button putting it back. At its top its name
//! and, quietly, the sessions done and the time practised; under them the road through V大's eight
//! stages (`paint_road`); then the month's calendar (`gallery_book_days.rs`; a day tapped shows its
//! card: its time, its sheets, which open, and its colours) and, beside it, progress seen
//! (`gallery_book_progress.rs`: the colour's first picks, the first and latest lines, the seals).
//! No legends, no days in a row, nothing to fill: what was done, drawn.
//!
//! The Today card (its road; its action word starting or continuing practice) is here too.

#[path = "gallery_book_days.rs"]
mod days;
#[path = "gallery_book_progress.rs"]
mod progress;

use std::collections::HashSet;

use egui::{Align2, Id, Order, Pos2, Rect, Sense, TextStyle, Vec2};
use openpaint_app::{GalleryIntent, Intent};
use openpaint_practice::{Date, Road, STAGE_SESSIONS, STAGES, STAGES_READY, Seal};
use openpaint_practice_app::{BookProjection, OptionsIntent, TodayProjection};

use super::GalleryScreen;
use crate::components::carry::Opening;
use crate::components::chrome::{self, Source};
use crate::components::{
    Dot, ISLAND_TOP, Place, TodayCard, UNDER_ISLANDS, curve, curve_y, inked, island, paint_dot,
    paint_seal, paper, pencilled, seal_turn, text_button, today_card,
};
use crate::design::{INK, INK_SECONDARY, PANEL, PENCIL, SPACE, TOUCH_TARGET};
use crate::text::{self as t, Language, Words};

/// The road's band, in points.
const ROAD_HEIGHT: f32 = 152.0;
/// A stage's node's radius, and the room a stage done and one ahead take along the road.
const NODE: f32 = 15.0;
const STAGE_BEHIND: f32 = 132.0;
const STAGE_AHEAD: f32 = 64.0;

/// The book's state while the gallery shows.
#[derive(Debug, Default)]
pub(crate) struct BookState {
    /// Where it grew from (the Today card's sheet, points) while it shows.
    from: Option<Rect>,
    /// It has grown over the whole gallery (the last pass).
    covers: bool,
    /// The month shown, by its first day; none: today's.
    month: Option<Date>,
    /// The day whose card shows, and its cell (points).
    day: Option<(i64, Rect)>,
    /// The records whose sheets were asked of the store.
    asked: HashSet<u64>,
    /// How far a finger has swiped across the calendar so far (points, rightwards).
    swipe: f32,
}

impl BookState {
    /// Opens it, grown out of `from`, on today's month.
    pub(super) fn open(&mut self, from: Rect) {
        *self = Self {
            from: Some(from),
            ..Self::default()
        };
    }

    /// Closes it. Returns whether it was open.
    pub(super) fn close(&mut self) -> bool {
        let open = self.from.is_some();
        (self.from, self.day, self.covers) = (None, None, false);
        open
    }

    pub(super) const fn shows(&self) -> bool {
        self.from.is_some()
    }

    /// It lies over the whole gallery: nothing of the gallery needs drawing.
    pub(super) const fn covers(&self) -> bool {
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
}

impl GalleryScreen {
    /// The Today card in `rect`; returns where its sheet lies. Its sheet opens the practice
    /// book; its action word continues the open session (opening its sheet) or starts one of its
    /// stage, or of the next once this one's sessions are done, the sheet growing out of the
    /// card; its options open the practice options.
    pub(super) fn today(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        (today, enabled): (&TodayProjection, bool),
        (taps, options): (&mut Vec<Intent>, &mut Vec<OptionsIntent>),
    ) -> Rect {
        let mut slot = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let stage = t::stage(t::language(ui.ctx()), today.stage);
        let (action, starts) = match (today.open, today.next) {
            (Some(_), _) => (t::CONTINUE_PRACTICE, today.stage),
            (None, Some(next)) => (t::START_NEXT_STAGE, next),
            (None, None) => (t::START_PRACTICE, today.stage),
        };
        let pictures = today.packs.iter().any(|pack| pack.pictures > 0);
        let card = TodayCard {
            stage: &stage,
            road: (today.done, today.sessions),
            hint: (today.stage == 2 && !pictures).then_some(t::SHAPES_NEED_PACK),
            action,
            enabled,
        };
        let tapped = today_card(&mut slot, &card);
        if tapped.action {
            match today.open {
                Some(sheet) => taps.push(Intent::Gallery(GalleryIntent::Open(sheet))),
                None => options.push(OptionsIntent::Practice(starts)),
            }
            self.opening = Some(Opening {
                from: tapped.rect,
                id: today.open,
                picture: None,
            });
        } else if tapped.book {
            self.book.open(tapped.rect);
        } else if tapped.options {
            self.practice.open(rect);
        }
        tapped.rect
    }

    /// The book's island: Gallery, which puts the book back into the Today card.
    pub(super) fn book_island(&mut self, ctx: &egui::Context) {
        let (back, _) = island(ctx, "book island", Place::top_left(), |ui| {
            text_button(ui, t::GALLERY, true).clicked()
        });
        if back {
            self.book.close();
        }
    }

    /// The book's page while it shows, grown out of the Today card, with the day's card over it.
    pub(super) fn book_page(
        &mut self,
        ctx: &egui::Context,
        today: &TodayProjection,
        (taps, options): (&mut Vec<Intent>, &mut Vec<OptionsIntent>),
    ) {
        let Some(from) = self.book.from else {
            return;
        };
        self.ask_sheets(&today.book, options);
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
        let month = self.book.month;
        let (tapped, _) = paper(ctx, id, source, area, |ui| {
            ui.set_min_size(screen.size());
            page(ui, screen, (today, month, &mut self.book.swipe))
        });
        let chosen = self.book.day;
        let carded = chosen.and_then(|day| days::day_card(ctx, &today.book, day, screen));
        match carded.or(tapped) {
            Some(BookTap::Day(day)) => self.book.day = day,
            Some(BookTap::Month(first)) => (self.book.month, self.book.day) = (Some(first), None),
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

    /// Asks the store for the sheets the page shows (the lines' sessions, the day's) not read
    /// or asked yet.
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
    }
}

/// The page on `screen`: its head, the road, the calendar and progress; what was tapped.
fn page(
    ui: &mut egui::Ui,
    screen: Rect,
    (today, month, swipe): (&TodayProjection, Option<Date>, &mut f32),
) -> Option<BookTap> {
    let away = ui.interact(screen, ui.id().with("page"), Sense::click());
    let book = &today.book;
    let language = t::language(ui.ctx());
    head(ui, screen, book, language);
    let [road, calendar, side] = layout(screen);
    paint_road(ui, road, &book.book.road, language);
    let shown = month.unwrap_or_else(|| Date::of(book.today).first_of_month());
    let tapped = days::calendar(ui, calendar, (book, shown), swipe);
    progress::progress(ui, side, book);
    tapped.or_else(|| away.clicked().then_some(BookTap::Day(None)))
}

/// Where the road, the calendar and progress lie on `screen` (points).
pub(super) fn layout(screen: Rect) -> [Rect; 3] {
    let road = Rect::from_min_size(
        Pos2::new(screen.left() + 3.0 * SPACE, UNDER_ISLANDS + SPACE),
        Vec2::new(screen.width() - 6.0 * SPACE, ROAD_HEIGHT),
    );
    let below = Rect::from_min_max(
        Pos2::new(road.left(), road.bottom() + 3.0 * SPACE),
        Pos2::new(road.right(), screen.bottom() - 3.0 * SPACE),
    );
    let (calendar, side) = columns(below);
    [road, calendar, side]
}

/// The calendar's room and progress's beside it (side by side when the page is wide, else one
/// over the other).
fn columns(below: Rect) -> (Rect, Rect) {
    if below.width() >= below.height() {
        let middle = below.center().x;
        let calendar =
            Rect::from_min_max(below.min, Pos2::new(middle - 2.0 * SPACE, below.bottom()));
        let side = Rect::from_min_max(Pos2::new(middle + 2.0 * SPACE, below.top()), below.max);
        (calendar, side)
    } else {
        let middle = below.top() + below.height() * 0.5;
        let calendar = Rect::from_min_max(below.min, Pos2::new(below.right(), middle - SPACE));
        let side = Rect::from_min_max(Pos2::new(below.left(), middle + SPACE), below.max);
        (calendar, side)
    }
}

/// The page's name in the top band's middle and, quietly at its right, the sessions done and the
/// time practised.
fn head(ui: &egui::Ui, screen: Rect, book: &BookProjection, language: Language) {
    let painter = ui.painter();
    let y = ISLAND_TOP + TOUCH_TARGET / 2.0;
    let heading = TextStyle::Heading.resolve(ui.style());
    let name = Words::from(t::PRACTICE_BOOK).shown(ui.ctx());
    painter.text(
        Pos2::new(screen.center().x, y),
        Align2::CENTER_CENTER,
        name,
        heading,
        INK,
    );
    let totals = t::book_totals(language, book.book.sessions, book.book.seconds);
    let body = TextStyle::Body.resolve(ui.style());
    let right = Pos2::new(screen.right() - 3.0 * SPACE, y);
    painter.text(right, Align2::RIGHT_CENTER, totals, body, INK_SECONDARY);
}

/// The road through the stages across `rect`: the stages behind and the one practised inked at
/// the left, its sessions along a long stretch, the stages ahead in pencil, closer together, at
/// the right; the stages there are by name, later ones by number; a finished stage sealed.
fn paint_road(ui: &egui::Ui, rect: Rect, road: &Road, language: Language) {
    let painter = ui.painter();
    let stage = road.stage.clamp(1, STAGES);
    let (left, right) = (rect.left() + 8.0 * SPACE, rect.right() - NODE - SPACE);
    let node_x = |s: u8| {
        if s <= stage {
            left + f32::from(s - 1) * STAGE_BEHIND
        } else {
            right - f32::from(STAGES - s) * STAGE_AHEAD
        }
    };
    let done = road.done_in(stage);
    let count = STAGE_SESSIONS.max(done + 1);
    let end = if stage < STAGES {
        node_x(stage + 1) - 2.0 * NODE
    } else {
        right
    };
    let start = node_x(stage) + 2.0 * NODE;
    let step = (end - start) / (count - 1) as f32;
    let dots: Vec<Pos2> = (0..count)
        .map(|index| {
            let x = start + step * index as f32;
            Pos2::new(x, curve_y(rect, x))
        })
        .collect();
    pencilled(painter, &curve(rect, left, right));
    let inked_to = done
        .checked_sub(1)
        .map_or(node_x(stage), |last| dots[last as usize].x);
    inked(painter, curve(rect, left, inked_to));
    for (index, at) in dots.iter().enumerate() {
        let dot = match index as u32 {
            i if i < done => Dot::Done,
            i if i == done => Dot::Next,
            _ => Dot::Ahead,
        };
        paint_dot(painter, *at, dot);
    }
    if let Some(next) = dots.get(done as usize) {
        let small = TextStyle::Small.resolve(ui.style());
        let word = t::NEXT_SESSION.get(language);
        let above = *next - Vec2::new(0.0, 2.5 * SPACE);
        painter.text(above, Align2::CENTER_BOTTOM, word, small, INK_SECONDARY);
    }
    for s in 1..=STAGES {
        let x = node_x(s);
        let finished = road.done_in(s) >= STAGE_SESSIONS;
        paint_node(
            ui,
            Pos2::new(x, curve_y(rect, x)),
            (s, s <= stage, finished),
            language,
        );
    }
}

/// Stage `stage`'s node at `at`: inked when reached, else in pencil; by name below while the app
/// holds the stage; sealed once finished.
fn paint_node(
    ui: &egui::Ui,
    at: Pos2,
    (stage, reached, finished): (u8, bool, bool),
    language: Language,
) {
    let painter = ui.painter();
    let ink = if reached { INK } else { PENCIL };
    painter.circle(at, NODE, PANEL, egui::Stroke::new(1.5, ink));
    let body = TextStyle::Body.resolve(ui.style());
    painter.text(at, Align2::CENTER_CENTER, stage.to_string(), body, ink);
    if stage <= STAGES_READY {
        let topic = t::stage_topic(stage);
        let small = TextStyle::Small.resolve(ui.style());
        let below = at + Vec2::new(0.0, NODE + SPACE);
        let quiet = if reached { INK_SECONDARY } else { PENCIL };
        painter.text(below, Align2::CENTER_TOP, topic.get(language), small, quiet);
    }
    if finished {
        let lines = t::seal_lines(language, Seal::Stage(stage));
        let at = at + Vec2::new(NODE * 0.9, -NODE * 0.9);
        paint_seal(
            painter,
            (at, 2.0 * NODE),
            (&lines, seal_turn(usize::from(stage))),
            1.0,
        );
    }
}
