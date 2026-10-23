//! A practice sheet's exercises over its canvas (G9): each pair revealed so far as two frames,
//! the original's and the empty one for the copy, the shown pair's in ink and the earlier ones
//! faint; each original in its frame (a line or a group of lines in ink, a swatch filling the
//! frame, a single shape's or a colour copy's picture with its credit small under the frame where
//! its licence asks for one, a colour combination's or a value scale's in `practice_paint_marks.rs`,
//! a box's or an ellipse's in `practice_geometry_marks.rs`).
//! While compared, the original's lines lie over the copy, in place or aligned onto it, thick and
//! clear in a semi-transparent marking red the copy shows through; a picture lies over the copy
//! (a shape's faint with its outline in that red, a colour copy's as strongly as the user sets);
//! a colour shows as a patch inside the other. All of it is drawn by the screen over the canvas
//! through the view, never into the artwork: the originals cannot be drawn on.

use std::fmt;
use std::sync::Arc;

use egui::epaint::TextShape;
use egui::{Color32, FontId, Mesh, Order, Pos2, Shape, Stroke, TextureHandle};
use openpaint_app::{Overlay, PictureImage, PracticeProjection, SessionProjection, ViewMapping};
use openpaint_practice::{FRAME, Frame, ItemRef, Original, Rgb8};

use crate::components::{Alpha, canvas_point, picture_texture};
use crate::design::{CORRECTION, DIVIDER, INK, INK_SECONDARY, PANEL_RULE_WIDTH};
use crate::practice_geometry_marks as geometry_marks;
use crate::practice_paint_marks as paint_marks;
use crate::text::{self as t, Words};

/// An original line's width, in canvas pixels, and the least it shows at, in points.
const LINE_WIDTH: f32 = 3.0;
const LEAST_LINE: f32 = 1.25;
/// The overlay's width, in canvas pixels (a frame is 400) and at least in points, and how
/// strongly it lies over the copy.
const OVERLAY_WIDTH: f32 = 12.0;
const LEAST_OVERLAY: f32 = 4.0;
const OVERLAY_SHOWN: f32 = 0.5;
/// A picture's credit under its frame: its type size and its distance below, in canvas pixels of
/// a session's frame.
const CREDIT_SIZE: f32 = 9.0;
const CREDIT_GAP: f32 = 6.0;

/// The session's pictures as textures, uploaded as the store reads them (none: one that could not
/// be read), and its colour combinations' pictures.
#[derive(Default)]
pub(crate) struct SheetPictures {
    shown: Vec<(ItemRef, Option<TextureHandle>)>,
    painted: paint_marks::PaintedPictures,
}

impl fmt::Debug for SheetPictures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sheet pictures: {}", self.shown.len())
    }
}

impl SheetPictures {
    /// Uploads the pictures `images` holds that are new, lets go of those it no longer holds.
    fn sync(&mut self, ctx: &egui::Context, images: &[(ItemRef, Option<Arc<PictureImage>>)]) {
        self.shown
            .retain(|(item, _)| images.iter().any(|(held, _)| held == item));
        for (item, image) in images {
            if self.shown.iter().any(|(shown, _)| shown == item) {
                continue;
            }
            let texture = image.as_ref().map(|image| {
                let name = format!("practice-{}-{}", item.pack, item.item);
                picture_texture(ctx, name, (image.size, &image.pixels, Alpha::Straight))
            });
            self.shown.push((item.clone(), texture));
        }
    }

    /// `item`'s texture: `None` while not read yet, `Some(None)` when it could not be.
    fn texture(&self, item: &ItemRef) -> Option<Option<&TextureHandle>> {
        let (_, texture) = self.shown.iter().find(|(shown, _)| shown == item)?;
        Some(texture.as_ref())
    }
}

