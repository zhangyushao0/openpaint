//! Contact sheets: pictures in a grid of numbered cells on grey, so every candidate and every
//! item of a pack is looked at before it goes in (a white studio background shows as a white
//! box, a transparent one as the grey), each pack item's outline drawn over its picture: red
//! where it goes into the pack, blue where the list says the item has none (the outline looked at
//! and judged wrong). A composition's value masses (S96) take the cell after its picture, both
//! numbered alike: the answer in its three greys, a cell to a cell of the answer, its mass edges
//! over the picture and over the greys, red where they go into the pack, blue where the list says
//! they do not read. A crop (S101) likewise beside its colour areas, each area filled with its
//! colour, the edges of its weak contrasts over both, red where the crop goes into the pack, blue
//! where the list leaves it out. The numbers are drawn in a 3 x 5 pixel font; the command prints what each
//! number is.

use std::path::Path;

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};
use openpaint_practice::{Areas, Outline, Point, ValueMasses};
use openpaint_sheet::{CELL, draw_in, fill, fitted, number, thumbnail, write_rgba};

use crate::error::PackError;

/// The gap between cells and the cells a row.
const GAP: u32 = 14;
const COLUMNS: u32 = 6;
const BACKGROUND: Rgba<u8> = Rgba([150, 150, 150, 255]);
const CELL_GROUND: Rgba<u8> = Rgba([205, 205, 205, 255]);
const ANSWER_GROUND: Rgba<u8> = Rgba([255, 255, 255, 255]);

/// A picture for a cell: 8-bit sRGB RGBA of `size`, with an outline to draw over it, or value
/// masses to show beside it.
pub struct Picture {
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
    pub outline: Option<(Outline, Mark)>,
    pub masses: Option<(ValueMasses, Mark)>,
    /// A crop's colour areas (S101): beside it, each area filled with its colour; the edges of
    /// its weak contrasts over both.
    pub areas: Option<(Areas, Mark)>,
}

/// What an outline drawn on a sheet is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// It goes into the pack: red.
    Kept,
    /// Derived, but the list says the item has none: blue.
    Dropped,
}

/// An outline's width on a sheet, in sheet pixels.
const OUTLINE_WIDTH: f64 = 2.0;

/// Writes `pictures` as a contact sheet PNG at `path`, numbered from `first` in order (a picture
/// with value masses or colour areas taking two cells of its number).
///
/// # Errors
/// `Io` or `Png` when the file cannot be written.
pub fn write_sheet(pictures: &[Picture], first: usize, path: &Path) -> Result<(), PackError> {
    let cells: Vec<(usize, &Picture, bool)> = (pictures.iter().enumerate())
        .flat_map(|(index, picture)| {
            let paired = picture.masses.is_some() || picture.areas.is_some();
            let answer = paired.then_some((first + index, picture, true));
            std::iter::once((first + index, picture, false)).chain(answer)
        })
        .collect();
    let count = u32::try_from(cells.len()).unwrap_or(u32::MAX).max(1);
    let rows = count.div_ceil(COLUMNS);
    let width = COLUMNS * (CELL + GAP) + GAP;
    let height = rows * (CELL + GAP) + GAP;
    let mut sheet = RgbaImage::from_pixel(width, height, BACKGROUND);
    for (index, (number_of, picture, answer)) in cells.into_iter().enumerate() {
        let index = index as u32;
        let x = GAP + (index % COLUMNS) * (CELL + GAP);
        let y = GAP + (index / COLUMNS) * (CELL + GAP);
        // An answer's light grey reads against white, not the cells' grey.
        let ground = if answer { ANSWER_GROUND } else { CELL_GROUND };
        fill(&mut sheet, [x, y], [CELL, CELL], ground);
        let shown = match (&picture.masses, &picture.areas, answer) {
            (Some((masses, _)), _, true) => masses_picture(masses, picture.size),
            (_, Some((areas, _)), true) => areas_picture(areas, picture.size),
            _ => thumbnail(picture.size, &picture.rgba, FilterType::Triangle),
        };
        if let Some(thumb) = shown {
            let left = x + (CELL - thumb.width()) / 2;
            let top = y + (CELL - thumb.height()) / 2;
            imageops::overlay(&mut sheet, &thumb, i64::from(left), i64::from(top));
            let at = [f64::from(left), f64::from(top)];
            if let Some((outline, mark)) = &picture.outline {
                let scale = f64::from(thumb.width()) / f64::from(picture.size[0]);
                let place = |p: Point| [at[0] + p[0] * scale, at[1] + p[1] * scale];
                draw_lines(&mut sheet, &outline.lines(), &place, *mark);
            }
            let size = [f64::from(thumb.width()), f64::from(thumb.height())];
            let place = |p: Point| [at[0] + p[0] * size[0], at[1] + p[1] * size[1]];
            if let Some((masses, mark)) = &picture.masses {
                draw_lines(&mut sheet, &masses.edges(), &place, *mark);
            }
            if let Some((areas, mark)) = &picture.areas {
                let weak =
                    (areas.weak_pairs().into_iter()).flat_map(|(a, b)| areas.edges(a, Some(b)));
                draw_lines(&mut sheet, &weak.collect::<Vec<_>>(), &place, *mark);
            }
        }
        number(&mut sheet, [x, y], number_of);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(write_rgba(path, [width, height], sheet.as_raw())?)
}

/// `masses` in their greys, a cell to each of theirs, as large as their picture of `size` shows.
fn masses_picture(masses: &ValueMasses, size: [u32; 2]) -> Option<RgbaImage> {
    let greys = RgbaImage::from_raw(masses.size[0], masses.size[1], masses.picture())?;
    let [width, height] = fitted(size);
    Some(imageops::resize(&greys, width, height, FilterType::Nearest))
}

/// `areas` each filled with its colour, a cell to each of theirs, as large as their picture of
/// `size` shows.
fn areas_picture(areas: &Areas, size: [u32; 2]) -> Option<RgbaImage> {
    let [w, h] = areas.size.map(|side| u32::try_from(side).unwrap_or(1));
    let pixels: Vec<u8> = (areas.map().iter())
        .flat_map(|area| {
            let [r, g, b] = areas.colours[usize::from(*area)];
            [r, g, b, 255]
        })
        .collect();
    let flat = RgbaImage::from_raw(w, h, pixels)?;
    let [width, height] = fitted(size);
    Some(imageops::resize(&flat, width, height, FilterType::Nearest))
}

/// `lines` (points `place` puts on the sheet) as a mark's colour, `OUTLINE_WIDTH` wide.
fn draw_lines(
    sheet: &mut RgbaImage,
    lines: &[Vec<Point>],
    place: &dyn Fn(Point) -> [f64; 2],
    mark: Mark,
) {
    let colour = match mark {
        Mark::Kept => Rgba([225, 25, 25, 255]),
        Mark::Dropped => Rgba([30, 90, 235, 255]),
    };
    draw_in(sheet, lines, place, (colour, OUTLINE_WIDTH));
}
