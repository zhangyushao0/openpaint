//! A seal (S89): a milestone of practice stamped in vermilion as if by hand, its square outline
//! with its two short lines stacked inside (two characters in Chinese, two words in English),
//! turned a few degrees. The practice book's only colour besides the user's own.

use egui::epaint::TextShape;
use egui::{FontId, Pos2, Shape, Stroke, Vec2};

use crate::palette::VERMILION;

/// How far each seal is turned, in degrees, by its place in a row: as stamped by hand.
const TURNS: [f32; 8] = [-4.0, 3.0, -2.0, 5.0, -5.0, 2.0, -3.0, 4.0];

/// The turn, in radians, of the seal at `place` in a row.
#[must_use]
pub fn seal_turn(place: usize) -> f32 {
    TURNS[place % TURNS.len()].to_radians()
}

/// `v` turned by `turn` radians.
fn turned(v: Vec2, turn: f32) -> Vec2 {
    let (sin, cos) = turn.sin_cos();
    Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

/// Paints a seal of `side` points round `centre`, turned `turn` radians, saying `lines`, at
/// `alpha` of its ink.
pub fn paint_seal(
    painter: &egui::Painter,
    (centre, side): (Pos2, f32),
    (lines, turn): (&[String; 2], f32),
    alpha: f32,
) {
    let ink = VERMILION.gamma_multiply(alpha.clamp(0.0, 1.0));
    let half = side / 2.0;
    let corners: Vec<Pos2> = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]
        .iter()
        .map(|[x, y]| centre + turned(Vec2::new(x * half, y * half), turn))
        .collect();
    painter.add(Shape::closed_line(corners, Stroke::new(side * 0.06, ink)));
    for (row, line) in lines.iter().enumerate() {
        let wide = line.chars().count() > 1;
        let mut size = if wide { side * 0.27 } else { side * 0.36 };
        let mut galley = painter.layout_no_wrap(line.clone(), FontId::proportional(size), ink);
        let room = side * 0.78;
        if galley.size().x > room {
            size *= room / galley.size().x;
            galley = painter.layout_no_wrap(line.clone(), FontId::proportional(size), ink);
        }
        let down = if row == 0 { -0.21 } else { 0.21 };
        let middle = centre + turned(Vec2::new(0.0, down * side), turn);
        let corner = middle + turned(-galley.size() / 2.0, turn);
        painter.add(TextShape::new(corner, galley, ink).with_angle(turn));
    }
}