/// The sheet of `projection`'s practice, if it is one, through `mapping` on a screen of
/// `pixels_per_point`, its pictures from `pictures`.
pub(crate) fn show_marks(
    ctx: &egui::Context,
    projection: &SessionProjection,
    (mapping, pixels_per_point): (&ViewMapping, f32),
    pictures: &mut SheetPictures,
) {
    let Some(practice) = projection.practice() else {
        return;
    };
    pictures.sync(ctx, &practice.images);
    pictures.painted.sync(ctx, &practice.sheet.painted);
    let layer = egui::LayerId::new(Order::Background, egui::Id::new("practice sheet"));
    let painter = ctx.layer_painter(layer);
    let screen = ctx.content_rect();
    let at = |point: [f64; 2]| canvas_point(mapping, pixels_per_point, point.map(|v| v as f32));
    let line_width = (LINE_WIDTH * mapping.scale() / pixels_per_point).max(LEAST_LINE);
    let sheet = &practice.sheet;
    for (index, pair) in sheet.pairs.iter().enumerate().take(practice.revealed) {
        let corners = |frame: &Frame| frame.corners().map(at);
        let [original, copy] = [corners(&pair.original), corners(&pair.copy)];
        let shown = egui::Rect::from_points(&original).union(egui::Rect::from_points(&copy));
        if !shown.intersects(screen) {
            continue;
        }
        let frame = if practice.current == Some(index) {
            Stroke::new(PANEL_RULE_WIDTH, INK)
        } else {
            Stroke::new(1.0, DIVIDER)
        };
        match &sheet.originals[index] {
            Original::Lines(lines) => {
                for line in lines {
                    let points = (line.iter()).map(|p| at(pair.original.to_canvas(*p)));
                    painter.add(Shape::line(points.collect(), Stroke::new(line_width, INK)));
                }
            }
            Original::Colour(rgb) => {
                let swatch = Shape::convex_polygon(original.to_vec(), colour(*rgb), Stroke::NONE);
                painter.add(swatch);
            }
            Original::Picture(_) => {
                let scale = mapping.scale() / pixels_per_point;
                let placed: (&dyn Fn([f64; 2]) -> Pos2, f32) = (&at, scale);
                let shown = (index, &pair.original);
                show_picture(&painter, practice, shown, placed, pictures);
            }
            Original::Areas(_) | Original::Scale(_) => {
                let painted = &pictures.painted;
                paint_marks::show_original(&painter, practice, (index, pair), &at, painted);
            }
            Original::Box(_) | Original::FreeBoxes(_) | Original::Ellipse(_) => {
                let scale = mapping.scale() / pixels_per_point;
                let widths = (line_width, scale);
                geometry_marks::show_original(&painter, practice, (index, pair), &at, widths);
            }
        }
        painter.add(Shape::closed_line(original.to_vec(), frame));
        painter.add(Shape::closed_line(copy.to_vec(), frame));
    }
    let width = (OVERLAY_WIDTH * mapping.scale() / pixels_per_point).max(LEAST_OVERLAY);
    if let Some(overlay) = &practice.overlay {
        show_overlay(&painter, practice, overlay, (&at, width), pictures);
    }
    paint_marks::show_probe(&painter, practice, &at);
}

/// Exercise `index`'s picture fitted in its `frame` (or, where no pack kept holds it now, that
/// it was removed: the strokes copied from it stay), its credit under the frame; `at` maps
/// canvas points to the screen, `scale` is points per canvas pixel.
fn show_picture(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    (index, frame): (usize, &Frame),
    (at, scale): (&dyn Fn([f64; 2]) -> Pos2, f32),
    pictures: &SheetPictures,
) {
    let picture = practice.sheet.pictures.get(index).and_then(Option::as_ref);
    let texture = picture.and_then(|p| pictures.texture(&p.item));
    // Frame units run one to the frame's width: its middle lies half its height down.
    let middle = at(frame.to_canvas([0.5, 0.5 * frame.size[1] / frame.unit()]));
    // In step with the frame: a colour copy's frame is four times a session's.
    let grown = (frame.unit() / FRAME) as f32;
    let font = FontId::proportional((14.0 * scale * grown).max(6.0));
    match (picture, texture) {
        (Some(picture), Some(Some(texture))) => {
            let [x, y] = picture.fit.origin;
            let [w, h] = picture.fit.size;
            let place = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
            let corners = place.map(|p| at(frame.to_canvas(p)));
            painter.add(quad(texture, corners, Color32::WHITE));
        }
        (Some(_), None) => {}
        _ => {
            let missing = Words::from(t::PICTURE_REMOVED).shown(painter.ctx());
            let centre = egui::Align2::CENTER_CENTER;
            painter.text(middle, centre, missing, font, INK_SECONDARY);
        }
    }
    let Some(credit) = picture.and_then(|p| p.credit.as_ref()) else {
        return;
    };
    let left = at(frame.to_canvas([0.0, 1.0]));
    let right = at(frame.to_canvas([1.0, 1.0]));
    let angle = (right.y - left.y).atan2(right.x - left.x);
    let below = at([
        frame.origin[0],
        frame.origin[1] + frame.size[1] + f64::from(CREDIT_GAP * grown),
    ]);
    let size = CREDIT_SIZE * scale * grown;
    let width = left.distance(right);
    let galley = painter.layout(
        credit.clone(),
        FontId::proportional(size),
        INK_SECONDARY,
        width,
    );
    painter.add(TextShape::new(below, galley, INK_SECONDARY).with_angle(angle));
}

