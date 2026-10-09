//! The setting slider (S46): one row across a panel for one of many numbers, as the brush
//! editor lists them. At the left what it sets, in ink, with its value under it in the quieter
//! ink; at the right the track, one touch target high, which alone takes the hand: the knob jumps
//! to where it presses and follows it, lifting settles the value. While held, a bubble over the
//! knob shows the value, as the side sliders' do, where the hand does not cover it. Unavailable,
//! the row fades to pencil grey and takes nothing.

use egui::{Align2, Order, Pos2, Rect, Sense, TextStyle, Vec2, WidgetInfo};

use crate::components::slider::paint_track;
use crate::components::{SliderChange, controls};
use crate::design::{INK, INK_DISABLED, INK_SECONDARY, SPACE, TOUCH_TARGET, panel_frame};

/// The width of the words left of the track, in points.
const WORDS: f32 = 16.0 * SPACE;
/// Radius of the knob, in points.
const KNOB: f32 = 1.25 * SPACE;

/// What the setting slider shows.
pub struct SettingSlider<'a> {
    /// What it sets: its words and its name.
    pub label: &'a str,
    /// The value as it reads, such as "Continuous" or "2.0 diameters".
    pub shown: &'a str,
    /// Where the value sits along the track, `0..=1`.
    pub position: f32,
    pub enabled: bool,
}

/// Shows the row; returns the position the hand set this frame, if it set one.
pub fn setting_slider(ui: &mut egui::Ui, slider: &SettingSlider<'_>) -> Option<SliderChange> {
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (row, _) = ui.allocate_exact_size(size, Sense::hover());
    let track_rect = Rect::from_min_max(
        Pos2::new(row.left() + 2.0 * SPACE + WORDS, row.top()),
        Pos2::new(row.right() - SPACE, row.bottom()),
    );
    let sense = if slider.enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let id = ui.id().with(slider.label);
    let response = ui.interact(track_rect, id, sense);
    let enabled = slider.enabled;
    response.widget_info(|| WidgetInfo::slider(enabled, f64::from(slider.position), slider.label));
    controls::register(ui, slider.label, track_rect);
    let track = track_rect.shrink2(Vec2::new(KNOB, 0.0));
    let settled = response.drag_stopped() || response.clicked();
    let held = enabled && response.is_pointer_button_down_on();
    let change = response
        .interact_pointer_pos()
        .filter(|_| held || (enabled && settled))
        .map(|pos| SliderChange {
            value: ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0),
            settled,
        })
        .filter(|change| change.settled || change.value != slider.position);
    let position = change.map_or(slider.position, |change| change.value);
    let knob = Pos2::new(track.left() + position * track.width(), track.center().y);
    if ui.is_rect_visible(row) {
        words(ui, row, slider);
        let ends = [track.left_center(), track.right_center()];
        paint_track(ui.painter(), ends, knob, enabled);
    }
    if held {
        bubble(ui, knob, slider.shown);
    }
    change
}

/// The name, and the value under it.
fn words(ui: &egui::Ui, row: Rect, slider: &SettingSlider<'_>) {
    let painter = ui.painter();
    let (ink, quiet) = if slider.enabled {
        (INK, INK_SECONDARY)
    } else {
        (INK_DISABLED, INK_DISABLED)
    };
    let left = row.left() + 2.0 * SPACE;
    let body = TextStyle::Body.resolve(ui.style());
    let small = TextStyle::Small.resolve(ui.style());
    let name = Pos2::new(left, row.center().y + 1.0);
    painter.text(name, Align2::LEFT_BOTTOM, slider.label, body, ink);
    let value = Pos2::new(left, row.center().y + 2.0);
    painter.text(value, Align2::LEFT_TOP, slider.shown, small, quiet);
}

/// The value in a small panel over the knob, above the hand, while the slider is held.
fn bubble(ui: &egui::Ui, knob: Pos2, shown: &str) {
    let painter = ui
        .ctx()
        .layer_painter(egui::LayerId::new(Order::Tooltip, ui.id().with("bubble")));
    let font = TextStyle::Body.resolve(ui.style());
    let galley = painter.layout_no_wrap(shown.to_owned(), font, INK);
    let size = galley.size() + Vec2::new(4.0 * SPACE, 2.0 * SPACE);
    let bottom = knob.y - TOUCH_TARGET / 2.0 - SPACE;
    // Over the knob, but whole on the screen.
    let right = ui.ctx().content_rect().right() - SPACE;
    let left = (knob.x - size.x / 2.0).min(right - size.x);
    let rect = Rect::from_min_size(Pos2::new(left, bottom - size.y), size);
    painter.add(panel_frame(egui::Margin::ZERO).paint(rect));
    let text = rect.center() - galley.size() / 2.0;
    painter.galley(text, galley, INK);
}
