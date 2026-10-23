//! What the practice card says: how to practise each kind (our own words after V大's method:
//! look first, mark the ends and the most bent points, draw in one sure stroke, correct over
//! rather than erase; name a colour, pick once by eye, compare and correct two or three times),
//! a comparison (a line: in place or aligned, no numbers; a colour: both on the wheel with the
//! way from yours to the original, its differences in V大's words), and a session's summary (the
//! colour judgement's radar over the earlier sessions', the colour means beside theirs).

use openpaint_app::{Hsv, Intent, Overlay, Pen, PracticeIntent, PracticeProjection};
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

/// The comparison wheel's side on the card, in points.
const COMPARE_WHEEL: f32 = 280.0;
/// Degrees of hue past which a pick is another hue altogether, named rather than compared.
const OTHER_HUE: f64 = 45.0;

/// The instructions for lines (or colours); returns whether Got it was tapped.
pub(crate) fn instructions(ui: &mut egui::Ui, lines: bool) -> bool {
    let (heading, text): (&str, [&str; 4]) = if lines {
        (
            "Copy a line",
            [
                "Copy the line into the empty frame beside it.",
                "Look before you draw: find its two ends and where it bends most, and judge \
                 where they sit in the frame. Mark them lightly with Sketch if you like.",
                "Draw the line with Draw in one sure stroke. If it goes wrong, don't erase: draw \
                 the better line over it.",
                "Then tap Compare: the original lies over your line in red, in place or aligned \
                 to it. Correct in blue and compare again, as often as you like.",
            ],
        )
    } else {
        (
            "Match a colour",
            [
                "Fill the empty frame with the colour of the swatch beside it.",
                "Name it first: its hue, how pure or grey, how light or dark. Then pick it once \
                 by eye on the wheel and tap Fill frame.",
                "Compare by eye: redder or yellower? Lighter or darker? Purer or greyer? Pick \
                 and fill again two or three times.",
                "Then tap Compare: the wheel shows where your colour lies against the original. \
                 The swatch can't be picked with the eyedropper before.",
            ],
        )
    };
    ui.heading(heading);
    for line in text {
        card_text(ui, line, false);
    }
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        text_button(ui, "Got it", true).clicked()
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
    ui.heading(format!("Line {place}"));
    ui.horizontal(|ui| {
        choices(ui, "overlay", |ui, group| {
            for (aligned, word) in [(false, "In place"), (true, "Aligned")] {
                let chosen = practice.aligned == aligned;
                if text_choice(ui, group, word, chosen).clicked() && !chosen {
                    push_once(taps, Intent::Practice(PracticeIntent::Aligned(aligned)));
                }
            }
        });
    });
    let shows = if practice.aligned {
        "In red: the original moved and sized onto your line, never turned. Its shape alone \
         shows, wherever you placed it."
    } else {
        "In red: the original where it lies in its frame. Where you placed it shows too."
    };
    card_text(ui, shows, false);
    let blue = practice.pen.is_some_and(|(pen, _)| pen == Pen::Correct);
    let next = if blue {
        "Draw over your line in blue to correct it, then tap Compare again."
    } else {
        "Draw over your line to correct it, then tap Compare again."
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
    ui.heading(format!("Colour {place}"));
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
    card_text(ui, &difference((original, last), now), false);
    if let Some(result) = &practice.result {
        let metrics = &result.metrics;
        let shown =
            |metric| value(metrics, metric).map_or_else(|| "—".to_owned(), |v| text(metric, v));
        let corrections = value(metrics, Metric::Corrections).unwrap_or(0.0);
        let plural = if corrections == 1.0 { "" } else { "s" };
        let stored = format!(
            "Kept from your first Compare: ΔE {} at the first pick, {} at the last, after \
             {corrections:.0} correction{plural}.",
            shown(Metric::FirstDelta),
            shown(Metric::FinalDelta),
        );
        card_text(ui, &stored, true);
    }
    card_text(
        ui,
        "Yours in ink, the original in red, the arrows from yours to it. ΔE: Oklab × 100; \
         about 2 is just noticeable.",
        true,
    );
}

/// How `copy` differs from `original` (`by`), as V大 compares: redder or yellower, purer or
/// greyer, lighter or darker; another hue altogether (`OTHER_HUE` or more) by its name; or grey
/// where the other has a hue.
fn difference((original, copy): (Rgb8, Rgb8), by: Difference) -> String {
    let mut parts = Vec::new();
    let hue_of = |c: Rgb8| hsv(c)[0];
    match by.hue.filter(|h| h.abs() >= 1.0) {
        Some(hue) if hue.abs() >= OTHER_HUE => {
            let (mine, theirs) = (hue_name(hue_of(copy)), hue_name(hue_of(original)));
            parts.push(format!("{mine} where it is {theirs}"));
        }
        Some(hue) => {
            let word = hue_towards(hue_of(original), hue);
            parts.push(format!("{:.0}° {word}", hue.abs()));
        }
        None => {
            let grey = |c: Rgb8| chroma(c) < LEAST_CHROMA;
            let name = |c: Rgb8| if grey(c) { "grey" } else { hue_name(hue_of(c)) };
            if grey(copy) != grey(original) {
                parts.push(format!("{} where it is {}", name(copy), name(original)));
            }
        }
    }
    if by.saturation.abs() >= 0.01 {
        let way = if by.saturation > 0.0 {
            "purer"
        } else {
            "greyer"
        };
        parts.push(format!("{:.0} % {way}", by.saturation.abs() * 100.0));
    }
    if by.value.abs() >= 0.01 {
        let way = if by.value > 0.0 { "lighter" } else { "darker" };
        parts.push(format!("{:.0} % {way}", by.value.abs() * 100.0));
    }
    if parts.is_empty() {
        "The same hue, purity and lightness.".to_owned()
    } else {
        format!("Yours is {}.", parts.join(", "))
    }
}

/// The summary: what was compared, the colour judgement over the earlier sessions', and the
/// colour means beside theirs.
pub(crate) fn summary(ui: &mut egui::Ui, practice: &PracticeProjection, kinds: &[KindSummary]) {
    ui.heading(format!("Session {} done", practice.number));
    for kind in kinds {
        let name = if kind.lines { "Lines" } else { "Colours" };
        card_text(
            ui,
            &format!("{name}: {} of {} compared", kind.checked, kind.held),
            false,
        );
    }
    let Some(colours) = kinds.iter().find(|kind| !kind.lines) else {
        return;
    };
    if let Some([this, before]) = colours.judgement {
        let axes: Vec<RadarAxis<'_>> = Axis::ALL
            .iter()
            .map(|&axis| {
                (
                    axis_name(axis),
                    share(this.of(axis)),
                    share(before.of(axis)),
                )
            })
            .collect();
        radar(ui, &axes, DIALOG_WIDTH - 4.0 * SPACE);
    }
    card_columns(ui, &["Colours", "this", "before"], true);
    for (metric, this, before) in &colours.rows {
        let shown = |mean: &Option<f64>| mean.map_or_else(|| "—".to_owned(), |v| text(*metric, v));
        card_columns(ui, &[label(*metric), &shown(this), &shown(before)], false);
    }
    let note = format!(
        "The shape: your colour judgement, the rim exact. Hue falls to the centre at 30° off, \
         saturation and value at 25 %, dark and low-saturation colours at ΔE 10; first to final: \
         how much of the first pick's error your corrections took away. Dashed: your earlier \
         sessions in stage {}.",
        practice.stage
    );
    card_text(ui, &note, true);
}

fn share(value: Option<f64>) -> Option<f32> {
    value.map(|v| v as f32)
}

/// An axis's name on the radar.
const fn axis_name(axis: Axis) -> &'static str {
    match axis {
        Axis::Hue => "Hue",
        Axis::Saturation => "Saturation",
        Axis::Value => "Value",
        Axis::Dark => "Dark",
        Axis::LowSaturation => "Greyish",
        Axis::FirstToFinal => "First → final",
    }
}

/// A metric's name on the card.
const fn label(metric: Metric) -> &'static str {
    match metric {
        Metric::FirstDelta => "First pick ΔE",
        Metric::FinalDelta => "Final ΔE",
        Metric::FirstHue | Metric::FinalHue => "Hue",
        Metric::FirstSaturation | Metric::FinalSaturation => "Saturation",
        Metric::FirstValue | Metric::FinalValue => "Value",
        Metric::Corrections => "Corrections",
    }
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
