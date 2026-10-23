//! What the practice card says: how to practise each kind (our own words after V大's method:
//! look first, mark the ends and the most bent points, draw in one sure stroke, correct over
//! rather than erase; name a colour, pick once by eye, compare and correct two or three times),
//! a comparison (a line: in place or aligned, no numbers; a colour: both on the wheel with the
//! way from yours to the original, its differences in V大's words), and a session's summary (the
//! colour judgement's radar over the earlier sessions', the colour means beside theirs). The
//! words are `text/practice.rs`'s, in the pass's language.

use openpaint_app::{Hsv, Intent, Language, Overlay, Pen, PracticeIntent, PracticeProjection};
use openpaint_core::Color;
use openpaint_practice::{
    Axis, Difference, KindSummary, LEAST_CHROMA, Metric, Rgb8, chroma, hsv, hue_name, hue_towards,
    value,
};

use crate::components::{
    RadarAxis, card_columns, card_text, choices, color_compare, radar, text_button, text_choice,
};
use crate::design::{CORRECTION, DIALOG_WIDTH, SPACE};
use crate::taps::push_once;
use crate::text::{self as t, Words, language};

/// The comparison wheel's side on the card, in points.
const COMPARE_WHEEL: f32 = 280.0;
/// Degrees of hue past which a pick is another hue altogether, named rather than compared.
const OTHER_HUE: f64 = 45.0;

/// The instructions for lines (or colours); returns whether Got it was tapped.
pub(crate) fn instructions(ui: &mut egui::Ui, lines: bool) -> bool {
    let (heading, text) = t::instructions(lines);
    ui.heading(Words::from(heading).shown(ui.ctx()));
    for line in text {
        card_text(ui, line, false);
    }
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        text_button(ui, t::GOT_IT, true).clicked()
    })
    .inner
}

/// The shown exercise as compared.
pub(crate) fn compared(ui: &mut egui::Ui, practice: &PracticeProjection, taps: &mut Vec<Intent>) {
    let (place, _) = practice.place;
    match &practice.overlay {
        Some(Overlay::Line(_)) => line(ui, practice, place, taps),
        Some(Overlay::Colour { original, last, .. }) => {
            colour(ui, practice, place, (*original, *last));
        }
        None => {}
    }
}

/// Line `place` compared: in place or aligned, and how to go on.
fn line(ui: &mut egui::Ui, practice: &PracticeProjection, place: usize, taps: &mut Vec<Intent>) {
    ui.heading(t::exercise(language(ui.ctx()), true, place));
    ui.horizontal(|ui| {
        choices(ui, "overlay", |ui, group| {
            for (aligned, word) in [(false, t::IN_PLACE), (true, t::ALIGNED)] {
                let chosen = practice.aligned == aligned;
                if text_choice(ui, group, word, chosen).clicked() && !chosen {
                    push_once(taps, Intent::Practice(PracticeIntent::Aligned(aligned)));
                }
            }
        });
    });
    let shows = if practice.aligned {
        t::SHOWS_ALIGNED
    } else {
        t::SHOWS_IN_PLACE
    };
    card_text(ui, shows, false);
    let blue = practice.pen.is_some_and(|(pen, _)| pen == Pen::Correct);
    let next = if blue {
        t::CORRECT_IN_BLUE
    } else {
        t::CORRECT_OVER
    };
    card_text(ui, next, true);
}

/// Colour `place` compared: the colour laid now (`last`) and the `original` on the wheel, the
/// way from one to the other in words, and the numbers stored at the first Compare.
fn colour(
    ui: &mut egui::Ui,
    practice: &PracticeProjection,
    place: usize,
    (original, last): (Rgb8, Rgb8),
) {
    let language = language(ui.ctx());
    ui.heading(t::exercise(language, false, place));
    let as_hsv = |rgb: Rgb8| {
        let [r, g, b] = rgb.map(|c| f32::from(c) / 255.0);
        Color::rgb(r, g, b).map_or(
            Hsv {
                h: 0.0,
                s: 0.0,
                v: 0.0,
            },
            Hsv::of,
        )
    };
    let hued = [last, original].map(|c| chroma(c) >= LEAST_CHROMA);
    ui.vertical_centered(|ui| {
        let colours = (as_hsv(last), as_hsv(original));
        color_compare(ui, colours, (hued, CORRECTION), COMPARE_WHEEL);
    });
    let now = Difference::of(original, last);
    card_columns(ui, &["ΔE", &format!("{:.1}", now.delta * 100.0)], false);
    let differs = difference(language, (original, last), now);
    card_text(ui, Words::made(&differs), false);
    if let Some(result) = &practice.result {
        let metrics = &result.metrics;
        let shown =
            |metric| value(metrics, metric).map_or_else(|| "—".to_owned(), |v| text(metric, v));
        let corrections = value(metrics, Metric::Corrections).unwrap_or(0.0);
        let deltas = (shown(Metric::FirstDelta), shown(Metric::FinalDelta));
        let stored = t::kept_from_first(language, (&deltas.0, &deltas.1), corrections);
        card_text(ui, Words::made(&stored), true);
    }
    card_text(ui, t::WHEEL_NOTE, true);
}

