//! What the practice card says: how to practise each family (our own words after V大's method:
//! look first, mark the ends and the most bent points, draw in one sure stroke, correct over
//! rather than erase; combined lines seen as one shape, the outer key points first, then the
//! inside, compared rather than guessed; name a colour, pick once by eye, compare and correct two
//! or three times), each family's How saying once what Compare shows; a comparison, which shows
//! rather than tells (D108): lines in place or aligned, no numbers, a line only where Aligned is
//! unavailable; a colour both on the wheel and in the colour table; week 2's colour work and a
//! colour copy in `practice_card_colour.rs`, boxes and ellipses in `practice_card_geometry.rs`;
//! and a session's summary (the colour judgement's radar over the earlier sessions', the colour
//! means beside theirs). The words are `text/practice.rs`'s, in the pass's language.

use openpaint_app::{Hsv, Intent, Overlay, PracticeIntent, PracticeProjection};
use openpaint_core::Color;
use openpaint_practice::{Axis, Family, HUELESS, KindSummary, Metric, Rgb8, oklch};

use crate::components::{
    RadarAxis, card_columns, card_text, choices, color_compare, colour_table, hint, radar,
    text_button, text_choice_wide,
};
use crate::design::{CORRECTION, DIALOG_WIDTH, SPACE};
use crate::taps::push_once;
use crate::text::{self as t, Words, language};
use crate::{practice_card_colour, practice_card_geometry};

/// The comparison wheel's side on the card, in points.
const COMPARE_WHEEL: f32 = 280.0;
/// The summary's radar's side when the colour combinations' and value scales' rows follow it.
const SUMMARY_RADAR: f32 = 260.0;

/// The instructions for a family; returns whether Got it was tapped.
pub(crate) fn instructions(ui: &mut egui::Ui, family: Family) -> bool {
    let (heading, text) = t::instructions(family);
    ui.heading(Words::from(heading).shown(ui.ctx()));
    for line in text {
        card_text(ui, *line, false);
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
        t::shown_name(language, practice.sheet.name.as_str()).into_owned()
    } else {
        t::exercise(language, (family, practice.warming_up), practice.place.0)
    };
    let drawn = practice.comparing && family.drawn();
    match &practice.overlay {
        Some(Overlay::Colour { original, last, .. }) => {
            ui.heading(heading);
            pair_on_wheel(ui, (*original, *last), COMPARE_WHEEL);
        }
        _ if drawn && family.geometric() => {
            ui.heading(heading);
            practice_card_geometry::compared(ui, practice);
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
/// (while its fit is found, saying so); a shape without an outline says why Aligned is greyed.
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
    if family == Family::Shape && !practice.aligned_offered {
        card_text(ui, t::NO_OUTLINE, false);
    }
}

/// `original` and `mine` on the wheel (`side` points), the way from one to the other, and the
/// colour table under it.
pub(crate) fn pair_on_wheel(ui: &mut egui::Ui, (original, mine): (Rgb8, Rgb8), side: f32) {
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
    // A colour shows on the ring wherever the eye sees a hue: an exact grey's wheel hue is noise.
    let hued = [mine, original].map(|c| oklch(c)[1] >= HUELESS);
    ui.vertical_centered(|ui| {
        let colours = (as_hsv(mine), as_hsv(original));
        color_compare(ui, colours, (hued, CORRECTION), side);
    });
    colour_table(ui, (original, mine));
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
}

fn share(value: Option<f64>) -> Option<f32> {
    value.map(|v| v as f32)
}

/// A metric's value in its plain unit: colour differences × 100, as ΔE is shown.
pub(crate) fn metric_text(metric: Metric, value: f64) -> String {
    match metric {
        Metric::FirstDelta | Metric::FinalDelta | Metric::AreasDelta => {
            format!("{:.1}", value * 100.0)
        }
        Metric::FirstLightness
        | Metric::FinalLightness
        | Metric::FirstChroma
        | Metric::FinalChroma
        | Metric::FirstHue
        | Metric::FinalHue => format!("{:+.1}", value * 100.0),
        Metric::ScaleOff => format!("{:.0} %", value * 100.0),
        Metric::Corrections
        | Metric::AreasWeak
        | Metric::AreasMadeStrong
        | Metric::ScaleUneven
        | Metric::ScaleReversed => format!("{value:.1}"),
    }
}
