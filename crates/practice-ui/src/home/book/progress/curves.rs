//! The book's Progress page (S147, the mock's 进步): one small chart a skill practised, in the
//! course's order, three across where the page is wide: its name over it; its ability from its
//! first practice day to its last as an ink line, days in order, a gap of over a week drawn as a
//! short break; around it a wash one standard deviation wide; a dot on the line where each level
//! began, the level's three small dots over it filled as the skill's; a pencil baseline with the
//! first and the last date under its ends, a day alone a dot over its date. A skill practised with
//! nothing judged yet is named over a bare baseline (S149). One scale for every chart, so a flat
//! skill reads flat.
//! A skill whose re-test pair was answered shows its two copies beside its chart, dated, Re-test
//! (复测) under them. No axis, no number, no legend.

use egui::{Align2, Color32, Pos2, Rect, Shape, Stroke, TextStyle, UiBuilder, Vec2};
use openpaint_practice::{CopyRef, Curve, SkillLevel};
use openpaint_practice_app::BookProjection;
use openpaint_ui::design::{INK, INK_SECONDARY, PANEL, SPACE};

use crate::components::paint_copy;
use crate::palette::PENCIL;
use crate::text as t;

/// A chart's height, its name's line, the gap between charts and a chart's least width, points.
const CHART: f32 = 164.0;
const NAME: f32 = 30.0;
const GAP: f32 = 36.0;
const LEAST: f32 = 360.0;
/// Within a chart: room at the ends, the plot's top and baseline, a re-test card's side.
const ROOM: f32 = 14.0;
const TOP: f32 = 22.0;
const BASELINE: f32 = 140.0;
const CARD: f32 = 52.0;
/// A gap of more days than this breaks the line, by this many days' steps.
const BREAK_DAYS: i64 = 7;
const BREAK_STEPS: f32 = 2.5;
/// The scale's least reach up and down, logits.
const LEAST_UP: f64 = 2.0;
const LEAST_DOWN: f64 = 0.6;
/// The wash's colour, the line's width, a level dot's radius and its small dots'.
const WASH: Color32 = Color32::from_rgb(231, 226, 217);
const LINE: f32 = 2.0;
const LEVEL_DOT: f32 = 4.0;
const SMALL_DOT: f32 = 3.0;
/// A day alone between breaks, its dot's radius.
const SOLO: f32 = 3.5;

