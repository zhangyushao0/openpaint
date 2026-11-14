//! The Pomodoro's ring (S124): a clock's state drawn as a ring, never a filled block. A pale ring
//! with an ink arc from twelve o'clock round for the time gone (the block's in the bar and on the
//! home, the break's on the block's end), whole in ink once a break is over, whole in the seal's
//! red at the time; on the home before a block, the ring with one hand.

use std::f32::consts::{FRAC_PI_2, TAU};

use egui::{Color32, Pos2, Shape, Stroke, Vec2};
use openpaint_ui::design::{DIVIDER, INK};

/// The pale ring's line and the arc's, in points.
const PALE_LINE: f32 = 2.0;
const ARC_LINE: f32 = 2.5;
/// Points on a whole arc.
const ARC_STEPS: usize = 48;

/// How a ring shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ring {
    /// Counting: `gone` (0 to 1) of it inked from twelve o'clock round, the rest pale.
    Counting { gone: f32 },
    /// Held: pale all round, the arc gone faint.
    Held { gone: f32 },
    /// Whole in `ink`: the seal's red at the time, ink once a break is over.
    Whole(Color32),
    /// The home's glyph before a block: the ring with one hand to twelve.
    Glyph,
}

/// Paints `ring` round `centre`, `radius` points.
pub fn paint_ring(painter: &egui::Painter, (centre, radius): (Pos2, f32), ring: Ring) {
    match ring {
        Ring::Counting { gone } => {
            painter.circle_stroke(centre, radius, Stroke::new(PALE_LINE, DIVIDER));
            arc(painter, (centre, radius), gone, INK);
        }
        Ring::Held { gone } => {
            painter.circle_stroke(centre, radius, Stroke::new(PALE_LINE, DIVIDER));
            arc(painter, (centre, radius), gone, INK.gamma_multiply(0.35));
        }
        Ring::Whole(ink) => {
            painter.circle_stroke(centre, radius, Stroke::new(ARC_LINE, ink));
        }
        Ring::Glyph => {
            painter.circle_stroke(centre, radius, Stroke::new(PALE_LINE, INK));
            let hand = [centre, centre - Vec2::new(0.0, radius * 0.7)];
            painter.line_segment(hand, Stroke::new(PALE_LINE, INK));
        }
    }
}

/// `share` (0 to 1) of the circle from twelve o'clock clockwise, in `ink`.
fn arc(painter: &egui::Painter, (centre, radius): (Pos2, f32), share: f32, ink: Color32) {
    let share = share.clamp(0.0, 1.0);
    let steps = ((ARC_STEPS as f32 * share).ceil() as usize).max(1);
    if share <= 0.0 {
        return;
    }
    let points: Vec<Pos2> = (0..=steps)
        .map(|step| {
            let angle = -FRAC_PI_2 + TAU * share * step as f32 / steps as f32;
            centre + radius * Vec2::angled(angle)
        })
        .collect();
    painter.add(Shape::line(points, Stroke::new(ARC_LINE, ink)));
}
