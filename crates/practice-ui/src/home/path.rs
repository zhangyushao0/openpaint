//! The path page (S122), grown out of All units over the whole home, Home (its island) or the
//! back button putting it back: its name in the top band; across the top the eight units as rings
//! on a pencilled line, the one shown ringed in ink, those open in ink and the rest in pencil, each
//! a tap away (a unit not open yet is entered all the same); under them the unit shown, its skills
//! as tiles (`components/skill_tile.rs`: each skill's three dots, the next lesson's ringed, one
//! lacking its pictures greyed with what it lacks; a tile tapped starts its lesson), the seals
//! earned and Practice book; beside them the next lesson, the review on the side and the unit's
//! full lesson (about as many minutes as its kinds take: a warm-up, then each skill of the unit).

use egui::{Align, Align2, Id, Layout, Order, Pos2, Rect, Stroke, TextStyle, UiBuilder, Vec2};
use openpaint_app::Language;
use openpaint_practice::Offer;
use openpaint_practice_app::{PathProjection, Start, UnitShown};
use openpaint_ui::components::chrome::{self, Source};
use openpaint_ui::components::{
    ISLAND_TOP, Place, UNDER_ISLANDS, controls, island, paper, text_button,
};
use openpaint_ui::design::{
    INK, INK_SECONDARY, Outline, PANEL, SPACE, TOUCH_TARGET, paint_mark, panel_frame,
};

use super::column::{label, next_lesson, review, rule};
use super::{HomePart, Taps};
use crate::components::{SkillTile, paint_seal, seal_turn, skill_tile};
use crate::palette::PENCIL;
use crate::text::{self as t, Words};

/// The units' band's height, a unit's ring's radius, the side column's width, a tile's width at
/// most and a seal's side, in points.
const UNITS: f32 = 13.0 * SPACE;
const RING: f32 = 18.0;
const SIDE: f32 = 400.0;
const TILE: f32 = 260.0;
const SEAL: f32 = 56.0;

/// The path page's state while the home shows.
#[derive(Debug, Default)]
pub(crate) struct PathPage {
    /// Where it grew from (All units, points) while it shows.
    from: Option<Rect>,
    /// It has grown over the whole home (the last pass).
    covers: bool,
    /// The unit shown.
    unit: u8,
}

impl PathPage {
    /// Opens it, grown out of `from`, on `unit`.
    pub(super) fn open(&mut self, from: Rect, unit: u8) {
        *self = Self {
            from: Some(from),
            covers: false,
            unit: unit.max(1),
        };
    }

    /// Closes it. Returns whether it was open.
    pub(super) fn close(&mut self) -> bool {
        let open = self.from.is_some();
        (self.from, self.covers) = (None, false);
        open
    }

    pub(super) const fn shows(&self) -> bool {
        self.from.is_some()
    }

    /// It lies over the whole home: nothing of the home needs drawing.
    pub(super) const fn covers(&self) -> bool {
        self.from.is_some() && self.covers
    }
}

/// What a tap on the page asks.
enum Tapped {
    Unit(u8),
    Start(Start, Rect),
    Book(Rect),
}

impl HomePart {
    /// The path page's island: Home, which puts the page back.
    pub(super) fn path_island(&mut self, ctx: &egui::Context) {
        let (back, _) = island(ctx, "path island", Place::top_left(), |ui| {
            text_button(ui, t::HOME, true).clicked()
        });
        if back {
            self.path.close();
        }
    }

    /// The page while it shows, grown out of All units.
    pub(super) fn path_page(&mut self, ctx: &egui::Context, path: &PathProjection, taps: Taps<'_>) {
        let Some(from) = self.path.from else {
            return;
        };
        let screen = ctx.content_rect();
        let id = Id::new("practice path");
        let source = Source::Rect {
            rect: from,
            radius: (0.0, 0.0),
            veil: false,
        };
        let grown = chrome::declare(ctx, id, source, Order::Middle, true).map_or(1.0, |s| s.grown);
        self.path.covers = grown >= 1.0;
        let area = egui::Area::new(id)
            .order(Order::Middle)
            .movable(false)
            .constrain(false)
            .fade_in(false)
            .fixed_pos(screen.min);
        let unit = self.path.unit;
        let (tapped, _) = paper(ctx, id, source, area, |ui| {
            ui.set_min_size(screen.size());
            page(ui, screen, (path, unit))
        });
        match tapped {
            Some(Tapped::Unit(unit)) => self.path.unit = unit,
            Some(Tapped::Start(start, from)) => self.start(start, from, path, taps),
            Some(Tapped::Book(from)) => {
                self.path.close();
                self.book.open(from);
            }
            None => {}
        }
    }
}

