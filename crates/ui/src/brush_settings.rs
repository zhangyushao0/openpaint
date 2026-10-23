//! The brush editor's settings, one scrolling column grouped by a word each: Stroke
//! (Spacing, StreamLine, the taper), Shape (the stamp's shape as words in two columns, its
//! aspect, rotation, angle, jitters, scatter and the edge's softness), Grain, Paint (Covering or
//! Builds up, and under Builds up the tip, flow, maximum density and paper), Size (the size
//! slider's ends) and Dynamics (two rows opening the curves' pages). Every slider stops where the
//! engine stops; a setting that does nothing for the brush now is pencil grey.

use openpaint_app::{
    Axis, BrushChange, BrushIntent, BrushProjection, Dynamic, EditedBrush, Intent, Knob, Language,
};
use openpaint_core::{Rotation, ShapeTexture, Stamp, Tip};

use crate::components::{
    Lead, SettingSlider, choices, hint, label_row, menu_row, section, setting_slider,
    text_choice_wide, word_columns,
};
use crate::design::{INK, SPACE, TOUCH_TARGET};
use crate::taps::push_brush;
use crate::text::{self as t, Text, Words, language};

/// The stamps' shapes in the order the words show, two to a row.
const STAMPS: [Stamp; 7] = [
    Stamp::Disc,
    Stamp::Square,
    Stamp::Ellipse,
    Stamp::Texture(ShapeTexture::Chalk),
    Stamp::Texture(ShapeTexture::Grainy),
    Stamp::Texture(ShapeTexture::Splatter),
    Stamp::Texture(ShapeTexture::RoughSquare),
];
const ROTATIONS: [Rotation; 3] = [Rotation::Fixed, Rotation::Direction, Rotation::Azimuth];

fn edit(taps: &mut Vec<Intent>, change: BrushChange, settled: bool) {
    push_brush(taps, BrushIntent::Edit { change, settled });
}

/// The column. Returns the dynamics page a row opened, if one was tapped.
pub(crate) fn settings(
    ui: &mut egui::Ui,
    brush: &BrushProjection,
    taps: &mut Vec<Intent>,
) -> Option<Dynamic> {
    let edited = &brush.edited;
    section(ui, t::STROKE);
    knob(ui, edited, Knob::Spacing, taps);
    streamline(ui, brush.streamline.amount(), taps);
    for taper in [Knob::TaperStart, Knob::TaperEnd, Knob::TaperTip] {
        knob(ui, edited, taper, taps);
    }
    section(ui, t::SHAPE);
    stamps(ui, edited, taps);
    knob(ui, edited, Knob::Aspect, taps);
    rotations(ui, edited, taps);
    for shape in [
        Knob::Angle,
        Knob::RotationJitter,
        Knob::SizeJitter,
        Knob::Scatter,
    ] {
        knob(ui, edited, shape, taps);
    }
    knob(ui, edited, Knob::Softness, taps);
    section(ui, t::GRAIN);
    knob(ui, edited, Knob::GrainStrength, taps);
    knob(ui, edited, Knob::GrainScale, taps);
    paint(ui, edited, taps);
    section(ui, t::SIZE);
    knob(ui, edited, Knob::SizeMin, taps);
    knob(ui, edited, Knob::SizeMax, taps);
    section(ui, t::DYNAMICS);
    let mut opened = None;
    for dynamic in [Dynamic::Size, Dynamic::Opacity] {
        let responds = responds(language(ui.ctx()), edited, dynamic);
        if menu_row(ui, t::dynamic(dynamic), Words::made(&responds), true).clicked() {
            opened = Some(dynamic);
        }
    }
    opened
}

/// What the dynamics follow: the axes whose curve is not 1 everywhere, or None.
fn responds(language: Language, edited: &EditedBrush, dynamic: Dynamic) -> String {
    let axes: Vec<&str> = Axis::ALL
        .into_iter()
        .filter(|axis| edited.curve(dynamic, *axis).iter().any(|[_, y]| *y != 1.0))
        .map(|axis| t::axis(axis).get(language))
        .collect();
    if axes.is_empty() {
        t::NONE.get(language).to_owned()
    } else {
        axes.join(" · ")
    }
}

/// A number's slider over its range now; grey when it does nothing for the brush.
fn knob(ui: &mut egui::Ui, edited: &EditedBrush, knob: Knob, taps: &mut Vec<Intent>) {
    let range = edited.range(knob);
    let value = edited.value(knob);
    let shown = shown(language(ui.ctx()), knob, value);
    let slider = SettingSlider {
        label: t::knob(knob).into(),
        shown: &shown,
        position: range.map_or(0.0, |range| range.position(value)),
        enabled: range.is_some(),
    };
    if let (Some(change), Some(range)) = (setting_slider(ui, &slider), range) {
        let set = BrushChange::Set(knob, range.at(change.value));
        edit(taps, set, change.settled);
    }
}

/// How a value reads in `language`.
pub(crate) fn shown(language: Language, knob: Knob, value: f32) -> String {
    let percent = |value: f32| format!("{} %", (value * 100.0).round());
    match knob {
        Knob::Spacing if value <= 0.0 => t::CONTINUOUS.get(language).to_owned(),
        Knob::Spacing | Knob::TaperTip | Knob::Aspect | Knob::Softness => percent(value),
        Knob::GrainStrength | Knob::MaxDensity => percent(value),
        Knob::TaperStart | Knob::TaperEnd => t::times(language, value, false),
        Knob::SizeJitter => format!("± {}", percent(value)),
        Knob::Angle => format!("{}°", value.round()),
        Knob::RotationJitter => format!("± {}°", value.round()),
        Knob::Scatter => t::times(language, value, true),
        Knob::GrainScale => format!("{} px", value.round()),
        Knob::Flow => format!("{value:.2}"),
        Knob::SizeMin | Knob::SizeMax if value < 10.0 => format!("{value:.1} px"),
        Knob::SizeMin | Knob::SizeMax => format!("{} px", value.round()),
    }
}

