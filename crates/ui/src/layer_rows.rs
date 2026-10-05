//! The layer panel's rows, top of the stack first: a group's row (its chevron, name, opacity
//! and eye) above its layers, which sit one step to the right and hide while the group is
//! closed; a Clipping Mask's row a further step right with the arrow down to its base. Each
//! layer row starts with a picture of its layer alone (S38) and ends with its blend mode's
//! letter (Procreate's "N") before its eye. A layer's mask is a row under it, one step further
//! right, with the mask's grey as its picture; the one drawn on (paint or mask) is outlined. A
//! row held still lifts and is dragged to another place (`layer_drag.rs`).

use std::collections::{HashMap, HashSet};

use egui::scroll_area::ScrollSource;
use openpaint_app::{GroupRow, Intent, LayerIntent, LayerList, LayerRow, MaskRow};
use openpaint_core::{BlendMode, GroupId, LayerId};

use crate::components::{ListRow, RowPicture, list_row};
use crate::design::{Icon, SPACE, TOUCH_TARGET};
use crate::layer_drag::{Drag, held, placed, slot};

/// How far a group's layers, and a Clipping Mask within its chain, sit right of the row above.
const INDENT: f32 = 3.0 * SPACE;
/// How much of the first row below the list's end shows: enough to cut through its name.
const CUT_ROW: f32 = 0.65;

/// The pictures loaded so far: a layer's paint (`false`) or its mask (`true`).
pub(crate) type Pictures = HashMap<(LayerId, bool), egui::TextureHandle>;

/// The panel's own state the rows read and change.
pub(crate) struct RowState<'a> {
    /// The closed groups; a group row's tap opens or closes it here.
    pub(crate) collapsed: &'a mut HashSet<GroupId>,
    pub(crate) drag: &'a mut Option<Drag>,
    pub(crate) pictures: &'a Pictures,
    /// A picture's size in points: the canvas fitted in `design::LAYER_PICTURE`.
    pub(crate) picture: egui::Vec2,
}

/// Opacity shown as a whole percentage.
pub(crate) fn percent(opacity: f32) -> String {
    format!("{} %", (opacity * 100.0).round())
}

/// The letter a row shows for its blend mode.
fn letter(blend: BlendMode) -> &'static str {
    match blend {
        BlendMode::Normal => "N",
        BlendMode::Multiply => "M",
        BlendMode::Screen => "S",
        BlendMode::Overlay => "O",
    }
}

/// What the panel shows of the layers: `room` points of rows at most (more scroll). Taps go to
/// `taps`; while a row is carried the list does not scroll and shows the layer where it lands.
pub(crate) fn rows(
    ui: &mut egui::Ui,
    (list, enabled): (&LayerList, bool),
    room: f32,
    state: RowState<'_>,
    taps: &mut Vec<Intent>,
) {
    let whole = (room / TOUCH_TARGET - CUT_ROW).floor().max(1.0);
    let source = if state.drag.is_some() {
        ScrollSource::NONE
    } else {
        ScrollSource::ALL
    };
    egui::ScrollArea::vertical()
        .max_height((whole + CUT_ROW) * TOUCH_TARGET)
        .scroll_source(source)
        .show(ui, |ui| {
            let top = ui.max_rect().top();
            let pitch = TOUCH_TARGET + ui.spacing().item_spacing.y;
            let (hand, down) = ui.input(|input| {
                let at = input.pointer.interact_pos().or(input.pointer.latest_pos());
                (at.map(|at| at.y - top), input.pointer.any_down())
            });
            let RowState {
                collapsed,
                drag,
                pictures,
                picture,
            } = state;
            carry(list, (hand, down, pitch), (drag, collapsed), taps);
            let shown = drag
                .as_ref()
                .map_or_else(|| list.clone(), |d| placed(list, d.layer, d.place));
            let lifted = drag.as_ref().zip(hand).map(|(d, hand)| {
                let row = slot(&shown, d.layer, collapsed) as f32 * pitch;
                // Never above the list's top, where it would be cut.
                (d.layer, d.top(hand).max(0.0) - row)
            });
            let mut above = None;
            for row in shown.rows.iter().rev() {
                if row.group != above
                    && let Some(group) = row.group.and_then(|id| shown.group(id))
                {
                    group_row(ui, (group, enabled), collapsed, taps);
                }
                above = row.group;
                let carried = lifted.filter(|(layer, _)| *layer == row.id);
                if row.group.is_some_and(|id| collapsed.contains(&id)) && carried.is_none() {
                    continue;
                }
                let shot = |mask| {
                    let texture = pictures.get(&(row.id, mask)).map(egui::TextureHandle::id);
                    Some(RowPicture {
                        texture,
                        size: picture,
                    })
                };
                let lift = carried.map(|(_, lift)| lift);
                let pressed = layer_row(ui, (row, &shown), (enabled, lift, shot(false)), taps);
                if drag.is_none() && enabled && held(ui, &pressed) {
                    let grab = hand.map_or(0.0, |hand| hand - (pressed.rect.top() - top));
                    *drag = Drag::lift(list, row.id, grab);
                }
                if let Some(mask) = row.mask {
                    mask_row(ui, (row, mask, &shown), (enabled, lift, shot(true)), taps);
                }
            }
        });
}