/// The page on `screen` showing `unit`; what was tapped.
fn page(ui: &mut egui::Ui, screen: Rect, (path, unit): (&PathProjection, u8)) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let heading = TextStyle::Heading.resolve(ui.style());
    let name = Words::from(t::PRACTICE).shown(ui.ctx());
    let top = Pos2::new(screen.center().x, ISLAND_TOP + TOUCH_TARGET / 2.0);
    ui.painter()
        .text(top, Align2::CENTER_CENTER, name, heading, INK);
    let inner = Rect::from_min_max(
        Pos2::new(screen.left() + 6.0 * SPACE, UNDER_ISLANDS),
        Pos2::new(screen.right() - 6.0 * SPACE, screen.bottom() - 4.0 * SPACE),
    );
    let band = Rect::from_min_size(inner.min, Vec2::new(inner.width(), UNITS));
    let picked = units(ui, band, (path, unit), language);
    let below = Rect::from_min_max(Pos2::new(inner.left(), band.bottom()), inner.max);
    let side = Rect::from_min_max(Pos2::new(below.right() - SIDE, below.top()), below.max);
    let skills = Rect::from_min_max(
        below.min,
        Pos2::new(side.left() - 6.0 * SPACE, below.bottom()),
    );
    let shown = path.units.iter().find(|shown| shown.number == unit);
    let tapped = unit_skills(ui, skills, (path, shown, unit));
    let offered = offers(ui, side, (path, unit));
    picked.map(Tapped::Unit).or(tapped).or(offered)
}

/// The eight units across `band`, each a ring with its number and, while the app holds its
/// lessons, its name under it; the unit tapped.
fn units(
    ui: &mut egui::Ui,
    band: Rect,
    (path, shown): (&PathProjection, u8),
    language: Language,
) -> Option<u8> {
    let count = path.units.len().max(1);
    let step = (band.width() - 2.0 * RING) / (count - 1).max(1) as f32;
    let y = band.top() + RING + SPACE;
    let line = [
        Pos2::new(band.left() + RING, y),
        Pos2::new(band.right() - RING, y),
    ];
    let pencil = Stroke::new(1.25, PENCIL);
    ui.painter()
        .add(egui::Shape::dashed_line(&line, pencil, 6.0, 7.0));
    let mut tapped = None;
    for (place, unit) in path.units.iter().enumerate() {
        let centre = Pos2::new(band.left() + RING + step * place as f32, y);
        let ink = if unit.open { INK } else { PENCIL };
        let target = Rect::from_center_size(centre, Vec2::splat(TOUCH_TARGET.max(2.0 * RING)));
        let english = t::unit_number(Language::English, unit.number);
        let response = ui.interact(
            target,
            ui.id().with(("unit", unit.number)),
            egui::Sense::click(),
        );
        controls::register(ui, &english, target);
        let painter = ui.painter();
        painter.circle(centre, RING, PANEL, Stroke::new(1.5, ink));
        if unit.number == shown {
            // The unit shown: the active ring round its own.
            let mark = Rect::from_center_size(centre, Vec2::splat(2.0 * RING + SPACE));
            paint_mark(painter, Outline::round(mark), 1.0);
        }
        let body = TextStyle::Body.resolve(ui.style());
        painter.text(
            centre,
            Align2::CENTER_CENTER,
            unit.number.to_string(),
            body,
            ink,
        );
        if let Some(name) = t::unit_name(unit.number) {
            let small = TextStyle::Small.resolve(ui.style());
            let under = centre + Vec2::new(0.0, RING + SPACE);
            let quiet = if unit.open { INK_SECONDARY } else { PENCIL };
            painter.text(under, Align2::CENTER_TOP, name.get(language), small, quiet);
        }
        if response.clicked() {
            tapped = Some(unit.number);
        }
    }
    tapped
}

