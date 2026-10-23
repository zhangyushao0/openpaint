//! What the practice card says of week 2's colour work and of a long colour copy (S82):
//!
//! - a colour combination compared: how strongly the original shows over the copy (a slider),
//!   each area as a chip of two colours (the original's over yours) to choose one, the chosen
//!   area's colours on the wheel with the way from yours to the original and in V大's words, or
//!   the point the eyedropper compared; how the weak contrasts were kept;
//! - a value scale compared: how many cells lie within a quarter step of even, and in words each
//!   cell off or out of order (the bars under the row show them);
//! - a colour copy compared: the slider, and the point the eyedropper compared on the wheel;
//! - in a session's summary, the combinations' and scales' means beside the earlier sessions'.
//!
//! The words are `text/practice_colour.rs`'s, in the pass's language.

use egui::{Rect, Sense, Stroke, Vec2};
use openpaint_app::{Intent, Overlay, PracticeIntent, PracticeProjection, Probe};
use openpaint_practice::{Family, KindSummary, Original, Rgb8, ScaleCheck};

use crate::components::{
    Choices, SettingSlider, card_columns, card_text, choices, controls, setting_slider,
};
use crate::design::{DIVIDER, INK, Outline, PANEL, TOUCH_TARGET};
use crate::practice_card::{metric_text, pair_on_wheel};
use crate::practice_marks::colour;
use crate::taps::push_once;
use crate::text::{self as t, Language, language};

/// The wheel's side on a colour combination's card, in points: smaller than a colour's, so the
/// card fits under the exercise's tools on a landscape screen.
const AREAS_WHEEL: f32 = 180.0;
/// The study's wheel.
const STUDY_WHEEL: f32 = 240.0;
/// A chip's width, in points.
const CHIP: f32 = 40.0;

/// A colour combination, a value scale or a colour copy compared; false when `practice` shows
/// none of those.
pub(crate) fn compared(
    ui: &mut egui::Ui,
    practice: &PracticeProjection,
    taps: &mut Vec<Intent>,
) -> bool {
    match &practice.overlay {
        Some(Overlay::Areas { .. }) => areas(ui, practice, taps),
        Some(Overlay::Scale(check)) => scale(ui, check.as_deref()),
        Some(Overlay::Picture { shown, .. }) if practice.family == Some(Family::Study) => {
            shown_slider(ui, *shown, taps);
            card_text(ui, t::SHOWS_STUDY, true);
            match practice.probe {
                Some(probe) => probed(ui, probe, STUDY_WHEEL),
                None => card_text(ui, t::PROBE_HINT, true),
            }
        }
        _ => return false,
    }
    true
}

/// How strongly the original shows over the copy.
fn shown_slider(ui: &mut egui::Ui, shown: f32, taps: &mut Vec<Intent>) {
    let percent = format!("{:.0} %", shown * 100.0);
    let slider = SettingSlider {
        label: t::ORIGINAL_SHOWN.into(),
        shown: &percent,
        position: shown,
        enabled: true,
    };
    if let Some(change) = setting_slider(ui, &slider) {
        push_once(taps, Intent::Practice(PracticeIntent::Shown(change.value)));
    }
}

/// The point the eyedropper compared: both colours on the wheel, the way from yours.
fn probed(ui: &mut egui::Ui, probe: Probe, side: f32) {
    pair_on_wheel(ui, (probe.original, probe.copy), side);
}

fn areas(ui: &mut egui::Ui, practice: &PracticeProjection, taps: &mut Vec<Intent>) {
    let (
        Some(Overlay::Areas {
            shown,
            read,
            chosen,
            ..
        }),
        Some(index),
    ) = (&practice.overlay, practice.current)
    else {
        return;
    };
    let Original::Areas(areas) = &practice.sheet.originals[index] else {
        return;
    };
    shown_slider(ui, *shown, taps);
    let copy = read.as_ref().map(|read| read.copy.colours.as_slice());
    chips(ui, (&areas.colours, copy), *chosen, taps);
    match (practice.probe, read) {
        (Some(probe), _) => probed(ui, probe, AREAS_WHEEL),
        (None, None) => card_text(ui, t::READING_COPY, true),
        (None, Some(read)) => match read.copy.colours.get(*chosen).copied().flatten() {
            Some(mine) => pair_on_wheel(ui, (areas.colours[*chosen], mine), AREAS_WHEEL),
            None => card_text(ui, t::NOT_PAINTED, false),
        },
    }
    if let Some(read) = read {
        let judged: Vec<_> = (read.copy.contrasts.iter())
            .filter(|c| c.weak() && c.copy.is_some())
            .collect();
        let strong = judged.iter().filter(|c| c.made_strong()).count();
        let language = language(ui.ctx());
        if strong > 0 {
            let line = t::weak_made_strong(language, (strong, judged.len()));
            card_text(ui, crate::text::Words::made(&line), false);
        } else if !judged.is_empty() {
            card_text(ui, t::WEAK_KEPT, false);
        }
    }
    card_text(ui, t::SHOWS_AREAS, true);
}