/// The brush's StreamLine for the tool, as the brush list's slider sets it.
fn streamline(ui: &mut egui::Ui, amount: f32, taps: &mut Vec<Intent>) {
    let shown = format!("{} %", (amount * 100.0).round());
    let slider = SettingSlider {
        label: t::STREAMLINE.into(),
        shown: &shown,
        position: amount,
        enabled: true,
    };
    if let Some(change) = setting_slider(ui, &slider) {
        let intent = BrushIntent::StreamLine {
            amount: change.value,
            settled: change.settled,
        };
        push_brush(taps, intent);
    }
}

/// The stamps' shapes as words in two columns, the chosen in the ink pill (a continuous
/// stroke's round tip is the disc); while the paper keeps the stroke continuous, only the disc.
fn stamps(ui: &mut egui::Ui, edited: &EditedBrush, taps: &mut Vec<Intent>) {
    let stamps = stamp_words(edited);
    let words = stamps.map(|(_, label, on, enabled)| (Words::from(label), on, enabled));
    if let Some(at) = word_columns(ui, "stamps", &words, Lead::Indent) {
        edit(taps, BrushChange::Stamp(stamps[at].0), true);
    }
    if !edited.offers_stamps() {
        hint(ui, t::PAPER_CONTINUOUS);
    }
}

/// The shapes' words: the shape, its word, whether it is chosen (a continuous stroke's round
/// tip is the disc) and whether it may be (only the disc while the paper keeps the stroke
/// continuous).
fn stamp_words(edited: &EditedBrush) -> [(Stamp, Text, bool, bool); 7] {
    let chosen = edited.stamp().unwrap_or(Stamp::Disc);
    STAMPS.map(|stamp| {
        let enabled = stamp == Stamp::Disc || edited.offers_stamps();
        (stamp, t::stamp(stamp), stamp == chosen, enabled)
    })
}

/// Which way stamps turn, under its name; grey for a continuous stroke.
fn rotations(ui: &mut egui::Ui, edited: &EditedBrush, taps: &mut Vec<Intent>) {
    let chosen = edited.rotation();
    label_row(ui, t::ROTATION, chosen.is_some());
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(SPACE);
        choices(ui, "rotation", |ui, group| {
            for rotation in ROTATIONS {
                let on = chosen == Some(rotation);
                let enabled = chosen.is_some();
                let label = t::rotation(rotation);
                let word = text_choice_wide(ui, group, label, (enabled, on), TOUCH_TARGET);
                if word.clicked() && !on {
                    edit(taps, BrushChange::Rotation(rotation), true);
                }
            }
        });
    });
}

/// Covering or Builds up; under Builds up the tip, flow, maximum density and paper.
fn paint(ui: &mut egui::Ui, edited: &EditedBrush, taps: &mut Vec<Intent>) {
    section(ui, t::PAINT);
    let tip = edited.tip();
    let half = ((ui.available_width() - 2.0 * SPACE) / 2.0).floor();
    ui.horizontal(|ui| {
        ui.add_space(2.0 * SPACE);
        choices(ui, "deposit", |ui, group| {
            for (builds, label) in [(false, t::COVERING), (true, t::BUILDS_UP)] {
                let on = tip.is_some() == builds;
                if text_choice_wide(ui, group, label, (true, on), half).clicked() && !on {
                    edit(taps, BrushChange::BuildsUp(builds), true);
                }
            }
        });
    });
    let Some(tip) = tip else {
        return;
    };
    let tips = [
        (Tip::Flat, t::TIP_FLAT, true),
        (Tip::Bell, t::BELL, edited.offers_bell()),
    ];
    pair(
        ui,
        t::TIP,
        tips.map(|(each, label, on)| (label, each == tip, on)),
        |at| {
            edit(taps, BrushChange::Tip(tips[at].0), true);
        },
    );
    knob(ui, edited, Knob::Flow, taps);
    knob(ui, edited, Knob::MaxDensity, taps);
    let paper = edited.paper();
    let papers = [
        (t::NONE, !paper, true),
        (t::SKETCH_PAPER, paper, edited.offers_paper()),
    ];
    pair(ui, t::PAPER, papers, |at| {
        edit(taps, BrushChange::Paper(at == 1), true);
    });
}

/// A row: `label` at the left, then two words (label, chosen, enabled); `tapped` with the
/// index of a word tapped that was not chosen.
fn pair(
    ui: &mut egui::Ui,
    label: Text,
    words: [(Text, bool, bool); 2],
    mut tapped: impl FnMut(usize),
) {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        let size = egui::vec2(13.0 * SPACE, TOUCH_TARGET);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let font = egui::TextStyle::Body.resolve(ui.style());
        let at = rect.left_center();
        let shown = Words::from(label).shown(ui.ctx());
        ui.painter()
            .text(at, egui::Align2::LEFT_CENTER, shown, font, INK);
        choices(ui, label.id(), |ui, group| {
            for (at, (word, on, enabled)) in words.into_iter().enumerate() {
                let choice = text_choice_wide(ui, group, word, (enabled, on), TOUCH_TARGET);
                if choice.clicked() && !on {
                    tapped(at);
                }
            }
        });
    });
}

#[cfg(test)]
#[path = "brush_settings_tests.rs"]
mod tests;
