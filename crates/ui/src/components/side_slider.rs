//! The side slider: a value from 0 at the bottom to 1 at the top, one touch target wide, for the
//! side bar. It says what it sets without words: a mark at each end (a small and a large dot
//! for size, a faint and a solid disc for opacity), and a label while the pen hovers over it.
//! The knob jumps to where the hand presses and follows it; lifting the hand settles the value.
//! While it is held, a bubble beside the side bar, level with the knob, shows the value and a
//! preview: the brush's circle at its real size on the canvas, or a swatch at that opacity.

use egui::{Color32, Order, Pos2, Rect, Sense, Stroke, TextStyle, Vec2, WidgetInfo};

use crate::components::SliderChange;
use crate::components::slider::paint_track;
use crate::design::{
    INK, INK_DISABLED, Icon, PANEL_RULE, SIDE_SLIDER, SPACE, TOUCH_TARGET, paint_icon, panel_frame,
};

/// Radius of the knob, in points.
const KNOB: f32 = 1.25 * SPACE;
/// Side of an end mark, in points.
const END: f32 = 2.5 * SPACE;
/// Side of the opacity preview's swatch, in points.
const SWATCH: f32 = 6.0 * SPACE;

/// What the bubble previews while the slider is held.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Preview {
    /// The brush's circle: its diameter on screen, in points, and its colour.
    Size { diameter: f32, color: Color32 },
    /// The brush's colour at an opacity from 0 to 1.
    Opacity { color: Color32, opacity: f32 },
}

/// What the side slider shows.
pub struct SideSlider<'a> {
    /// Names it for accessibility and the pen's hover label.
    pub label: &'a str,
    /// In `0..=1`.
    pub value: f32,
    /// The value as the bubble shows it while the slider is held, such as "24 px".
    pub shown: &'a str,
    /// The marks at the bottom (low) and top (high) ends.
    pub ends: [Icon; 2],
    /// What the bubble previews for the value now.
    pub preview: Preview,
    pub enabled: bool,
    /// The track's length in points, at most `SIDE_SLIDER`.
    pub length: f32,
}

/// The slider's height in points for a track of `length`: the track and its two end marks.
#[must_use]
pub fn side_slider_height(length: f32) -> f32 {
    length.min(SIDE_SLIDER) + 2.0 * END
}

/// Shows the slider; returns the value the hand set this frame, if it set one.
pub fn side_slider(ui: &mut egui::Ui, slider: &SideSlider<'_>) -> Option<SliderChange> {
    let sense = if slider.enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let size = Vec2::new(TOUCH_TARGET, side_slider_height(slider.length));
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response
        .widget_info(|| WidgetInfo::slider(slider.enabled, f64::from(slider.value), slider.label));
    let track = rect.shrink2(Vec2::new(0.0, END + KNOB));
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
        paint(ui, rect, (track, knob), slider);
    }
    if held {
        bubble(ui, knob, slider);
    } else {
        response.on_hover_text(slider.label);
    }
    change
}

fn paint(ui: &egui::Ui, rect: Rect, (track, knob): (Rect, Pos2), slider: &SideSlider<'_>) {
    let painter = ui.painter();
    let ink = if slider.enabled { INK } else { INK_DISABLED };
    let [low, high] = slider.ends;
    let end = |y: f32| Rect::from_center_size(Pos2::new(rect.center().x, y), Vec2::splat(END));
    paint_icon(painter, high, end(rect.top() + END / 2.0), ink);
    paint_icon(painter, low, end(rect.bottom() - END / 2.0), ink);
    let x = track.center().x;
    let (top, bottom) = (Pos2::new(x, track.top()), Pos2::new(x, track.bottom()));
    paint_track(painter, [bottom, top], knob, slider.enabled);
}

/// The value and its preview in a small panel to the right of the side bar, level with `knob`,
/// above the canvas.
fn bubble(ui: &egui::Ui, knob: Pos2, slider: &SideSlider<'_>) {
    let painter = ui
        .ctx()
        .layer_painter(egui::LayerId::new(Order::Tooltip, ui.id().with("bubble")));
    let font = TextStyle::Body.resolve(ui.style());
    let galley = painter.layout_no_wrap(slider.shown.to_owned(), font, INK);
    let picture = match slider.preview {
        Preview::Size { diameter, .. } => diameter.max(2.0),
        Preview::Opacity { .. } => SWATCH,
    };
    let width = galley.size().x.max(picture) + 4.0 * SPACE;
    let height = galley.size().y + picture + 3.0 * SPACE;
    // Past the side bar's frame: half the slider, the frame's margin and a space.
    let left = knob.x + TOUCH_TARGET / 2.0 + 3.0 * SPACE;
    let rect = Rect::from_min_size(
        Pos2::new(left, knob.y - height / 2.0),
        Vec2::new(width, height),
    );
    painter.add(panel_frame(egui::Margin::ZERO).paint(rect));
    let text = Pos2::new(rect.center().x - galley.size().x / 2.0, rect.top() + SPACE);
    let centre = Pos2::new(rect.center().x, rect.bottom() - 1.5 * SPACE - picture / 2.0);
    painter.galley(text, galley, INK);
    match slider.preview {
        Preview::Size { diameter, color } => {
            painter.circle(
                centre,
                diameter.max(2.0) / 2.0,
                color,
                Stroke::new(1.0, PANEL_RULE),
            );
        }
        Preview::Opacity { color, opacity } => {
            let swatch = Rect::from_center_size(centre, Vec2::splat(SWATCH));
            painter.rect(
                swatch,
                SPACE / 2.0,
                Color32::WHITE,
                Stroke::new(1.0, PANEL_RULE),
                egui::StrokeKind::Inside,
            );
            painter.circle_filled(
                centre,
                SWATCH / 2.0 - SPACE / 2.0,
                color.gamma_multiply(opacity),
            );
        }
    }
}
