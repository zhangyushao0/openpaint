//! A key shift's marks over a practice sheet (S107, `practice_marks.rs`), drawn by the screen
//! through the view: its tendency printed beside the copy's frame as a given, a disc of its colour
//! with its name small and grey under it; compared, nothing laid over the copy (a key shift has no
//! answer), only what is off: the areas whose lightness left the crop's or that did not move
//! toward the tendency outlined red on the copy, the edge where two areas apart merged red, the
//! area the card shows outlined in ink on both frames. In the Grey view both frames show in grey
//! (Oklab lightness) and only the lightness failures are outlined.

use std::fmt;
use std::sync::Arc;

use egui::{Color32, FontId, Pos2, Shape, Stroke, TextureHandle};
use openpaint_app::{KeyRead, Overlay, PracticeProjection};
use openpaint_practice::{Frame, Pair, Point};

use crate::components::{Alpha, picture_texture};
use crate::design::{CORRECTION, INK, INK_SECONDARY, PANEL};
use crate::practice_marks::{colour, quad};
use crate::text::{self as t, Words};

/// The tendency's disc beside the copy's frame: its radius and its gap from the frame, its name's
/// size and gap under it, canvas pixels of a session's frame.
const DISC: f32 = 22.0;
const DISC_GAP: f32 = 18.0;
const NAME_SIZE: f32 = 11.0;
const NAME_GAP: f32 = 6.0;
/// An outline's width and the paper's halo under it, in points.
const OUTLINE: f32 = 2.0;
const HALO: f32 = 4.0;

/// A compared key shift's frames in grey as textures, made once a reading.
#[derive(Default)]
pub(crate) struct GreyPictures {
    read: Option<Arc<KeyRead>>,
    textures: [Option<TextureHandle>; 2],
}

impl fmt::Debug for GreyPictures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "grey pictures: {}", self.read.is_some())
    }
}

impl GreyPictures {
    /// Uploads the grey frames of `overlay`'s reading once it shows in grey; lets go otherwise.
    pub(crate) fn sync(&mut self, ctx: &egui::Context, overlay: Option<&Overlay>) {
        let Some(Overlay::KeyShift {
            read: Some(read),
            grey: true,
            ..
        }) = overlay
        else {
            *self = Self::default();
            return;
        };
        if self
            .read
            .as_ref()
            .is_some_and(|held| Arc::ptr_eq(held, read))
        {
            return;
        }
        let texture = |name: &str, image: &openpaint_app::PictureImage| {
            picture_texture(ctx, name, (image.size, &image.pixels, Alpha::Straight))
        };
        self.textures = [
            (read.grey_original.as_deref()).map(|image| texture("key-grey-original", image)),
            Some(texture("key-grey-copy", &read.grey_copy)),
        ];
        self.read = Some(Arc::clone(read));
    }
}

/// Exercise `index`'s tendency beside its copy's frame, if it is a key shift; `at` maps canvas
/// points to the screen, `scale` is points per canvas pixel.
pub(crate) fn show_tendency(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    (index, pair): (usize, &Pair),
    (at, scale): (&dyn Fn([f64; 2]) -> Pos2, f32),
) {
    let Some(Some(tendency)) = practice.sheet.tendencies.get(index) else {
        return;
    };
    let frame = pair.copy;
    let x = frame.origin[0] + frame.size[0] + f64::from(DISC_GAP + DISC);
    let y = frame.origin[1] + f64::from(DISC);
    let centre = at([x, y]);
    let radius = DISC * scale;
    painter.circle(
        centre,
        radius,
        colour(tendency.colour()),
        Stroke::new(1.0, INK),
    );
    let below = at([x, y + f64::from(DISC + NAME_GAP)]);
    let name = Words::from(t::TENDENCY).shown(painter.ctx());
    let font = FontId::proportional((NAME_SIZE * scale).max(6.0));
    painter.text(below, egui::Align2::CENTER_TOP, name, font, INK_SECONDARY);
}

/// A compared key shift's marks over its frames (in grey in the Grey view, `greys`' textures), a
/// red mark at least `width` points wide.
pub(crate) fn show_overlay(
    painter: &egui::Painter,
    practice: &PracticeProjection,
    overlay: &Overlay,
    (at, width): (&dyn Fn([f64; 2]) -> Pos2, f32),
    greys: &GreyPictures,
) {
    let (
        Some(index),
        Overlay::KeyShift {
            read,
            outline,
            grey,
            ..
        },
    ) = (practice.current, overlay)
    else {
        return;
    };
    let pair = practice.sheet.pairs[index];
    let red = width.max(OUTLINE);
    let Some(read) = read else {
        return;
    };
    if *grey {
        let picture = practice.sheet.pictures.get(index).and_then(Option::as_ref);
        if let (Some(picture), Some(texture)) = (picture, &greys.textures[0]) {
            let corners = picture
                .fit
                .corners()
                .map(|p| at(pair.original.to_canvas(p)));
            painter.add(quad(texture, corners, Color32::WHITE));
        }
        if let Some(texture) = &greys.textures[1] {
            painter.add(quad(texture, pair.copy.corners().map(at), Color32::WHITE));
        }
        for area in &read.check.off_lightness {
            let lines = read.outline(*area).unwrap_or_default();
            lined(painter, (&pair.copy, lines), at, (red, CORRECTION));
        }
        return;
    }
    for frame in [pair.original, pair.copy] {
        lined(painter, (&frame, outline), at, (OUTLINE, INK));
    }
    for (_, lines) in &read.outlines {
        lined(painter, (&pair.copy, lines), at, (red, CORRECTION));
    }
    lined(painter, (&pair.copy, &read.edges), at, (red, CORRECTION));
}

/// `lines` (frame units) of `frame`, `width` points wide in `ink` over the paper's halo.
fn lined(
    painter: &egui::Painter,
    (frame, lines): (&Frame, &[Vec<Point>]),
    at: &dyn Fn([f64; 2]) -> Pos2,
    (width, ink): (f32, Color32),
) {
    let path =
        |line: &Vec<Point>| -> Vec<Pos2> { line.iter().map(|p| at(frame.to_canvas(*p))).collect() };
    for stroke in [Stroke::new(width + HALO, PANEL), Stroke::new(width, ink)] {
        for line in lines {
            painter.add(Shape::line(path(line), stroke));
        }
    }
}
