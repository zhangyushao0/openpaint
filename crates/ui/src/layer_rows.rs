//! The layer panel's rows, top of the stack first: a group's row (its chevron, name, opacity
//! and eye) above its layers, which sit one step to the right and hide while the group is
//! closed; a Clipping Mask's row a further step right with the arrow down to its base. Each
//! layer row ends with its blend mode's letter (Procreate's "N") before its eye.

use std::collections::HashSet;

use openpaint_app::{GroupRow, Intent, LayerIntent, LayerList, LayerRow};
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
                layer_row(ui, (row, list.active), enabled, taps);
            }
        });
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
    (row, active): (&LayerRow, openpaint_core::LayerId),
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
    let indent =
        INDENT * (f32::from(u8::from(row.group.is_some())) + f32::from(u8::from(row.clipped)));
    let response = list_row(
        ui,
        &ListRow {
            key: egui::Id::new(("layer row", row.id.get())),
            label: row.name.as_str(),
            detail: &detail.join(" · "),
            indent,
            lead: row.clipped.then_some(Icon::Clipped),
            selected: row.id == active,
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
