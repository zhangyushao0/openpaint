//! The layer panel's footer, for the highlighted layer (Procreate keeps these in the layer's
//! options): its opacity, its blend mode (Normal, Multiply, Screen, Overlay), Alpha Lock,
//! Clipping Mask, Add Mask (a plus and the words: adds a white mask), Merge Down (into the
//! layer below; grey, with why at its row's end, where it would change what shows), Rename,
//! Group (Ungroup in a group), duplicate and delete (moving is dragging a row,
//! `layer_drag.rs`); under them, for a layer in a group, the group's name and opacity. While
//! the layer's mask is highlighted instead: Invert, Clear and delete the mask.

use openpaint_app::{EditorAction, Intent, LayerIntent, LayerList, LayerRow, SessionProjection};
use openpaint_core::BlendMode;

use crate::components::{
    Lead, command_row, hint, icon_button, slider, switch_row, text_button, value_row, word_columns,
};
use crate::design::Icon;
use crate::layer_rows::percent;
use crate::taps::push_once;
use crate::text::{self as t, Words};

/// Rows of touch targets the footer takes, with the group's two when the layer is in one; the
/// mask's options take two.
pub(crate) const fn footer_rows(grouped: bool, on_mask: bool) -> f32 {
    match (on_mask, grouped) {
        (true, _) => 2.0,
        (false, true) => 11.0,
        (false, false) => 9.0,
    }
}

fn offered(projection: &SessionProjection, action: EditorAction) -> bool {
    projection.availability(action).is_offered()
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
    value_row(ui, t::OPACITY, &percent(active.opacity));
    if let Some(change) = slider(ui, t::OPACITY, active.opacity, enabled) {
        let intent = Intent::Layer(LayerIntent::SetOpacity {
            layer: active.id,
            opacity: change.value,
            settled: change.settled,
        });
        push_once(taps, intent);
    }
    blends(ui, active, enabled, taps);
    if switch_row(ui, t::ALPHA_LOCK, active.alpha_locked, enabled).clicked() {
        taps.push(Intent::Layer(LayerIntent::SetAlphaLock {
            layer: active.id,
            locked: !active.alpha_locked,
        }));
    }
    if switch_row(ui, t::CLIPPING_MASK, active.clipping, enabled).clicked() {
        taps.push(Intent::Layer(LayerIntent::SetClipping {
            layer: active.id,
            clipping: !active.clipping,
        }));
    }
    let can_mask = offered(projection, EditorAction::AddMask);
    let add = (Icon::Add, t::ADD_MASK.into(), Words::EMPTY);
    if command_row(ui, add, can_mask).clicked() {
        taps.push(Intent::Layer(LayerIntent::AddMask(active.id)));
    }
    let can_merge = offered(projection, EditorAction::MergeDown);
    let why = (list.merge_down).map_or(Words::EMPTY, |why| t::unmergeable(why).into());
    if command_row(ui, (Icon::MergeDown, t::MERGE_DOWN.into(), why), can_merge).clicked() {
        taps.push(Intent::Layer(LayerIntent::MergeDown(active.id)));
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
    hint(ui, t::BLACK_HIDES);
    ui.horizontal(|ui| {
        let enabled = offered(projection, EditorAction::ChangeMask);
        if text_button(ui, t::INVERT, enabled).clicked() {
            taps.push(Intent::Layer(LayerIntent::InvertMask(active.id)));
        }
        if text_button(ui, t::CLEAR, enabled).clicked() {
            taps.push(Intent::Layer(LayerIntent::ClearMask(active.id)));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let enabled = offered(projection, EditorAction::EditMask);
            if icon_button(ui, Icon::Delete, t::DELETE_MASK, enabled).clicked() {
                taps.push(Intent::Layer(LayerIntent::DeleteMask(active.id)));
            }
        });
    });
}

/// The four modes in two even columns, the layer's own carrying the ink pill (one group: the
/// pill slides across the grid to the mode chosen).
fn blends(ui: &mut egui::Ui, active: &LayerRow, enabled: bool, taps: &mut Vec<Intent>) {
    let words =
        BlendMode::ALL.map(|blend| (t::blend(blend).into(), active.blend == blend, enabled));
    if let Some(at) = word_columns(ui, "blend modes", &words, Lead::None) {
        let (layer, blend) = (active.id, BlendMode::ALL[at]);
        taps.push(Intent::Layer(LayerIntent::SetBlend { layer, blend }));
    }
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
        let rename = text_button(ui, t::RENAME, edit);
        let (label, intent, action) = match active.group {
            Some(group) => (
                t::UNGROUP,
                LayerIntent::Ungroup(group),
                EditorAction::EditLayer,
            ),
            None => (
                t::GROUP,
                LayerIntent::Group(active.id),
                EditorAction::GroupLayer,
            ),
        };
        if text_button(ui, label, offered(projection, action)).clicked() {
            taps.push(Intent::Layer(intent));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let enabled = offered(projection, EditorAction::DeleteLayer);
            if icon_button(ui, Icon::Delete, t::DELETE_LAYER, enabled).clicked() {
                taps.push(Intent::Layer(LayerIntent::Delete(active.id)));
            }
            let room = offered(projection, EditorAction::AddLayer);
            if icon_button(ui, Icon::Duplicate, t::DUPLICATE_LAYER, room).clicked() {
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
    let name = t::shown_name(t::language(ui.ctx()), group.name.as_str());
    value_row(ui, Words::made(&name), &percent(group.opacity));
    if let Some(change) = slider(ui, t::GROUP_OPACITY, group.opacity, enabled) {
        let intent = Intent::Layer(LayerIntent::SetGroupOpacity {
            group: group.id,
            opacity: change.value,
            settled: change.settled,
        });
        push_once(taps, intent);
    }
}
