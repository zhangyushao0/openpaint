//! Value masses' comparison over a practice sheet (S96), drawn by the screen through the view as
//! every original is (`practice_marks.rs`), never into the artwork: the answer's three greys laid
//! over the picture's place in the copy's frame as strongly as the card's slider sets, its mass
//! edges in the marking red as thick as the pen and see-through (S88), and where the copy's grey
//! differs from the answer's over more than a pen's width, hatched in the grey it should be.

use egui::{Color32, Pos2, Shape, Stroke};
use openpaint_app::{Overlay, PracticeProjection};
use openpaint_practice::{Frame, Point};

use crate::design::CORRECTION;
use crate::practice_marks::{quad, value_colour};
use crate::practice_paint_marks::PaintedPictures;

/// How strongly the red edges lie over the copy, as every comparison's red.
const RED: f32 = 0.5;
/// The hatching's line, a share of the pen's width.
const HATCH: f32 = 0.6;

/// Value masses compared over the shown copy; a red edge `width` points wide.
pub(crate) fn show_overlay(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    overlay: &Overlay,
    (at, width): (&dyn Fn([f64; 2]) -> Pos2, f32),
    painted: &PaintedPictures,
) {
    let (
        Overlay::Masses {
            corners,
            shown,
            read,
        },
        Some(index),
    ) = (overlay, practice.current)
    else {
        return;
    };
    let frame = practice.sheet.pairs[index].copy;
    if let Some(texture) = painted.of(practice, index) {
        let corners = corners.map(|p| at(p.map(f64::from)));
        painter.add(quad(
            texture,
            corners,
            Color32::WHITE.gamma_multiply(*shown),
        ));
    }
    if let Some(read) = read {
        let hatch = width * HATCH;
        for (value, ends) in &read.hatching {
            let line = ends.map(|p| at(frame.to_canvas(p)));
            painter.line_segment(line, Stroke::new(hatch, value_colour(*value)));
        }
    }
    let edges = (practice.sheet.pictures.get(index))
        .and_then(Option::as_ref)
        .and_then(|picture| picture.masses.as_ref());
    if let Some(masses) = edges {
        let red = CORRECTION.gamma_multiply(RED);
        for edge in &masses.edges {
            painter.add(Shape::line(on(&frame, edge, at), Stroke::new(width, red)));
        }
    }
}

/// `line` (frame units) of `frame` on the screen.
fn on(frame: &Frame, line: &[Point], at: &dyn Fn([f64; 2]) -> Pos2) -> Vec<Pos2> {
    line.iter().map(|p| at(frame.to_canvas(*p))).collect()
}
