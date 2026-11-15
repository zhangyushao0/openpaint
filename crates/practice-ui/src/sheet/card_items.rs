//! A compared exercise's card items (S109, `openpaint_practice::CardItem`), whatever kind gave them,
//! shown rather than told (D108): a line of words, a failed check's line after a red dot,
//! Construction (看画法) and Grey (看黑白), how strongly the original shows (a slider), a chip for
//! each area (the original's colour over yours, failing ones outlined red, a tap choosing one), a
//! lightness bar (the original's tick over it, yours under it, red when off, the two in numbers
//! with their difference) and a division's table of its levels.

use egui::{Align2, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextStyle, Vec2};
use openpaint_practice::Choices as Chosen;
use openpaint_practice::{LIGHTNESS_OFF, Level, LevelRow, MOST_MAIN, Rgb8, Say, from_oklab, oklab};
use openpaint_practice_app::PracticeIntent;

use openpaint_ui::components::{
    Choices, SettingSlider, card_text, choices, controls, setting_slider, text_choice,
    text_choice_wide,
};
use openpaint_ui::design::{DIVIDER, INK, INK_SECONDARY, Outline, PANEL, SPACE, TOUCH_TARGET};
use openpaint_ui::taps::push_once;

use super::marks::{colour, level_colour};
use crate::palette::CORRECTION;
use crate::text::{self as t, Language, Text, Words, language};

/// A chip's width, in points.
const CHIP: f32 = 40.0;
/// The lightness bar's height and its ticks' reach past it, in points.
const BAR: f32 = 14.0;
const TICK: f32 = 8.0;
/// A failure's red dot's radius, in points.
const RED_DOT: f32 = 4.0;
/// The levels' table's number columns' width, and a level's colour line before its name, points.
const COLUMN: f32 = 96.0;
const SWATCH: f32 = 3.0 * SPACE;

/// A line of words, in the language shown.
pub(crate) fn says(ui: &mut egui::Ui, say: &Say) {
    let line = t::said(language(ui.ctx()), say);
    card_text(ui, Words::made(&line), false);
}

/// A check that failed: its line after a red dot.
pub(crate) fn failed(ui: &mut egui::Ui, say: &Say) {
    let line = t::said(language(ui.ctx()), say);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(3.0 * RED_DOT), Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), RED_DOT, CORRECTION);
        card_text(ui, Words::made(&line), false);
    });
}

/// A toggle (Construction, Grey): its word in the ink pill while on; a tap turns it.
pub(crate) fn toggle(
    ui: &mut egui::Ui,
    (id, word, on): (&str, Text, bool),
    taps: &mut Vec<PracticeIntent>,
    intent: fn(bool) -> PracticeIntent,
) {
    ui.horizontal(|ui| {
        choices(ui, id, |ui, group| {
            if text_choice(ui, group, word, on).clicked() {
                push_once(taps, intent(!on));
            }
        });
    });
}

/// How strongly the original shows over the copy.
pub(crate) fn shown_slider(ui: &mut egui::Ui, shown: f32, taps: &mut Vec<PracticeIntent>) {
    let percent = format!("{:.0} %", shown * 100.0);
    let slider = SettingSlider {
        label: t::ORIGINAL_SHOWN.into(),
        shown: &percent,
        position: shown,
        enabled: true,
    };
    if let Some(change) = setting_slider(ui, &slider) {
        push_once(taps, PracticeIntent::Shown(change.value));
    }
}

/// A chip for each area: the original's colour over yours (the paper while not painted, the
/// `marked` outlined red), the chosen one ringed; a tap chooses one.
pub(crate) fn chips(
    ui: &mut egui::Ui,
    (originals, copy): (&[Rgb8], Option<&[Option<Rgb8>]>),
    (chosen, marked): (usize, &[usize]),
    taps: &mut Vec<PracticeIntent>,
) {
    let language = language(ui.ctx());
    ui.horizontal(|ui| {
        choices(ui, "areas", |ui, group: &mut Choices| {
            for (area, original) in originals.iter().enumerate() {
                let mine = copy.and_then(|copy| copy.get(area).copied().flatten());
                let name = t::area(Language::English, area + 1);
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(CHIP + 8.0, TOUCH_TARGET), Sense::click());
                controls::register(ui, &name, rect);
                let shown = t::area(language, area + 1);
                let response = response.on_hover_text(shown);
                if area == chosen {
                    group.chosen(Outline::ring(rect));
                }
                paint_chip(ui, rect, (*original, mine), marked.contains(&area));
                if response.clicked() && area != chosen {
                    push_once(taps, PracticeIntent::Area(area));
                }
            }
        });
    });
}

