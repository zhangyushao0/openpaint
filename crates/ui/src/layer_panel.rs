//! The layer panel: the Layers button in the right island opens it as a popover over
//! the canvas (Procreate's model). Header: "Layers", the new group button and the add button.
//! Rows (`layer_rows.rs`): the layers, top of the stack first, the one drawn on highlighted,
//! each with its blend letter and eye, a group's row above its layers. Footer
//! (`layer_options.rs`): the highlighted layer's options and its group's. Delete asks nothing:
//! undo brings the layer back, as it does every option. Any contact landing on the canvas
//! outside the panel closes it; a pen stroke there also draws.

use std::collections::HashSet;

use openpaint_app::{EditorAction, Intent, LayerIntent, SessionProjection};
use openpaint_core::GroupId;

use crate::components::{Opener, ScreenArea, icon_button, icon_toggle, popover};
use crate::design::{Icon, SPACE, TOUCH_TARGET};
use crate::layer_options::{footer, footer_rows};
use crate::layer_rows::rows;

/// Rows of touch targets the panel keeps free of layer rows besides its footer: the header and
/// the room above and below the panel.
const KEPT_ROWS: f32 = 3.0;

/// The panel's UI-local state.
#[derive(Debug, Default)]
pub(crate) struct LayerPanel {
    open: bool,
    /// Where the open panel lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
    /// Where the Layers button lies, in points: the panel comes out of it.
    button: Option<egui::Rect>,
    /// The closed groups: the panel's own state, no artwork change and not stored.
    collapsed: HashSet<GroupId>,
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
        let collapsed = &mut self.collapsed;
        let rect = popover(ctx, "layers", (opener, self.open), |ui| {
            header(ui, projection, taps);
            let grouped = list.active_row().is_some_and(|row| row.group.is_some());
            let kept = (KEPT_ROWS + footer_rows(grouped, list.on_mask)) * TOUCH_TARGET;
            let room = ui.ctx().content_rect().height() - kept;
            let enabled = projection
                .availability(EditorAction::EditLayer)
                .is_offered();
            rows(ui, (list, enabled), room, collapsed, taps);
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

fn header(ui: &mut egui::Ui, projection: &SessionProjection, taps: &mut Vec<Intent>) {
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.heading("Layers");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let enabled = projection.availability(EditorAction::AddLayer).is_offered();
            if icon_button(ui, Icon::Add, "Add layer", enabled).clicked() {
                taps.push(Intent::Layer(LayerIntent::Add));
            }
            if icon_button(ui, Icon::Group, "New group", enabled).clicked() {
                taps.push(Intent::Layer(LayerIntent::NewGroup));
            }
        });
    });
}
