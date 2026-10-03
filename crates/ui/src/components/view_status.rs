//! The view status: one quiet line, "250 % · 30°", centred near the top of the canvas while the
//! fingers move the view and for a moment after, then fading away (restraint, D32). The zoom
//! is the actual size: 100 % shows one canvas pixel per screen pixel. It takes no contacts.

use egui::{Align2, Color32, Frame, Margin, Order, Vec2};

use crate::components::TOP_BAR_HEIGHT;
use crate::design::{CHROME, CORNER, ON_CHROME, SPACE};

/// The line for a view of `scale` screen pixels per canvas pixel turned by `rotation` radians.
#[must_use]
pub fn view_text(scale: f32, rotation: f32) -> String {
    let percent = (scale * 100.0).round();
    let degrees = rotation.to_degrees().round();
    // -0 reads as 0; a half turn reads as 180, not -180.
    let degrees = if degrees == 0.0 || degrees == -180.0 {
        degrees.abs()
    } else {
        degrees
    };
    format!("{percent} % · {degrees}°")
}

/// Shows `text` at `strength` (1 whole, 0 gone) under the top bar's middle.
pub fn view_status(ctx: &egui::Context, text: &str, strength: f32) {
    let fade = |color: Color32| color.gamma_multiply(strength.clamp(0.0, 1.0));
    egui::Area::new(egui::Id::new("view status"))
        .order(Order::Foreground)
        .interactable(false)
        .fade_in(false)
        .anchor(
            Align2::CENTER_TOP,
            Vec2::new(0.0, TOP_BAR_HEIGHT + 2.0 * SPACE),
        )
        .show(ctx, |ui| {
            Frame::new()
                .fill(fade(CHROME))
                .corner_radius(CORNER)
                .inner_margin(Margin::symmetric((2.0 * SPACE) as i8, SPACE as i8))
                .show(ui, |ui| {
                    let line = egui::RichText::new(text).color(fade(ON_CHROME));
                    ui.add(egui::Label::new(line).extend());
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_reads_the_actual_size_and_the_turn_in_whole_numbers() {
        assert_eq!(view_text(0.92, 0.0), "92 % · 0°");
        assert_eq!(view_text(2.5, -0.000_1), "250 % · 0°");
        assert_eq!(view_text(1.0, std::f32::consts::FRAC_PI_6), "100 % · 30°");
        assert_eq!(view_text(1.0, -std::f32::consts::PI), "100 % · 180°");
    }
}
