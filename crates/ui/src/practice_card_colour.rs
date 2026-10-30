//! What the practice card shows of week 2's colour work and of a long colour copy (S82, D108);
//! what the frames show (the weak contrasts made strong in red, a scale's even row) is not said
//! again, and the How cards say what the marks mean:
//!
//! - a colour combination compared: how strongly the original shows over the copy (a slider),
//!   each area as a chip of two colours (the original's over yours) to choose one, the chosen
//!   area's colours on the wheel and in the colour table, or the point the eyedropper compared;
//!   a line only where the area is not painted;
//! - a value scale compared: its title and how many cells are uneven (S97); the even row under
//!   the copy's speaks;
//! - a colour copy compared: the slider, and the point the eyedropper compared;
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
use crate::text::{self as t, Language, Words, language};

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
            if let Some(probe) = practice.probe {
                probed(ui, probe, STUDY_WHEEL);
            }
        }
        // Value masses (S96): the slider; the answer's edges and hatching speak.
        Some(Overlay::Masses { shown, read, .. }) => {
            shown_slider(ui, *shown, taps);
            if read.is_none() {
                card_text(ui, t::READING_COPY, true);
            }
        }
        _ => return false,
    }
    true
}

/// A whole copy's point the eyedropper compared, where its colour blocks were painted (S96).
pub(crate) fn whole_copy_probe(ui: &mut egui::Ui, practice: &PracticeProjection) {
    if let Some(probe) = practice.probe {
        probed(ui, probe, STUDY_WHEEL);
    }
}

/// How strongly the original shows over the copy (a division's and a whole copy's too, S95).
pub(crate) fn shown_slider(ui: &mut egui::Ui, shown: f32, taps: &mut Vec<Intent>) {
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

/// The point the eyedropper compared: both colours on the wheel and in the table.
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

/// A value scale compared (S97): the even row under the copy's speaks, and how many cells are
/// uneven when any; until the copy is read, saying so.
fn scale(ui: &mut egui::Ui, check: Option<&ScaleCheck>) {
    match check {
        None => card_text(ui, t::READING_COPY, true),
        Some(check) if !check.uneven.is_empty() => {
            let line = t::cells_uneven(language(ui.ctx()), check.uneven.len());
            card_text(ui, Words::made(&line), false);
        }
        Some(_) => {}
    }
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