fn paint_chip(ui: &egui::Ui, rect: Rect, (original, mine): (Rgb8, Option<Rgb8>), marked: bool) {
    let chip = Rect::from_center_size(rect.center(), Vec2::new(CHIP - 12.0, TOUCH_TARGET - 16.0));
    let (top, bottom) = chip.split_top_bottom_at_fraction(0.5);
    let painter = ui.painter();
    painter.rect_filled(top, 0.0, colour(original));
    painter.rect_filled(bottom, 0.0, mine.map_or(PANEL, colour));
    let outline = if marked {
        Stroke::new(2.5, CORRECTION)
    } else {
        Stroke::new(1.0, INK)
    };
    painter.rect_stroke(chip, 0.0, outline, StrokeKind::Outside);
    if mine.is_none() {
        let middle = [bottom.left_center(), bottom.right_center()];
        painter.line_segment(middle, Stroke::new(1.0, DIVIDER));
    }
}

/// The lightness bar from black to white: the original's lightness ticked over it, yours under it
/// (red when it left the original's), and both in numbers (Oklab `L` × 100) with the difference.
pub(crate) fn lightness(ui: &mut egui::Ui, original: Rgb8, mine: Rgb8) {
    let [l0, l1] = [original, mine].map(|c| oklab(c)[0].clamp(0.0, 1.0));
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, BAR + 2.0 * TICK), Sense::hover());
    let bar = Rect::from_min_size(
        rect.min + Vec2::new(0.0, TICK),
        Vec2::new(rect.width(), BAR),
    );
    let painter = ui.painter();
    let mut mesh = egui::Mesh::default();
    for step in 0..=16u8 {
        let l = f64::from(step) / 16.0;
        let x = bar.left() + bar.width() * l as f32;
        let grey = colour(from_oklab([l, 0.0, 0.0]));
        mesh.colored_vertex(Pos2::new(x, bar.top()), grey);
        mesh.colored_vertex(Pos2::new(x, bar.bottom()), grey);
        if step > 0 {
            let i = 2 * u32::from(step);
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    painter.add(Shape::mesh(mesh));
    painter.rect_stroke(bar, 0.0, Stroke::new(1.0, DIVIDER), StrokeKind::Outside);
    let at = |l: f64| bar.left() + bar.width() * l as f32;
    let off = (l1 - l0).abs() > LIGHTNESS_OFF;
    let mine_ink = if off { CORRECTION } else { INK };
    let above = [
        Pos2::new(at(l0), rect.top()),
        Pos2::new(at(l0), bar.bottom()),
    ];
    painter.line_segment(above, Stroke::new(2.5, INK));
    let under = [
        Pos2::new(at(l1), bar.top()),
        Pos2::new(at(l1), rect.bottom()),
    ];
    painter.line_segment(under, Stroke::new(2.5, mine_ink));
    let numbers = format!(
        "{:.0} → {:.0}   {:+.0}",
        l0 * 100.0,
        l1 * 100.0,
        (l1 - l0) * 100.0
    );
    ui.add_space(SPACE / 2.0);
    ui.vertical_centered(|ui| card_text(ui, Words::made(&numbers), false));
}

/// A division's table (S95): each level named beside a short line in its colour, the lines drawn
/// and how far they run on the mean, the main lines' count in the marking red past V大's five.
pub(crate) fn levels(ui: &mut egui::Ui, rows: &[LevelRow; 3]) {
    let language = language(ui.ctx());
    let heads = [t::LINES_DRAWN, t::MEAN_LENGTH].map(|head| head.get(language));
    row(ui, (None, ""), [heads[0], heads[1]], None);
    for level in Level::ALL {
        let LevelRow { lines, length } = rows[level.index()];
        let count = lines.to_string();
        let mean = length.map_or_else(|| "—".to_owned(), t::mean_length);
        let red = (level == Level::Main && lines > MOST_MAIN).then_some(CORRECTION);
        let name = t::level(level).get(language);
        row(ui, (Some(level), name), [&count, &mean], red);
    }
}

/// A choice's or a view's word by its key, its key itself where no table knows it.
fn word_of(key: &str) -> Words<'_> {
    t::choice_word(key).map_or_else(|| Words::made(key), Words::from)
}

/// A row of chips (S112), three to a line, those `chosen` holds on in the ink pill; a tap turns
/// one while `open` (before the first Compare), else they are only shown.
pub(crate) fn toggles(
    ui: &mut egui::Ui,
    keys: &[&str],
    (chosen, open): (Chosen, bool),
    taps: &mut Vec<PracticeIntent>,
) {
    let width = (ui.available_width() / 3.0 - SPACE).floor();
    for (line, row) in keys.chunks(3).enumerate() {
        ui.horizontal(|ui| {
            for (place, key) in (line * 3..).zip(row) {
                let chip = u8::try_from(place).unwrap_or(u8::MAX);
                let on = chosen.on(chip);
                choices(ui, &format!("toggle {place}"), |ui, group| {
                    let word = text_choice_wide(ui, group, word_of(key), (open, on), width);
                    if word.clicked() && open {
                        push_once(taps, PracticeIntent::Toggle(chip));
                    }
                });
            }
        });
    }
}