/// The unit shown in `rect`: its title, its skills' tiles, the seals earned and Practice book.
fn unit_skills(
    ui: &mut egui::Ui,
    rect: Rect,
    (path, shown, unit): (&PathProjection, Option<&UnitShown>, u8),
) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let mut column = ui.new_child(
        UiBuilder::new()
            .id_salt("path unit")
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    let open = shown.is_some_and(|unit| unit.open);
    let title = egui::RichText::new(t::unit_title(language, unit))
        .heading()
        .color(if open { INK } else { INK_SECONDARY });
    column.label(title);
    column.add_space(2.0 * SPACE);
    let skills = shown.map_or(&[][..], |unit| &unit.skills[..]);
    let next = path.next.map(|offer| offer.skill);
    let across = ((column.available_width() + 2.0 * SPACE) / (TILE + 2.0 * SPACE))
        .floor()
        .max(1.0);
    let width = ((column.available_width() - 2.0 * SPACE * (across - 1.0)) / across).min(TILE);
    let mut tapped = None;
    for row in skills.chunks(across as usize) {
        column.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(2.0 * SPACE);
            for skill in row {
                let name = t::skill_name(skill.skill);
                let lacks = skill.lacks.map(|needs| Words::from(t::needs(needs)));
                let tile = SkillTile {
                    skill: skill.skill,
                    name: name.into(),
                    dots: skill.dots,
                    lacks,
                    next: next == Some(skill.skill) && !path.first,
                    small: false,
                };
                let response = skill_tile(ui, &tile, width);
                if response.clicked() {
                    tapped = Some(Tapped::Start(Start::Skill(skill.skill), response.rect));
                }
            }
        });
        column.add_space(2.0 * SPACE);
    }
    if skills.is_empty() {
        label(&mut column, t::UNIT_IN_THE_MAKING);
    }
    let foot = Rect::from_min_max(
        Pos2::new(rect.left(), rect.bottom() - SEAL - SPACE),
        rect.max,
    );
    seals_and_book(ui, foot, path).or(tapped)
}

/// The seals earned along `foot`, then Practice book.
fn seals_and_book(ui: &mut egui::Ui, foot: Rect, path: &PathProjection) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let mut x = foot.left() + SEAL / 2.0;
    for (place, earned) in path.book.book.seals.iter().enumerate() {
        let centre = Pos2::new(x, foot.center().y);
        let lines = t::seal_lines(language, earned.seal);
        paint_seal(
            ui.painter(),
            (centre, SEAL),
            (&lines, seal_turn(place)),
            1.0,
        );
        x += SEAL + 2.0 * SPACE;
    }
    let row = Rect::from_min_max(Pos2::new(x - SEAL / 2.0, foot.top()), foot.max);
    let mut row = ui.new_child(
        UiBuilder::new()
            .id_salt("path book")
            .max_rect(row)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let book = text_button(&mut row, t::PRACTICE_BOOK, true);
    book.clicked().then_some(Tapped::Book(book.rect))
}

/// The next lesson, the review on the side and the unit's full lesson in `side`, on a panel.
fn offers(ui: &mut egui::Ui, side: Rect, (path, unit): (&PathProjection, u8)) -> Option<Tapped> {
    let mut column = ui.new_child(
        UiBuilder::new()
            .id_salt("path offers")
            .max_rect(side)
            .layout(Layout::top_down(Align::Min)),
    );
    let margin = egui::Margin::same((3.0 * SPACE) as i8);
    panel_frame(margin)
        .show(&mut column, |ui| {
            ui.set_width(side.width() - 6.0 * SPACE);
            ui.spacing_mut().item_spacing = Vec2::new(SPACE, SPACE);
            let next = next_lesson(ui, path).map(|from| Tapped::Start(Start::Next, from));
            let review = review(ui, path).map(|from| Tapped::Start(Start::Review, from));
            let full = (path.units.iter()).find(|shown| shown.number == unit);
            let full = full_block(ui, (unit, full.and_then(|shown| shown.full)));
            next.or(review).or(full)
        })
        .inner
}

/// The full lesson of `unit` (its offer, `full`): about how many minutes it takes, a warm-up then
/// each skill of the unit; Full lesson greyed while it lays none.
fn full_block(ui: &mut egui::Ui, (unit, full): (u8, Option<Offer>)) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    ui.add_space(SPACE);
    rule(ui);
    label(ui, t::LONGER);
    let tapped = ui
        .horizontal(|ui| {
            if let Some(full) = full {
                ui.label(t::about_minutes(language, full.minutes));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                text_button(ui, t::FULL_LESSON, full.is_some())
            })
            .inner
        })
        .inner;
    label(ui, t::FULL_LINE);
    tapped
        .clicked()
        .then_some(Tapped::Start(Start::Full(unit), tapped.rect))
}
