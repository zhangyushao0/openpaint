//! The layer panel: the Layers button at the right end of the top bar opens it as a popover over
//! the canvas (Procreate's model). Header: "Layers" and the add button. Rows: the layers, top of
//! the stack first, the one drawn on highlighted, each with its eye. Footer, for the highlighted
//! layer: its opacity slider, move up, move down and delete. Delete asks nothing: undo brings
//! the layer back. Any contact landing on the canvas outside the panel closes it; a pen stroke
//! there also draws.

use openpaint_app::{EditorAction, Intent, LayerIntent, LayerList, SessionProjection};

use crate::components::{ListRow, ScreenArea, icon_button, icon_toggle, list_row, popover, slider};
use crate::design::{Icon, SPACE, TOUCH_TARGET};

/// Opacity shown as a whole percentage.
fn percent(opacity: f32) -> String {
    format!("{} %", (opacity * 100.0).round())
}

/// The panel's UI-local state.
#[derive(Debug, Default)]
pub(crate) struct LayerPanel {
    open: bool,
    /// Where the open panel lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
}

impl LayerPanel {
    /// The Layers button, inside the top bar. Returns whether it opened the panel.
    pub(crate) fn button(&mut self, ui: &mut egui::Ui) -> bool {
        if icon_toggle(ui, Icon::Layers, "Layers", self.open).clicked() {
            self.open = !self.open;
            return self.open;
        }
        false
    }

    /// The panel when it is open, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        let list = projection.layers().filter(|_| self.open);
        let Some(list) = list else {
            self.area = None;
            return;
        };
        let rect = popover(ctx, "layers", |ui| {
            header(ui, projection, taps);
            rows(ui, list, projection, taps);
            footer(ui, list, projection, taps);
        });
        self.area = Some(ScreenArea::from_points(rect, ctx.pixels_per_point()));
    }

    /// Where the open panel lies, which the canvas must not take contacts from.
    pub(crate) fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    /// One tap outside closes the panel.
    pub(crate) fn close(&mut self) {
        self.open = false;
        self.area = None;
    }
}

fn offered(projection: &SessionProjection, action: EditorAction) -> bool {
    projection.availability(action).is_available()
}

fn header(ui: &mut egui::Ui, projection: &SessionProjection, taps: &mut Vec<Intent>) {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.heading("Layers");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let enabled = offered(projection, EditorAction::AddLayer);
            if icon_button(ui, Icon::Add, "Add layer", enabled).clicked() {
                taps.push(Intent::Layer(LayerIntent::Add));
            }
        });
    });
}

/// Top of the stack first; scrolls when the layers outgrow the screen.
fn rows(
    ui: &mut egui::Ui,
    list: &LayerList,
    projection: &SessionProjection,
    taps: &mut Vec<Intent>,
) {
    let enabled = offered(projection, EditorAction::EditLayer);
    let room = ui.ctx().content_rect().height() - 6.0 * TOUCH_TARGET;
    egui::ScrollArea::vertical()
        .max_height(room.max(TOUCH_TARGET))
        .show(ui, |ui| {
            for row in list.rows.iter().rev() {
                let detail = if row.opacity < 1.0 {
                    percent(row.opacity)
                } else {
                    String::new()
                };
                let (icon, icon_label) = if row.visible {
                    (Icon::Visible, "Hide layer")
                } else {
                    (Icon::Hidden, "Show layer")
                };
                let response = list_row(
                    ui,
                    &ListRow {
                        label: row.name.as_str(),
                        detail: &detail,
                        selected: row.id == list.active,
                        icon,
                        icon_label,
                        enabled,
                    },
                );
                if response.icon.clicked() {
                    let visible = !row.visible;
                    taps.push(Intent::Layer(LayerIntent::SetVisible {
                        layer: row.id,
                        visible,
                    }));
                } else if response.row.clicked() {
                    taps.push(Intent::Layer(LayerIntent::Select(row.id)));
                }
            }
        });
}

/// The active layer's opacity and its moves.
fn footer(
    ui: &mut egui::Ui,
    list: &LayerList,
    projection: &SessionProjection,
    taps: &mut Vec<Intent>,
) {
    let Some(active) = list.active_row() else {
        return;
    };
    ui.separator();
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.label("Opacity");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(2.0 * SPACE);
            ui.label(percent(active.opacity));
        });
    });
    let enabled = offered(projection, EditorAction::EditLayer);
    if let Some(change) = slider(ui, "Opacity", active.opacity, enabled) {
        let intent = Intent::Layer(LayerIntent::SetOpacity {
            layer: active.id,
            opacity: change.value,
            settled: change.settled,
        });
        // egui may run a pass twice; one value is sent once.
        if taps.last() != Some(&intent) {
            taps.push(intent);
        }
    }
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
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let enabled = offered(projection, EditorAction::DeleteLayer);
            if icon_button(ui, Icon::Delete, "Delete layer", enabled).clicked() {
                taps.push(Intent::Layer(LayerIntent::Delete(active.id)));
            }
        });
    });
}
