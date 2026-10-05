//! The layer panel's footer, for the highlighted layer (Procreate keeps these in the layer's
//! options): its opacity, its blend mode (Normal, Multiply, Screen, Overlay), Alpha Lock,
//! Clipping Mask, Add Mask (a plus and the words: adds a white mask), Rename, Group (Ungroup in
//! a group), duplicate and delete (S38: moving is dragging a row, `layer_drag.rs`); under them,
//! for a layer in a group, the group's name and opacity. While the layer's
//! mask is highlighted instead: Invert, Clear and delete the mask.

use openpaint_app::{EditorAction, Intent, LayerIntent, LayerList, LayerRow, SessionProjection};
use openpaint_core::BlendMode;

use crate::components::{
    choices, command_row, hint, icon_button, slider, switch_row, text_button, text_choice_wide,
    value_row,
};
use crate::design::Icon;
use crate::layer_rows::percent;

/// Rows of touch targets the footer takes, with the group's two when the layer is in one; the
/// mask's options take two.
pub(crate) const fn footer_rows(grouped: bool, on_mask: bool) -> f32 {
    match (on_mask, grouped) {
        (true, _) => 2.0,
        (false, true) => 10.0,
        (false, false) => 8.0,
    }
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
) -> Option<egui::Rect> {
    let active = list.active_row()?;
    if list.on_mask {
        mask_options(ui, active, projection, taps);
        return None;
    }
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
    let can_mask = offered(projection, EditorAction::AddMask);
    if command_row(ui, Icon::Add, "Add Mask", can_mask).clicked() {
        taps.push(Intent::Layer(LayerIntent::AddMask(active.id)));
    }
    let rename = commands(ui, active, projection, taps);
    group(ui, list, active, enabled, taps);
    rename
}

/// While the active layer's mask is the target: what painting there does (a quieter hint),
/// Invert and Clear as bare words, and delete. Turning it off is its row's eye.
fn mask_options(
    ui: &mut egui::Ui,
    active: &LayerRow,
    projection: &SessionProjection,
    taps: &mut Vec<Intent>,
) {
    ui.separator();
    hint(ui, "Black hides · white shows");
    ui.horizontal(|ui| {
        let enabled = offered(projection, EditorAction::ChangeMask);
        if text_button(ui, "Invert", enabled).clicked() {
            taps.push(Intent::Layer(LayerIntent::InvertMask(active.id)));
        }
        if text_button(ui, "Clear", enabled).clicked() {
            taps.push(Intent::Layer(LayerIntent::ClearMask(active.id)));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let enabled = offered(projection, EditorAction::EditMask);
            if icon_button(ui, Icon::Delete, "Delete mask", enabled).clicked() {
                taps.push(Intent::Layer(LayerIntent::DeleteMask(active.id)));
            }
        });
    });
}

/// The four modes in two even columns, the layer's own carrying the ink pill (one group: the
/// pill slides across the grid to the mode chosen).
fn blends(ui: &mut egui::Ui, active: &LayerRow, enabled: bool, taps: &mut Vec<Intent>) {
    let half = (ui.available_width() / 2.0).floor();
    choices(ui, "blend modes", |ui, group| {
        for pair in BlendMode::ALL.chunks(2) {
            ui.horizontal(|ui| {
                for blend in pair {
                    let chosen = active.blend == *blend;
                    let word = (enabled, chosen);
                    let tapped = text_choice_wide(ui, group, name(*blend), word, half);
                    if tapped.clicked() && !chosen {
                        taps.push(Intent::Layer(LayerIntent::SetBlend {
                            layer: active.id,
                            blend: *blend,
                        }));
                    }
                }
            });
        }
    });
}

/// Rename and Group (Ungroup in a group) as words, then the copy and delete icons at the end.
/// Returns where Rename lies when it was tapped: its dialog grows from there.
fn commands(
    ui: &mut egui::Ui,
    active: &LayerRow,
    projection: &SessionProjection,
    taps: &mut Vec<Intent>,
) -> Option<egui::Rect> {
    ui.horizontal(|ui| {
        let edit = offered(projection, EditorAction::EditLayer);
        let rename = text_button(ui, "Rename", edit);
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
            let room = offered(projection, EditorAction::AddLayer);
            if icon_button(ui, Icon::Duplicate, "Duplicate layer", room).clicked() {
                taps.push(Intent::Layer(LayerIntent::Duplicate(active.id)));
            }
        });
        rename.clicked().then_some(rename.rect)
    })
    .inner
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