/// The page's charts in `rect`, scrolling when they run past its foot.
pub(in crate::home::book) fn curves(ui: &mut egui::Ui, rect: Rect, book: &BookProjection) {
    let Some(progress) = &book.progress else {
        return;
    };
    let across = ((rect.width() + GAP) / (LEAST + GAP))
        .floor()
        .clamp(1.0, 3.0) as usize;
    let width = (rect.width() - GAP * (across - 1) as f32) / across as f32;
    let scale = Scale::of(&progress.curves);
    let mut page = ui.new_child(UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt("book curves")
        .show(&mut page, |ui| {
            let rows = progress.curves.len().div_ceil(across);
            let height = rows as f32 * (NAME + CHART + GAP);
            let (area, _) =
                ui.allocate_exact_size(Vec2::new(rect.width(), height), egui::Sense::hover());
            for (place, curve) in progress.curves.iter().enumerate() {
                let (column, row) = ((place % across) as f32, (place / across) as f32);
                let min = area.min + Vec2::new(column * (width + GAP), row * (NAME + CHART + GAP));
                chart(
                    ui,
                    Rect::from_min_size(min, Vec2::new(width, NAME + CHART)),
                    curve,
                    (book, scale),
                );
            }
        });
}

/// The copies the page's re-test pairs show that were not read yet.
pub(in crate::home::book) fn cards_wanted(book: &BookProjection) -> Vec<CopyRef> {
    let Some(progress) = &book.progress else {
        return Vec::new();
    };
    (progress.curves.iter())
        .flat_map(|curve| curve.retest.iter().flatten().map(|(copy, _)| *copy))
        .filter(|copy| !book.cards.contains_key(copy))
        .collect()
}

/// Where a chart's values lie: logits up and down from the first day's.
#[derive(Debug, Clone, Copy)]
struct Scale {
    up: f64,
    down: f64,
}

impl Scale {
    fn of(curves: &[Curve]) -> Self {
        let mut scale = Self {
            up: LEAST_UP,
            down: LEAST_DOWN,
        };
        for curve in curves {
            for (a, b) in curve.ability.iter().zip(&curve.band) {
                scale.up = scale.up.max(a + b);
                scale.down = scale.down.max(b - a);
            }
        }
        scale
    }

    /// The height of `value` within the plot from `top` to `bottom`.
    fn at(self, value: f64, (top, bottom): (f32, f32)) -> f32 {
        let share = (self.up - value) / (self.up + self.down);
        top + (bottom - top) * share.clamp(0.0, 1.0) as f32
    }
}

/// `curve`'s chart in `rect`: its name, wash, line, levels, baseline, dates, re-test pair.
fn chart(ui: &egui::Ui, rect: Rect, curve: &Curve, (book, scale): (&BookProjection, Scale)) {
    let painter = ui.painter();
    let language = t::language(ui.ctx());
    let name = t::skill_name(curve.skill).get(language);
    let heading = TextStyle::Body.resolve(ui.style());
    painter.text(rect.left_top(), Align2::LEFT_TOP, name, heading, INK);
    let top = rect.top() + NAME;
    let beside = curve.retest.is_some();
    let right = rect.right()
        - if beside {
            2.0 * CARD + 3.0 * ROOM
        } else {
            ROOM
        };
    let plot = (top + TOP, top + BASELINE - ROOM);
    let xs = places(&curve.days, (rect.left() + ROOM, right));
    let point = |i: usize, value: f64| Pos2::new(xs[i], scale.at(value, plot));
    drawn(painter, curve, point);
    let base = top + BASELINE;
    painter.line_segment(
        [Pos2::new(rect.left() + ROOM, base), Pos2::new(right, base)],
        Stroke::new(1.0, PENCIL),
    );
    breaks(painter, &curve.days, &xs, base);
    let small = TextStyle::Small.resolve(ui.style());
    let ends = [(curve.days.first(), rect.left() + ROOM, Align2::LEFT_TOP)];
    let last = (curve.days.len() > 1).then_some((curve.days.last(), right, Align2::RIGHT_TOP));
    for (day, x, align) in ends.into_iter().chain(last) {
        let Some(day) = day else {
            continue;
        };
        let date = t::copy_date(language, *day);
        let at = Pos2::new(x, base + SPACE);
        painter.text(at, align, date, small.clone(), INK_SECONDARY);
    }
    if let Some(pair) = &curve.retest {
        retest(ui, Pos2::new(right + 2.0 * ROOM, top + TOP), pair, book);
    }
}

/// `curve`'s wash, its line in runs between breaks (a day alone a dot) and its levels' dots, each
/// day's value placed by `point`.
fn drawn(painter: &egui::Painter, curve: &Curve, point: impl Fn(usize, f64) -> Pos2) {
    for segment in segments(&curve.days) {
        for pair in segment.windows(2) {
            let [a, b] = [pair[0], pair[1]];
            let quad = vec![
                point(a, curve.ability[a] + curve.band[a]),
                point(b, curve.ability[b] + curve.band[b]),
                point(b, curve.ability[b] - curve.band[b]),
                point(a, curve.ability[a] - curve.band[a]),
            ];
            painter.add(Shape::convex_polygon(quad, WASH, Stroke::NONE));
        }
        let line: Vec<Pos2> = (segment.iter())
            .map(|i| point(*i, curve.ability[*i]))
            .collect();
        if line.len() == 1 {
            painter.circle_filled(line[0], SOLO, INK);
        }
        painter.add(Shape::line(line, Stroke::new(LINE, INK)));
    }
    for (at, level) in &curve.levels {
        let on = point(*at, curve.ability[*at]);
        painter.circle(on, LEVEL_DOT, INK, Stroke::new(1.5, PANEL));
        level_dots(painter, on - Vec2::new(0.0, 14.0), *level);
    }
}

/// Each day's place across from `left` to `right`, a gap of over a week a break's steps wide; a day
/// alone at `left`, over its date.
fn places(days: &[i64], (left, right): (f32, f32)) -> Vec<f32> {
    let mut steps = vec![0.0_f32];
    for pair in days.windows(2) {
        let step = if pair[1] - pair[0] > BREAK_DAYS {
            BREAK_STEPS
        } else {
            1.0
        };
        steps.push(steps.last().copied().unwrap_or(0.0) + step);
    }
    let span = steps.last().copied().unwrap_or(0.0);
    if span <= 0.0 {
        return vec![left; days.len()];
    }
    steps
        .iter()
        .map(|s| left + (right - left) * s / span)
        .collect()
}

/// The days' indices in runs no gap of over a week breaks.
fn segments(days: &[i64]) -> Vec<Vec<usize>> {
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for (i, day) in days.iter().enumerate() {
        let joined = i > 0 && day - days[i - 1] <= BREAK_DAYS;
        match runs.last_mut() {
            Some(run) if joined => run.push(i),
            _ => runs.push(vec![i]),
        }
    }
    runs
}

/// A short break mark on the baseline in each gap of over a week.
fn breaks(painter: &egui::Painter, days: &[i64], xs: &[f32], base: f32) {
    let stroke = Stroke::new(1.25, INK_SECONDARY);
    for (i, pair) in days.windows(2).enumerate() {
        if pair[1] - pair[0] <= BREAK_DAYS {
            continue;
        }
        let middle = (xs[i] + xs[i + 1]) / 2.0;
        for dx in [-2.0, 2.0] {
            let x = middle + dx;
            painter.line_segment(
                [Pos2::new(x - 2.0, base), Pos2::new(x + 2.0, base - 8.0)],
                stroke,
            );
        }
    }
}

/// A level's three small dots centred at `at`, filled up to the level.
fn level_dots(painter: &egui::Painter, at: Pos2, level: SkillLevel) {
    let filled = usize::from(level.code()) + 1;
    for dot in 0..3 {
        let centre = at + Vec2::new((dot as f32 - 1.0) * 10.0, 0.0);
        if dot < filled {
            painter.circle_filled(centre, SMALL_DOT, INK);
        } else {
            painter.circle_stroke(centre, SMALL_DOT - 0.5, Stroke::new(1.0, INK));
        }
    }
}

/// A re-test's two copies from `at`, older first, each dated, Re-test under them.
fn retest(ui: &egui::Ui, at: Pos2, pair: &[(CopyRef, i64); 2], book: &BookProjection) {
    let language = t::language(ui.ctx());
    let small = TextStyle::Small.resolve(ui.style());
    for (place, (copy, day)) in pair.iter().enumerate() {
        let min = at + Vec2::new(place as f32 * (CARD + ROOM), 0.0);
        let frame = Rect::from_min_size(min, Vec2::splat(CARD));
        paint_copy(ui, frame, book.cards.get(copy).map(|card| &**card), false);
        let date = t::copy_date(language, *day);
        let under = frame.left_bottom() + Vec2::new(0.0, SPACE);
        ui.painter()
            .text(under, Align2::LEFT_TOP, date, small.clone(), INK_SECONDARY);
    }
    let label = t::RETEST.get(language);
    let under = at + Vec2::new(0.0, CARD + 4.0 * SPACE);
    ui.painter()
        .text(under, Align2::LEFT_TOP, label, small, INK_SECONDARY);
}
