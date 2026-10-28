//! A painted exercise's marks over a practice sheet (S82), drawn by the screen through the view as
//! every original is (`practice_marks.rs`):
//!
//! - a colour combination's picture in its frame; compared, the picture over the copy in place
//!   as strongly as the user sets, the area the card compares outlined in ink on both frames, and
//!   the edges where the copy made a weak contrast strong in the marking red;
//! - a value scale: its given ends as two halves of the row in the original's frame (once
//!   compared, the even scale there instead, side by side with the copy's row), and in the copy's
//!   frame its cells, the ends filled as given, the cell in hand outlined in ink; compared, under
//!   the row a bar per cell as high as its value (in that value's colour) with the even step a
//!   red tick across it, the bar outlined red where the cell is uneven or out of order;
//! - the point the eyedropper compared, ringed on both frames.

use std::fmt;
use std::sync::Arc;

use egui::{Color32, Pos2, Shape, Stroke, TextureHandle};
use openpaint_app::{Overlay, PictureImage, PracticeProjection};
use openpaint_practice::{BARS, Frame, Original, Pair, Point, ROW, ScaleCheck, ValueScale};

use crate::components::{Alpha, picture_texture};
use crate::design::{CORRECTION, DIVIDER, INK, PANEL, PANEL_RULE_WIDTH};
use crate::practice_marks::{colour, quad};

/// An outline's or a mark's line, in points; the paper's halo under it; a probe's ring's radius.
const OUTLINE: f32 = 2.0;
const HALO: f32 = 4.0;
const PROBE: f32 = 10.0;
/// A bar's half width and its even step's tick's, shares of a cell's width.
const BAR: f64 = 0.22;
const TICK: f64 = 0.36;

/// The colour combinations' pictures as textures, uploaded once a sheet.
#[derive(Default)]
pub(crate) struct PaintedPictures {
    shown: Vec<(Arc<PictureImage>, TextureHandle)>,
}

impl fmt::Debug for PaintedPictures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "painted pictures: {}", self.shown.len())
    }
}

impl PaintedPictures {
    /// Uploads the pictures of `painted` not uploaded yet, lets go of the others.
    pub(crate) fn sync(&mut self, ctx: &egui::Context, painted: &[Option<Arc<PictureImage>>]) {
        let held =
            |image: &Arc<PictureImage>| painted.iter().flatten().any(|p| Arc::ptr_eq(p, image));
        self.shown.retain(|(image, _)| held(image));
        for (index, image) in painted.iter().enumerate() {
            let Some(image) = image else {
                continue;
            };
            if self.texture(image).is_none() {
                let name = format!("practice-areas-{index}");
                let texture =
                    picture_texture(ctx, name, (image.size, &image.pixels, Alpha::Straight));
                self.shown.push((Arc::clone(image), texture));
            }
        }
    }

    fn texture(&self, image: &Arc<PictureImage>) -> Option<&TextureHandle> {
        (self.shown.iter())
            .find(|(shown, _)| Arc::ptr_eq(shown, image))
            .map(|(_, texture)| texture)
    }

    /// Exercise `index`'s picture's texture, if it is a colour combination's.
    fn of(&self, practice: &PracticeProjection, index: usize) -> Option<&TextureHandle> {
        let image = practice.sheet.painted.get(index)?.as_ref()?;
        self.texture(image)
    }
}

/// Exercise `index`'s original in its frame and its copy's cells; `at` maps canvas points to the
/// screen.
pub(crate) fn show_original(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    (index, pair): (usize, &Pair),
    at: &dyn Fn([f64; 2]) -> Pos2,
    painted: &PaintedPictures,
) {
    match &practice.sheet.originals[index] {
        Original::Areas(_) => {
            if let Some(texture) = painted.of(practice, index) {
                let corners = pair.original.corners().map(at);
                painter.add(quad(texture, corners, Color32::WHITE));
            }
        }
        Original::Scale(scale) => show_scale(painter, practice, (index, pair), scale, at),
        _ => {}
    }
}

/// The rectangle `[top left, bottom right]` (frame units) of `frame` on the screen.
fn rect(
    frame: &Frame,
    [[left, top], [right, bottom]]: [Point; 2],
    at: &dyn Fn([f64; 2]) -> Pos2,
) -> Vec<Pos2> {
    [[left, top], [right, top], [right, bottom], [left, bottom]]
        .into_iter()
        .map(|p| at(frame.to_canvas(p)))
        .collect()
}

