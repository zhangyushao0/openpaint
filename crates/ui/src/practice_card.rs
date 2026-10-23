//! What the practice card says: how to practise each kind (our own words after V大's method:
//! look first, fix the ends and the most bent points against the frame, correct over rather than
//! erase; name a colour, pick once by eye, compare and correct two or three times), a check's
//! numbers in plain units, and a session's means beside the earlier sessions'.

use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{
    KindSummary, LEAST_CHROMA, Metric, Metrics, Original, Rgb8, chroma, hsv, hue_name, value,
};

use crate::components::{card_columns, card_text, text_button};

/// The instructions for lines (or colours); returns whether Got it was tapped.
pub(crate) fn instructions(ui: &mut egui::Ui, lines: bool) -> bool {
    let (heading, text): (&str, [&str; 4]) = if lines {
        (
            "Copy a line",
            [
                "Copy the line into the empty frame beside it.",
                "Look before you draw: find its two ends and where it bends most, and judge \
                 where they sit in the frame, at a half or a third.",
                "Draw it in one sure stroke. If it goes wrong, don't erase: draw the better \
                 line over it.",
                "Then tap Compare to lay the original over your line.",
            ],
        )
    } else {
        (
            "Match a colour",
            [
                "Fill the empty frame with the colour of the swatch beside it.",
                "Name it first: its hue, how pure or grey, how light or dark. Then pick it \
                 once by eye and fill.",
                "Compare by eye: redder or yellower? Lighter or darker? Purer or greyer? \
                 Correct two or three times.",
                "The swatch can't be picked with the eyedropper until you tap Compare.",
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

/// The shown exercise's numbers once checked.
pub(crate) fn results(ui: &mut egui::Ui, practice: &PracticeProjection) {
    let (Some(index), Some(metrics)) = (practice.current, &practice.result) else {
        return;
    };
    let (place, _) = practice.place;
    match &practice.sheet.originals[index] {
        Original::Line(_) => line_results(ui, place, metrics),
        Original::Colour(original) => {
            let laid = match practice.overlay {
                Some(Overlay::Colour { first, last, .. }) => [Some(first), Some(last)],
                _ => [None; 2],
            };
            colour_results(ui, (place, *original, laid), metrics);
        }
    }
}

/// A metric's value as the card shows it, "—" without one.
fn shown(metrics: &Metrics, metric: Metric) -> String {
    value(metrics, metric).map_or_else(|| "—".to_owned(), |v| text(metric, v))
}

/// Line `place`'s numbers.
fn line_results(ui: &mut egui::Ui, place: usize, metrics: &Metrics) {
    ui.heading(format!("Line {place}"));
    for metric in [Metric::KeyPoints, Metric::Frame, Metric::Shape] {
        card_columns(ui, &[label(metric), &shown(metrics, metric)], false);
    }
    let strokes = value(metrics, Metric::Strokes).unwrap_or(0.0);
    let seconds = value(metrics, Metric::Seconds).unwrap_or(0.0);
    let plural = if strokes == 1.0 { "" } else { "s" };
    let stroke = format!("{strokes:.0} stroke{plural} in {seconds:.1} s");
    card_columns(ui, &["Stroke", &stroke], false);
    for metric in [Metric::Speed, Metric::Wobble] {
        card_columns(ui, &[label(metric), &shown(metrics, metric)], false);
    }
    card_text(
        ui,
        "Distances in % of the frame's side; speed in frame sides a second. In red: the \
         original and its key points.",
        true,
    );
}

/// Colour `place`'s numbers against its swatch `original`, the first and last colours `laid`
/// as the overlay holds them.
fn colour_results(
    ui: &mut egui::Ui,
    (place, original, laid): (usize, Rgb8, [Option<Rgb8>; 2]),
    metrics: &Metrics,
) {
    ui.heading(format!("Colour {place}"));
    let firsts = [
        Metric::FirstDelta,
        Metric::FirstHue,
        Metric::FirstSaturation,
        Metric::FirstValue,
    ];
    let finals = [
        Metric::FinalDelta,
        Metric::FinalHue,
        Metric::FinalSaturation,
        Metric::FinalValue,
    ];
    for ((name, set), copy) in [("First pick", firsts), ("Final", finals)]
        .into_iter()
        .zip(laid)
    {
        card_columns(ui, &[name, &shown(metrics, set[0])], false);
        let parts = [set[1], set[2], set[3]].map(|metric| value(metrics, metric));
        card_text(ui, &difference((original, copy), parts), true);
    }
    let corrections = value(metrics, Metric::Corrections).unwrap_or(0.0);
    card_columns(ui, &["Corrections", &format!("{corrections:.0}")], false);
    card_text(
        ui,
        "ΔE: how different the two look (Oklab × 100); about 2 is just noticeable. Each colour \
         shows inside the other's frame.",
        true,
    );
}

/// How a colour (`copy`, when known) differs from `original`: its hue towards a named
/// neighbour (or another hue altogether, or grey where the other has colour), purer or greyer,
/// lighter or darker.
fn difference(
    (original, copy): (Rgb8, Option<Rgb8>),
    [hue, saturation, light]: [Option<f64>; 3],
) -> String {
    let original_hue = hsv(original)[0];
    let mut parts = Vec::new();
    match hue.filter(|h| h.abs() >= 1.0) {
        Some(hue) if hue.abs() >= 45.0 => {
            let (mine, theirs) = (hue_name(original_hue + hue), hue_name(original_hue));
            parts.push(format!("{mine} where it is {theirs}"));
        }
        Some(hue) => {
            let towards = hue_name(original_hue + 30.0 * hue.signum());
            parts.push(format!("{:.0}° towards {towards}", hue.abs()));
        }
        None => {
            // Too grey for a hue: say which one has none.
            let grey = |c: Rgb8| chroma(c) < LEAST_CHROMA;
            let name = |c: Rgb8| if grey(c) { "grey" } else { hue_name(hsv(c)[0]) };
            if let Some(copy) = copy.filter(|copy| grey(*copy) != grey(original)) {
                parts.push(format!("{} where it is {}", name(copy), name(original)));
            }
        }
    }
    if let Some(s) = saturation.filter(|s| s.abs() >= 0.01) {
        let way = if s > 0.0 { "purer" } else { "greyer" };
        parts.push(format!("{:.0} % {way}", s.abs() * 100.0));
    }
    if let Some(v) = light.filter(|v| v.abs() >= 0.01) {
        let way = if v > 0.0 { "lighter" } else { "darker" };
        parts.push(format!("{:.0} % {way}", v.abs() * 100.0));
    }
    if parts.is_empty() {
        "The same hue, purity and lightness.".to_owned()
    } else {
        format!("Yours is {}.", parts.join(", "))
    }
}

/// The summary: per kind, this session's means beside the earlier sessions'.
pub(crate) fn summary(ui: &mut egui::Ui, practice: &PracticeProjection, kinds: &[KindSummary]) {
    ui.heading(format!("Session {} done", practice.number));
    for kind in kinds {
        let name = if kind.lines { "Lines" } else { "Colours" };
        let head = format!("{name}: {} of {} compared", kind.checked, kind.held);
        card_columns(ui, &[&head, "this", "before"], true);
        for (metric, this, before) in &kind.rows {
            let shown =
                |mean: &Option<f64>| mean.map_or_else(|| "—".to_owned(), |v| text(*metric, v));
            card_columns(ui, &[label(*metric), &shown(this), &shown(before)], false);
        }
    }
    let note = format!(
        "Before: the mean of your earlier sessions in stage {}. Distances in % of the frame's \
         side, ΔE × 100.",
        practice.stage
    );
    card_text(ui, &note, true);
}

/// A metric's name on the card.
const fn label(metric: Metric) -> &'static str {
    match metric {
        Metric::KeyPoints => "Key points",
        Metric::Frame => "In the frame",
        Metric::Shape => "Shape",
        Metric::Speed => "Speed",
        Metric::Wobble => "Wobble",
        Metric::Strokes => "Strokes",
        Metric::Erased => "Erased",
        Metric::Seconds => "Time",
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
        Metric::KeyPoints | Metric::Frame | Metric::Shape | Metric::Wobble => {
            format!("{:.1} %", value * 100.0)
        }
        Metric::Speed => format!("{value:.2} /s"),
        Metric::Seconds => format!("{value:.1} s"),
        Metric::FirstDelta | Metric::FinalDelta => format!("{:.1}", value * 100.0),
        Metric::FirstHue | Metric::FinalHue => format!("{value:+.0}°"),
        Metric::FirstSaturation
        | Metric::FinalSaturation
        | Metric::FirstValue
        | Metric::FinalValue => format!("{:+.0} %", value * 100.0),
        Metric::Strokes | Metric::Erased => format!("{value:.0}"),
        Metric::Corrections => format!("{value:.1}"),
    }
}