/// A chip for each area: the original's colour over yours (the paper while not painted), the
/// chosen one ringed; a tap chooses one.
fn chips(
    ui: &mut egui::Ui,
    (originals, copy): (&[Rgb8], Option<&[Option<Rgb8>]>),
    chosen: usize,
    taps: &mut Vec<Intent>,
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
                paint_chip(ui, rect, (*original, mine));
                if response.clicked() && area != chosen {
                    push_once(taps, Intent::Practice(PracticeIntent::Area(area)));
                }
            }
        });
    });
}

fn paint_chip(ui: &egui::Ui, rect: Rect, (original, mine): (Rgb8, Option<Rgb8>)) {
    let chip = Rect::from_center_size(rect.center(), Vec2::new(CHIP - 12.0, TOUCH_TARGET - 16.0));
    let (top, bottom) = chip.split_top_bottom_at_fraction(0.5);
    let painter = ui.painter();
    painter.rect_filled(top, 0.0, colour(original));
    painter.rect_filled(bottom, 0.0, mine.map_or(PANEL, colour));
    painter.rect_stroke(chip, 0.0, Stroke::new(1.0, INK), egui::StrokeKind::Outside);
    if mine.is_none() {
        let middle = [bottom.left_center(), bottom.right_center()];
        painter.line_segment(middle, Stroke::new(1.0, DIVIDER));
    }
}

fn scale(ui: &mut egui::Ui, check: Option<&ScaleCheck>) {
    let language = language(ui.ctx());
    let Some(check) = check else {
        card_text(ui, t::READING_COPY, true);
        return;
    };
    let inner = 1..check.values.len().saturating_sub(1);
    let filled: Vec<usize> = (inner.clone())
        .filter(|cell| check.values[*cell].is_some())
        .collect();
    let even = filled
        .iter()
        .filter(|cell| !check.uneven.contains(cell))
        .count();
    if check.uneven.is_empty() && check.reversed.is_empty() && filled.len() == inner.len() {
        card_text(ui, t::SCALE_EVEN, false);
    } else {
        let line = t::scale_even(language, (even, filled.len()));
        card_text(ui, crate::text::Words::made(&line), false);
    }
    let mut lines = Vec::new();
    for cell in inner {
        match check.values[cell] {
            None => lines.push(t::cell_empty(language, cell + 1)),
            Some(value) if check.uneven.contains(&cell) => {
                lines.push(t::cell_off(language, cell + 1, value < check.targets[cell]));
            }
            Some(_) => {}
        }
    }
    lines.extend((check.reversed.iter()).map(|cell| t::cells_reversed(language, cell + 1)));
    for line in &lines {
        card_text(ui, crate::text::Words::made(line), false);
    }
    card_text(ui, t::SCALE_NEXT, false);
    card_text(ui, t::SCALE_NOTE, true);
}

/// The summary's rows for colour combinations and value scales: this session's means beside the
/// earlier sessions'.
pub(crate) fn summary_rows(ui: &mut egui::Ui, kinds: &[KindSummary]) {
    let language = language(ui.ctx());
    for kind in kinds.iter().filter(|kind| !kind.rows.is_empty()) {
        let head = match kind.family {
            Family::Areas => t::COMBINATIONS,
            Family::Scale => t::SCALES,
            _ => continue,
        };
        let heads = [head, t::THIS_SESSION, t::BEFORE].map(|head| head.get(language));
        card_columns(ui, &heads, true);
        for (metric, this, before) in &kind.rows {
            let shown = |mean: &Option<f64>| {
                mean.map_or_else(|| "—".to_owned(), |v| metric_text(*metric, v))
            };
            let name = t::metric(*metric).get(language);
            card_columns(ui, &[name, &shown(this), &shown(before)], false);
        }
    }
}
