//! The side bar: a narrow column over the canvas at the left edge, centred on it (Procreate's
//! default place), with what the hand reaches for most while drawing: the brush size slider on
//! top, the eyedropper, the opacity slider, then undo and redo. Each slider shows its value
//! beside the bar while it is held; letting go keeps the value for the current tool's brush. A
//! contact starting on the bar never paints.

use openpaint_app::{
    BrushIntent, BrushProjection, EditorAction, HistoryIntent, Intent, SessionProjection,
    ToolIntent,
};

use crate::components::{
    ScreenArea, SideSlider, TOP_BAR_HEIGHT, icon_button, icon_toggle, side_panel, side_slider,
};
use crate::design::{Icon, SIDE_SLIDER, SPACE, TOUCH_TARGET};

/// The history buttons, top to bottom.
const HISTORY_BUTTONS: [(Icon, &str, EditorAction, HistoryIntent); 2] = [
    (Icon::Undo, "Undo", EditorAction::Undo, HistoryIntent::Undo),
    (Icon::Redo, "Redo", EditorAction::Redo, HistoryIntent::Redo),
];

#[derive(Debug, Default)]
pub(crate) struct SideBar {
    /// Where the bar lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
}

impl SideBar {
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        let length = slider_length(ctx.content_rect().height());
        // Two sliders, the gaps (one around the eyedropper, a double one after the sliders, one
        // between the history buttons), three buttons and the margins.
        let height = 2.0 * length + 5.0 * SPACE + 3.0 * TOUCH_TARGET + 2.0 * SPACE;
        let rect = side_panel(ctx, "side bar", height, |ui| {
            if let Some(brush) = projection.brush() {
                sliders(ui, brush, length, taps);
                ui.add_space(SPACE);
            }
            for (icon, label, action, intent) in HISTORY_BUTTONS {
                // Greyed only when there is no step: while drawing they look the same and a
                // tap is refused at the intent.
                let enabled = projection.availability(action).is_offered();
                if icon_button(ui, icon, label, enabled).clicked() {
                    taps.push(Intent::History(intent));
                }
            }
        });
        self.area = Some(ScreenArea::from_points(rect, ctx.pixels_per_point()));
    }

    /// Where the bar lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    pub(crate) fn hide(&mut self) {
        self.area = None;
    }
}

/// Each slider's length on a screen `height` points tall: `SIDE_SLIDER` where the bar fits
/// under the top bar with a space around it, shorter (never under two touch targets) where it
/// would not.
fn slider_length(height: f32) -> f32 {
    // Around the sliders: the space outside the bar, its margins, the gaps between its parts
    // and its three buttons.
    let around = 2.0 * SPACE + 2.0 * SPACE + 5.0 * SPACE + 3.0 * TOUCH_TARGET;
    ((height - TOP_BAR_HEIGHT - around) / 2.0).clamp(2.0 * TOUCH_TARGET, SIDE_SLIDER)
}

fn sliders(ui: &mut egui::Ui, brush: &BrushProjection, length: f32, taps: &mut Vec<Intent>) {
    let size = SideSlider {
        label: "Brush size",
        value: brush.size_position,
        shown: &pixels(brush.size),
        enabled: true,
        length,
    };
    if let Some(change) = side_slider(ui, &size) {
        push(
            taps,
            BrushIntent::Size {
                position: change.value,
                settled: change.settled,
            },
        );
    }
    // Between the sliders, as Procreate's: the eyedropper. On, the next contact on the canvas
    // picks a colour; tapped again, the pen draws again.
    let label = "Eyedropper";
    if icon_toggle(ui, Icon::Eyedropper, label, brush.picking).clicked() {
        let intent = Intent::Tool(ToolIntent::Eyedropper(!brush.picking));
        if taps.last() != Some(&intent) {
            taps.push(intent);
        }
    }
    let opacity = SideSlider {
        label: "Brush opacity",
        value: brush.opacity,
        shown: &format!("{} %", (brush.opacity * 100.0).round()),
        enabled: true,
        length,
    };
    if let Some(change) = side_slider(ui, &opacity) {
        push(
            taps,
            BrushIntent::Opacity {
                opacity: change.value,
                settled: change.settled,
            },
        );
    }
}

/// egui may run a pass twice; one value is sent once.
fn push(taps: &mut Vec<Intent>, intent: BrushIntent) {
    let intent = Intent::Brush(intent);
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}

/// A diameter as the bubble shows it: whole pixels, a tenth below ten.
fn pixels(size: f32) -> String {
    if size < 10.0 {
        format!("{size:.1} px")
    } else {
        format!("{size:.0} px")
    }
}
