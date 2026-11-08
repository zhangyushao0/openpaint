//! A practice sheet's exercises over its canvas (G9): each pair revealed so far as two frames,
//! the original's and the empty one for the copy (a kind built from given parts the one frame
//! drawn in, S97), the shown pair's in ink and the earlier ones faint; in them what each exercise
//! gives and, while compared, what its comparison lays over the copy: marks its kind made
//! (`openpaint_practice::SheetMark`, S109) which `mark_draw.rs` draws without knowing the
//! kind. The pictures they lay are uploaded here: the packs' as the store reads them (a single
//! shape's with its ground faded, S99), those the practice crate draws (a generated combination's,
//! an object's, value masses' answer) and a key shift's frames in grey (S107). All of it is drawn
//! by the screen over the canvas through the view, never into the artwork: the originals cannot be
//! drawn on.

use std::fmt;
use std::sync::Arc;

use egui::epaint::TextShape;
use egui::{Align2, Color32, FontId, Mesh, Order, Pos2, Shape, Stroke, TextureHandle};
use openpaint_app::ViewMapping;
use openpaint_practice::{FRAME, Frame, ItemRef, Rgb8};
use openpaint_practice_app::{Greys, INK_DIAMETER, PictureImage, PracticeProjection};

use openpaint_ui::components::{Alpha, canvas_point, picture_texture};
use openpaint_ui::design::{DIVIDER, INK, INK_SECONDARY, PANEL, PANEL_RULE_WIDTH};

use super::mark_draw::{Widths, draw};
use super::mark_lines::At;
use crate::text::{self as t, Words};

/// An original line's width, in canvas pixels, and the least it shows at, in points.
const LINE_WIDTH: f32 = 3.0;
const LEAST_LINE: f32 = 1.25;
/// The overlay's least width, in points (its width is the Draw pen's, `INK_DIAMETER` canvas
/// pixels).
const LEAST_OVERLAY: f32 = 1.0;
/// A picture's credit under its frame: its type size and its distance below, in canvas pixels of
/// a session's frame.
const CREDIT_SIZE: f32 = 9.0;
const CREDIT_GAP: f32 = 6.0;
/// The eyedropper's point's ring: its line, the paper's halo under it, its radius, in points.
const PROBE_LINE: f32 = 2.0;
const PROBE_HALO: f32 = 4.0;
const PROBE: f32 = 10.0;

/// The session's pictures as textures, uploaded as the store reads them (none: one that could not
/// be read), its single shapes' with their ground faded (S99), the pictures the practice crate
/// draws, and a compared key shift's frames in grey.
#[derive(Default)]
pub(crate) struct SheetPictures {
    shown: Vec<(ItemRef, Option<TextureHandle>)>,
    grounded: Vec<(ItemRef, TextureHandle)>,
    painted: Vec<(Arc<PictureImage>, TextureHandle)>,
    greys: Option<(Arc<Greys>, [Option<TextureHandle>; 2])>,
}

impl fmt::Debug for SheetPictures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sheet pictures: {}", self.shown.len())
    }
}

impl SheetPictures {
    /// Uploads the pictures `practice` holds that are new, lets go of those it no longer holds.
    fn sync(&mut self, ctx: &egui::Context, practice: &PracticeProjection) {
        let images = &practice.images;
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
        let grounded = &practice.grounded;
        self.grounded
            .retain(|(item, _)| grounded.iter().any(|(held, _)| held == item));
        for (item, image) in grounded {
            if self.grounded.iter().any(|(shown, _)| shown == item) {
                continue;
            }
            let name = format!("practice-ground-{}-{}", item.pack, item.item);
            let texture = picture_texture(ctx, name, (image.size, &image.pixels, Alpha::Straight));
            self.grounded.push((item.clone(), texture));
        }
        self.sync_painted(ctx, &practice.sheet.painted);
        self.sync_greys(ctx, practice.greys.as_ref());
    }

    /// Uploads the pictures of `painted` not uploaded yet, lets go of the others.
    fn sync_painted(&mut self, ctx: &egui::Context, painted: &[Option<Arc<PictureImage>>]) {
        let held =
            |image: &Arc<PictureImage>| painted.iter().flatten().any(|p| Arc::ptr_eq(p, image));
        self.painted.retain(|(image, _)| held(image));
        for (index, image) in painted.iter().enumerate() {
            let Some(image) = image else {
                continue;
            };
            if !self
                .painted
                .iter()
                .any(|(shown, _)| Arc::ptr_eq(shown, image))
            {
                let name = format!("practice-areas-{index}");
                let texture =
                    picture_texture(ctx, name, (image.size, &image.pixels, Alpha::Straight));
                self.painted.push((Arc::clone(image), texture));
            }
        }
    }

    /// Uploads a key shift's grey frames once they show; lets go otherwise.
    fn sync_greys(&mut self, ctx: &egui::Context, greys: Option<&Arc<Greys>>) {
        let Some(greys) = greys else {
            self.greys = None;
            return;
        };
        if (self.greys.as_ref()).is_some_and(|(held, _)| Arc::ptr_eq(held, greys)) {
            return;
        }
        let texture = |name: &str, image: &PictureImage| {
            picture_texture(ctx, name, (image.size, &image.pixels, Alpha::Straight))
        };
        let textures = [
            (greys.original.as_deref()).map(|image| texture("key-grey-original", image)),
            Some(texture("key-grey-copy", &greys.copy)),
        ];
        self.greys = Some((Arc::clone(greys), textures));
    }

