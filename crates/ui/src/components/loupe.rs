//! The eyedropper's loupe (draweave's): while a contact picks a colour, a 76-point disc floats
//! up and to the right of it, so neither the finger nor the pen hides it (below the contact
//! near the screen's top). Its top half is the colour under the contact now, its bottom half the
//! current colour, which stays until the contact lifts. Paper line art: a paper ring in an ink
//! outline around the two colours, an ink hairline round them.

use egui::{Id, Order, Pos2, Rect, Sense, Stroke, Vec2};
use openpaint_core::Color;

use crate::components::color32;
use crate::design::{HAIRLINE, PANEL, PANEL_RULE, PANEL_RULE_WIDTH, SPACE};

/// The loupe's side, in points.
pub const LOUPE: f32 = 76.0;
/// Its top-left corner from the contact, in points.
pub const LOUPE_OFFSET: Vec2 = Vec2::new(22.0, -98.0);
/// Width of the paper ring around the colours, in points.
const RING: f32 = 4.0;

/// Where the loupe's top-left corner lies for a contact at `contact` on `screen` (points): at
/// the offset, below the contact when that would leave the top, always a space inside.
#[must_use]
pub fn loupe_origin(contact: Pos2, screen: Rect) -> Pos2 {
    let mut at = contact + LOUPE_OFFSET;
    if at.y < screen.top() + SPACE {
        at.y = contact.y + LOUPE_OFFSET.x;
    }
    let max = (screen.max - Vec2::splat(LOUPE + SPACE)).max(screen.min + Vec2::splat(SPACE));
    at.clamp(screen.min + Vec2::splat(SPACE), max)
}

/// Shows the loupe for a contact at `contact` (points): `under` it now over `current`.
pub fn loupe(ctx: &egui::Context, contact: Pos2, under: Color, current: Color) {
    let origin = loupe_origin(contact, ctx.content_rect());
    egui::Area::new(Id::new("loupe"))
        .order(Order::Tooltip)
        .interactable(false)
        .fade_in(false)
        .fixed_pos(origin)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(LOUPE), Sense::hover());
            let painter = ui.painter();
            let centre = rect.center();
            let outer = LOUPE / 2.0 - PANEL_RULE_WIDTH;
            let outline = Stroke::new(PANEL_RULE_WIDTH, PANEL_RULE);
            painter.circle(centre, outer, PANEL, outline);
            let inner = outer - RING;
            painter.circle_filled(centre, inner, color32(current));
            let top = Rect::from_min_max(rect.min, Pos2::new(rect.right(), centre.y));
            painter
                .with_clip_rect(top)
                .circle_filled(centre, inner, color32(under));
            painter.circle_stroke(centre, inner, Stroke::new(HAIRLINE, PANEL_RULE));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loupe_stays_clear_of_the_hand_and_on_the_screen() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 1000.0));
        assert_eq!(
            loupe_origin(Pos2::new(500.0, 500.0), screen),
            Pos2::new(522.0, 402.0)
        );
        let near_top = loupe_origin(Pos2::new(500.0, 40.0), screen);
        assert_eq!(near_top, Pos2::new(522.0, 62.0), "below the contact");
        let corner = loupe_origin(Pos2::new(1590.0, 990.0), screen);
        assert_eq!(corner, Pos2::new(1600.0 - LOUPE - SPACE, 892.0));
    }
}
