//! Contact sheets: pictures in a grid of numbered cells on grey, so every candidate and every
//! item of a pack is looked at before it goes in (a white studio background shows as a white
//! box, a transparent one as the grey). The numbers are drawn in a 3 x 5 pixel font; the command
//! prints what each number is.

use std::path::Path;

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};

use crate::error::HarnessError;
use crate::output::write_rgba;

/// A cell's side, the gap between cells and the cells a row.
const CELL: u32 = 300;
const GAP: u32 = 14;
const COLUMNS: u32 = 6;
/// A digit's pixel, in sheet pixels.
const DOT: u32 = 4;
const BACKGROUND: Rgba<u8> = Rgba([150, 150, 150, 255]);
const CELL_GROUND: Rgba<u8> = Rgba([205, 205, 205, 255]);

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

/// A picture for a cell: 8-bit sRGB RGBA of `size`.
pub struct Picture {
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
}

/// Writes `pictures` as a contact sheet PNG at `path`, numbered from 1 in order.
///
/// # Errors
/// `Io` or `Png` when the file cannot be written.
pub fn write_sheet(pictures: &[Picture], path: &Path) -> Result<(), HarnessError> {
    let count = u32::try_from(pictures.len()).unwrap_or(u32::MAX).max(1);
    let rows = count.div_ceil(COLUMNS);
    let width = COLUMNS * (CELL + GAP) + GAP;
    let height = rows * (CELL + GAP) + GAP;
    let mut sheet = RgbaImage::from_pixel(width, height, BACKGROUND);
    for (index, picture) in pictures.iter().enumerate() {
        let index = index as u32;
        let x = GAP + (index % COLUMNS) * (CELL + GAP);
        let y = GAP + (index / COLUMNS) * (CELL + GAP);
        fill(&mut sheet, [x, y], [CELL, CELL], CELL_GROUND);
        if let Some(thumb) = thumbnail(picture) {
            let left = x + (CELL - thumb.width()) / 2;
            let top = y + (CELL - thumb.height()) / 2;
            imageops::overlay(&mut sheet, &thumb, i64::from(left), i64::from(top));
        }
        number(&mut sheet, [x, y], index as usize + 1);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    write_rgba(path, [width, height], sheet.as_raw())
}

/// `picture` fitted inside a cell.
fn thumbnail(picture: &Picture) -> Option<RgbaImage> {
    let [width, height] = picture.size;
    let image = RgbaImage::from_raw(width, height, picture.rgba.clone())?;
    let scale = (f64::from(CELL) / f64::from(width.max(height))).min(1.0);
    let side = |length: u32| ((f64::from(length) * scale).round() as u32).max(1);
    Some(imageops::resize(
        &image,
        side(width),
        side(height),
        FilterType::Triangle,
    ))
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