/// How `copy` differs from `original` (`by`), as V大 compares: redder or yellower, purer or
/// greyer, lighter or darker; another hue altogether (`OTHER_HUE` or more) by its name; or grey
/// where the other has a hue.
fn difference(language: Language, (original, copy): (Rgb8, Rgb8), by: Difference) -> String {
    let mut parts = Vec::new();
    let hue_of = |c: Rgb8| hsv(c)[0];
    let name = |c: Rgb8| t::HUE_NAMES[hue_name(hue_of(c))];
    match by.hue.filter(|h| h.abs() >= 1.0) {
        Some(hue) if hue.abs() >= OTHER_HUE => {
            parts.push(t::instead(language, name(copy), name(original)));
        }
        Some(hue) => {
            let word = t::TOWARDS[hue_towards(hue_of(original), hue)];
            parts.push(t::by(language, &format!("{:.0}°", hue.abs()), word));
        }
        None => {
            let grey = |c: Rgb8| chroma(c) < LEAST_CHROMA;
            let named = |c: Rgb8| if grey(c) { t::GREY } else { name(c) };
            if grey(copy) != grey(original) {
                parts.push(t::instead(language, named(copy), named(original)));
            }
        }
    }
    if by.saturation.abs() >= 0.01 {
        let way = if by.saturation > 0.0 {
            t::PURER
        } else {
            t::GREYER
        };
        let amount = format!("{:.0} %", by.saturation.abs() * 100.0);
        parts.push(t::by(language, &amount, way));
    }
    if by.value.abs() >= 0.01 {
        let way = if by.value > 0.0 {
            t::LIGHTER
        } else {
            t::DARKER
        };
        let amount = format!("{:.0} %", by.value.abs() * 100.0);
        parts.push(t::by(language, &amount, way));
    }
    t::yours_is(language, &parts)
}

/// The summary: what was compared, the colour judgement over the earlier sessions', and the
/// colour means beside theirs.
pub(crate) fn summary(ui: &mut egui::Ui, practice: &PracticeProjection, kinds: &[KindSummary]) {
    let language = language(ui.ctx());
    ui.heading(t::session_done(language, practice.number));
    for kind in kinds {
        let line = t::compared_of(language, kind.lines, (kind.checked, kind.held));
        card_text(ui, Words::made(&line), false);
    }
    let Some(colours) = kinds.iter().find(|kind| !kind.lines) else {
        return;
    };
    if let Some([this, before]) = colours.judgement {
        let axes: Vec<RadarAxis<'_>> = Axis::ALL
            .iter()
            .map(|&axis| {
                (
                    t::judgement_axis(axis).get(language),
                    share(this.of(axis)),
                    share(before.of(axis)),
                )
            })
            .collect();
        radar(ui, &axes, DIALOG_WIDTH - 4.0 * SPACE);
    }
    let heads = [t::COLOURS, t::THIS_SESSION, t::BEFORE].map(|head| head.get(language));
    card_columns(ui, &heads, true);
    for (metric, this, before) in &colours.rows {
        let shown = |mean: &Option<f64>| mean.map_or_else(|| "—".to_owned(), |v| text(*metric, v));
        let name = t::metric(*metric).get(language);
        card_columns(ui, &[name, &shown(this), &shown(before)], false);
    }
    let note = t::radar_note(language, practice.stage);
    card_text(ui, Words::made(&note), true);
}

fn share(value: Option<f64>) -> Option<f32> {
    value.map(|v| v as f32)
}

/// A metric's value in its plain unit.
fn text(metric: Metric, value: f64) -> String {
    match metric {
        Metric::FirstDelta | Metric::FinalDelta => format!("{:.1}", value * 100.0),
        Metric::FirstHue | Metric::FinalHue => format!("{value:+.0}°"),
        Metric::FirstSaturation
        | Metric::FinalSaturation
        | Metric::FirstValue
        | Metric::FinalValue => format!("{:+.0} %", value * 100.0),
        Metric::Corrections => format!("{value:.1}"),
    }
}
