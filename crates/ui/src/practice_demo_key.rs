//! A key shift's How (S107, `practice_demo.rs`): under its two frames a strip, the chip of the
//! area being painted under the crop (its colour, an arrow, its shifted colour) and the tendency's
//! disc under the copy with its name beside it, ringed in pencil while it is looked at and after.

use egui::{Align2, FontId, Pos2, Rect, Stroke, Vec2};
use openpaint_practice::ShiftDemo;

use crate::design::{INK, INK_SECONDARY, PENCIL};
use crate::practice_demo_marks::Frames;
use crate::practice_marks::colour;
use crate::text::{self as t, Words};

/// The strip's height, in points; a chip's swatch side, the disc's radius and its ring's.
pub(crate) const STRIP: f32 = 40.0;
const SWATCH: f32 = 22.0;
const DISC: f32 = 10.0;
const RING: f32 = 15.0;
/// Seconds the ring takes to draw.
const RINGING: f64 = 0.5;

/// The strip of `shift` `seconds` after the start, under `frames` from `top` down.
pub(crate) fn strip(
    painter: &egui::Painter,
    shift: &ShiftDemo,
    seconds: f64,
    (frames, top): (&Frames, f32),
) {
    let middle = top + STRIP / 2.0;
    let chip = (shift.chips.iter())
        .rev()
        .find(|(from, _)| *from <= seconds);
    if let (Some((_, [original, shifted])), Some(frame)) = (chip, frames.original) {
        let centre = Pos2::new(frame.center().x, middle);
        let swatch = |x: f32| Rect::from_center_size(Pos2::new(x, middle), Vec2::splat(SWATCH));
        let [left, right] = [centre.x - SWATCH * 1.2, centre.x + SWATCH * 1.2];
        for (x, rgb) in [(left, *original), (right, *shifted)] {
            painter.rect_filled(swatch(x), 0.0, colour(rgb));
            painter.rect_stroke(
                swatch(x),
                0.0,
                Stroke::new(1.0, INK),
                egui::StrokeKind::Outside,
            );
        }
        let [from, to] = [left + SWATCH * 0.65, right - SWATCH * 0.65];
        let line = Stroke::new(1.5, INK);
        painter.line_segment([Pos2::new(from, middle), Pos2::new(to, middle)], line);
        let head = Vec2::new(-4.0, 3.0);
        let tip = Pos2::new(to, middle);
        painter.line_segment([tip, tip + head], line);
        painter.line_segment([tip, tip + Vec2::new(head.x, -head.y)], line);
    }
    let centre = Pos2::new(frames.copy.center().x - 2.0 * DISC, middle);
    painter.circle(centre, DISC, colour(shift.tendency), Stroke::new(1.0, INK));
    if seconds >= shift.looked {
        let drawn = ((seconds - shift.looked) / RINGING).clamp(0.0, 1.0) as f32;
        let points: Vec<Pos2> = (0..=32)
            .map(|k| k as f32 / 32.0 * drawn * std::f32::consts::TAU)
            .map(|angle| centre + Vec2::new(angle.cos(), -angle.sin()) * RING)
            .collect();
        painter.add(egui::Shape::line(points, Stroke::new(1.25, PENCIL)));
    }
    let name = Words::from(t::TENDENCY).shown(painter.ctx());
    let beside = Pos2::new(centre.x + RING + 6.0, middle);
    let font = FontId::proportional(13.0);
    painter.text(beside, Align2::LEFT_CENTER, name, font, INK_SECONDARY);
}
