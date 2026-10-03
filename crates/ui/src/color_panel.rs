//! The colour panel: the colour button at the end of the right island (a swatch of the current
//! colour, Procreate's place) opens it as a popover over the canvas: header "Colour", the colour
//! wheel (hue ring, saturation and value square), then the recent colours, newest first. The
//! colour changes while the hand moves on the wheel and is shown at once in the button; lifting
//! the hand makes it a recent colour. Tapping a recent colour paints with it. The panel stays
//! until a contact lands on the canvas outside it; a pen stroke there paints in the new colour.
//! Dragging the button onto the canvas instead drops the colour there (`color_drop.rs`).

use egui::Align;
use openpaint_app::{BrushProjection, Hsv, Intent, SessionProjection, ToolIntent};

use crate::components::{ScreenArea, color_wheel, color32, popover, swatch};
use crate::design::{COLOR_WHEEL, INK_SECONDARY, RECENT_SWATCH, SPACE, TOUCH_TARGET};

/// The panel's UI-local state.
#[derive(Debug, Default)]
pub(crate) struct ColorPanel {
    open: bool,
    /// Where the open panel lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
    /// Where the colour button lies, from the last pass.
    button: Option<ScreenArea>,
    /// The hue, saturation and value last set here: a grey or black has no hue of its own, so
    /// the wheel keeps the one the hand left while the colour is still the one it set.
    shown: Option<Hsv>,
}

impl ColorPanel {
    /// The colour button, in the right island. Returns whether it opened the panel.
    pub(crate) fn button(&mut self, ui: &mut egui::Ui, projection: &SessionProjection) -> bool {
        let Some(brush) = projection.brush() else {
            self.button = None;
            return false;
        };
        let response = swatch(ui, brush.color, "Colour", TOUCH_TARGET, self.open);
        self.button = Some(ScreenArea::from_points(
            response.rect,
            ui.ctx().pixels_per_point(),
        ));
        if response.clicked() {
            self.open = !self.open;
            return self.open;
        }
        false
    }

    /// Where the colour button is, which a colour drop starts from.
    pub(crate) const fn button_area(&self) -> Option<ScreenArea> {
        self.button
    }

    /// The panel when it is open, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        let brush = projection.brush().filter(|_| self.open);
        let Some(brush) = brush else {
            self.area = None;
            return;
        };
        let hsv = self
            .shown
            .filter(|shown| color32(shown.color()) == color32(brush.color))
            .unwrap_or_else(|| Hsv::of(brush.color));
        let mut set = None;
        let rect = popover(ctx, "colours", |ui| {
            ui.horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                ui.add_space(2.0 * SPACE);
                ui.heading("Colour");
            });
            ui.vertical_centered(|ui| {
                if let Some(change) = color_wheel(ui, hsv, COLOR_WHEEL) {
                    set = Some(change.hsv);
                    push(taps, change.hsv.color(), change.settled);
                }
            });
            ui.add_space(SPACE);
            recent(ui, brush, taps);
        });
        if let Some(hsv) = set {
            self.shown = Some(hsv);
        }
        self.area = Some(ScreenArea::from_points(rect, ctx.pixels_per_point()));
    }

    /// Where the open panel lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.area = None;
    }
}

/// The recent colours in one row, or a line saying where they come from.
fn recent(ui: &mut egui::Ui, brush: &BrushProjection, taps: &mut Vec<Intent>) {
    ui.horizontal(|ui| {
        ui.set_height(RECENT_SWATCH);
        if brush.recent.is_empty() {
            ui.add_space(2.0 * SPACE);
            ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                ui.colored_label(INK_SECONDARY, "Colours you choose appear here.");
            });
            return;
        }
        for color in brush.recent.iter() {
            let current = *color == brush.color;
            if swatch(ui, *color, "Recent colour", RECENT_SWATCH, current).clicked() {
                push(taps, *color, true);
            }
        }
    });
}

/// egui may run a pass twice; one change is sent once.
fn push(taps: &mut Vec<Intent>, color: openpaint_core::Color, settled: bool) {
    let intent = Intent::Tool(ToolIntent::Color { color, settled });
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}
