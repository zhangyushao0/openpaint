//! An adjustment's panel (`adjust_panel`): its name, its controls and the commands. Hue,
//! Saturation and Brightness are sliders with their value (50 % changes nothing, as in
//! Procreate). Curves shows Gamma (every channel), Red, Green and Blue as a choice and the
//! chosen one's curve in the curve editor. Gaussian Blur and Sharpen are one slider each, a
//! percentage (Radius: 100 % is a standard deviation of `Blur::MAX_RADIUS` canvas pixels;
//! Amount: 100 % adds `Sharpen::STRENGTH` times the detail). At the bottom Reset (back to no
//! change; for Curves the curve shown) and, at the right, Cancel and Apply, bare words. Every
//! value goes to the session at once; the canvas shows it.

use openpaint_app::{AdjustIntent, Intent};
use openpaint_core::{Adjustment, Blur, Curves, Hsb, Sharpen, ToneChannel, ToneCurve};

use crate::components::{choices, curve_editor, slider, text_button, text_choice, value_row};
use crate::design::{SPACE, TOUCH_TARGET};
use crate::layer_rows::percent;

/// The curves the choice offers, with Procreate's names.
const CHANNELS: [(&str, ToneChannel); 4] = [
    ("Gamma", ToneChannel::Composite),
    ("Red", ToneChannel::Red),
    ("Green", ToneChannel::Green),
    ("Blue", ToneChannel::Blue),
];

/// The panel's rows for `adjustment`, the curve editor showing `channel`.
pub(crate) fn controls(
    ui: &mut egui::Ui,
    adjustment: &Adjustment,
    channel: &mut ToneChannel,
    taps: &mut Vec<Intent>,
) {
    let reset = match adjustment {
        Adjustment::Hsb(hsb) => {
            heading(ui, "Hue, Saturation, Brightness");
            hsb_sliders(ui, *hsb, taps);
            (*hsb != Hsb::NONE).then_some(Adjustment::Hsb(Hsb::NONE))
        }
        Adjustment::Curves(curves) => {
            heading(ui, "Curves");
            curve_rows(ui, curves, channel, taps);
            let shown = curves.curve(*channel);
            (!shown.is_identity())
                .then(|| Adjustment::Curves(curves.with(*channel, ToneCurve::identity())))
        }
        Adjustment::Blur(blur) => {
            heading(ui, "Gaussian Blur");
            let share = blur.radius / Blur::MAX_RADIUS;
            if let Some(value) = one_slider(ui, "Radius", share) {
                let set = Adjustment::Blur(Blur::new(value * Blur::MAX_RADIUS));
                push(taps, Intent::Adjust(AdjustIntent::Set(set)));
            }
            (*blur != Blur::NONE).then_some(Adjustment::Blur(Blur::NONE))
        }
        Adjustment::Sharpen(sharpen) => {
            heading(ui, "Sharpen");
            if let Some(value) = one_slider(ui, "Amount", sharpen.amount) {
                let set = Adjustment::Sharpen(Sharpen::new(value));
                push(taps, Intent::Adjust(AdjustIntent::Set(set)));
            }
            (*sharpen != Sharpen::NONE).then_some(Adjustment::Sharpen(Sharpen::NONE))
        }
    };
    ui.add_space(SPACE);
    commands(ui, reset, taps);
}

fn hsb_sliders(ui: &mut egui::Ui, hsb: Hsb, taps: &mut Vec<Intent>) {
    let values = [hsb.hue, hsb.saturation, hsb.brightness];
    for (index, label) in ["Hue", "Saturation", "Brightness"].into_iter().enumerate() {
        value_row(ui, label, &percent(values[index]));
        if let Some(change) = slider(ui, label, values[index], true) {
            let mut set = values;
            set[index] = change.value;
            let set = Hsb::new(set[0], set[1], set[2]);
            push(
                taps,
                Intent::Adjust(AdjustIntent::Set(Adjustment::Hsb(set))),
            );
        }
    }
}

/// One slider with its percentage (Procreate's); the value the hand set, if it moved.
fn one_slider(ui: &mut egui::Ui, label: &str, value: f32) -> Option<f32> {
    value_row(ui, label, &percent(value));
    slider(ui, label, value, true).map(|change| change.value)
}

fn curve_rows(
    ui: &mut egui::Ui,
    curves: &Curves,
    channel: &mut ToneChannel,
    taps: &mut Vec<Intent>,
) {
    ui.horizontal(|ui| {
        choices(ui, "curve channel", |ui, group| {
            for (label, each) in CHANNELS {
                if text_choice(ui, group, label, *channel == each).clicked() {
                    *channel = each;
                }
            }
        });
    });
    ui.add_space(SPACE);
    if let Some(change) = curve_editor(ui, "Curve", curves.curve(*channel)) {
        let set = Adjustment::Curves(curves.with(*channel, change.curve));
        push(taps, Intent::Adjust(AdjustIntent::Set(set)));
    }
}

/// Reset at the left (grey with nothing to reset), Cancel and Apply at the right.
fn commands(ui: &mut egui::Ui, reset: Option<Adjustment>, taps: &mut Vec<Intent>) {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        if text_button(ui, "Reset", reset.is_some()).clicked()
            && let Some(reset) = reset
        {
            push(taps, Intent::Adjust(AdjustIntent::Set(reset)));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if text_button(ui, "Apply", true).clicked() {
                push(taps, Intent::Adjust(AdjustIntent::Apply));
            }
            if text_button(ui, "Cancel", true).clicked() {
                push(taps, Intent::Adjust(AdjustIntent::Cancel));
            }
        });
    });
}

/// A panel's name over its rows.
pub(crate) fn heading(ui: &mut egui::Ui, name: &str) {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.heading(name);
    });
}

/// egui may run a pass twice; one tap or value is sent once.
pub(crate) fn push(taps: &mut Vec<Intent>, intent: Intent) {
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}