    /// `item`'s texture: `None` while not read yet, `Some(None)` when it could not be.
    pub(crate) fn texture(&self, item: &ItemRef) -> Option<Option<&TextureHandle>> {
        let (_, texture) = self.shown.iter().find(|(shown, _)| shown == item)?;
        Some(texture.as_ref())
    }

    /// `item`'s texture with its ground faded, once made.
    pub(crate) fn grounded(&self, item: &ItemRef) -> Option<&TextureHandle> {
        let (_, texture) = self.grounded.iter().find(|(shown, _)| shown == item)?;
        Some(texture)
    }

    /// Exercise `index`'s picture the practice crate drew, if it has one.
    pub(crate) fn painted(
        &self,
        practice: &PracticeProjection,
        index: usize,
    ) -> Option<&TextureHandle> {
        let image = practice.sheet.painted.get(index)?.as_ref()?;
        (self.painted.iter())
            .find(|(shown, _)| Arc::ptr_eq(shown, image))
            .map(|(_, texture)| texture)
    }

    /// A key shift's frame in grey: the original's (`0`) or the copy's (`1`).
    pub(crate) fn grey(&self, frame: usize) -> Option<&TextureHandle> {
        self.greys.as_ref()?.1.get(frame)?.as_ref()
    }
}

/// The open practice sheet through `mapping` on a screen of `pixels_per_point`, its pictures
/// from `pictures`.
pub(crate) fn show_marks(
    ctx: &egui::Context,
    practice: &PracticeProjection,
    (mapping, pixels_per_point): (&ViewMapping, f32),
    pictures: &mut SheetPictures,
) {
    pictures.sync(ctx, practice);
    let layer = egui::LayerId::new(Order::Background, egui::Id::new("practice sheet"));
    let painter = ctx.layer_painter(layer);
    let screen = ctx.content_rect();
    let at = |point: [f64; 2]| canvas_point(mapping, pixels_per_point, point.map(|v| v as f32));
    // Points per canvas pixel; an original's line width and the overlay's beside it.
    let scale = mapping.scale() / pixels_per_point;
    let widths = Widths {
        line: (LINE_WIDTH * scale).max(LEAST_LINE),
        overlay: (INK_DIAMETER * scale).max(LEAST_OVERLAY),
        scale,
    };
    let sheet = &practice.sheet;
    let marks = At::new(&at, sheet.size);
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
        for mark in sheet.givens.get(index).into_iter().flatten() {
            draw(&painter, (practice, index), mark, (marks, widths), pictures);
        }
        // A kind built from given parts has the one frame drawn in (S97).
        if !sheet.families[index].constructed() {
            painter.add(Shape::closed_line(original.to_vec(), frame));
        }
        painter.add(Shape::closed_line(copy.to_vec(), frame));
    }
    if let (Some(overlay), Some(index)) = (&practice.overlay, practice.current) {
        for mark in overlay {
            draw(&painter, (practice, index), mark, (marks, widths), pictures);
        }
    }
    show_probe(&painter, practice, &at);
}

/// The point the eyedropper compared, ringed on both frames of the shown exercise.
fn show_probe(
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
        painter.circle_stroke(centre, PROBE, Stroke::new(PROBE_HALO, PANEL));
        painter.circle_stroke(centre, PROBE, Stroke::new(PROBE_LINE, INK));
    }
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

pub(crate) fn colour([r, g, b]: Rgb8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// A value's grey (S96).
pub(crate) fn value_colour(value: openpaint_practice::Value) -> Color32 {
    colour(value.grey())
}

/// A division's level's colour, as its pen draws it (S95).
pub(crate) fn level_colour(level: openpaint_practice::Level) -> Color32 {
    colour(level.colour())
}

/// The exercise's pack picture filling its frame of its shape (S99; a single shape's with its
/// ground faded), or, where no pack kept holds it now, that it was removed (the strokes copied
/// from it stay); its credit under the frame.
pub(crate) fn pictured(
    (painter, at): (&egui::Painter, At<'_>),
    (practice, index): (&PracticeProjection, usize),
    (frame, scale): (&Frame, f32),
    pictures: &SheetPictures,
) {
    let at = |p: [f64; 2]| at.of(p);
    let picture = practice.sheet.pictures.get(index).and_then(Option::as_ref);
    let grounded = picture.and_then(|p| pictures.grounded(&p.item));
    let texture = picture.and_then(|p| pictures.texture(&p.item));
    // In step with the frame: a colour copy's frame is four times a session's.
    let grown = (frame.unit() / FRAME) as f32;
    let font = FontId::proportional((14.0 * scale * grown).max(6.0));
    match (picture, texture) {
        (Some(picture), Some(Some(texture))) => {
            let corners = picture.fit.corners().map(|p| at(frame.to_canvas(p)));
            painter.add(quad(grounded.unwrap_or(texture), corners, Color32::WHITE));
        }
        (Some(_), None) => {}
        _ => {
            let missing = Words::from(t::PICTURE_REMOVED).shown(painter.ctx());
            let centre = Align2::CENTER_CENTER;
            painter.text(at(frame.middle()), centre, missing, font, INK_SECONDARY);
        }
    }
    let Some(credit) = picture.and_then(|p| p.credit.as_ref()) else {
        return;
    };
    let [_, _, right, left] = frame.corners().map(at);
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
