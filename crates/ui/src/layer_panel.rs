//! The layer panel: the Layers button in the right island opens it as a popover over
//! the canvas (Procreate's model). Header: "Layers" and the add button. Rows: the layers, top of
//! the stack first, the one drawn on highlighted, each with its eye ("Alpha Lock" beside the
//! name of a layer whose alpha is locked). Footer, for the highlighted layer: its opacity
//! slider, the Alpha Lock switch (Procreate keeps it in the layer's options), move up, move
//! down and delete. Delete asks nothing: undo brings the layer back, as it does Alpha Lock. Any contact landing on the canvas outside the panel closes it; a pen stroke
//! there also draws.

use openpaint_app::{EditorAction, Intent, LayerIntent, LayerList, SessionProjection};

use crate::components::{
    ListRow, Opener, ScreenArea, icon_button, icon_toggle, list_row, popover, slider, switch_row,
};
use crate::design::{Icon, SPACE, TOUCH_TARGET};

/// How much of the first row below the list's end shows: enough to cut through its name.
const CUT_ROW: f32 = 0.65;

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
    /// Where the Layers button lies, in points: the panel comes out of it.
    button: Option<egui::Rect>,
}

impl LayerPanel {
    /// The Layers button, in the right island. Returns whether it opened the panel.
    pub(crate) fn button(&mut self, ui: &mut egui::Ui) -> bool {
        let response = icon_toggle(ui, Icon::Layers, "Layers", self.open);
        self.button = Some(response.rect);
        if response.clicked() {
            self.open = !self.open;
            return self.open;
        }
        false
    }

    /// The panel while it is open or closing, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.area = None;
        let Some(list) = projection.layers() else {
            return;
        };
        let opener = Opener {
            island: "right island",
            button: self.button.unwrap_or(egui::Rect::ZERO),
        };
        let rect = popover(ctx, "layers", (opener, self.open), |ui| {
            header(ui, projection, taps);
            rows(ui, list, projection, taps);
            footer(ui, list, projection, taps);
        });
        self.area = rect.map(|rect| ScreenArea::from_points(rect, ctx.pixels_per_point()));
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
    projection.availability(action).is_offered()
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

/// Top of the stack first; scrolls when the layers outgrow the screen. No scroll bar at rest:
/// the list stops partway through a row, its name cut, and fades there, so more rows are
/// plainly below.
fn rows(
    ui: &mut egui::Ui,
    list: &LayerList,
    projection: &SessionProjection,
    taps: &mut Vec<Intent>,
) {
    let enabled = offered(projection, EditorAction::EditLayer);
    let room = ui.ctx().content_rect().height() - 7.0 * TOUCH_TARGET;
    let whole = (room / TOUCH_TARGET - CUT_ROW).floor().max(1.0);
    egui::ScrollArea::vertical()
        .max_height((whole + CUT_ROW) * TOUCH_TARGET)
        .show(ui, |ui| {
            for row in list.rows.iter().rev() {
                let detail = match (row.alpha_locked, row.opacity < 1.0) {
                    (true, true) => format!("Alpha Lock · {}", percent(row.opacity)),
                    (true, false) => "Alpha Lock".to_owned(),
                    (false, true) => percent(row.opacity),
                    (false, false) => String::new(),
                };
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
    if switch_row(ui, "Alpha Lock", active.alpha_locked, enabled).clicked() {
        taps.push(Intent::Layer(LayerIntent::SetAlphaLock {
            layer: active.id,
            locked: !active.alpha_locked,
        }));
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