fn show_scale(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    (index, pair): (usize, &Pair),
    scale: &ValueScale,
    at: &dyn Fn([f64; 2]) -> Pos2,
) {
    let shown = practice.current == Some(index);
    let targets = scale.targets();
    let fill = |frame: &Frame, cell: [Point; 2], value: f64| {
        let colour = colour(scale.colour(value));
        painter.add(Shape::convex_polygon(
            rect(frame, cell, at),
            colour,
            Stroke::NONE,
        ));
    };
    let last = scale.cells - 1;
    if shown && practice.result.is_some() {
        for (cell, target) in targets.iter().enumerate() {
            fill(&pair.original, scale.cell(cell), *target);
        }
    } else {
        let [left, top, right, bottom] = ROW;
        let middle = (left + right) / 2.0;
        fill(&pair.original, [[left, top], [middle, bottom]], targets[0]);
        fill(
            &pair.original,
            [[middle, top], [right, bottom]],
            targets[last],
        );
    }
    for end in [0, last] {
        fill(&pair.copy, scale.cell(end), targets[end]);
    }
    for cell in 0..scale.cells {
        let in_hand = shown && practice.cell == Some(cell);
        let line = if in_hand {
            Stroke::new(PANEL_RULE_WIDTH, INK)
        } else {
            Stroke::new(1.0, DIVIDER)
        };
        painter.add(Shape::closed_line(
            rect(&pair.copy, scale.cell(cell), at),
            line,
        ));
    }
}

/// A painted copy's comparison over its frame; a red mark `width` points wide.
pub(crate) fn show_overlay(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    overlay: &Overlay,
    (at, width): (&dyn Fn([f64; 2]) -> Pos2, f32),
    painted: &PaintedPictures,
) {
    let Some(index) = practice.current else {
        return;
    };
    let pair = practice.sheet.pairs[index];
    match (overlay, &practice.sheet.originals[index]) {
        (
            Overlay::Areas {
                shown,
                read,
                outline,
                ..
            },
            Original::Areas(_),
        ) => {
            if let Some(texture) = painted.of(practice, index) {
                let corners = pair.copy.corners().map(at);
                painter.add(quad(
                    texture,
                    corners,
                    Color32::WHITE.gamma_multiply(*shown),
                ));
            }
            for frame in [pair.original, pair.copy] {
                segments(painter, (&frame, outline), at, (OUTLINE, INK));
            }
            if let Some(read) = read {
                let red = width.max(OUTLINE);
                segments(painter, (&pair.copy, &read.strong), at, (red, CORRECTION));
            }
        }
        (Overlay::Scale(Some(check)), Original::Scale(scale)) => {
            bars(painter, &pair.copy, (scale, check), at);
        }
        _ => {}
    }
}

/// `edges` (lines in frame units) of `frame`, `width` points wide in `ink` over the paper's halo.
fn segments(
    painter: &egui::Painter,
    (frame, edges): (&Frame, &[Vec<Point>]),
    at: &dyn Fn([f64; 2]) -> Pos2,
    (width, ink): (f32, Color32),
) {
    let line =
        |edge: &Vec<Point>| -> Vec<Pos2> { edge.iter().map(|p| at(frame.to_canvas(*p))).collect() };
    for stroke in [Stroke::new(width + HALO, PANEL), Stroke::new(width, ink)] {
        for edge in edges {
            painter.add(Shape::line(line(edge), stroke));
        }
    }
}

/// Under the copy's row: each cell's value as a bar in its colour, its even step a red tick.
fn bars(
    painter: &egui::Painter,
    frame: &Frame,
    (scale, check): (&ValueScale, &ScaleCheck),
    at: &dyn Fn([f64; 2]) -> Pos2,
) {
    let [top, bottom] = BARS;
    let height = bottom - top;
    let base = [
        frame.to_canvas([ROW[0], bottom]),
        frame.to_canvas([ROW[2], bottom]),
    ];
    painter.add(Shape::line_segment(base.map(at), Stroke::new(1.0, DIVIDER)));
    for cell in 0..scale.cells {
        let [[left, _], [right, _]] = scale.cell(cell);
        let (middle, width) = ((left + right) / 2.0, right - left);
        if let Some(value) = check.values[cell] {
            let off = check.uneven.contains(&cell) || check.reversed.contains(&cell);
            let line = if off {
                Stroke::new(OUTLINE, CORRECTION)
            } else {
                Stroke::new(1.0, INK)
            };
            let bar = [
                [middle - BAR * width, bottom - value * height],
                [middle + BAR * width, bottom],
            ];
            let fill = colour(scale.colour(value));
            painter.add(Shape::convex_polygon(rect(frame, bar, at), fill, line));
        }
        let y = bottom - check.targets[cell] * height;
        let tick = [[middle - TICK * width, y], [middle + TICK * width, y]];
        let tick = tick.map(|p| at(frame.to_canvas(p)));
        painter.add(Shape::line_segment(tick, Stroke::new(OUTLINE, CORRECTION)));
    }
}

/// The point the eyedropper compared, ringed on both frames of the shown exercise.
pub(crate) fn show_probe(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    at: &dyn Fn([f64; 2]) -> Pos2,
) {
    let (Some(probe), Some(index)) = (practice.probe, practice.current) else {
        return;
    };
    let pair = practice.sheet.pairs[index];
    for frame in [pair.original, pair.copy] {
        let centre = at(frame.to_canvas(probe.at));
        painter.circle_stroke(centre, PROBE, Stroke::new(HALO, PANEL));
        painter.circle_stroke(centre, PROBE, Stroke::new(OUTLINE, INK));
    }
}
