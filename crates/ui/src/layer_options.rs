//! The layer panel's footer, for the highlighted layer (Procreate keeps these in the layer's
//! options): its opacity, its blend mode (Normal, Multiply, Screen, Overlay), Alpha Lock,
//! Clipping Mask, move up, move down, Group (Ungroup in a group) and delete; under them, for a
//! layer in a group, the group's name and opacity.

use openpaint_app::{EditorAction, Intent, LayerIntent, LayerList, LayerRow, SessionProjection};
use openpaint_core::BlendMode;

use crate::components::{icon_button, slider, switch_row, text_button, text_choice_wide};
use crate::design::{Icon, SPACE, TOUCH_TARGET};
use crate::layer_rows::percent;

/// Rows of touch targets the footer takes, with the group's two when the layer is in one.
pub(crate) const fn footer_rows(grouped: bool) -> f32 {
    if grouped { 9.0 } else { 7.0 }
}

fn offered(projection: &SessionProjection, action: EditorAction) -> bool {
    projection.availability(action).is_offered()
}

fn name(blend: BlendMode) -> &'static str {
    match blend {
        BlendMode::Normal => "Normal",
        BlendMode::Multiply => "Multiply",
        BlendMode::Screen => "Screen",
        BlendMode::Overlay => "Overlay",
    }
}

/// A label at the left and a value at the right, one touch target high.
fn value_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(2.0 * SPACE);
            ui.label(value);
        });
    });
}

/// One value of a slider touch, sent once (egui may run a pass twice).
fn push_once(taps: &mut Vec<Intent>, intent: Intent) {
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}

pub(crate) fn footer(
    ui: &mut egui::Ui,
    list: &LayerList,
    projection: &SessionProjection,
    taps: &mut Vec<Intent>,
) {
    let Some(active) = list.active_row() else {
        return;
    };
    let enabled = offered(projection, EditorAction::EditLayer);
    ui.separator();
    value_row(ui, "Opacity", &percent(active.opacity));
    if let Some(change) = slider(ui, "Opacity", active.opacity, enabled) {
        let intent = Intent::Layer(LayerIntent::SetOpacity {
            layer: active.id,
            opacity: change.value,
            settled: change.settled,
        });
        push_once(taps, intent);
    }
    blends(ui, active, enabled, taps);
    if switch_row(ui, "Alpha Lock", active.alpha_locked, enabled).clicked() {
        taps.push(Intent::Layer(LayerIntent::SetAlphaLock {
            layer: active.id,
            locked: !active.alpha_locked,
        }));
    }
    if switch_row(ui, "Clipping Mask", active.clipping, enabled).clicked() {
        taps.push(Intent::Layer(LayerIntent::SetClipping {
            layer: active.id,
            clipping: !active.clipping,
        }));
    }
    moves(ui, active, projection, taps);
    group(ui, list, active, enabled, taps);
}

/// The four modes in two even columns, the layer's own outlined.
fn blends(ui: &mut egui::Ui, active: &LayerRow, enabled: bool, taps: &mut Vec<Intent>) {
    let half = (ui.available_width() / 2.0).floor();
    for pair in BlendMode::ALL.chunks(2) {
        ui.horizontal(|ui| {
            for blend in pair {
                let chosen = active.blend == *blend;
                let tapped = text_choice_wide(ui, name(*blend), (enabled, chosen), half);
                if tapped.clicked() && !chosen {
                    taps.push(Intent::Layer(LayerIntent::SetBlend {
                        layer: active.id,
                        blend: *blend,
                    }));
                }
            }
        });
    }
}

fn moves(
    ui: &mut egui::Ui,
    active: &LayerRow,
    projection: &SessionProjection,
    taps: &mut Vec<Intent>,
) {
    ui.horizontal(|ui| {
        let moves = [
            (Icon::Raise, "Move layer up", EditorAction::RaiseLayer),
            (Icon::Lower, "Move layer down", EditorAction::LowerLayer),
        ];
        for (icon, label, action) in moves {
            if icon_button(ui, icon, label, offered(projection, action)).clicked() {
                taps.push(Intent::Layer(match action {
                    EditorAction::RaiseLayer => LayerIntent::Raise(active.id),
                    _ => LayerIntent::Lower(active.id),
                }));
            }
        }
        let (label, intent, action) = match active.group {
            Some(group) => (
                "Ungroup",
                LayerIntent::Ungroup(group),
                EditorAction::EditLayer,
            ),
            None => (
                "Group",
                LayerIntent::Group(active.id),
                EditorAction::GroupLayer,
            ),
        };
        if text_button(ui, label, offered(projection, action)).clicked() {
            taps.push(Intent::Layer(intent));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let enabled = offered(projection, EditorAction::DeleteLayer);
            if icon_button(ui, Icon::Delete, "Delete layer", enabled).clicked() {
                taps.push(Intent::Layer(LayerIntent::Delete(active.id)));
            }
        });
    });
}

/// The active layer's group: its name and opacity, the whole group fading with one slider.
fn group(
    ui: &mut egui::Ui,
    list: &LayerList,
    active: &LayerRow,
    enabled: bool,
    taps: &mut Vec<Intent>,
) {
    let Some(group) = active.group.and_then(|id| list.group(id)) else {
        return;
    };
    ui.separator();
    value_row(ui, group.name.as_str(), &percent(group.opacity));
    if let Some(change) = slider(ui, "Group opacity", group.opacity, enabled) {
        let intent = Intent::Layer(LayerIntent::SetGroupOpacity {
            group: group.id,
            opacity: change.value,
            settled: change.settled,
        });
        push_once(taps, intent);
    }
}
