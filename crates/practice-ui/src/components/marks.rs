//! The path's small marks, state drawn as a dot or a ring, never a filled block: an exercise's
//! result (Accurate a dot, Almost a ring half filled, Again an empty ring) and a skill's three
//! dots (each earned one filled, the others rings), the one just earned inking in.

use std::f32::consts::PI;

use egui::{Color32, Pos2, Shape, Stroke, Vec2};
use openpaint_practice::Outcome;

/// A ring's line, in points.
const RING_LINE: f32 = 1.5;
/// Points on a half disc's arc.
const ARC_STEPS: usize = 16;
/// How many dots a skill has.
pub const DOTS: u8 = 3;

/// `outcome`'s mark round `centre`, `radius` points, in `ink`.
pub fn paint_result(
    painter: &egui::Painter,
    (centre, radius): (Pos2, f32),
    outcome: Outcome,
    ink: Color32,
) {
    match outcome {
        Outcome::Clean => {
            painter.circle_filled(centre, radius, ink);
        }
        Outcome::Near => {
            let ring = radius - RING_LINE / 2.0;
            let half: Vec<Pos2> = (0..=ARC_STEPS)
                .map(|step| {
                    let angle = PI / 2.0 + PI * step as f32 / ARC_STEPS as f32;
                    centre + ring * Vec2::angled(angle)
                })
                .collect();
            painter.add(Shape::convex_polygon(half, ink, Stroke::NONE));
            painter.circle_stroke(centre, ring, Stroke::new(RING_LINE, ink));
        }
        Outcome::Redo => {
            painter.circle_stroke(
                centre,
                radius - RING_LINE / 2.0,
                Stroke::new(RING_LINE, ink),
            );
        }
    }
}

/// A skill's three dots from `left` (the first's centre) `pitch` points apart, `radius` points
/// each, `earned` of them filled in `ink`, the others rings; the last earned inking in by `fresh`
/// (0 to 1) when it was just earned.
pub fn paint_dots(
    painter: &egui::Painter,
    (left, pitch, radius): (Pos2, f32, f32),
    (earned, fresh): (u8, Option<f32>),
    ink: Color32,
) {
    for dot in 0..DOTS {
        let centre = left + Vec2::new(f32::from(dot) * pitch, 0.0);
        let ring = Stroke::new(RING_LINE, ink);
        match (dot + 1).cmp(&earned) {
            std::cmp::Ordering::Less => {
                painter.circle_filled(centre, radius, ink);
            }
            std::cmp::Ordering::Equal => {
                let inked = fresh.unwrap_or(1.0).clamp(0.0, 1.0);
                painter.circle_stroke(centre, radius - RING_LINE / 2.0, ring);
                painter.circle_filled(centre, radius * inked, ink);
            }
            std::cmp::Ordering::Greater => {
                painter.circle_stroke(centre, radius - RING_LINE / 2.0, ring);
            }
        }
    }
}
