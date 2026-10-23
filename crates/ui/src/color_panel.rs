//! The colour panel: the colour button at the end of the right island (a swatch of the current
//! colour, Procreate's place) opens it as a popover over the canvas: header "Colour" with the
//! hex field (`color_palette.rs`), the colour wheel (hue ring, saturation and value square),
//! the recent colours, newest first, then the palette. The colour changes while the hand
//! moves on the wheel and is shown at once in the button; lifting the hand makes it a recent
//! colour. Tapping a recent colour or a swatch paints with it. The panel stays until a contact
//! lands on the canvas outside it; a pen stroke there paints in the new colour. Dragging the
//! button onto the canvas instead drops the colour there (`color_drop.rs`).

use egui::Align;
use openpaint_app::{BrushProjection, Hsv, Intent, SessionProjection, ToolIntent};

use crate::color_palette::PaletteState;
use crate::components::{
    Opener, ScreenArea, color_wheel, color32, panel_heading_with, popover, swatch,
};
use crate::design::{COLOR_WHEEL, INK_SECONDARY, RECENT_SWATCH, SPACE, TOUCH_TARGET};
use crate::taps::push_once;

/// The panel's UI-local state.
#[derive(Debug, Default)]
pub(crate) struct ColorPanel {
    open: bool,
    /// Where the open panel lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
    /// Where the colour button lies, from the last pass, in pixels and in points.
    button: Option<ScreenArea>,
    opener: Option<egui::Rect>,
    /// The hue, saturation and value last set here: a grey or black has no hue of its own, so
    /// the wheel keeps the one the hand left while the colour is still the one it set.
    shown: Option<Hsv>,
    /// The palette and the hex field.
    palette: PaletteState,
}

impl ColorPanel {
    /// The colour button, in the right island. Returns whether it opened the panel.
    pub(crate) fn button(&mut self, ui: &mut egui::Ui, projection: &SessionProjection) -> bool {
        let Some(brush) = projection.brush() else {
            self.button = None;
            return false;
        };
        let response = swatch(ui, brush.color, "Colour", TOUCH_TARGET, self.open);
        self.opener = Some(response.rect);
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

    /// The panel while it is open or closing, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.area = None;
        let (Some(brush), Some(button)) = (projection.brush(), self.opener) else {
            return;
        };
        let opener = Opener {
            island: "right island",
            button,
        };
        let (palette, shown) = (&mut self.palette, &mut self.shown);
        let rect = popover(ctx, "colours", (opener, self.open), |ui| {
            panel_heading_with(ui, "Colour", |ui| {
                ui.add_space(SPACE);
                palette.hex_field(ui, brush.color, taps);
            });
            palette.hex_hint(ui);
            ui.vertical_centered(|ui| wheel(ui, shown, brush, taps));
            ui.add_space(SPACE);
            recent(ui, brush, taps);
            palette.palette(ui, brush, taps);
            ui.add_space(SPACE);
        });
        self.area = rect.map(|rect| ScreenArea::from_points(rect, ctx.pixels_per_point()));
    }

    /// The wheel outside the panel, at the current colour of `brush`: a practice colour's, beside
    /// its frame. The same wheel as the panel's, keeping the hue the hand left on a grey.
    pub(crate) fn inline_wheel(
        &mut self,
        ui: &mut egui::Ui,
        brush: &BrushProjection,
        taps: &mut Vec<Intent>,
    ) {
        wheel(ui, &mut self.shown, brush, taps);
    }

    /// Where the open panel lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.area = None;
        self.palette.close();
    }
}

/// The colour wheel at `brush`'s colour (with the hue `shown` last kept while the colour is the
/// one it set), its changes pushed to `taps` and kept in `shown`.
fn wheel(
    ui: &mut egui::Ui,
    shown: &mut Option<Hsv>,
    brush: &BrushProjection,
    taps: &mut Vec<Intent>,
) {
    let hsv = shown
        .filter(|shown| color32(shown.color()) == color32(brush.color))
        .unwrap_or_else(|| Hsv::of(brush.color));
    if let Some(change) = color_wheel(ui, hsv, COLOR_WHEEL) {
        *shown = Some(change.hsv);
        push(taps, change.hsv.color(), change.settled);
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

fn push(taps: &mut Vec<Intent>, color: openpaint_core::Color, settled: bool) {
    push_once(taps, Intent::Tool(ToolIntent::Color { color, settled }));
}