/// A row of small pictures of bars, each with its word under it (S112), the one `chosen` holds
/// ringed; a tap chooses one while `open`.
pub(crate) fn shapes(
    ui: &mut egui::Ui,
    shapes: &[(&str, [f32; 5])],
    (chosen, open): (Chosen, bool),
    taps: &mut Vec<PracticeIntent>,
) {
    let count = shapes.len().max(1) as f32;
    let width = (ui.available_width() / count - SPACE).floor();
    let height = TOUCH_TARGET + ui.text_style_height(&TextStyle::Small) + SPACE;
    ui.horizontal(|ui| {
        choices(ui, "shapes", |ui, group| {
            for (place, (key, bars)) in shapes.iter().enumerate() {
                let shape = u8::try_from(place).unwrap_or(u8::MAX);
                let sense = if open { Sense::click() } else { Sense::hover() };
                let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), sense);
                let words = word_of(key);
                controls::register(ui, words.id(), rect);
                if chosen.shape == Some(shape) {
                    group.chosen(Outline::row(rect));
                }
                paint_shape(ui, rect, bars, words.shown(ui.ctx()));
                if response.clicked() && open {
                    push_once(taps, PracticeIntent::Shape(shape));
                }
            }
        });
    });
}

/// `bars` (0 to 1) side by side in the upper part of `rect`, `word` small under them.
fn paint_shape(ui: &egui::Ui, rect: Rect, bars: &[f32; 5], word: &str) {
    let painter = ui.painter();
    let field = Rect::from_min_size(
        rect.min + Vec2::new(SPACE, SPACE / 2.0),
        Vec2::new(rect.width() - 2.0 * SPACE, TOUCH_TARGET - SPACE),
    );
    let bar = field.width() / 9.0;
    for (place, share) in bars.iter().enumerate() {
        let left = field.left() + 2.0 * bar * place as f32;
        let top = field.bottom() - field.height() * share.clamp(0.0, 1.0);
        let shown = Rect::from_min_max(
            egui::pos2(left, top),
            egui::pos2(left + bar, field.bottom()),
        );
        painter.rect_filled(shown, 0.0, INK);
    }
    let font = TextStyle::Small.resolve(ui.style());
    let under = egui::pos2(rect.center().x, field.bottom() + SPACE / 2.0);
    painter.text(under, Align2::CENTER_TOP, word, font, INK);
}

/// Compare's views (S112), the one shown in the ink pill; a tap shows another.
pub(crate) fn views(
    ui: &mut egui::Ui,
    keys: &[&str],
    shown: usize,
    taps: &mut Vec<PracticeIntent>,
) {
    let width = (ui.available_width() / 3.0 - SPACE).floor();
    choices(ui, "views", |ui, group| {
        for (line, row) in keys.chunks(3).enumerate() {
            ui.horizontal(|ui| {
                for (place, key) in (line * 3..).zip(row) {
                    let on = place == shown;
                    let word = text_choice_wide(ui, group, word_of(key), (true, on), width);
                    if word.clicked() && !on {
                        let view = u8::try_from(place).unwrap_or(u8::MAX);
                        push_once(taps, PracticeIntent::View(view));
                    }
                }
            });
        }
    });
}

/// A row of the table: a level's colour line and its name (none: the heads' row, quiet), and two
/// numbers at the right, the first in `red` where given.
fn row(
    ui: &mut egui::Ui,
    (level, name): (Option<Level>, &str),
    [first, second]: [&str; 2],
    red: Option<egui::Color32>,
) {
    let font = TextStyle::Body.resolve(ui.style());
    let height = ui.text_style_height(&TextStyle::Body) + SPACE;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter();
    let ink = if level.is_some() { INK } else { INK_SECONDARY };
    let middle = rect.center().y;
    if let Some(level) = level {
        let ends = [rect.left(), rect.left() + SWATCH - SPACE].map(|x| egui::pos2(x, middle));
        painter.line_segment(ends, Stroke::new(3.0, level_colour(level)));
        let left = egui::pos2(rect.left() + SWATCH, middle);
        painter.text(left, Align2::LEFT_CENTER, name, font.clone(), ink);
    }
    let columns = [(first, red.unwrap_or(ink), COLUMN), (second, ink, 0.0)];
    for (text, colour, before) in columns {
        let at = egui::pos2(rect.right() - before, middle);
        painter.text(at, Align2::RIGHT_CENTER, text, font.clone(), colour);
    }
}
