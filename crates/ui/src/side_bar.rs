//! The side bar: a narrow island over the canvas at the left edge, centred on it (Procreate's
//! default place), with what the hand reaches for most while drawing: the brush size slider on
//! top, the eyedropper, the opacity slider, then undo and redo. Each slider says what it sets by
//! the marks at its ends (a small and a large dot; a faint and a solid disc) and, under the
//! hovering pen, its name; while it is held, a bubble beside the bar shows the value with the
//! brush's circle at its real size on the canvas or a swatch at that opacity. Letting go keeps
//! the value for the current tool's brush. A contact starting on the bar never paints.

use openpaint_app::{
    BrushIntent, BrushProjection, EditorAction, HistoryIntent, Intent, SessionProjection,
    ToolIntent,
};

use crate::components::{
    Preview, ScreenArea, SideSlider, UNDER_ISLANDS, color32, icon_button, icon_toggle, side_panel,
    side_slider, side_slider_height,
};
use crate::design::{Icon, SIDE_SLIDER, SPACE, TOUCH_TARGET};

/// The history buttons, top to bottom.
const HISTORY_BUTTONS: [(Icon, &str, EditorAction, HistoryIntent); 2] = [
    (Icon::Undo, "Undo", EditorAction::Undo, HistoryIntent::Undo),
    (Icon::Redo, "Redo", EditorAction::Redo, HistoryIntent::Redo),
];
/// Between the side bar's parts: one around the eyedropper, a double one after the sliders, one
/// between the history buttons, in points.
const GAPS: f32 = 5.0 * SPACE;

#[derive(Debug, Default)]
pub(crate) struct SideBar {
    /// Where the bar lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
}

impl SideBar {
    /// The bar for `projection`; `scale` is screen pixels per canvas pixel now, for the size
    /// preview.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        scale: f32,
        taps: &mut Vec<Intent>,
    ) {
        let length = slider_length(ctx.content_rect().height());
        // Two sliders, the gaps, three buttons and the margins.
        let height = 2.0 * side_slider_height(length) + GAPS + 3.0 * TOUCH_TARGET + 2.0 * SPACE;
        let rect = side_panel(ctx, "side bar", height, |ui| {
            if let Some(brush) = projection.brush() {
                let screen = scale / ctx.pixels_per_point();
                sliders(ui, brush, (length, screen), taps);
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

/// Each slider's track length on a screen `height` points tall: `SIDE_SLIDER` where the bar fits
/// between the top islands and a space above the bottom edge, shorter (never under one touch
/// target) where it would not.
fn slider_length(height: f32) -> f32 {
    // Around the tracks: the space under the bar, its margins, the gaps, the end marks and the
    // three buttons.
    let around = 3.0 * SPACE + GAPS + 2.0 * side_slider_height(0.0) + 3.0 * TOUCH_TARGET;
    ((height - UNDER_ISLANDS - around) / 2.0).clamp(TOUCH_TARGET, SIDE_SLIDER)
}

/// The size and opacity sliders with the eyedropper between them; `points` is screen points
/// per canvas pixel now.
fn sliders(
    ui: &mut egui::Ui,
    brush: &BrushProjection,
    (length, points): (f32, f32),
    taps: &mut Vec<Intent>,
) {
    let color = color32(brush.color);
    let size = SideSlider {
        label: "Brush size",
        value: brush.size_position,
        shown: &pixels(brush.size),
        ends: [Icon::SizeSmall, Icon::SizeLarge],
        preview: Preview::Size {
            diameter: brush.size * points,
            color,
        },
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
        ends: [Icon::OpacityLight, Icon::OpacityDark],
        preview: Preview::Opacity {
            color,
            opacity: brush.opacity,
        },
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
