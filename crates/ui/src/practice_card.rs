//! What the practice card says: how to practise each family (our own words after V大's method:
//! look first, mark the ends and the most bent points, draw in one sure stroke, correct over
//! rather than erase; combined lines seen as one shape, the outer key points first, then the
//! inside, compared rather than guessed; name a colour, pick once by eye, compare and correct two
//! or three times), a comparison (lines: in place or aligned, no numbers; a colour: both on the
//! wheel with the way from yours to the original, its differences in V大's words; week 2's colour
//! work and a colour copy in `practice_card_colour.rs`), and a session's summary (the colour
//! judgement's radar over the earlier sessions', the colour means beside theirs). The words are
//! `text/practice.rs`'s, in the pass's language.

use openpaint_app::{Hsv, Intent, Language, Overlay, Pen, PracticeIntent, PracticeProjection};
use openpaint_core::Color;
use openpaint_practice::{
    Axis, Difference, Family, KindSummary, LEAST_CHROMA, Metric, Rgb8, chroma, hsv, hue_name,
    hue_towards, value,
};

use crate::components::{
    RadarAxis, card_columns, card_text, choices, color_compare, hint, radar, text_button,
    text_choice_wide,
};
use crate::design::{CORRECTION, DIALOG_WIDTH, SPACE};
use crate::practice_card_colour;
use crate::taps::push_once;
use crate::text::{self as t, Words, language};

/// The comparison wheel's side on the card, in points.
const COMPARE_WHEEL: f32 = 280.0;
/// The summary's radar's side when the colour combinations' and value scales' rows follow it.
const SUMMARY_RADAR: f32 = 260.0;
/// Degrees of hue past which a pick is another hue altogether, named rather than compared.
const OTHER_HUE: f64 = 45.0;

/// The instructions for a family; returns whether Got it was tapped.
pub(crate) fn instructions(ui: &mut egui::Ui, family: Family) -> bool {
    let (heading, text) = t::instructions(family);
    ui.heading(Words::from(heading).shown(ui.ctx()));
    for line in text {
        card_text(ui, line, false);
    }
    // At the top of its row: centred, the row would take the height the card had before (a
    // summary's) and keep it.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        text_button(ui, t::GOT_IT, true).clicked()
    })
    .inner
}

/// The shown exercise as compared.
pub(crate) fn compared(ui: &mut egui::Ui, practice: &PracticeProjection, taps: &mut Vec<Intent>) {
    let Some(family) = practice.family else {
        return;
    };
    let language = language(ui.ctx());
    let heading = if family == Family::Study {
        t::colour_copy(language, practice.number)
    } else {
        t::exercise(language, (family, practice.warming_up), practice.place.0)
    };
    let drawn = practice.comparing && family.drawn();
    match &practice.overlay {
        Some(Overlay::Colour { original, last, .. }) => {
            ui.heading(heading);
            colour(ui, practice, (*original, *last));
        }
        _ if drawn => {
            ui.heading(heading);
            lines(ui, practice, family, taps);
        }
        Some(_) if family.painted() => {
            ui.heading(heading);
            practice_card_colour::compared(ui, practice, taps);
        }
        _ => {}
    }
}

/// A drawn exercise compared (a line, a group of them, a single shape): in place or aligned
/// (while its fit is found, saying so), and how to go on.
fn lines(ui: &mut egui::Ui, practice: &PracticeProjection, family: Family, taps: &mut Vec<Intent>) {
    ui.horizontal(|ui| {
        choices(ui, "overlay", |ui, group| {
            for (aligned, word) in [(false, t::IN_PLACE), (true, t::ALIGNED)] {
                let chosen = (practice.aligned && practice.aligned_offered) == aligned;
                let offered = !aligned || practice.aligned_offered;
                let tapped = text_choice_wide(ui, group, word, (offered, chosen), 0.0).clicked();
                if tapped && !chosen && offered {
                    push_once(taps, Intent::Practice(PracticeIntent::Aligned(aligned)));
                }
            }
        });
        if practice.fitting {
            hint(ui, t::ALIGNING);
        }
    });
    let blue = practice.pen.is_some_and(|(pen, _)| pen == Pen::Correct);
    let [shows, next] = match family {
        Family::Shape => t::compared_shape(practice.aligned, practice.aligned_offered, blue),
        _ => t::compared_lines(family == Family::Combined, practice.aligned, blue),
    };
    card_text(ui, shows, false);
    card_text(ui, next, true);
}

