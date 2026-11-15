//! The pictures abstract analysis's marks lie over (S112, D122): a layer of the sheet of its own,
//! "Picture", made the first time at the bottom (under the sheet's first layer, which holds every
//! exercise's strokes) and kept locked, so a pen, a fill or the eraser never reach it and it is
//! never the layer drawn on. Each exercise whose kind lays its picture under (`Facts::under`)
//! places its pack picture there as one transform edit, the picture as read for the screen
//! reduced to `LAID_SIDE` on its longer side and placed into its frame, lifting only that frame's
//! rectangle, so a second picture never clears the first. The session amends it into the version
//! shown: never an undo step. The k-th edit is the sheet's k-th such exercise; one whose picture
//! could not be read keeps its place with a clear patch.

use std::sync::Arc;

use openpaint_core::{
    Artwork, CanvasSize, CoreError, LayerCommand, LayerId, LayerName, Patch, Placement, RegionMask,
};

use super::{command, named};
use crate::pictures::PictureImage;

/// The picture layer's name, as the app gives it (stored in English, shown in the language).
pub const PICTURE_LAYER: &str = "Picture";
/// The longest side a picture is laid at under the marks, in pixels: twice a lesson's frame, the
/// 2x view tier's resolution.
pub const LAID_SIDE: u32 = 800;
/// How much larger than a clear patch the place it keeps is.
const CLEAR_SCALE: f32 = 64.0;

/// A picture to lay under an exercise's marks: as read for the screen (none: it could not be
/// read, its place kept), and the rectangle it fills on the canvas: its top left and its size,
/// canvas pixels.
#[derive(Debug, Clone)]
pub(crate) struct Under {
    pub(crate) image: Option<Arc<PictureImage>>,
    pub(crate) place: [[f32; 2]; 2],
}

/// How many pictures `artwork`'s picture layer holds.
pub(crate) fn pictures_laid(artwork: &Artwork) -> usize {
    let layer = named(artwork, PICTURE_LAYER);
    (artwork.layers().iter())
        .find(|held| Some(held.id()) == layer)
        .map_or(0, |held| held.edits().len())
}

/// `artwork` with `under` laid on its picture layer in order, the layer drawn on before drawn on
/// again.
///
/// # Errors
/// `LayerLimit` when the sheet has no room for the layer; `InvalidValue` for a place a picture
/// cannot be placed into.
pub(crate) fn laid_under(artwork: &Artwork, under: &[Under]) -> Result<Artwork, CoreError> {
    if under.is_empty() {
        return Ok(artwork.clone());
    }
    let active = artwork.active();
    let (mut artwork, layer) = picture_layer(artwork)?;
    for picture in under {
        let locked = |locked| LayerCommand::SetLocked { layer, locked };
        artwork = command(&artwork, locked(false))?;
        artwork = command(&artwork, LayerCommand::Select(layer))?;
        let (patch, placement) = patched(artwork.canvas(), picture)?;
        let [[x, y], [w, h]] = picture.place;
        let corners = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        let lift =
            RegionMask::polygon(artwork.canvas(), &corners).ok_or(CoreError::InvalidValue {
                what: "a picture's place",
            })?;
        artwork = artwork
            .transform_region(Some(lift), patch, placement)?
            .artwork;
        artwork = command(&artwork, locked(true))?;
    }
    command(&artwork, LayerCommand::Select(active))
}

/// `artwork` with its picture layer, made locked at the bottom where it has none, and the layer.
fn picture_layer(artwork: &Artwork) -> Result<(Artwork, LayerId), CoreError> {
    if let Some(layer) = named(artwork, PICTURE_LAYER) {
        return Ok((artwork.clone(), layer));
    }
    let bottom = (artwork.layers().first())
        .map(openpaint_core::Layer::id)
        .ok_or(CoreError::InvalidValue {
            what: "a sheet's layers",
        })?;
    let artwork = command(artwork, LayerCommand::Select(bottom))?;
    let artwork = command(&artwork, LayerCommand::Create)?;
    let layer = artwork.active();
    let name = LayerName::new(PICTURE_LAYER)?;
    let artwork = command(&artwork, LayerCommand::Rename { layer, name })?;
    let artwork = command(&artwork, LayerCommand::Move { layer, by: -1 })?;
    let locked = LayerCommand::SetLocked {
        layer,
        locked: true,
    };
    Ok((command(&artwork, locked)?, layer))
}

/// `under`'s pixels as a patch at the canvas's corner and the placement that lays them into its
/// place; a clear patch where its picture could not be read.
fn patched(canvas: CanvasSize, under: &Under) -> Result<(Patch, Placement), CoreError> {
    let [_, [w, h]] = under.place;
    let (size, pixels) = match &under.image {
        Some(image) => reduced(image, [canvas.width(), canvas.height()]),
        None => {
            let side = |length: f32| ((length / CLEAR_SCALE).ceil() as u32).max(1);
            let size = [side(w), side(h)];
            (size, vec![0; size[0] as usize * size[1] as usize * 4])
        }
    };
    let patch = Patch::new(canvas, [0, 0], size, pixels)?;
    let [x, y] = under.place[0];
    let scale = [w / size[0] as f32, h / size[1] as f32];
    let placement = Placement::new([scale[0], 0.0, 0.0, scale[1]], [x, y]);
    let placement = placement.ok_or(CoreError::InvalidValue {
        what: "a picture's place",
    })?;
    Ok((patch, placement))
}

/// `image` premultiplied, as a patch holds its pixels, its longer side reduced to `LAID_SIDE`
/// where longer (and within a canvas of `room`), each pixel the mean of those it covers.
fn reduced(image: &PictureImage, room: [u32; 2]) -> ([u32; 2], Vec<u8>) {
    let [width, height] = image.size.map(|side| side.max(1));
    let shares = [
        f64::from(LAID_SIDE) / f64::from(width.max(height)),
        f64::from(room[0]) / f64::from(width),
        f64::from(room[1]) / f64::from(height),
        1.0,
    ];
    let share = shares.into_iter().fold(f64::INFINITY, f64::min);
    let side = |length: u32| ((f64::from(length) * share).floor() as u32).max(1);
    let size = [side(width), side(height)];
    let span = |cell: u32, cells: u32, length: u32| {
        let from = (u64::from(cell) * u64::from(length) / u64::from(cells)) as usize;
        let to = (u64::from(cell + 1) * u64::from(length) / u64::from(cells)) as usize;
        from..to.max(from + 1).min(length as usize)
    };
    let mut out = Vec::with_capacity(size[0] as usize * size[1] as usize * 4);
    for cy in 0..size[1] {
        let rows = span(cy, size[1], height);
        for cx in 0..size[0] {
            let columns = span(cx, size[0], width);
            let (mut sum, mut count) = ([0_u64; 4], 0_u64);
            for y in rows.clone() {
                for x in columns.clone() {
                    let at = 4 * (y * width as usize + x);
                    let Some(pixel) = image.pixels.get(at..at + 4) else {
                        continue;
                    };
                    let alpha = u64::from(pixel[3]);
                    for (total, channel) in sum.iter_mut().zip(&pixel[..3]) {
                        *total += u64::from(*channel) * alpha;
                    }
                    sum[3] += alpha;
                    count += 1;
                }
            }
            let count = count.max(1);
            let colour = |total: u64| ((total + 255 * count / 2) / (255 * count)).min(255) as u8;
            let alpha = ((sum[3] + count / 2) / count).min(255) as u8;
            out.extend([colour(sum[0]), colour(sum[1]), colour(sum[2]), alpha]);
        }
    }
    (size, out)
}
