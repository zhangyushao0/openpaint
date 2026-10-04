//! The layer panel's rows, top of the stack first: a group's row (its chevron, name, opacity
//! and eye) above its layers, which sit one step to the right and hide while the group is
//! closed; a Clipping Mask's row a further step right with the arrow down to its base. Each
//! layer row ends with its blend mode's letter (Procreate's "N") before its eye. A layer's
//! mask is a row under it, one step further right; the one drawn on (paint or mask) is outlined.

use std::collections::HashSet;

use openpaint_app::{GroupRow, Intent, LayerIntent, LayerList, LayerRow, MaskRow};
use openpaint_core::{BlendMode, GroupId};

use crate::components::{ListRow, list_row};
use crate::design::{Icon, SPACE, TOUCH_TARGET};

/// How far a group's layers, and a Clipping Mask within its chain, sit right of the row above.
const INDENT: f32 = 3.0 * SPACE;
/// How much of the first row below the list's end shows: enough to cut through its name.
const CUT_ROW: f32 = 0.65;

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

/// What the panel shows of the layers: `room` points of rows at most (more scroll), the groups
/// in `collapsed` closed. Taps go to `taps`; a group row's tap opens or closes it here.
pub(crate) fn rows(
    ui: &mut egui::Ui,
    (list, enabled): (&LayerList, bool),
    room: f32,
    collapsed: &mut HashSet<GroupId>,
    taps: &mut Vec<Intent>,
) {
    let whole = (room / TOUCH_TARGET - CUT_ROW).floor().max(1.0);
    egui::ScrollArea::vertical()
        .max_height((whole + CUT_ROW) * TOUCH_TARGET)
        .show(ui, |ui| {
            let mut above = None;
            for row in list.rows.iter().rev() {
                if row.group != above
                    && let Some(group) = row.group.and_then(|id| list.group(id))
                {
                    group_row(ui, (group, enabled), collapsed, taps);
                }
                above = row.group;
                if row.group.is_some_and(|id| collapsed.contains(&id)) {
                    continue;
                }
                layer_row(ui, (row, list), enabled, taps);
                if let Some(mask) = row.mask {
                    mask_row(ui, (row, mask, list), enabled, taps);
                }
            }
        });
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
    enabled: bool,
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
            selected: row.id == list.active && list.on_mask,
            icon,
            icon_label,
            enabled,
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
            selected: false,
            icon,
            icon_label,
            enabled,
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

fn layer_row(
    ui: &mut egui::Ui,
    (row, list): (&LayerRow, &LayerList),
    enabled: bool,
    taps: &mut Vec<Intent>,
) {
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
            selected: row.id == list.active && !list.on_mask,
            icon,
            icon_label,
            enabled,
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
}