/// `original` and `mine` on the wheel (`side` points), the way from one to the other, their
/// difference as ΔE and in V大's words.
pub(crate) fn pair_on_wheel(ui: &mut egui::Ui, (original, mine): (Rgb8, Rgb8), side: f32) {
    let language = language(ui.ctx());
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
    let hued = [mine, original].map(|c| chroma(c) >= LEAST_CHROMA);
    ui.vertical_centered(|ui| {
        let colours = (as_hsv(mine), as_hsv(original));
        color_compare(ui, colours, (hued, CORRECTION), side);
    });
    let now = Difference::of(original, mine);
    card_columns(ui, &["ΔE", &format!("{:.1}", now.delta * 100.0)], false);
    let differs = difference(language, (original, mine), now);
    card_text(ui, Words::made(&differs), false);
}

/// A colour compared: the colour laid now (`last`) and the `original` on the wheel, the way from
/// one to the other in words, and the numbers stored at the first Compare.
fn colour(ui: &mut egui::Ui, practice: &PracticeProjection, (original, last): (Rgb8, Rgb8)) {
    let language = language(ui.ctx());
    pair_on_wheel(ui, (original, last), COMPARE_WHEEL);
    if let Some(result) = &practice.result {
        let metrics = &result.metrics;
        let shown = |metric| {
            value(metrics, metric).map_or_else(|| "—".to_owned(), |v| metric_text(metric, v))
        };
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
        let line = t::compared_of(language, kind.family, (kind.checked, kind.held));
        card_text(ui, Words::made(&line), false);
    }
    let Some(colours) = kinds.iter().find(|kind| kind.family == Family::Colour) else {
        practice_card_colour::summary_rows(ui, kinds);
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
        // Smaller where the combinations' and scales' rows follow, so the card fits the screen.
        let more =
            (kinds.iter()).any(|kind| kind.family != Family::Colour && !kind.rows.is_empty());
        let side = if more {
            SUMMARY_RADAR
        } else {
            DIALOG_WIDTH - 4.0 * SPACE
        };
        radar(ui, &axes, side);
    }
    let heads = [t::COLOURS, t::THIS_SESSION, t::BEFORE].map(|head| head.get(language));
    card_columns(ui, &heads, true);
    for (metric, this, before) in &colours.rows {
        let shown =
            |mean: &Option<f64>| mean.map_or_else(|| "—".to_owned(), |v| metric_text(*metric, v));
        let name = t::metric(*metric).get(language);
        card_columns(ui, &[name, &shown(this), &shown(before)], false);
    }
    practice_card_colour::summary_rows(ui, kinds);
    let note = t::radar_note(language, practice.stage);
    card_text(ui, Words::made(&note), true);
}

fn share(value: Option<f64>) -> Option<f32> {
    value.map(|v| v as f32)
}

/// A metric's value in its plain unit.
pub(crate) fn metric_text(metric: Metric, value: f64) -> String {
    match metric {
        Metric::FirstDelta | Metric::FinalDelta | Metric::AreasDelta => {
            format!("{:.1}", value * 100.0)
        }
        Metric::FirstHue | Metric::FinalHue => format!("{value:+.0}°"),
        Metric::FirstSaturation
        | Metric::FinalSaturation
        | Metric::FirstValue
        | Metric::FinalValue => format!("{:+.0} %", value * 100.0),
        Metric::ScaleOff => format!("{:.0} %", value * 100.0),
        Metric::Corrections
        | Metric::AreasWeak
        | Metric::AreasMadeStrong
        | Metric::ScaleUneven
        | Metric::ScaleReversed => format!("{value:.1}"),
    }
}