/// `texture` laid on the quad `corners` (top left, top right, bottom right, bottom left), tinted.
pub(crate) fn quad(texture: &TextureHandle, corners: [Pos2; 4], tint: Color32) -> Shape {
    let mut mesh = Mesh::with_texture(texture.id());
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (pos, [u, v]) in corners.into_iter().zip(uv) {
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv: egui::pos2(u, v),
            color: tint,
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    Shape::mesh(mesh)
}

/// The shown exercise's comparison over its copy; a line's `width` points wide.
fn show_overlay(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    overlay: &Overlay,
    (at, width): (&dyn Fn([f64; 2]) -> Pos2, f32),
    pictures: &SheetPictures,
) {
    let Some(index) = practice.current else {
        return;
    };
    let pair = practice.sheet.pairs[index];
    let red = CORRECTION.gamma_multiply(OVERLAY_SHOWN);
    let canvas = |p: &[f32; 2]| at(p.map(f64::from));
    match (overlay, &practice.sheet.originals[index]) {
        (Overlay::Lines(lines), Original::Lines(_)) => {
            for line in lines {
                let points: Vec<Pos2> = line.iter().map(canvas).collect();
                painter.add(Shape::line(points, Stroke::new(width, red)));
            }
        }
        (
            Overlay::Picture {
                corners,
                outline,
                shown,
            },
            Original::Picture(_),
        ) => {
            let item = practice.sheet.pictures[index].as_ref().map(|p| &p.item);
            if let Some(Some(texture)) = item.and_then(|item| pictures.texture(item)) {
                let faint = Color32::WHITE.gamma_multiply(*shown);
                painter.add(quad(texture, corners.map(|p| canvas(&p)), faint));
            }
            // An outline holds the shape's every turn: half a line's width keeps them apart.
            for line in outline {
                let points: Vec<Pos2> = line.iter().map(canvas).collect();
                painter.add(Shape::line(points, Stroke::new(width / 2.0, red)));
            }
        }
        (Overlay::Colour { original, last, .. }, Original::Colour(_)) => {
            swapped(painter, &pair, at, (*original, *last));
        }
        (Overlay::Areas { .. } | Overlay::Scale(_), _) => {
            let painted = &pictures.painted;
            paint_marks::show_overlay(painter, practice, overlay, (at, width), painted);
        }
        (Overlay::Box(_) | Overlay::FreeBoxes(_) | Overlay::Ellipse(_), _) => {
            geometry_marks::show_overlay(painter, practice, overlay, (at, width));
        }
        _ => {}
    }
}

/// A colour compared: each colour as a patch in the middle of the other's frame.
fn swapped(
    painter: &egui::Painter,
    pair: &openpaint_practice::Pair,
    at: &dyn Fn([f64; 2]) -> Pos2,
    (original, last): (Rgb8, Rgb8),
) {
    let patch = |frame: &Frame| {
        let [[x, y], _, [right, bottom], _] = frame.corners();
        let (inset, side) = ((right - x) / 4.0, (bottom - y) / 2.0);
        let [x, y] = [x + inset, y + inset];
        vec![[x, y], [x + side, y], [x + side, y + side], [x, y + side]]
            .into_iter()
            .map(at)
            .collect::<Vec<Pos2>>()
    };
    let copy = Shape::convex_polygon(patch(&pair.copy), colour(original), Stroke::NONE);
    painter.add(copy);
    let shown = Shape::convex_polygon(patch(&pair.original), colour(last), Stroke::NONE);
    painter.add(shown);
}

pub(crate) fn colour([r, g, b]: Rgb8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