/// The carried row follows the hand at `hand` points below the list's top; let go (`down`
/// false), it lands where it shows, as one move.
fn carry(
    list: &LayerList,
    (hand, down, pitch): (Option<f32>, bool, f32),
    (drag, collapsed): (&mut Option<Drag>, &mut HashSet<GroupId>),
    taps: &mut Vec<Intent>,
) {
    if let (Some(carried), Some(hand), true) = (drag.as_mut(), hand, down) {
        carried.follow(list, (hand, pitch), collapsed);
    }
    if !down && let Some(dropped) = drag.take() {
        taps.extend(dropped.drop());
        // Dropped into a closed group: it opens, so the layer is seen where it went.
        if let Some(group) = dropped.place.group {
            collapsed.remove(&group);
        }
    }
}

/// How far a layer's row sits right: one step in a group, one more clipping to a base.
fn indent_of(row: &LayerRow) -> f32 {
    INDENT * (f32::from(u8::from(row.group.is_some())) + f32::from(u8::from(row.clipped)))
}

/// A layer's mask under its row, one step further right: chosen, strokes and fills go to it.
/// Its eye turns it on and off.
fn mask_row(
    ui: &mut egui::Ui,
    (row, mask, list): (&LayerRow, MaskRow, &LayerList),
    (enabled, lifted, picture): (bool, Option<f32>, Option<RowPicture>),
    taps: &mut Vec<Intent>,
) {
    let detail = match (mask.enabled, mask.inverted) {
        (false, _) => "Off",
        (true, true) => "Inverted",
        (true, false) => "",
    };
    let (icon, icon_label) = if mask.enabled {
        (Icon::Visible, "Turn mask off")
    } else {
        (Icon::Hidden, "Turn mask on")
    };
    let response = list_row(
        ui,
        &ListRow {
            key: egui::Id::new(("mask row", row.id.get())),
            label: "Layer Mask",
            detail,
            indent: indent_of(row) + INDENT,
            lead: Some(Icon::Mask),
            picture,
            selected: row.id == list.active && list.on_mask,
            icon,
            icon_label,
            enabled,
            lifted,
        },
    );
    if response.icon.clicked() {
        taps.push(Intent::Layer(LayerIntent::SetMaskEnabled {
            layer: row.id,
            enabled: !mask.enabled,
        }));
    } else if response.row.clicked() && mask.enabled {
        taps.push(Intent::Layer(LayerIntent::SelectMask(row.id)));
    }
}

fn group_row(
    ui: &mut egui::Ui,
    (group, enabled): (&GroupRow, bool),
    collapsed: &mut HashSet<GroupId>,
    taps: &mut Vec<Intent>,
) {
    let closed = collapsed.contains(&group.id);
    let (icon, icon_label) = if group.visible {
        (Icon::Visible, "Hide group")
    } else {
        (Icon::Hidden, "Show group")
    };
    let detail = if group.opacity < 1.0 {
        percent(group.opacity)
    } else {
        String::new()
    };
    let response = list_row(
        ui,
        &ListRow {
            key: egui::Id::new(("group row", group.id.get())),
            label: group.name.as_str(),
            detail: &detail,
            indent: 0.0,
            lead: Some(if closed {
                Icon::Collapsed
            } else {
                Icon::Expanded
            }),
            picture: None,
            selected: false,
            icon,
            icon_label,
            enabled,
            lifted: None,
        },
    );
    if response.icon.clicked() {
        taps.push(Intent::Layer(LayerIntent::SetGroupVisible {
            group: group.id,
            visible: !group.visible,
        }));
    } else if response.row.clicked() && !collapsed.remove(&group.id) {
        collapsed.insert(group.id);
    }
}

/// A layer's row; returns the row's response (its press can lift it).
fn layer_row(
    ui: &mut egui::Ui,
    (row, list): (&LayerRow, &LayerList),
    (enabled, lifted, picture): (bool, Option<f32>, Option<RowPicture>),
    taps: &mut Vec<Intent>,
) -> egui::Response {
    let mut detail = Vec::new();
    if row.alpha_locked {
        detail.push("Alpha Lock".to_owned());
    }
    if row.opacity < 1.0 {
        detail.push(percent(row.opacity));
    }
    detail.push(letter(row.blend).to_owned());
    let (icon, icon_label) = if row.visible {
        (Icon::Visible, "Hide layer")
    } else {
        (Icon::Hidden, "Show layer")
    };
    let response = list_row(
        ui,
        &ListRow {
            key: egui::Id::new(("layer row", row.id.get())),
            label: row.name.as_str(),
            detail: &detail.join(" · "),
            indent: indent_of(row),
            lead: row.clipped.then_some(Icon::Clipped),
            picture,
            selected: row.id == list.active && !list.on_mask,
            icon,
            icon_label,
            enabled,
            lifted,
        },
    );
    if response.icon.clicked() {
        taps.push(Intent::Layer(LayerIntent::SetVisible {
            layer: row.id,
            visible: !row.visible,
        }));
    } else if response.row.clicked() {
        taps.push(Intent::Layer(LayerIntent::Select(row.id)));
    }
    response.row
}
