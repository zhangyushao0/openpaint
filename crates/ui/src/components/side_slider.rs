//! The side slider: a value from 0 at the bottom to 1 at the top, one touch target wide and at
//! most `SIDE_SLIDER` long, for the side bar. The knob jumps to where the hand presses and follows
//! it; lifting the hand settles the value. While it is held, its value shows in a bubble beside
//! the side bar, level with the knob.

use egui::{Order, Pos2, Rect, Sense, Stroke, TextStyle, Vec2, WidgetInfo};

use crate::components::SliderChange;
use crate::design::{
    CHROME, CORNER, ON_CHROME, ON_CHROME_DISABLED, OUTLINE, SIDE_SLIDER, SPACE, TOUCH_TARGET,
};

/// Radius of the knob, in points.
const KNOB: f32 = 1.5 * SPACE;
/// Thickness of the track, in points.
const TRACK: f32 = SPACE / 2.0;

/// What the side slider shows.
pub struct SideSlider<'a> {
    /// Names it for accessibility.
    pub label: &'a str,
    /// In `0..=1`.
    pub value: f32,
    /// The value as the bubble shows it while the slider is held, such as "24 px".
    pub shown: &'a str,
    pub enabled: bool,
    /// In points, at most `SIDE_SLIDER`.
    pub length: f32,
}

/// Shows the slider; returns the value the hand set this frame, if it set one.
pub fn side_slider(ui: &mut egui::Ui, slider: &SideSlider<'_>) -> Option<SliderChange> {
    let sense = if slider.enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let length = slider.length.min(SIDE_SLIDER);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(TOUCH_TARGET, length), sense);
    response
        .widget_info(|| WidgetInfo::slider(slider.enabled, f64::from(slider.value), slider.label));
    let track = rect.shrink2(Vec2::new(0.0, KNOB));
    let settled = response.drag_stopped() || response.clicked();
    let held = slider.enabled && response.is_pointer_button_down_on();
    let change = response
        .interact_pointer_pos()
        .filter(|_| held || (slider.enabled && settled))
        .map(|pos| SliderChange {
            value: ((track.bottom() - pos.y) / track.height()).clamp(0.0, 1.0),
            settled,
        })
        .filter(|change| change.settled || change.value != slider.value);
    let value = change.map_or(slider.value, |change| change.value);
    let knob = Pos2::new(track.center().x, track.bottom() - value * track.height());
    if ui.is_rect_visible(rect) {
        paint(ui, track, knob, slider.enabled);
    }
    if held {
        bubble(ui, knob, slider.shown);
    }
    change
}

fn paint(ui: &egui::Ui, track: Rect, knob: Pos2, enabled: bool) {
    let painter = ui.painter();
    let color = if enabled {
        ON_CHROME
    } else {
        ON_CHROME_DISABLED
    };
    let x = track.center().x;
    let (top, bottom) = (Pos2::new(x, track.top()), Pos2::new(x, track.bottom()));
    painter.line_segment([top, bottom], Stroke::new(TRACK, ON_CHROME_DISABLED));
    painter.line_segment([bottom, knob], Stroke::new(TRACK, color));
    painter.circle_filled(knob, KNOB, color);
}

/// `text` in a small panel to the right of the side bar, level with `knob`, above the canvas.
fn bubble(ui: &egui::Ui, knob: Pos2, text: &str) {
    let painter = ui
        .ctx()
        .layer_painter(egui::LayerId::new(Order::Tooltip, ui.id().with("bubble")));
    let font = TextStyle::Body.resolve(ui.style());
    let galley = painter.layout_no_wrap(text.to_owned(), font, ON_CHROME);
    let size = galley.size() + Vec2::new(4.0 * SPACE, 2.0 * SPACE);
    // Past the side bar's frame: half the slider, the frame's margin and a space.
    let left = knob.x + TOUCH_TARGET / 2.0 + 3.0 * SPACE;
    let rect = Rect::from_min_size(Pos2::new(left, knob.y - size.y / 2.0), size);
    painter.rect(
        rect,
        CORNER,
        CHROME,
        Stroke::new(1.0, OUTLINE),
        egui::StrokeKind::Inside,
    );
    painter.galley(rect.center() - galley.size() / 2.0, galley, ON_CHROME);
}
