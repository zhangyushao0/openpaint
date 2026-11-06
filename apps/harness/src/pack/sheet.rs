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

use crate::error::HarnessError;
use crate::output::write_rgba;

/// A cell's side, the gap between cells and the cells a row.
const CELL: u32 = 480;
const GAP: u32 = 14;
const COLUMNS: u32 = 6;
/// A digit's pixel, in sheet pixels.
const DOT: u32 = 4;
const BACKGROUND: Rgba<u8> = Rgba([150, 150, 150, 255]);
const CELL_GROUND: Rgba<u8> = Rgba([205, 205, 205, 255]);
const ANSWER_GROUND: Rgba<u8> = Rgba([255, 255, 255, 255]);

/// The digits 0 to 9, 3 x 5 dots each, rows top first, three bits a row (left the highest).
const DIGITS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];

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
pub fn write_sheet(pictures: &[Picture], first: usize, path: &Path) -> Result<(), HarnessError> {
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
    write_rgba(path, [width, height], sheet.as_raw())
}

/// A picture of `size` as large as a cell shows it.
fn fitted([width, height]: [u32; 2]) -> [u32; 2] {
    let scale = (f64::from(CELL) / f64::from(width.max(height).max(1))).min(1.0);
    [width, height].map(|length| ((f64::from(length) * scale).round() as u32).max(1))
}

/// The picture `rgba` of `size` fitted inside a cell, resized by `filter`.
fn thumbnail(size: [u32; 2], rgba: &[u8], filter: FilterType) -> Option<RgbaImage> {
    let image = RgbaImage::from_raw(size[0], size[1], rgba.to_vec())?;
    let [width, height] = fitted(size);
    Some(imageops::resize(&image, width, height, filter))
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
    for line in lines {
        for pair in line.windows(2) {
            let [a, b] = [place(pair[0]), place(pair[1])];
            let steps = ((b[0] - a[0]).hypot(b[1] - a[1]) * 2.0).ceil().max(1.0) as usize;
            for step in 0..=steps {
                let t = step as f64 / steps as f64;
                let [cx, cy] = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
                let half = OUTLINE_WIDTH / 2.0;
                let corner = |v: f64| (v - half).round().max(0.0) as u32;
                let side = OUTLINE_WIDTH as u32;
                fill(sheet, [corner(cx), corner(cy)], [side, side], colour);
            }
        }
    }
}

fn fill(sheet: &mut RgbaImage, [x, y]: [u32; 2], [width, height]: [u32; 2], colour: Rgba<u8>) {
    for row in y..(y + height).min(sheet.height()) {
        for column in x..(x + width).min(sheet.width()) {
            sheet.put_pixel(column, row, colour);
        }
    }
}

/// `value` in black on a white tab at the cell's top left corner.
fn number(sheet: &mut RgbaImage, [x, y]: [u32; 2], value: usize) {
    let digits: Vec<usize> = value
        .to_string()
        .bytes()
        .map(|b| usize::from(b - b'0'))
        .collect();
    let width = digits.len() as u32 * 4 * DOT + DOT;
    fill(sheet, [x, y], [width, 7 * DOT], Rgba([255, 255, 255, 255]));
    for (place, digit) in digits.into_iter().enumerate() {
        for (row, bits) in DIGITS[digit].iter().enumerate() {
            for column in 0..3 {
                if bits & (4 >> column) != 0 {
                    let left = x + DOT + (place as u32 * 4 + column) * DOT;
                    let top = y + DOT + row as u32 * DOT;
                    fill(sheet, [left, top], [DOT, DOT], Rgba([0, 0, 0, 255]));
                }
            }
        }
    }
}
