//! A division's and a whole copy's marks over a practice sheet (S95), drawn by the screen through
//! the view as every original is (`practice_marks.rs`), never into the artwork: the picture in the
//! original's frame (`practice_marks.rs` lays it, with its credit) and the copy's frame of its
//! shape (S99), empty, the division and the copy drawn in it. Compared, a division's lines lie over
//! its picture (as strong as the card's slider sets) in their levels' colours as thick as the pen
//! and inside the copy's frame (V大's red, green and blue: no answer, the picture under them is the
//! check).

use egui::{Color32, Pos2, Rect, Stroke};
use openpaint_practice::{DivisionCheck, Frame, Level, within};

/// A division's level's colour, as its pen draws it.
pub(crate) fn level_colour(level: Level) -> Color32 {
    let [r, g, b] = level.colour();
    Color32::from_rgb(r, g, b)
}

/// A division's lines over its copy in `frame`, each in its level's colour, `width` points (the
/// pen's width) with round joins and ends, so a stroke's hook shows no spike; kept to the frame:
/// cut at its edges, and on the screen inside its bounds (exact while the view is not turned).
pub(crate) fn show_lines(
    painter: &egui::Painter,
    frame: &Frame,
    check: &DivisionCheck,
    (at, width): (&dyn Fn([f64; 2]) -> Pos2, f32),
) {
    let painter = painter.with_clip_rect(Rect::from_points(&frame.corners().map(at)));
    let size = frame.extent();
    for line in &check.lines {
        let colour = level_colour(line.level);
        for part in within(&line.points, size) {
            let points: Vec<Pos2> = part.iter().map(|p| at(frame.to_canvas(*p))).collect();
            for segment in points.windows(2) {
                painter.line_segment([segment[0], segment[1]], Stroke::new(width, colour));
            }
            for point in points {
                painter.circle_filled(point, width / 2.0, colour);
            }
        }
    }
}
