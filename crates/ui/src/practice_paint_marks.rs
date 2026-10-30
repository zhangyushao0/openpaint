//! A painted exercise's marks over a practice sheet (S82), drawn by the screen through the view as
//! every original is (`practice_marks.rs`):
//!
//! - a colour combination's picture in its frame; compared, the picture over the copy in place
//!   as strongly as the user sets, the area the card compares outlined in ink on both frames, and
//!   the edges where the copy made a weak contrast strong in the marking red;
//! - a value scale, in the one frame drawn in (S97): its cells, the ends filled as given, the
//!   cell in hand outlined in ink; compared, the even scale as a row under the copy's, cell under
//!   cell (a hued scale keeping its hue), a cell of the copy's that is uneven outlined red
//!   together with its even one;
//! - the point the eyedropper compared, ringed on both frames.

use std::fmt;
use std::sync::Arc;

use egui::{Color32, Pos2, Shape, Stroke, TextureHandle};
use openpaint_app::{Overlay, PictureImage, PracticeProjection};
use openpaint_practice::{Frame, Original, Pair, Point, ROW, ScaleCheck, ValueScale};

use crate::components::{Alpha, picture_texture};
use crate::design::{CORRECTION, DIVIDER, INK, PANEL, PANEL_RULE_WIDTH};
use crate::practice_marks::{colour, quad};

/// An outline's or a mark's line, in points; the paper's halo under it; a probe's ring's radius.
const OUTLINE: f32 = 2.0;
const HALO: f32 = 4.0;
const PROBE: f32 = 10.0;

/// The colour combinations' and objects' pictures as textures, uploaded once a sheet.
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

    /// Exercise `index`'s picture's texture, if it is a colour combination's or an object's.
    pub(crate) fn of(&self, practice: &PracticeProjection, index: usize) -> Option<&TextureHandle> {
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
    for end in [0, scale.cells - 1] {
        fill(
            painter,
            (&pair.copy, scale.cell(end)),
            at,
            scale.colour(targets[end]),
        );
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
        (Overlay::Scale(check), Original::Scale(scale)) => {
            even_row(painter, &pair.copy, (scale, check.as_deref()), at);
        }
        _ => {}
    }
}

/// `cell` (its top left and bottom right, frame units) of `frame` filled with `rgb`.
fn fill(
    painter: &egui::Painter,
    (frame, cell): (&Frame, [Point; 2]),
    at: &dyn Fn([f64; 2]) -> Pos2,
    rgb: openpaint_practice::Rgb8,
) {
    let corners = rect(frame, cell, at);
    painter.add(Shape::convex_polygon(corners, colour(rgb), Stroke::NONE));
}

/// The even scale as a row under the copy's, cell under cell; once the copy is read, each of its
/// uneven cells outlined red together with its even one.
fn even_row(
    painter: &egui::Painter,
    frame: &Frame,
    (scale, check): (&ValueScale, Option<&ScaleCheck>),
    at: &dyn Fn([f64; 2]) -> Pos2,
) {
    for (cell, target) in scale.targets().iter().enumerate() {
        let even = scale.even_cell(cell);
        fill(painter, (frame, even), at, scale.colour(*target));
        let outline = rect(frame, even, at);
        painter.add(Shape::closed_line(outline, Stroke::new(1.0, DIVIDER)));
    }
    let Some(check) = check else {
        return;
    };
    let top = ROW[1];
    for cell in &check.uneven {
        let [[left, _], [right, bottom]] = scale.even_cell(*cell);
        let both = rect(frame, [[left, top], [right, bottom]], at);
        painter.add(Shape::closed_line(
            both.clone(),
            Stroke::new(OUTLINE + HALO, PANEL),
        ));
        painter.add(Shape::closed_line(both, Stroke::new(OUTLINE, CORRECTION)));
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
